pub mod models;
mod schema;

use crate::ext::PathExt;
use diesel::prelude::*;
use models::*;
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
        use schema::game;

        Ok(game::table
            .filter(game::id.eq(id))
            .first::<Game>(&self.conn)?)
    }

    pub fn get_mods_of_game(&self, game: &Game) -> anyhow::Result<Vec<FactorioMod>> {
        use schema::{factorio_mod, game_mod};

        let mod_names = GameMod::belonging_to(game).select(game_mod::factorio_mod);
        Ok(factorio_mod::table
            .filter(factorio_mod::name.eq_any(mod_names))
            .load::<FactorioMod>(&self.conn)?)
    }

    pub fn set_mods_of_game(&self, game_mods: &[NewGameMod]) -> anyhow::Result<()> {
        use schema::game_mod;

        diesel::replace_into(game_mod::table)
            .values(game_mods)
            .execute(&self.conn)?;

        Ok(())
    }

    pub fn insert_game(&self, new_game: NewGame) -> anyhow::Result<i32> {
        use schema::{game, game::dsl};

        diesel::insert_into(game::table)
            .values(&new_game)
            .execute(&self.conn)?;

        Ok(dsl::game
            .order(dsl::id.desc())
            .first::<models::Game>(&self.conn)?
            .id)
    }

    pub fn update_game(&self, id: i32, insert: NewGame) -> anyhow::Result<()> {
        use schema::game::dsl;

        diesel::update(dsl::game.find(id))
            .set(dsl::path.eq(insert.path))
            .execute(&self.conn)?;
        Ok(())
    }

    pub fn set_factorio_mod(&self, factorio_mod: models::NewFactorioMod) -> anyhow::Result<()> {
        use schema::factorio_mod;

        diesel::replace_into(factorio_mod::table)
            .values(factorio_mod)
            .execute(&self.conn)?;

        Ok(())
    }

    pub fn set_mod_release(&self, release: NewModRelease) -> anyhow::Result<()> {
        use schema::mod_release;

        diesel::replace_into(mod_release::table)
            .values(release)
            .execute(&self.conn)?;

        Ok(())
    }

    pub fn set_release_dependencies(&self, release: &[NewReleaseDependency]) -> anyhow::Result<()> {
        use schema::release_dependency;

        diesel::replace_into(release_dependency::table)
            .values(release)
            .execute(&self.conn)?;

        Ok(())
    }
}
