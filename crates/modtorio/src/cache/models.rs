use super::schema::{factorio_mod, game, game_mod, mod_release, release_dependency};
use diesel::{Associations, Identifiable, Queryable};

#[derive(Debug, PartialEq, Queryable, Identifiable)]
#[table_name = "game"]
pub struct Game {
    pub id: i32,
    pub path: String,
}

#[derive(Debug, Insertable)]
#[table_name = "game"]
pub struct NewGame {
    pub path: String,
}

#[derive(Debug, PartialEq, Queryable, Identifiable)]
#[primary_key(name)]
#[table_name = "factorio_mod"]
pub struct FactorioMod {
    pub name: String,
    pub summary: Option<String>,
    pub last_updated: String,
}

#[derive(Debug, Insertable)]
#[table_name = "factorio_mod"]
pub struct NewFactorioMod {
    pub name: String,
    pub summary: Option<String>,
    pub last_updated: String,
}

#[derive(Debug, PartialEq, Queryable, Identifiable, Associations)]
#[primary_key(factorio_mod, game)]
#[table_name = "game_mod"]
#[belongs_to(FactorioMod, foreign_key = "factorio_mod")]
#[belongs_to(Game, foreign_key = "game")]
pub struct GameMod {
    pub game: i32,
    pub factorio_mod: String,
}

#[derive(Debug, PartialEq, Insertable)]
#[table_name = "game_mod"]
pub struct NewGameMod {
    pub game: i32,
    pub factorio_mod: String,
}

#[derive(Debug, PartialEq, Queryable, Identifiable)]
#[primary_key(factorio_mod, version)]
#[table_name = "mod_release"]
pub struct ModRelease {
    pub factorio_mod: String,
    pub download_url: String,
    pub file_name: String,
    pub released_on: String,
    pub version: String,
    pub sha1: String,
    pub factorio_version: String,
}

#[derive(Debug, PartialEq, Insertable)]
#[table_name = "mod_release"]
pub struct NewModRelease {
    pub factorio_mod: String,
    pub download_url: String,
    pub file_name: String,
    pub released_on: String,
    pub version: String,
    pub sha1: String,
    pub factorio_version: String,
}

#[derive(Debug, PartialEq, Queryable, Identifiable, Associations)]
#[primary_key(release_mod_name, release_version, name)]
#[table_name = "release_dependency"]
#[belongs_to(
    ModRelease,
    foreign_key = "release_mod_name",
    foreign_key = "release_version"
)]
pub struct ReleaseDependency {
    pub release_mod_name: String,
    pub release_version: String,
    pub name: String,
    pub requirement: i32,
    pub version_req: Option<String>,
}

#[derive(Debug, PartialEq, Insertable)]
#[table_name = "release_dependency"]
pub struct NewReleaseDependency {
    pub release_mod_name: String,
    pub release_version: String,
    pub name: String,
    pub requirement: i32,
    pub version_req: Option<String>,
}
