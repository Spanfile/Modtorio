mod models;
mod schema;

use crate::ext::PathExt;
use diesel::prelude::*;
pub use models::{Game, GameMod, ReleaseDependency};
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
        }
    }

    pub fn build(self) -> anyhow::Result<Cache> {
        let conn = SqliteConnection::establish(self.db_path.get_str()?)?;

        Ok(Cache { conn })
    }
}

impl Cache {
    pub fn get_game(&self, id: i32) -> anyhow::Result<Game> {
        use schema::game::dsl;
        Ok(dsl::game.filter(dsl::id.eq(id)).first(&self.conn)?)
    }

    pub fn insert_game(&self, path: &str) -> anyhow::Result<i32> {
        use schema::{game, game::dsl};

        let new_game = models::NewGame { path };

        diesel::insert_into(game::table)
            .values(&new_game)
            .execute(&self.conn)?;

        Ok(dsl::game
            .order(dsl::id.desc())
            .first::<Game>(&self.conn)?
            .id)
    }

    pub fn update_game(&self, id: i32, path: &str) -> anyhow::Result<()> {
        use schema::game::dsl;

        diesel::update(dsl::game.find(id))
            .set(dsl::path.eq(path))
            .execute(&self.conn)?;
        Ok(())
    }

    pub fn get_game_mod_of_game(&self, id: i32) -> anyhow::Result<Vec<GameMod>> {
        use schema::game_mod::dsl;

        Ok(dsl::game_mod
            .filter(dsl::game.eq(id))
            .get_results(&self.conn)?)
    }
}
