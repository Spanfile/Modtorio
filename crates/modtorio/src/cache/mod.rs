pub mod models;

use crate::{ext::PathExt, factorio::GameCacheId};
use models::*;
use rusqlite::{params, Connection, OptionalExtension, NO_PARAMS};
use std::{
    env,
    path::{Path, PathBuf},
    sync::{Arc, Mutex},
};
use tokio::task;

pub struct Cache {
    conn: Arc<Mutex<Connection>>,
}

#[derive(Debug)]
pub struct CacheBuilder {
    db_path: PathBuf,
}

impl CacheBuilder {
    pub fn new() -> Self {
        Self {
            db_path: PathBuf::from(
                env::var("DATABASE_URL").expect("couldn't read DATABASE_URL env variable"),
            ),
        }
    }

    pub fn with_db_path<P>(self, path: P) -> Self
    where
        P: AsRef<Path>,
    {
        Self {
            db_path: PathBuf::from(path.as_ref()),
        }
    }

    pub fn build(self) -> anyhow::Result<Cache> {
        let conn = Connection::open(self.db_path.get_str()?)?;

        Ok(Cache {
            conn: Arc::new(Mutex::new(conn)),
        })
    }
}

impl Cache {
    pub async fn get_game_ids(&self) -> anyhow::Result<Vec<GameCacheId>> {
        let conn = Arc::clone(&self.conn);
        let result = task::spawn_blocking(move || -> anyhow::Result<Vec<GameCacheId>> {
            let conn = conn.lock().unwrap();
            let mut stmt = conn.prepare("SELECT id FROM game")?;
            let mut ids = Vec::new();

            for row in stmt.query_map(NO_PARAMS, |row| row.get(0))? {
                ids.push(row?);
            }

            Ok(ids)
        })
        .await?;

        Ok(result?)
    }

    pub async fn get_game(&self, id: GameCacheId) -> anyhow::Result<Game> {
        let conn = Arc::clone(&self.conn);
        let result = task::spawn_blocking(move || -> anyhow::Result<Game> {
            let conn = conn.lock().unwrap();
            let mut stmt = conn.prepare("SELECT * FROM game WHERE id = ? LIMIT 1")?;

            Ok(stmt.query_row(params![id], |row| {
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
                 game_mod.factorio_mod WHERE game_mod.game == 1;",
            )?;
            let mut mods = Vec::new();

            for row in stmt.query_map(params![game.id], |row| {
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

    pub async fn set_mods_of_game(&self, mods: Vec<NewGameMod>) -> anyhow::Result<()> {
        let conn = Arc::clone(&self.conn);
        task::spawn_blocking(move || -> anyhow::Result<()> {
            let mut conn = conn.lock().unwrap();
            let tx = conn.transaction()?;
            let mut stmt = tx.prepare("REPLACE INTO game_mod (game, factorio_mod) VALUES(?, ?)")?;

            for m in &mods {
                stmt.execute(params![m.game, m.factorio_mod])?;
            }

            Ok(())
        })
        .await??;

        Ok(())
    }

    pub async fn insert_game(&self, new_game: NewGame) -> anyhow::Result<i64> {
        let conn = Arc::clone(&self.conn);
        let result = task::spawn_blocking(move || -> anyhow::Result<i64> {
            let mut conn = conn.lock().unwrap();
            let tx = conn.transaction()?;
            let id = {
                let mut stmt = tx.prepare("INSERT INTO game (path) VALUES (?)")?;
                stmt.insert(params![new_game.path])?
            };

            tx.commit()?;
            Ok(id)
        })
        .await?;

        Ok(result?)
    }

    pub async fn update_game(&self, id: GameCacheId, insert: NewGame) -> anyhow::Result<()> {
        let conn = Arc::clone(&self.conn);
        task::spawn_blocking(move || -> anyhow::Result<()> {
            let mut conn = conn.lock().unwrap();
            let tx = conn.transaction()?;

            tx.execute(
                "UPDATE game SET path = ? WHERE id = ?",
                params![insert.path, id],
            )?;

            Ok(tx.commit()?)
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
            let mut stmt = conn.prepare("SELECT * FROM factorio_mod WHERE name = ? LIMIT 1")?;

            Ok(stmt
                .query_row(params![factorio_mod], |row| {
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

    pub async fn set_factorio_mod(
        &self,
        factorio_mod: models::NewFactorioMod,
    ) -> anyhow::Result<()> {
        let conn = Arc::clone(&self.conn);
        task::spawn_blocking(move || -> anyhow::Result<()> {
            let mut conn = conn.lock().unwrap();
            let tx = conn.transaction()?;
            {
                let mut stmt = tx.prepare(
                    "REPLACE INTO factorio_mod (name, summary, last_updated) VALUES(?, ?, ?)",
                )?;

                stmt.execute(params![
                    factorio_mod.name,
                    factorio_mod.summary,
                    factorio_mod.last_updated,
                ])?;
            }

            tx.commit()?;
            Ok(())
        })
        .await??;

        Ok(())
    }

    pub async fn get_mod_releases(&self, factorio_mod: String) -> anyhow::Result<Vec<ModRelease>> {
        let conn = Arc::clone(&self.conn);
        let result = task::spawn_blocking(move || -> anyhow::Result<Vec<ModRelease>> {
            let conn = conn.lock().unwrap();
            let mut stmt = conn.prepare("SELECT * FROM mod_release WHERE factorio_mod = ?")?;
            let mut mods = Vec::new();

            for m in stmt.query_map(params![factorio_mod], |row| {
                Ok(ModRelease {
                    factorio_mod: row.get(0)?,
                    download_url: row.get(1)?,
                    released_on: row.get(2)?,
                    version: row.get(3)?,
                    sha1: row.get(4)?,
                    factorio_version: row.get(5)?,
                })
            })? {
                mods.push(m?);
            }

            Ok(mods)
        })
        .await?;

        Ok(result?)
    }

    pub async fn set_mod_release(&self, release: NewModRelease) -> anyhow::Result<()> {
        let conn = Arc::clone(&self.conn);
        task::spawn_blocking(move || -> anyhow::Result<()> {
            let mut conn = conn.lock().unwrap();
            let tx = conn.transaction()?;
            {
                let mut stmt = tx.prepare(
                    "REPLACE INTO mod_release (factorio_mod, download_url, released_on, version, \
                     sha1, factorio_version) VALUES(?, ?, ?, ?, ?, ?)",
                )?;

                stmt.execute(params![
                    release.factorio_mod,
                    release.download_url,
                    release.released_on,
                    release.version,
                    release.sha1,
                    release.factorio_version,
                ])?;
            }

            tx.commit()?;
            Ok(())
        })
        .await??;

        Ok(())
    }

    pub async fn get_release_dependencies(
        &self,
        release_mod_name: String,
        release_version: String,
    ) -> anyhow::Result<Vec<ReleaseDependency>> {
        let conn = Arc::clone(&self.conn);
        let result = task::spawn_blocking(move || -> anyhow::Result<Vec<ReleaseDependency>> {
            let conn = conn.lock().unwrap();
            let mut stmt = conn.prepare(
                "SELECT * FROM release_dependency WHERE release_mod_name = ? AND release_version \
                 = ?",
            )?;
            let mut dependencies = Vec::new();

            for dep in stmt.query_map(params![release_mod_name, release_version], |row| {
                Ok(ReleaseDependency {
                    release_mod_name: row.get(0)?,
                    release_version: row.get(1)?,
                    name: row.get(2)?,
                    requirement: row.get(3)?,
                    version_req: row.get(4)?,
                })
            })? {
                dependencies.push(dep?);
            }

            Ok(dependencies)
        })
        .await?;

        Ok(result?)
    }

    pub async fn set_release_dependencies(
        &self,
        dependencies: Vec<NewReleaseDependency>,
    ) -> anyhow::Result<()> {
        let conn = Arc::clone(&self.conn);
        task::spawn_blocking(move || -> anyhow::Result<()> {
            let mut conn = conn.lock().unwrap();
            let tx = conn.transaction()?;
            {
                let mut stmt = tx.prepare(
                    "REPLACE INTO release_dependency (release_mod_name, release_version, name, \
                     requirement, version_req) VALUES(?, ?, ?, ?, ?)",
                )?;

                for rel in dependencies {
                    stmt.execute(params![
                        rel.release_mod_name,
                        rel.release_version,
                        rel.name,
                        rel.requirement,
                        rel.version_req,
                    ])?;
                }
            }

            tx.commit()?;
            Ok(())
        })
        .await??;

        Ok(())
    }
}
