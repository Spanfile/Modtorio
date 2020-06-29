mod cache_meta;
pub mod models;

use crate::{ext::PathExt, factorio::GameCacheId, util, util::HumanVersion};
pub use cache_meta::{CacheMetaField, CacheMetaValue};
use log::*;
use models::*;
use rusqlite::{named_params, Connection, OptionalExtension, NO_PARAMS};
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

        let checksums_match = db_exists && checksum_matches_meta(&cache, &encoded_checksum).await?;
        debug!(
            "Cache database exists: {}. Schema checksums match: {}",
            db_exists, checksums_match
        );

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

    pub async fn get_meta(&self, field: CacheMetaField) -> anyhow::Result<Option<CacheMetaValue>> {
        let conn = Arc::clone(&self.conn);
        let result = task::spawn_blocking(move || -> anyhow::Result<Option<CacheMetaValue>> {
            let conn = conn.lock().unwrap();
            let mut stmt = conn.prepare("SELECT * FROM _meta WHERE field = :field LIMIT 1")?;

            Ok(stmt
                .query_row_named(named_params! { ":field": field.to_string() }, |row| {
                    Ok(CacheMetaValue {
                        field: row.get(0)?,
                        value: row.get(1)?,
                    })
                })
                .optional()?)
        })
        .await?;

        Ok(result?)
    }

    pub async fn set_meta(&self, value: CacheMetaValue) -> anyhow::Result<()> {
        let conn = Arc::clone(&self.conn);
        let result = task::spawn_blocking(move || -> anyhow::Result<()> {
            let conn = conn.lock().unwrap();
            let mut stmt =
                conn.prepare("REPLACE INTO _meta (field, value) VALUES (:field, :value)")?;

            stmt.execute_named(named_params! {
                ":field": value.field,
                ":value": value.value,
            })?;

            Ok(())
        })
        .await?;

        Ok(result?)
    }
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

    pub async fn get_games(&self) -> anyhow::Result<Vec<Game>> {
        let conn = Arc::clone(&self.conn);
        let result = task::spawn_blocking(move || -> anyhow::Result<Vec<Game>> {
            let conn = conn.lock().unwrap();
            let mut stmt = conn.prepare("SELECT * FROM game")?;
            let mut games = Vec::new();

            for game in stmt.query_map(NO_PARAMS, |row| {
                Ok(Game {
                    id: row.get(0)?,
                    path: row.get(1)?,
                })
            })? {
                games.push(game?);
            }

            Ok(games)
        })
        .await?;

        Ok(result?)
    }

    pub async fn get_game(&self, id: GameCacheId) -> anyhow::Result<Game> {
        let conn = Arc::clone(&self.conn);
        let result = task::spawn_blocking(move || -> anyhow::Result<Game> {
            let conn = conn.lock().unwrap();
            let mut stmt = conn.prepare("SELECT * FROM game WHERE id = :id LIMIT 1")?;

            Ok(stmt.query_row_named(named_params! {":id": id}, |row| {
                Ok(Game {
                    id: row.get(0)?,
                    path: row.get(1)?,
                })
            })?)
        })
        .await?;

        Ok(result?)
    }

    pub async fn get_mods_of_game(&self, game: Game) -> anyhow::Result<Vec<FactorioMod>> {
        let conn = Arc::clone(&self.conn);
        let result = task::spawn_blocking(move || -> anyhow::Result<Vec<FactorioMod>> {
            let conn = conn.lock().unwrap();
            let mut stmt = conn.prepare(
                "SELECT * FROM game_mod INNER JOIN factorio_mod ON factorio_mod.name == \
                 game_mod.factorio_mod WHERE game_mod.game == :id;",
            )?;
            let mut mods = Vec::new();

            for row in stmt.query_map_named(named_params! { ":id": game.id }, |row| {
                Ok(FactorioMod {
                    name: row.get(0)?,
                    summary: row.get(1)?,
                    last_updated: row.get(2)?,
                })
            })? {
                mods.push(row?);
            }

            Ok(mods)
        })
        .await?;

        Ok(result?)
    }

    pub async fn set_mods_of_game(&self, mods: Vec<GameMod>) -> anyhow::Result<()> {
        let conn = Arc::clone(&self.conn);
        task::spawn_blocking(move || -> anyhow::Result<()> {
            let conn = conn.lock().unwrap();
            let mut stmt = conn.prepare(
                "REPLACE INTO game_mod (game, factorio_mod, mod_version, mod_zip, zip_checksum) \
                 VALUES(:game, :factorio_mod, :mod_version, :mod_zip, :zip_checksum)",
            )?;

            for m in &mods {
                stmt.execute_named(named_params! {
                    ":game": m.game,
                    ":factorio_mod": m.factorio_mod,
                    ":mod_version": m.mod_version,
                    ":mod_zip": m.mod_zip,
                    ":zip_checksum": m.zip_checksum
                })?;
            }

            Ok(())
        })
        .await??;

        Ok(())
    }

    pub async fn insert_game(&self, new_game: Game) -> anyhow::Result<i64> {
        let conn = Arc::clone(&self.conn);
        let result = task::spawn_blocking(move || -> anyhow::Result<i64> {
            let conn = conn.lock().unwrap();
            let mut stmt = conn.prepare("INSERT INTO game (path) VALUES (:path)")?;

            stmt.execute_named(named_params! { ":path": new_game.path })?;
            let id = conn.last_insert_rowid();

            Ok(id)
        })
        .await?;

        Ok(result?)
    }

    pub async fn update_game(&self, game: Game) -> anyhow::Result<()> {
        let conn = Arc::clone(&self.conn);
        task::spawn_blocking(move || -> anyhow::Result<()> {
            let conn = conn.lock().unwrap();
            conn.execute_named(
                "UPDATE game SET path = :path WHERE id = :id",
                named_params! { ":path": game.path, ":id": game.id },
            )?;

            Ok(())
        })
        .await??;

        Ok(())
    }

    pub async fn get_factorio_mod(
        &self,
        factorio_mod: String,
    ) -> anyhow::Result<Option<FactorioMod>> {
        let conn = Arc::clone(&self.conn);
        let result = task::spawn_blocking(move || -> anyhow::Result<Option<FactorioMod>> {
            let conn = conn.lock().unwrap();
            let mut stmt = conn.prepare("SELECT * FROM factorio_mod WHERE name = :name LIMIT 1")?;

            Ok(stmt
                .query_row_named(named_params! {":name": factorio_mod}, |row| {
                    Ok(FactorioMod {
                        name: row.get(0)?,
                        summary: row.get(1)?,
                        last_updated: row.get(2)?,
                    })
                })
                .optional()?)
        })
        .await?;

        Ok(result?)
    }

    pub async fn set_factorio_mod(&self, factorio_mod: models::FactorioMod) -> anyhow::Result<()> {
        let conn = Arc::clone(&self.conn);
        task::spawn_blocking(move || -> anyhow::Result<()> {
            let conn = conn.lock().unwrap();
            let mut stmt = conn.prepare(
                "REPLACE INTO factorio_mod (name, last_updated) VALUES(:name, :last_updated)",
            )?;

            stmt.execute_named(named_params! {
                ":name": factorio_mod.name,
                ":last_updated": factorio_mod.last_updated,
            })?;

            Ok(())
        })
        .await??;

        Ok(())
    }

    pub async fn get_mod_releases(&self, factorio_mod: String) -> anyhow::Result<Vec<ModRelease>> {
        let conn = Arc::clone(&self.conn);
        let result = task::spawn_blocking(move || -> anyhow::Result<Vec<ModRelease>> {
            let conn = conn.lock().unwrap();
            let mut stmt =
                conn.prepare("SELECT * FROM mod_release WHERE factorio_mod = :factorio_mod")?;
            let mut mods = Vec::new();

            for m in
                stmt.query_map_named(named_params! { ":factorio_mod": factorio_mod }, |row| {
                    Ok(ModRelease {
                        factorio_mod: row.get(0)?,
                        version: row.get(1)?,
                        download_url: row.get(2)?,
                        released_on: row.get(3)?,
                        sha1: row.get(4)?,
                        factorio_version: row.get(5)?,
                    })
                })?
            {
                mods.push(m?);
            }

            Ok(mods)
        })
        .await?;

        Ok(result?)
    }

    pub async fn set_mod_release(&self, release: ModRelease) -> anyhow::Result<()> {
        let conn = Arc::clone(&self.conn);
        task::spawn_blocking(move || -> anyhow::Result<()> {
            let conn = conn.lock().unwrap();
            let mut stmt = conn.prepare(
                "REPLACE INTO mod_release (factorio_mod, download_url, released_on, version, \
                 sha1, factorio_version) VALUES(:factorio_mod, :download_url, :released_on, \
                 :version, :sha1, :factorio_version)",
            )?;

            stmt.execute_named(named_params! {
                ":factorio_mod": release.factorio_mod,
                ":download_url": release.download_url,
                ":released_on": release.released_on,
                ":version": release.version,
                ":sha1": release.sha1,
                ":factorio_version": release.factorio_version,
            })?;

            Ok(())
        })
        .await??;

        Ok(())
    }

    pub async fn get_release_dependencies(
        &self,
        release_mod_name: String,
        release_version: HumanVersion,
    ) -> anyhow::Result<Vec<ReleaseDependency>> {
        let conn = Arc::clone(&self.conn);
        let result = task::spawn_blocking(move || -> anyhow::Result<Vec<ReleaseDependency>> {
            let conn = conn.lock().unwrap();
            let mut stmt = conn.prepare(
                "SELECT * FROM release_dependency WHERE release_mod_name = :release_mod_name AND \
                 release_version = :release_version",
            )?;
            let mut dependencies = Vec::new();

            for dep in
                stmt.query_map_named(named_params! { ":release_mod_name": release_mod_name, ":release_version": release_version }, |row| {
                    Ok(ReleaseDependency {
                        release_mod_name: row.get(0)?,
                        release_version: row.get(1)?,
                        name: row.get(2)?,
                        requirement: row.get(3)?,
                        version_req: row.get(4)?,
                    })
                })?
            {
                dependencies.push(dep?);
            }

            Ok(dependencies)
        })
        .await?;

        Ok(result?)
    }

    pub async fn set_release_dependencies(
        &self,
        dependencies: Vec<ReleaseDependency>,
    ) -> anyhow::Result<()> {
        let conn = Arc::clone(&self.conn);
        task::spawn_blocking(move || -> anyhow::Result<()> {
            let conn = conn.lock().unwrap();
            let mut stmt = conn.prepare(
                "REPLACE INTO release_dependency (release_mod_name, release_version, name, \
                 requirement, version_req) VALUES(:release_mod_name, :release_version, :name, \
                 :requirement, :version_req)",
            )?;

            for rel in dependencies {
                stmt.execute_named(named_params! {
                    ":release_mod_name": rel.release_mod_name,
                    ":release_version": rel.release_version,
                    ":name": rel.name,
                    ":requirement": rel.requirement,
                    ":version_req": rel.version_req,
                })?;
            }

            Ok(())
        })
        .await??;

        Ok(())
    }
}
