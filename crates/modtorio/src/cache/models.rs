#[derive(Debug, PartialEq)]
pub struct Game {
    pub id: i32,
    pub path: String,
}

#[derive(Debug)]
pub struct NewGame {
    pub path: String,
}

#[derive(Debug, PartialEq)]
pub struct FactorioMod {
    pub name: String,
    pub summary: Option<String>,
    pub last_updated: String,
}

#[derive(Debug)]
pub struct NewFactorioMod {
    pub name: String,
    pub summary: Option<String>,
    pub last_updated: String,
}

#[derive(Debug, PartialEq)]
pub struct GameMod {
    pub game: i32,
    pub factorio_mod: String,
}

#[derive(Debug, PartialEq)]
pub struct NewGameMod {
    pub game: i32,
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
pub struct NewModRelease {
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

#[derive(Debug, PartialEq)]
pub struct NewReleaseDependency {
    pub release_mod_name: String,
    pub release_version: String,
    pub name: String,
    pub requirement: i32,
    pub version_req: Option<String>,
}
