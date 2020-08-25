//! The program store, used to store persistent data about the program in an SQLite database.

pub mod models;
pub mod option;

use crate::{error::StoreError, factorio::GameStoreId, util, util::ext::PathExt};
use log::*;
use models::{FactorioMod, Game, GameMod, GameSettings, ModRelease, ReleaseDependency};
use refinery::embed_migrations;
use rusqlite::{Connection, OptionalExtension, NO_PARAMS};
use std::{
    ops::DerefMut,
    path::Path,
    sync::{Arc, Mutex},
};
use tokio::task;
use util::HumanVersion;

embed_migrations!();

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
    /// Location for the store database. Either a filesystem path, or in-memory.
    store_location: StoreLocation<P>,
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
    /// Returns a new `Builder` with a given database location.
    pub fn from_location(store_location: StoreLocation<P>) -> Self {
        Self { store_location }
    }

    /// Finalise the builder and return a new `Store`.
    pub async fn build(self) -> anyhow::Result<Store> {
        let conn = match self.store_location {
            StoreLocation::Memory => {
                // when opening an in-memory database, it will initially be empty, i.e. it didn't
                // exist beforehand
                Connection::open_in_memory()?
            }
            StoreLocation::File(path) => open_file_connection(path)?,
        };
        let conn = Arc::new(Mutex::new(conn));
        let store = Store { conn };

        store.run_migrations().await?;

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

/// Accepts a reference to an `Arc<Mutex<Connection>>` and a block where that reference can be used
/// to access the database connection. The block will run a blocking thread with
/// `task::spawn_blocking`. Returns what the given block returns.
#[macro_export]
macro_rules! sql {
    ($conn:ident => $b:block) => {
        Ok({
            let _c = Arc::clone(&$conn);
            task::spawn_blocking(move || -> anyhow::Result<_> {
                #[allow(unused_mut)]
                let mut $conn = _c.lock().unwrap();
                $b
            })
            .await??
        })
    };
}

impl Store {
    /// Runs the database migration scripts on this store and returns a report of applied migrations.
    async fn run_migrations(&self) -> anyhow::Result<refinery::Report> {
        let conn = &self.conn;
        sql!(conn => {
            Ok(migrations::runner().run(conn.deref_mut())?)
        })
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

    /// Delets all mods of a given `Game`, identified by its store ID.
    pub async fn remove_mods_of_game(&self, game_store_id: GameStoreId) -> anyhow::Result<()> {
        let conn = &self.conn;
        sql!(conn => {
            conn.execute_named(GameMod::delete(), &GameMod::select_params(&game_store_id))?;
            Ok(())
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

    async fn get_test_store() -> Store {
        let store = Builder::<String>::from_location(StoreLocation::Memory)
            .build()
            .await
            .expect("failed to build store");
        store
            .set_option(option::Value::new(
                option::Field::PortalUsername,
                Some("value".to_string()),
            ))
            .await
            .expect("failed to set option");
        store
    }

    #[tokio::test]
    async fn set_option() {
        let store = get_test_store().await;

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
        let store = get_test_store().await;

        let got_value = store
            .get_option(option::Field::PortalUsername)
            .await
            .expect("failed to get option value")
            .expect("store returned no value");

        assert_eq!(got_value.value(), Some("value"));
    }
}
