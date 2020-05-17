pub mod entities;

use log::*;
use rustorm::{EntityManager, Pool};
use std::{
    cell::RefCell,
    path::{Path, PathBuf},
};

const CACHE_DB_SCHEMA: &str = include_str!("schema.sql");

pub struct Cache {
    em: RefCell<EntityManager>,
}

#[derive(Debug)]
pub struct CacheBuilder {
    db_path: PathBuf,
    schema: String,
}

impl CacheBuilder {
    pub fn new() -> Self {
        Self {
            db_path: PathBuf::from("modtorio.db"),
            schema: String::from(CACHE_DB_SCHEMA),
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

    pub fn with_db_schema(self, schema: &str) -> Self {
        Self {
            schema: String::from(schema),
            ..self
        }
    }

    pub fn build(self) -> anyhow::Result<Cache> {
        let exists = self.db_path.exists();
        let db_url = format!("sqlite://{}", self.db_path.display());
        let mut pool = Pool::new();
        let em = pool.em(&db_url)?;

        if !exists {
            debug!("New database {} created, applying schema", db_url);
            em.execute_sql_with_return(self.schema);
        }

        Ok(Cache {
            em: RefCell::new(em),
        })
    }
}

impl Cache {
    fn one<'a, T>(&self, sql: &str, params: &[&'a dyn rustorm::ToValue]) -> anyhow::Result<T>
    where
        T: rustorm::dao::FromDao,
    {
        Ok(self
            .em
            .borrow_mut()
            .execute_sql_with_one_return(sql, params)?)
    }

    fn maybe_one<'a, T>(
        &self,
        sql: &str,
        params: &[&'a dyn rustorm::ToValue],
    ) -> anyhow::Result<Option<T>>
    where
        T: rustorm::dao::FromDao,
    {
        Ok(self
            .em
            .borrow_mut()
            .execute_sql_with_maybe_one_return(sql, params)?)
    }

    fn many<'a, T>(&self, sql: &str, params: &[&'a dyn rustorm::ToValue]) -> anyhow::Result<Vec<T>>
    where
        T: rustorm::dao::FromDao,
    {
        Ok(self.em.borrow_mut().execute_sql_with_return(sql, params)?)
    }

    pub fn get_game(&self, game_id: i32) -> anyhow::Result<entities::Game> {
        let sql = "SELECT * FROM game WHERE game.id = ?";
        self.one(sql, &[&game_id])
    }

    pub fn get_mod(&self, name: &str) -> anyhow::Result<Option<entities::Mod>> {
        let sql = "SELECT * FROM mod WHERE mod.name = ?";
        self.maybe_one(sql, &[&name.to_owned()])
    }

    pub fn get_releases_of_mod(&self, name: &str) -> anyhow::Result<Vec<entities::ModRelease>> {
        let sql = "SELECT * FROM mod_release WHERE mod_release.mod_name = ?";
        self.many(sql, &[&name.to_owned()])
    }

    pub fn get_dependencies_of_release(
        &self,
        release: i32,
    ) -> anyhow::Result<Vec<entities::ReleaseDependency>> {
        let sql = "SELECT * FROM release_dependency WHERE release_dependency.release = ?";
        self.many(sql, &[&release])
    }
}
