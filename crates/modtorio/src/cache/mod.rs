pub mod models;
mod schema;

use crate::ext::PathExt;
use diesel::prelude::*;
use std::{
    env,
    path::{Path, PathBuf},
};

pub struct Cache {
    conn: SqliteConnection,
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
            ..self
        }
    }

    pub fn build(self) -> anyhow::Result<Cache> {
        let conn = SqliteConnection::establish(self.db_path.get_str()?)?;

        Ok(Cache { conn })
    }
}

impl Cache {
    pub fn get_game(&self, game_id: i32) -> anyhow::Result<models::Game> {
        use schema::game::dsl::*;
        Ok(game.filter(id.eq(game_id)).first(&self.conn)?)
    }

    pub fn get_mod(&self, n: &str) -> anyhow::Result<Option<models::GameMod>> {
        use schema::game_mod::dsl::*;
        Ok(game_mod.filter(name.eq(n)).first(&self.conn).optional()?)
    }

    pub fn get_releases_of_mod(&self, name: &str) -> anyhow::Result<Vec<models::ModRelease>> {
        unimplemented!()
    }

    pub fn get_dependencies_of_release(
        &self,
        release: i32,
    ) -> anyhow::Result<Vec<models::ReleaseDependency>> {
        unimplemented!()
    }
}
