//! The program cache, used to cache game information to avoid heavy reloading when the program is
//! run.

pub mod models;

use crate::{factorio::GameCacheId, sql, util::HumanVersion};
use models::*;
use rusqlite::{Connection, OptionalExtension, NO_PARAMS};
use std::sync::{Arc, Mutex};
use tokio::task;

/// The program cache, used to cache game information to avoid heavy reloading when the program is
/// run.
pub struct Cache {
    /// Reference to the SQLite connection to the program database.
    pub(super) conn: Arc<Mutex<Connection>>,
}

impl Cache {
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
