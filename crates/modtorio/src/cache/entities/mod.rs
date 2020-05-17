use crate::{
    mod_common::Requirement,
    util::{HumanVersion, HumanVersionReq},
};
use chrono::{DateTime, Utc};
use rustorm::FromDao;

#[derive(Debug, FromDao)]
pub struct Game {
    pub id: i32,
    pub path: String,
}

#[derive(Debug, FromDao)]
pub struct Mod {
    pub name: String,
    pub summary: String,
    pub last_updated: DateTime<Utc>,
}

#[derive(Debug, FromDao)]
pub struct ModRelease {
    pub id: i32,
    pub mod_name: String,
    pub download_url: String,
    pub file_name: String,
    pub released_on: DateTime<Utc>,
    pub version: HumanVersion,
    pub sha1: String,
    pub factorio_version: HumanVersion,
}

#[derive(Debug, FromDao)]
pub struct ReleaseDependency {
    pub id: i32,
    pub release: i32,
    pub name: String,
    pub requirement: Requirement,
    pub version_req: Option<HumanVersionReq>,
}
