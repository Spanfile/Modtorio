pub mod models;
mod schema;

use crate::ext::PathExt;
use diesel::prelude::*;
use models::*;
use std::{
    env,
    ops::Deref,
    path::{Path, PathBuf},
    sync::{Arc, Mutex},
};
use tokio::task;

pub struct Cache {
    conn: Arc<Mutex<SqliteConnection>>,
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
        let conn = SqliteConnection::establish(self.db_path.get_str()?)?;

        Ok(Cache {
            conn: Arc::new(Mutex::new(conn)),
        })
    }
}

impl Cache {
    pub async fn get_game_ids(&self) -> anyhow::Result<Vec<i32>> {
        let conn = Arc::clone(&self.conn);
        let result = task::spawn_blocking(move || -> anyhow::Result<Vec<i32>> {
            use schema::game;

            let conn = conn.lock().unwrap();
            Ok(game::table.select(game::id).load::<i32>(conn.deref())?)
        })
        .await?;

        Ok(result?)
    }

    pub async fn get_game(&self, id: i32) -> anyhow::Result<Game> {
        let conn = Arc::clone(&self.conn);
        let result = task::spawn_blocking(move || -> anyhow::Result<Game> {
            use schema::game;

            let conn = conn.lock().unwrap();
            Ok(game::table
                .filter(game::id.eq(id))
                .first::<Game>(conn.deref())?)
        })
        .await?;

        Ok(result?)
    }

    pub async fn get_mods_of_game(&self, game: Game) -> anyhow::Result<Vec<FactorioMod>> {
        let conn = Arc::clone(&self.conn);
        let result = task::spawn_blocking(move || -> anyhow::Result<Vec<FactorioMod>> {
            use schema::{factorio_mod, game_mod};

            let conn = conn.lock().unwrap();
            let mod_names = GameMod::belonging_to(&game).select(game_mod::factorio_mod);
            Ok(factorio_mod::table
                .filter(factorio_mod::name.eq_any(mod_names))
                .load::<FactorioMod>(conn.deref())?)
        })
        .await?;

        Ok(result?)
    }

    pub async fn set_mods_of_game(&self, game_mods: Vec<NewGameMod>) -> anyhow::Result<()> {
        let conn = Arc::clone(&self.conn);
        task::spawn_blocking(move || -> anyhow::Result<()> {
            use schema::game_mod;

            let conn = conn.lock().unwrap();
            diesel::replace_into(game_mod::table)
                .values(&game_mods)
                .execute(conn.deref())?;
            Ok(())
        })
        .await??;

        Ok(())
    }

    pub async fn insert_game(&self, new_game: NewGame) -> anyhow::Result<i32> {
        let conn = Arc::clone(&self.conn);
        let result = task::spawn_blocking(move || -> anyhow::Result<i32> {
            use schema::{game, game::dsl};

            let conn = conn.lock().unwrap();
            diesel::insert_into(game::table)
                .values(&new_game)
                .execute(conn.deref())?;

            Ok(dsl::game
                .order(dsl::id.desc())
                .first::<models::Game>(conn.deref())?
                .id)
        })
        .await?;

        Ok(result?)
    }

    pub async fn update_game(&self, id: i32, insert: NewGame) -> anyhow::Result<()> {
        let conn = Arc::clone(&self.conn);
        task::spawn_blocking(move || -> anyhow::Result<()> {
            use schema::game::dsl;

            let conn = conn.lock().unwrap();
            diesel::update(dsl::game.find(id))
                .set(dsl::path.eq(insert.path))
                .execute(conn.deref())?;
            Ok(())
        })
        .await??;

        Ok(())
    }

    pub async fn get_factorio_mod(
        &self,
        factorio_mod: String,
    ) -> anyhow::Result<Option<models::FactorioMod>> {
        let conn = Arc::clone(&self.conn);
        let result =
            task::spawn_blocking(move || -> anyhow::Result<Option<models::FactorioMod>> {
                use schema::factorio_mod;

                let conn = conn.lock().unwrap();
                Ok(factorio_mod::table
                    .filter(factorio_mod::name.eq(factorio_mod))
                    .first::<models::FactorioMod>(conn.deref())
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

    pub async fn get_mod_releases(
        &self,
        factorio_mod: String,
    ) -> anyhow::Result<Vec<models::ModRelease>> {
        let conn = Arc::clone(&self.conn);
        let result = task::spawn_blocking(move || -> anyhow::Result<Vec<models::ModRelease>> {
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
    ) -> anyhow::Result<Vec<models::ReleaseDependency>> {
        let conn = Arc::clone(&self.conn);
        let result =
            task::spawn_blocking(move || -> anyhow::Result<Vec<models::ReleaseDependency>> {
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
