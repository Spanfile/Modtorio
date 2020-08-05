pub mod cache;
mod store_meta;

use crate::{config, ext::PathExt, util};
pub use cache::Cache;
use log::*;
use rusqlite::{Connection, OptionalExtension};
use std::{
    path::{Path, PathBuf},
    sync::{Arc, Mutex},
};
use store_meta::{Field, Value};
use tokio::task;

/// The default store database schema string.
const SCHEMA: &str = include_str!(concat!(env!("CARGO_MANIFEST_DIR"), "/schema.sql"));

pub struct Store {
    conn: Arc<Mutex<Connection>>,
    pub cache: Cache,
}

pub struct Builder {
    store_path: PathBuf,
    schema: String,
}

impl Builder {
    /// Returns a new Builder with each field filled with its default value.
    pub fn new() -> Self {
        Self {
            store_path: PathBuf::from(config::DEFAULT_STORE_FILE_LOCATION),
            schema: String::from(SCHEMA),
        }
    }

    /// Specify a custom filesystem path used to store the database file.
    #[allow(dead_code)]
    pub fn with_db_path<P>(self, path: P) -> Self
    where
        P: AsRef<Path>,
    {
        Self {
            store_path: PathBuf::from(path.as_ref()),
            ..self
        }
    }

    /// Specify a custom schema used to build the database.
    #[allow(dead_code)]
    pub fn with_schema(self, schema: String) -> Self {
        Self { schema, ..self }
    }

    /// Finalise the builder and return the built store object.
    ///
    /// During building the schema's checksum will be calculated and, if the store database already
    /// exists in the filesystem, compared against the existing stored checksum. If there's a
    /// mismatch, the schema will be applied over the existing database, deleting all data
    /// inside it. The new schema checksum will be then stored in the store [metadata].
    ///
    /// [metadata]: store_meta
    pub async fn build(self) -> anyhow::Result<Store> {
        let encoded_checksum = util::checksum::blake2b_string(&self.schema);
        trace!("Cache database schema checksum: {}", encoded_checksum);

        let db_exists = self.store_path.exists();
        let conn = Connection::open(self.store_path.get_str()?)?;
        let conn = Arc::new(Mutex::new(conn));

        let cache = Cache {
            conn: Arc::clone(&conn),
        };
        let store = Store { conn, cache };

        debug!("Cache database exists: {}", db_exists);

        let checksums_match = db_exists && checksum_matches_meta(&store, &encoded_checksum).await?;
        debug!("Schema checksums match: {}", checksums_match);

        if !db_exists || !checksums_match {
            debug!("Applying database schema...");
            trace!("{}", self.schema);

            store.apply_schema(self.schema).await?;
            store
                .set_meta(Value {
                    field: Field::SchemaChecksum,
                    value: Some(encoded_checksum),
                })
                .await?;
        }

        Ok(store)
    }
}

/// Compares a given store schema checksum string to what a given store's metadata possibly
/// contains. Returns a `Result<bool>` corresponding to whether the store's existing schema checksum
/// matches the wanted one. Returns `Ok(false)` if the store doesn't contain the [schema checksum
/// field][Field]. Returns an error if reading the database meta table fails.
///
/// [Field]: store_meta::Field#variant.SchemaChecksum
async fn checksum_matches_meta(store: &Store, wanted_checksum: &str) -> anyhow::Result<bool> {
    if let Some(metavalue) = store.get_meta(Field::SchemaChecksum).await? {
        if let Some(existing_checksum) = metavalue.value {
            trace!("Got existing schema checksum: {}", existing_checksum);
            return Ok(wanted_checksum == existing_checksum);
        }
    }

    Ok(false)
}

/// Accepts a reference to an `Arc<Mutex<Connection>>` and a block where that reference can be used
/// to access the database connection. The block will run a blocking thread with
/// `task::spawn_blocking`. Returns what the given block returns.
///
/// ```no_run
/// let conn = &self.conn;
/// sql!(conn => {
///     // use conn
/// })
/// ```
#[macro_export]
macro_rules! sql {
    ($conn:ident => $b:block) => {
        Ok({
            let _c = Arc::clone(&$conn);
            task::spawn_blocking(move || -> anyhow::Result<_> {
                let $conn = _c.lock().unwrap();
                $b
            })
            .await??
        })
    };
}

impl Store {
    /// Applies a given schema to the database.
    async fn apply_schema(&self, schema: String) -> anyhow::Result<()> {
        let conn = Arc::clone(&self.conn);
        let result = task::spawn_blocking(move || -> anyhow::Result<()> {
            conn.lock()
                .unwrap()
                .execute_batch(&format!("BEGIN TRANSACTION; {} COMMIT;", schema))?;
            Ok(())
        })
        .await?;

        Ok(result?)
    }

    /// Begins a new transaction in the database with `BEGIN TRANSACTION;`.
    pub fn begin_transaction(&self) -> anyhow::Result<()> {
        debug!("Beginning new cache transaction");
        Ok(self
            .conn
            .lock()
            .unwrap()
            .execute_batch("BEGIN TRANSACTION")?)
    }

    /// Commits an ongoing transaction in the database with `COMMIT`;
    pub fn commit_transaction(&self) -> anyhow::Result<()> {
        debug!("Committing cache transaction");
        Ok(self.conn.lock().unwrap().execute_batch("COMMIT")?)
    }

    /// Retrieves an optional meta value from the meta table with a given meta field.
    pub async fn get_meta(&self, field: Field) -> anyhow::Result<Option<Value>> {
        let conn = &self.conn;
        sql!(conn => {
            let mut stmt = conn.prepare(Value::select())?;

            Ok(stmt
                .query_row_named(&Value::select_params(&field), |row| {
                    Ok(row.into())
                })
                .optional()?)
        })
    }

    /// Stores a meta value to the meta table.
    pub async fn set_meta(&self, value: Value) -> anyhow::Result<()> {
        let conn = &self.conn;
        sql!(conn => {
            conn.execute_named(Value::replace_into(), &value.all_params())?;
            Ok(())
        })
    }
}
