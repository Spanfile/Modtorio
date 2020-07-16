mod cache_meta;
pub mod models;

use crate::{ext::PathExt, factorio::GameCacheId, util, util::HumanVersion};
pub use cache_meta::{CacheMetaField, CacheMetaValue};
use log::*;
use models::*;
use rusqlite::{Connection, OptionalExtension, NO_PARAMS};
use std::{
    path::{Path, PathBuf},
    sync::{Arc, Mutex},
};
use tokio::task;

const DB_PATH: &str = "modtorio.db";
const SCHEMA: &str = include_str!(concat!(env!("CARGO_MANIFEST_DIR"), "/schema.sql"));

pub struct Cache {
    conn: Arc<Mutex<Connection>>,
}

#[derive(Debug)]
pub struct CacheBuilder {
    db_path: PathBuf,
    schema: String,
}

impl CacheBuilder {
    pub fn new() -> Self {
        Self {
            db_path: PathBuf::from(DB_PATH),
            schema: String::from(SCHEMA),
        }
    }

    pub fn with_db_path<P>(self, path: P) -> Self
    where
        P: AsRef<Path>,
    {
        Self {
            db_path: PathBuf::from(path.as_ref()),
            ..self
        }
    }

    pub fn with_schema(self, schema: String) -> Self {
        Self { schema, ..self }
    }

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
                .set_meta(CacheMetaValue {
                    field: CacheMetaField::SchemaChecksum,
                    value: Some(encoded_checksum),
                })
                .await?;
        }

        Ok(cache)
    }
}

async fn checksum_matches_meta(cache: &Cache, encoded_checksum: &str) -> anyhow::Result<bool> {
    if let Some(metavalue) = cache.get_meta(CacheMetaField::SchemaChecksum).await? {
        if let Some(checksum) = metavalue.value {
            trace!("Got existing schema checksum: {}", checksum);
            return Ok(checksum == encoded_checksum);
        }
    }

    Ok(false)
}

impl Cache {
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
    pub fn begin_transaction(&self) -> anyhow::Result<()> {
        debug!("Beginning new cache transaction");
        Ok(self
            .conn
            .lock()
            .unwrap()
            .execute_batch("BEGIN TRANSACTION")?)
    }

    pub fn commit_transaction(&self) -> anyhow::Result<()> {
        debug!("Committing cache transaction");
        Ok(self.conn.lock().unwrap().execute_batch("COMMIT")?)
    }

    pub async fn get_meta(&self, field: CacheMetaField) -> anyhow::Result<Option<CacheMetaValue>> {
        let conn = &self.conn;
        sql!(conn => {
            let mut stmt = conn.prepare(CacheMetaValue::select())?;

            Ok(stmt
                .query_row_named(&CacheMetaValue::select_params(&field), |row| {
                    Ok(row.into())
                })
                .optional()?)
        })
    }

    pub async fn set_meta(&self, value: CacheMetaValue) -> anyhow::Result<()> {
        let conn = &self.conn;
        sql!(conn => {
            conn.execute_named(CacheMetaValue::replace_into(), &value.all_params())?;
            Ok(())
        })
    }

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

    pub async fn insert_game(&self, new_game: Game) -> anyhow::Result<i64> {
        let conn = &self.conn;
        sql!(conn => {
            conn.execute_named(Game::insert_into(), &new_game.all_params())?;
            let id = conn.last_insert_rowid();

            Ok(id)
        })
    }

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

    pub async fn set_factorio_mod(&self, factorio_mod: models::FactorioMod) -> anyhow::Result<()> {
        let conn = &self.conn;
        sql!(conn => {
            conn.execute_named(FactorioMod::replace_into(), &factorio_mod.all_params())?;
            Ok(())
        })
    }

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

    pub async fn set_mod_release(&self, release: ModRelease) -> anyhow::Result<()> {
        let conn = &self.conn;
        sql!(conn => {
            conn.execute_named(ModRelease::replace_into(), &release.all_params())?;
            Ok(())
        })
    }

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
