//! Provides the [`Cache`] object and object [models][Models] used to store various values to
//! speed up the program flow; for example game mod information to avoid reading each mod from the
//! filesystem every time the program is started.
//!
//! Uses an SQLite database as the filesystem store through the [`rusqlite`] driver.
//!
//! [Models]: models
//! [Cache]: Cache
//! [rusqlite]: rusqlite

mod cache_meta;
pub mod models;

use crate::{config, ext::PathExt, factorio::GameCacheId, util, util::HumanVersion};
pub use cache_meta::{Field, Value};
use log::*;
use models::*;
use rusqlite::{Connection, OptionalExtension, NO_PARAMS};
use std::{
    path::{Path, PathBuf},
    sync::{Arc, Mutex},
};
use tokio::task;

/// The default cache database schema string.
const SCHEMA: &str = include_str!(concat!(env!("CARGO_MANIFEST_DIR"), "/schema.sql"));

/// The program cache. Provides functions to read and write different [`models`][Models] to the
/// program cache.
///
/// The cache object is built using a [`Builder`].
///
/// Each asynchronous function that interacts with the cache database will run their work on a
/// blocking thread in the background.
///
/// [Models]: crate::cache::models
/// [Builder]: Builder
pub struct Cache {
    /// The SQLite connection.
    conn: Arc<Mutex<Connection>>,
}

/// Builds a [`Cache`] object using the builder pattern.
///
/// A default cache can be built simply.
/// ```no_run
/// let cache = CacheBuilder::new().build();
/// ```
///
/// Other values than the defaults can be provided with their corresponding functions.
/// ```no_run
/// let cache = CacheBuilder::new()
///     .with_db_path("other-db-than-default.db")
///     .with_schema("other-schema-than-default")
///     .build();
/// ```
///
/// [Cache]: super::Cache
#[derive(Debug)]
pub struct Builder {
    db_path: PathBuf,
    schema: String,
}

impl Builder {
    /// Returns a new Builder with each field filled with its default value.
    pub fn new() -> Self {
        Self {
            db_path: PathBuf::from(config::DEFAULT_STORE_FILE_LOCATION),
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
            db_path: PathBuf::from(path.as_ref()),
            ..self
        }
    }

    /// Specify a custom schema used to build the database.
    #[allow(dead_code)]
    pub fn with_schema(self, schema: String) -> Self {
        Self { schema, ..self }
    }

    /// Finalise the builder and return the built cache object.
    ///
    /// During building the schema's checksum will be calculated and, if the cache database already
    /// exists in the filesystem, compared against the existing stored checksum. If there's a
    /// mismatch, the schema will be applied over the existing database, deleting all data
    /// inside it. The new schema checksum will be then stored in the cache [metadata].
    ///
    /// [metadata]: crate::cache::cache_meta
    pub async fn build(self) -> anyhow::Result<Cache> {
        let encoded_checksum = util::checksum::blake2b_string(&self.schema);
        trace!("Cache database schema checksum: {}", encoded_checksum);

        let db_exists = self.db_path.exists();
        let conn = Connection::open(self.db_path.get_str()?)?;
        let cache = Cache {
            conn: Arc::new(Mutex::new(conn)),
        };

        debug!("Cache database exists: {}", db_exists);

        let checksums_match = db_exists && checksum_matches_meta(&cache, &encoded_checksum).await?;
        debug!("Schema checksums match: {}", checksums_match);

        if !db_exists || !checksums_match {
            debug!("Applying database schema...");
            trace!("{}", self.schema);

            cache.apply_schema(self.schema).await?;
            cache
                .set_meta(Value {
                    field: Field::SchemaChecksum,
                    value: Some(encoded_checksum),
                })
                .await?;
        }

        Ok(cache)
    }
}

/// Compares a given cache schema checksum string to what a given cache's metadata possibly
/// contains. Returns a `Result<bool>` corresponding to whether the cache's existing schema checksum
/// matches the wanted one. Returns `Ok(false)` if the cache doesn't contain the [schema checksum
/// field][Field]. Returns an error if reading the database meta table fails.
///
/// [Field]: cache_meta::Field#variant.SchemaChecksum
async fn checksum_matches_meta(cache: &Cache, wanted_checksum: &str) -> anyhow::Result<bool> {
    if let Some(metavalue) = cache.get_meta(Field::SchemaChecksum).await? {
        if let Some(existing_checksum) = metavalue.value {
            trace!("Got existing schema checksum: {}", existing_checksum);
            return Ok(wanted_checksum == existing_checksum);
        }
    }

    Ok(false)
}

impl Cache {
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

impl Cache {
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

    /// Retrieves all mods of a given `Game`, identified by its cache ID.
    pub async fn get_mods_of_game(
        &self,
        game_cache_id: GameCacheId,
    ) -> anyhow::Result<Vec<GameMod>> {
        let conn = &self.conn;
        sql!(conn => {
            let mut stmt = conn.prepare(GameMod::select())?;
            let mut mods = Vec::new();

            for row in stmt.query_map_named(&GameMod::select_params(&game_cache_id), |row| {
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
    pub async fn insert_game(&self, new_game: Game) -> anyhow::Result<i64> {
        let conn = &self.conn;
        sql!(conn => {
            conn.execute_named(Game::insert_into(), &new_game.all_params())?;
            let id = conn.last_insert_rowid();

            Ok(id)
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
    pub async fn get_factorio_mod(
        &self,
        factorio_mod: String,
    ) -> anyhow::Result<Option<FactorioMod>> {
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
    pub async fn set_release_dependencies(
        &self,
        dependencies: Vec<ReleaseDependency>,
    ) -> anyhow::Result<()> {
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
