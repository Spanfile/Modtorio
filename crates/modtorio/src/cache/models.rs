use crate::{
    factorio::GameCacheId,
    mod_common::Requirement,
    util::{HumanVersion, HumanVersionReq},
};
use chrono::{DateTime, Utc};
use derive::Model;

pub trait Model {
    fn select() -> &'static str;
    fn replace_into() -> &'static str;
    fn insert_into() -> &'static str;
    fn update() -> &'static str;
}

#[derive(Debug, PartialEq, Model)]
pub struct Game {
    pub id: GameCacheId,
    pub path: String,
}

#[derive(Debug, PartialEq)]
pub struct FactorioMod {
    pub name: String,
    pub author: String,
    pub contact: Option<String>,
    pub homepage: Option<String>,
    pub title: String,
    pub summary: Option<String>,
    pub description: String,
    pub changelog: Option<String>,
    pub last_updated: DateTime<Utc>,
}

#[derive(Debug, PartialEq)]
pub struct GameMod {
    pub game: GameCacheId,
    pub factorio_mod: String,
    pub mod_version: HumanVersion,
    pub mod_zip: String,
    pub zip_checksum: String,
}

#[derive(Debug, PartialEq)]
pub struct ModRelease {
    pub factorio_mod: String,
    pub download_url: String,
    pub released_on: DateTime<Utc>,
    pub version: HumanVersion,
    pub sha1: String,
    pub factorio_version: HumanVersion,
}

#[derive(Debug, PartialEq)]
pub struct ReleaseDependency {
    pub release_mod_name: String,
    pub release_version: HumanVersion,
    pub name: String,
    pub requirement: Requirement,
    pub version_req: Option<HumanVersionReq>,
}
