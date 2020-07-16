use crate::{
    factorio::GameCacheId,
    mod_common::Requirement,
    util::{HumanVersion, HumanVersionReq},
};
use chrono::{DateTime, Utc};
use derive::Model;

#[derive(Debug, PartialEq, Model)]
pub struct Game {
    #[index]
    #[ignore_in_all_params]
    pub id: GameCacheId,
    pub path: String,
}

#[derive(Debug, PartialEq, Model)]
pub struct FactorioMod {
    #[index]
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

#[derive(Debug, PartialEq, Model)]
pub struct GameMod {
    #[index]
    pub game: GameCacheId,
    pub factorio_mod: String,
    pub mod_version: HumanVersion,
    pub mod_zip: String,
    pub zip_checksum: String,
}

#[derive(Debug, PartialEq, Model)]
pub struct ModRelease {
    #[index]
    pub factorio_mod: String,
    pub version: HumanVersion,
    pub download_url: String,
    pub released_on: DateTime<Utc>,
    pub sha1: String,
    pub factorio_version: HumanVersion,
}

#[derive(Debug, PartialEq, Model)]
pub struct ReleaseDependency {
    #[index]
    pub release_mod_name: String,
    #[index]
    pub release_version: HumanVersion,
    pub name: String,
    pub requirement: Requirement,
    pub version_req: Option<HumanVersionReq>,
}
