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
    }

    pub async fn set_meta(&self, value: CacheMetaValue) -> anyhow::Result<()> {
        let conn = &self.conn;
        sql!(conn => {
            let mut stmt =
                conn.prepare("REPLACE INTO _meta (field, value) VALUES (:field, :value)")?;
            stmt.execute_named(&[
                (":field", &value.field as &dyn ::rusqlite::ToSql),
                (":value", &value.value as &dyn ::rusqlite::ToSql),
            ])?;
            Ok(())
        })
    }

    pub async fn get_games(&self) -> anyhow::Result<Vec<Game>> {
        let conn = &self.conn;
        sql!(conn => {
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
    }

    pub async fn get_mods_of_game(
        &self,
        game_cache_id: GameCacheId,
    ) -> anyhow::Result<Vec<GameMod>> {
        let conn = &self.conn;
        sql!(conn => {
            let mut stmt = conn.prepare("SELECT * FROM game_mod WHERE game_mod.game == :id")?;
            let mut mods = Vec::new();

            for row in stmt.query_map_named(named_params! { ":id": game_cache_id }, |row| {
                Ok(GameMod {
                    game: row.get(0)?,
                    factorio_mod: row.get(1)?,
                    mod_version: row.get(2)?,
                    mod_zip: row.get(3)?,
                    zip_checksum: row.get(4)?,
                })
            })? {
                mods.push(row?);
            }

            Ok(mods)
        })
    }

    pub async fn set_mods_of_game(&self, mods: Vec<GameMod>) -> anyhow::Result<()> {
        let conn = &self.conn;
        sql!(conn => {
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
    }

    pub async fn insert_game(&self, new_game: Game) -> anyhow::Result<i64> {
        let conn = &self.conn;
        sql!(conn => {
            let mut stmt = conn.prepare("INSERT INTO game (path) VALUES (:path)")?;

            stmt.execute_named(named_params! { ":path": new_game.path })?;
            let id = conn.last_insert_rowid();

            Ok(id)
        })
    }

    pub async fn update_game(&self, game: Game) -> anyhow::Result<()> {
        let conn = &self.conn;
        sql!(conn => {
            conn.execute_named(
                "UPDATE game SET path = :path WHERE id = :id",
                named_params! { ":path": game.path, ":id": game.id },
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
            let mut stmt = conn.prepare("SELECT * FROM factorio_mod WHERE name = :name LIMIT 1")?;

            Ok(stmt
                .query_row_named(named_params! {":name": factorio_mod}, |row| {
                    Ok(FactorioMod {
                        name: row.get(0)?,
                        author: row.get(1)?,
                        contact: row.get(2)?,
                        homepage: row.get(3)?,
                        title: row.get(4)?,
                        summary: row.get(5)?,
                        description: row.get(6)?,
                        changelog: row.get(7)?,
                        last_updated: row.get(8)?,
                    })
                })
                .optional()?)
        })
    }

    pub async fn set_factorio_mod(&self, factorio_mod: models::FactorioMod) -> anyhow::Result<()> {
        let conn = &self.conn;
        sql!(conn => {
            let mut stmt = conn.prepare(
                "REPLACE INTO factorio_mod (name, author, contact, homepage, title, summary, \
                 description, changelog, last_updated) VALUES(:name, :author, :contact, \
                 :homepage, :title, :summary, :description, :changelog, :last_updated)",
            )?;

            stmt.execute_named(named_params! {
                ":name": factorio_mod.name,
                ":author": factorio_mod.author,
                ":contact": factorio_mod.contact,
                ":homepage": factorio_mod.homepage,
                ":title": factorio_mod.title,
                ":summary": factorio_mod.summary,
                ":description": factorio_mod.description,
                ":changelog": factorio_mod.changelog,
                ":last_updated": factorio_mod.last_updated,
            })?;

            Ok(())
        })
    }

    pub async fn get_mod_releases(&self, factorio_mod: String) -> anyhow::Result<Vec<ModRelease>> {
        let conn = &self.conn;
        sql!(conn => {
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
    }

    pub async fn set_mod_release(&self, release: ModRelease) -> anyhow::Result<()> {
        let conn = &self.conn;
        sql!(conn => {
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
    }

    pub async fn get_release_dependencies(
        &self,
        release_mod_name: String,
        release_version: HumanVersion,
    ) -> anyhow::Result<Vec<ReleaseDependency>> {
        let conn = &self.conn;
        sql!(conn => {
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
    }

    pub async fn set_release_dependencies(
        &self,
        dependencies: Vec<ReleaseDependency>,
    ) -> anyhow::Result<()> {
        let conn = &self.conn;
        sql!(conn => {
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
    }
}
