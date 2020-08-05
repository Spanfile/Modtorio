pub mod cache;
pub mod option;
pub mod store_meta;

use crate::{ext::PathExt, opts::Opts, util};
pub use cache::Cache;
use log::*;
use rusqlite::{Connection, OptionalExtension};
use std::sync::{Arc, Mutex};
use tokio::task;

/// The default store database schema string.
const SCHEMA: &str = include_str!(concat!(env!("CARGO_MANIFEST_DIR"), "/schema.sql"));

pub struct Store {
    conn: Arc<Mutex<Connection>>,
    pub cache: Cache,
}

impl Store {
    pub async fn build(opts: &Opts) -> anyhow::Result<Store> {
        // TODO: since the schema is static, just calculate the checksum at build-time
        let encoded_checksum = util::checksum::blake2b_string(SCHEMA);
        trace!("Cache database schema checksum: {}", encoded_checksum);

        let db_exists = opts.store.exists();
        let conn = Connection::open(opts.store.get_str()?)?;
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
            trace!("{}", SCHEMA);

            store.apply_schema(SCHEMA).await?;
            store
                .set_meta(store_meta::Value {
                    field: store_meta::Field::SchemaChecksum,
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
    if let Some(metavalue) = store.get_meta(store_meta::Field::SchemaChecksum).await? {
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
    async fn apply_schema(&self, schema: &str) -> anyhow::Result<()> {
        let conn = Arc::clone(&self.conn);
        let schema = String::from(schema);
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
    pub async fn get_meta(
        &self,
        field: store_meta::Field,
    ) -> anyhow::Result<Option<store_meta::Value>> {
        let conn = &self.conn;
        sql!(conn => {
            let mut stmt = conn.prepare(store_meta::Value::select())?;

            Ok(stmt
                .query_row_named(&store_meta::Value::select_params(&field), |row| {
                    Ok(row.into())
                })
                .optional()?)
        })
    }

    /// Stores a meta value to the meta table.
    pub async fn set_meta(&self, value: store_meta::Value) -> anyhow::Result<()> {
        let conn = &self.conn;
        sql!(conn => {
            conn.execute_named(store_meta::Value::replace_into(), &value.all_params())?;
            Ok(())
        })
    }

    /// Retrieves an option value from the option table with a given option field.
    pub async fn get_option(&self, field: option::Field) -> anyhow::Result<Option<option::Value>> {
        let conn = &self.conn;
        sql!(conn => {
            let mut stmt = conn.prepare(option::Value::select())?;

            Ok(stmt
                .query_row_named(&option::Value::select_params(&field), |row| {
                    Ok(row.into())
                })
                .optional()?)
        })
    }

    /// Stores an option value to the options table.
    pub async fn set_option(&self, value: option::Value) -> anyhow::Result<()> {
        let conn = &self.conn;
        sql!(conn => {
            conn.execute_named(option::Value::replace_into(), &value.all_params())?;
            Ok(())
        })
    }
}
