use diesel::Queryable;

#[derive(Debug, Queryable)]
pub struct Game {
    pub id: i32,
    pub path: String,
}

#[derive(Debug, Queryable)]
pub struct GameMod {
    pub name: String,
    pub summary: Option<String>,
    pub last_updated: String,
}

#[derive(Debug, Queryable)]
pub struct ModRelease {
    pub id: i32,
    pub mod_name: String,
    pub download_url: String,
    pub file_name: String,
    pub released_on: String,
    pub version: String,
    pub sha1: String,
    pub factorio_version: String,
}

#[derive(Debug, Queryable)]
pub struct ReleaseDependency {
    pub id: i32,
    pub release: i32,
    pub name: String,
    pub requirement: String,
    pub version_req: Option<String>,
}
