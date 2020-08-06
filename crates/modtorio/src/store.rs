pub mod cache;
pub mod option;
pub mod store_meta;

use crate::{error::StoreError, ext::PathExt, util};
pub use cache::Cache;
use log::*;
use rusqlite::{Connection, OptionalExtension};
use std::{
    path::Path,
    sync::{Arc, Mutex},
};
use tokio::task;

include!(concat!(env!("OUT_DIR"), "/store_consts.rs"));

pub(crate) const MEMORY_STORE: &str = "_memory";
const MAX_STORE_FILE_PERMISSIONS: u32 = 0o600;

pub struct Store {
    conn: Arc<Mutex<Connection>>,
    pub cache: Cache,
}

pub struct Builder<P>
where
    P: AsRef<Path>,
{
    schema: String,
    schema_checksum: Option<String>,
    store_location: StoreLocation<P>,
    skip_storing_checksum: bool,
}

pub enum StoreLocation<P: AsRef<Path>> {
    Memory,
    File(P),
}

impl<P> Builder<P>
where
    P: AsRef<Path>,
{
    pub fn from_location(store_location: StoreLocation<P>) -> Self {
        Self {
            schema: String::from(SCHEMA),
            schema_checksum: Some(String::from(SCHEMA_CHECKSUM)),
            store_location,
            skip_storing_checksum: false,
        }
    }

    pub fn with_schema(self, schema: &str) -> Self {
        Self {
            schema: String::from(schema),
            schema_checksum: None,
            ..self
        }
    }

    pub fn skip_storing_checksum(self, skip: bool) -> Self {
        Self {
            skip_storing_checksum: skip,
            ..self
        }
    }

    pub async fn build(self) -> anyhow::Result<Store> {
        let schema_checksum = if let Some(checksum) = self.schema_checksum {
            checksum
        } else {
            trace!("Missing schema checksum, calculating");
            util::checksum::blake2b_string(&self.schema)
        };
        trace!("Cache database schema checksum: {}", schema_checksum);

        let (store_file_exists, conn) = match self.store_location {
            StoreLocation::Memory => {
                // when opening an in-memory database, it will initially be empty, i.e. it didn't
                // exist beforehand
                (false, Connection::open_in_memory()?)
            }
            StoreLocation::File(path) => (path.as_ref().exists(), open_file_connection(path)?),
        };
        let conn = Arc::new(Mutex::new(conn));

        let cache = Cache {
            conn: Arc::clone(&conn),
        };
        let store = Store { conn, cache };
        debug!("Cache database exists: {}", store_file_exists);

        let checksums_match =
            store_file_exists && checksum_matches_meta(&store, &schema_checksum).await?;
        debug!("Schema checksums match: {}", checksums_match);

        if !checksums_match {
            apply_store_schema(&store, &self.schema).await?;

            if !self.skip_storing_checksum {
                store_schema_checksum(&store, &schema_checksum).await?;
            }
        }

        Ok(store)
    }
}

fn open_file_connection<P>(path: P) -> anyhow::Result<Connection>
where
    P: AsRef<Path>,
{
    if path.as_ref().exists() {
        if util::file::ensure_permission(&path, MAX_STORE_FILE_PERMISSIONS)? {
            Ok(Connection::open(path)?)
        } else {
            Err(StoreError::InsufficientFilePermissions {
                path: String::from(path.as_ref().get_str()?),
                minimum: MAX_STORE_FILE_PERMISSIONS,
                actual: util::file::get_permissions(&path)?,
            }
            .into())
        }
    } else {
        let conn = Connection::open(&path)?;
        util::file::set_permissions(&path, MAX_STORE_FILE_PERMISSIONS)?;
        Ok(conn)
    }
}

async fn apply_store_schema(store: &Store, schema: &str) -> anyhow::Result<()> {
    trace!("Applying database schema...");
    trace!("{}", schema);

    store.apply_schema(schema).await?;
    Ok(())
}

async fn store_schema_checksum(store: &Store, checksum: &str) -> anyhow::Result<()> {
    trace!("Storing schema checksum...");

    store
        .set_meta(store_meta::Value {
            field: store_meta::Field::SchemaChecksum,
            value: Some(String::from(checksum)),
        })
        .await?;
    Ok(())
}

impl<P> From<P> for StoreLocation<P>
where
    P: AsRef<Path>,
{
    fn from(p: P) -> Self {
        if p.as_ref().get_str().expect("failed to get path as str") == MEMORY_STORE {
            StoreLocation::Memory
        } else {
            StoreLocation::File(p)
        }
    }
}

/// Compares a given store schema checksum string to what a given store's metadata possibly
/// contains. Returns a `Result<bool>` corresponding to whether the store's existing schema checksum
/// matches the wanted one. Returns `Ok(false)` if the store doesn't contain the [schema checksum
/// field][Field]. Returns an error if reading the database meta table fails.
///
/// [Field]: store_meta::Field#variant.SchemaChecksum
async fn checksum_matches_meta(store: &Store, wanted_checksum: &str) -> anyhow::Result<bool> {
    // TODO: the checksum won't match if the _meta table doesn't exist - return false instead of the
    // error
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

#[cfg(test)]
mod tests {
    use super::*;
    use crate::store;

    async fn get_test_store(schema: &str) -> Store {
        store::Builder::<String>::from_location(StoreLocation::Memory)
            .with_schema(schema)
            .skip_storing_checksum(true)
            .build()
            .await
            .expect("failed to build test store")
    }

    #[tokio::test]
    async fn set_meta() {
        const SCHEMA: &str = r#"CREATE TABLE "_meta" (
"field"	TEXT NOT NULL,
"value"	TEXT,
PRIMARY KEY("field")
);"#;
        let store = get_test_store(SCHEMA).await;

        store
            .begin_transaction()
            .expect("failed to begin transaction");
        store
            .set_meta(store_meta::Value {
                field: store_meta::Field::SchemaChecksum,
                value: Some(String::from("value")),
            })
            .await
            .expect("failed to set meta value");
        store
            .commit_transaction()
            .expect("failed to commit transaction");
    }

    #[tokio::test]
    async fn get_meta() {
        const SCHEMA: &str = r#"CREATE TABLE "_meta" (
"field"	TEXT NOT NULL,
"value"	TEXT,
PRIMARY KEY("field")
);
INSERT INTO _meta("field", "value") VALUES("SchemaChecksum", "value");"#;
        let store = get_test_store(SCHEMA).await;

        store
            .begin_transaction()
            .expect("failed to begin transaction");
        let got_value = store
            .get_meta(store_meta::Field::SchemaChecksum)
            .await
            .expect("failed to get meta value")
            .expect("store returned no value");

        assert_eq!(got_value.value, Some(String::from("value")));
    }

    #[tokio::test]
    async fn set_option() {
        const SCHEMA: &str = r#"CREATE TABLE "options" (
"field"	TEXT NOT NULL,
"value"	TEXT,
PRIMARY KEY("field")
);"#;
        let store = get_test_store(SCHEMA).await;

        store
            .begin_transaction()
            .expect("failed to begin transaction");
        store
            .set_option(option::Value {
                field: option::Field::PortalUsername,
                value: Some(String::from("value")),
            })
            .await
            .expect("failed to set meta value");
        store
            .commit_transaction()
            .expect("failed to commit transaction");
    }

    #[tokio::test]
    async fn get_option() {
        const SCHEMA: &str = r#"CREATE TABLE "options" (
"field"	TEXT NOT NULL,
"value"	TEXT,
PRIMARY KEY("field")
);
INSERT INTO options("field", "value") VALUES("PortalUsername", "value");"#;
        let store = get_test_store(SCHEMA).await;

        let got_value = store
            .get_option(option::Field::PortalUsername)
            .await
            .expect("failed to get option value")
            .expect("store returned no value");

        assert_eq!(got_value.value, Some(String::from("value")));
    }
}
