//! The program store, used to store persistent data about the program in an SQLite database.

pub mod models;
pub mod option;

use crate::{error::StoreError, factorio::GameStoreId, util, util::ext::PathExt};
use log::*;
use models::{FactorioMod, Game, GameMod, GameSettings, ModRelease, ReleaseDependency};
use rusqlite::{Connection, OptionalExtension, NO_PARAMS};
use std::{
    path::Path,
    sync::{Arc, Mutex},
};
use tokio::task;
use util::HumanVersion;

include!(concat!(env!("OUT_DIR"), "/store_consts.rs"));

/// The special value interpreted as using an in-memory SQLite database.
pub(crate) const MEMORY_STORE: &str = "_memory";
/// The maximum permissions the store database file can have (600: `r--------`)
const MAX_STORE_FILE_PERMISSIONS: u32 = 0o600;

/// Provides access to the program store and store. New instances are created with a
/// [`Builder`](Builder).
pub struct Store {
    /// The connection to the SQLite database file.
    conn: Arc<Mutex<Connection>>,
}

/// Builds new [`Store`](Store) instances.
pub struct Builder<P>
where
    P: AsRef<Path>,
{
    /// The SQL schema to use for the SQLite database.
    schema: String,
    /// An optional pre-calculated checksum for the SQL schema.
    schema_checksum: Option<String>,
    /// Location for the store database. Either a filesystem path, or in-memory.
    store_location: StoreLocation<P>,
    /// Should the schema checksum not be stored as an option in the program store.
    skip_storing_checksum: bool,
}

/// Specifies the location for the store database.
pub enum StoreLocation<P: AsRef<Path>> {
    /// Specifies an in-memory database.
    Memory,
    /// Specifies a filesystem path to save the database in.
    File(P),
}

impl<P> Builder<P>
where
    P: AsRef<Path>,
{
    /// Returns a new `Builder` with a given database location. The schema and its checksum are the
    /// defaults which are found in the constants `SCHEMA` and `SCHEMA_CHECKSUM`.
    pub fn from_location(store_location: StoreLocation<P>) -> Self {
        Self {
            schema: String::from(SCHEMA),
            schema_checksum: Some(String::from(SCHEMA_CHECKSUM)),
            store_location,
            skip_storing_checksum: false,
        }
    }

    /// Specifies a different schema. The pre-calculated schema checksum will be cleared and
    /// recalculated when finalising the builder.
    #[allow(dead_code)]
    pub fn with_schema(self, schema: &str) -> Self {
        Self {
            schema: String::from(schema),
            schema_checksum: None,
            ..self
        }
    }

    /// Specify whether to skip storing the schema checksum in the store options.
    #[allow(dead_code)]
    pub fn skip_storing_checksum(self, skip: bool) -> Self {
        Self {
            skip_storing_checksum: skip,
            ..self
        }
    }

    /// Finalise the builder and return a new `Store`.
    pub async fn build(self) -> anyhow::Result<Store> {
        let schema_checksum = if let Some(checksum) = self.schema_checksum {
            checksum
        } else {
            trace!("Missing schema checksum, calculating");
            util::checksum::blake2b_string(&self.schema)
        };
        trace!("Store database schema checksum: {}", schema_checksum);

        let (store_file_exists, conn) = match self.store_location {
            StoreLocation::Memory => {
                // when opening an in-memory database, it will initially be empty, i.e. it didn't
                // exist beforehand
                (false, Connection::open_in_memory()?)
            }
            StoreLocation::File(path) => (path.as_ref().exists(), open_file_connection(path)?),
        };
        let conn = Arc::new(Mutex::new(conn));

        let store = Store { conn };
        debug!("Store database exists: {}", store_file_exists);

        let checksums_match = store_file_exists && checksum_matches_meta(&store, &schema_checksum).await?;
        debug!("Schema checksums match: {}", checksums_match);

        if !checksums_match {
            // TODO: data migration when the schema changes
            warn!("Store database schema checksum mismatch - applying new schema");
            apply_store_schema(&store, &self.schema).await?;

            if !self.skip_storing_checksum {
                store_schema_checksum(&store, &schema_checksum).await?;
            }
        }

        Ok(store)
    }
}

/// Opens an SQLite connection to a given file path. If the file exists, its permissions are checked
/// to ensure they meet `MAX_STORE_FILE_PERMISSIONS`. If the file doesn't exist, a new one will be
/// created and its permissions will be set to `MAX_STORE_FILE_PERMISSIONS`.
///
/// # Errors
/// Returns `StoreError::InsufficientFilePermissions` if the existing file's permissions aren't
/// sufficient.
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
                maximum: MAX_STORE_FILE_PERMISSIONS,
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

/// Applies a given SQL schema to a given `Store`.
async fn apply_store_schema(store: &Store, schema: &str) -> anyhow::Result<()> {
    trace!("Applying database schema...");
    trace!("{}", schema);

    store.apply_schema(schema).await?;
    Ok(())
}

/// Stores a given schema checksum to the program store's `SchemaChecksum` option.
async fn store_schema_checksum(store: &Store, checksum: &str) -> anyhow::Result<()> {
    trace!("Storing schema checksum...");

    store
        .set_option(option::Value::new(
            option::Field::SchemaChecksum,
            Some(String::from(checksum)),
        ))
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
    if let Some(metavalue) = store.get_option(option::Field::SchemaChecksum).await? {
        if let Some(existing_checksum) = metavalue.value() {
            trace!("Got existing schema checksum: {}", existing_checksum);
            return Ok(wanted_checksum == existing_checksum);
        }
    }

    Ok(false)
}

/// Accepts a reference to an `Arc<Mutex<Connection>>` and a block where that reference can be used
/// to access the database connection. The block will run a blocking thread with
/// `task::spawn_blocking`. Returns what the given block returns.
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
        trace!("Beginning new store transaction");
        Ok(self.conn.lock().unwrap().execute_batch("BEGIN TRANSACTION")?)
    }

    /// Commits an ongoing transaction in the database with `COMMIT`;
    pub fn commit_transaction(&self) -> anyhow::Result<()> {
        trace!("Committing store transaction");
        Ok(self.conn.lock().unwrap().execute_batch("COMMIT")?)
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

    /// Retrieves an option value from the option table with a given option field.
    pub async fn get_settings(&self, game: GameStoreId) -> anyhow::Result<GameSettings> {
        let conn = &self.conn;
        sql!(conn => {
            let mut stmt = conn.prepare(GameSettings::select())?;

            Ok(stmt
                .query_row_named(&GameSettings::select_params(&game), |row| {
                    Ok(row.into())
                })?)
        })
    }

    /// Stores an option value to the options table.
    pub async fn set_settings(&self, settings: GameSettings) -> anyhow::Result<()> {
        let conn = &self.conn;
        sql!(conn => {
            conn.execute_named(GameSettings::replace_into(), &settings.all_params())?;
            Ok(())
        })
    }

    /// Retrieves all stored `Game`s.
    pub async fn get_games(&self) -> anyhow::Result<Vec<Game>> {
        let conn = &self.conn;
        sql!(conn => {
            let mut stmt = conn.prepare(Game::select_all())?;
            let mut games = Vec::new();

            for game in stmt.query_map(NO_PARAMS, |row| {
                Ok(row.into())
            })? {
                games.push(game?);
            }

            Ok(games)
        })
    }

    /// Retrieves all mods of a given `Game`, identified by its store ID.
    pub async fn get_mods_of_game(&self, game_store_id: GameStoreId) -> anyhow::Result<Vec<GameMod>> {
        let conn = &self.conn;
        sql!(conn => {
            let mut stmt = conn.prepare(GameMod::select())?;
            let mut mods = Vec::new();

            for row in stmt.query_map_named(&GameMod::select_params(&game_store_id), |row| {
                Ok(row.into())
            })? {
                mods.push(row?);
            }

            Ok(mods)
        })
    }

    /// Stores all the mods of a `Game`. Will replace existing stored mods in the database.
    pub async fn set_mods_of_game(&self, mods: Vec<GameMod>) -> anyhow::Result<()> {
        let conn = &self.conn;
        sql!(conn => {
            let mut stmt = conn.prepare(GameMod::replace_into())?;

            for m in &mods {
                stmt.execute_named(&m.all_params())?;
            }

            Ok(())
        })
    }

    /// Stores a new `Game`.
    pub async fn insert_game(&self, new_game: Game) -> anyhow::Result<GameStoreId> {
        let conn = &self.conn;
        sql!(conn => {
            conn.execute_named(Game::insert_into(), &new_game.all_params())?;
            let id = conn.last_insert_rowid();

            Ok(id as GameStoreId)
        })
    }

    /// Updates an existing stored `Game`.
    pub async fn update_game(&self, game: Game) -> anyhow::Result<()> {
        let conn = &self.conn;
        sql!(conn => {
            conn.execute_named(
                Game::update(),
                &game.all_params(),
            )?;

            Ok(())
        })
    }

    /// Retrieves an optional `FactorioMod`.
    pub async fn get_factorio_mod(&self, factorio_mod: String) -> anyhow::Result<Option<FactorioMod>> {
        let conn = &self.conn;
        sql!(conn => {
            let mut stmt = conn.prepare(FactorioMod::select())?;

            Ok(stmt
                .query_row_named(&FactorioMod::select_params(&factorio_mod), |row| {
                    Ok(row.into())
                })
                .optional()?)
        })
    }

    /// Stores a single `FactorioMod`.
    pub async fn set_factorio_mod(&self, factorio_mod: models::FactorioMod) -> anyhow::Result<()> {
        let conn = &self.conn;
        sql!(conn => {
            conn.execute_named(FactorioMod::replace_into(), &factorio_mod.all_params())?;
            Ok(())
        })
    }

    /// Retrieves all releases of a `FactorioMod`.
    pub async fn get_mod_releases(&self, factorio_mod: String) -> anyhow::Result<Vec<ModRelease>> {
        let conn = &self.conn;
        sql!(conn => {
            let mut stmt = conn.prepare(ModRelease::select())?;
            let mut mods = Vec::new();

            for m in
                stmt.query_map_named(&ModRelease::select_params(&factorio_mod), |row| {
                    Ok(row.into())
                })?
            {
                mods.push(m?);
            }

            Ok(mods)
        })
    }

    /// Stores a single `ModRelease`.
    pub async fn set_mod_release(&self, release: ModRelease) -> anyhow::Result<()> {
        let conn = &self.conn;
        sql!(conn => {
            conn.execute_named(ModRelease::replace_into(), &release.all_params())?;
            Ok(())
        })
    }

    /// Retrieves all `ReleaseDependencies` of a given `ModRelease` based on its mod's name and its
    /// version.
    pub async fn get_release_dependencies(
        &self,
        release_mod_name: String,
        release_version: HumanVersion,
    ) -> anyhow::Result<Vec<ReleaseDependency>> {
        let conn = &self.conn;
        sql!(conn => {
            let mut stmt = conn.prepare(ReleaseDependency::select())?;
            let mut dependencies = Vec::new();

            for dep in
                stmt.query_map_named(&ReleaseDependency::select_params(&release_mod_name, &release_version), |row| {
                    Ok(row.into())
                })?
            {
                dependencies.push(dep?);
            }

            Ok(dependencies)
        })
    }

    /// Stores all given `ReleaseDependencies`.
    pub async fn set_release_dependencies(&self, dependencies: Vec<ReleaseDependency>) -> anyhow::Result<()> {
        let conn = &self.conn;
        sql!(conn => {
            let mut stmt = conn.prepare(ReleaseDependency::replace_into())?;

            for rel in dependencies {
                stmt.execute_named(&rel.all_params())?;
            }

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
    async fn set_option() {
        const SCHEMA: &str = r#"CREATE TABLE "options" (
"field"	TEXT NOT NULL,
"value"	TEXT,
PRIMARY KEY("field")
);"#;
        let store = get_test_store(SCHEMA).await;

        store.begin_transaction().expect("failed to begin transaction");
        store
            .set_option(option::Value::new(
                option::Field::PortalUsername,
                Some(String::from("value")),
            ))
            .await
            .expect("failed to set meta value");
        store.commit_transaction().expect("failed to commit transaction");
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

        assert_eq!(got_value.value(), Some("value"));
    }
}
