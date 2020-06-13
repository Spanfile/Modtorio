pub mod models;

use crate::ext::PathExt;
use models::*;
use rusqlite::{params, Connection, OptionalExtension, NO_PARAMS};
use std::{
    env,
    ops::Deref,
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
    pub async fn get_game_ids(&self) -> anyhow::Result<Vec<i32>> {
        let conn = Arc::clone(&self.conn);
        let result = task::spawn_blocking(move || -> anyhow::Result<Vec<i32>> {
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

    pub async fn get_game(&self, id: i32) -> anyhow::Result<Game> {
        let conn = Arc::clone(&self.conn);
        let result = task::spawn_blocking(move || -> anyhow::Result<Game> {
            let conn = conn.lock().unwrap();
            let mut stmt = conn.prepare("SELECT * FROM game WHERE game.id = ? LIMIT 1")?;

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
                "SELECT factorio_mod.name, factorio_mod.summary, factorio_mod.last_updated FROM \
                 game_mod INNER JOIN factorio_mod ON factorio_mod.name == game_mod.factorio_mod \
                 WHERE game_mod.game == 1;",
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

    pub async fn set_mods_of_game(&self, game: i32, mods: Vec<String>) -> anyhow::Result<()> {
        let conn = Arc::clone(&self.conn);
        task::spawn_blocking(move || -> anyhow::Result<()> {
            let mut conn = conn.lock().unwrap();
            let tx = conn.transaction()?;

            tx.execute(
                "DELETE FROM game_mod WHERE game_mod.game == ?",
                params![game],
            );

            let mut stmt = tx.prepare("INSERT INTO game_mod (game, factorio_mod) VALUES(?, ?)")?;

            for m in &mods {
                stmt.execute(params![game, m]);
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
            let mut stmt = tx.prepare("INSERT INTO game (path) VALUES (?)")?;
            let id = stmt.insert(params![new_game.path])?;

            tx.commit()?;
            Ok(id)
        })
        .await?;

        Ok(result?)
    }

    pub async fn update_game(&self, id: i32, insert: NewGame) -> anyhow::Result<()> {
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
                })?
                .optional())
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
            use schema::factorio_mod;

            let conn = conn.lock().unwrap();
            diesel::replace_into(factorio_mod::table)
                .values(factorio_mod)
                .execute(conn.deref())?;
            Ok(())
        })
        .await??;

        Ok(())
    }

    pub async fn get_mod_releases(&self, factorio_mod: String) -> anyhow::Result<Vec<ModRelease>> {
        let conn = Arc::clone(&self.conn);
        let result = task::spawn_blocking(move || -> anyhow::Result<Vec<ModRelease>> {
            use schema::mod_release;

            let conn = conn.lock().unwrap();
            Ok(mod_release::table
                .filter(mod_release::factorio_mod.eq(factorio_mod))
                .load::<models::ModRelease>(conn.deref())?)
        })
        .await?;

        Ok(result?)
    }

    pub async fn set_mod_release(&self, release: NewModRelease) -> anyhow::Result<()> {
        let conn = Arc::clone(&self.conn);
        task::spawn_blocking(move || -> anyhow::Result<()> {
            use schema::mod_release;

            let conn = conn.lock().unwrap();
            diesel::replace_into(mod_release::table)
                .values(release)
                .execute(conn.deref())?;
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
            use schema::release_dependency;

            let conn = conn.lock().unwrap();
            Ok(release_dependency::table
                .filter(
                    release_dependency::release_mod_name
                        .eq(release_mod_name)
                        .and(release_dependency::release_version.eq(release_version)),
                )
                .load::<models::ReleaseDependency>(conn.deref())?)
        })
        .await?;

        Ok(result?)
    }

    pub async fn set_release_dependencies(
        &self,
        release: Vec<NewReleaseDependency>,
    ) -> anyhow::Result<()> {
        let conn = Arc::clone(&self.conn);
        task::spawn_blocking(move || -> anyhow::Result<()> {
            use schema::release_dependency;

            let conn = conn.lock().unwrap();
            diesel::replace_into(release_dependency::table)
                .values(release)
                .execute(conn.deref())?;

            Ok(())
        })
        .await??;

        Ok(())
    }
}
