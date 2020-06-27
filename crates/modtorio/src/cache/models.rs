use crate::factorio::GameCacheId;

#[derive(Debug, PartialEq)]
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
    pub version: String,
    pub factorio_version: String,
    pub last_updated: String,
}

#[derive(Debug, PartialEq)]
pub struct GameMod {
    pub game: GameCacheId,
    pub factorio_mod: String,
}

#[derive(Debug, PartialEq)]
pub struct ModRelease {
    pub factorio_mod: String,
    pub download_url: String,
    pub released_on: String,
    pub version: String,
    pub sha1: String,
    pub factorio_version: String,
}

#[derive(Debug, PartialEq)]
pub struct ReleaseDependency {
    pub release_mod_name: String,
    pub release_version: String,
    pub name: String,
    pub requirement: i32,
    pub version_req: Option<String>,
}
