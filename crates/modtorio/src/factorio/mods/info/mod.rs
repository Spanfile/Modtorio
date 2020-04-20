mod dependency;

use dependency::Dependency;
use semver::Version;
use serde::Deserialize;

#[derive(Debug, Deserialize)]
pub struct Info {
    pub name: String,
    pub version: Version,
    pub factorio_version: Version,
    pub title: String,
    pub author: String,
    pub homepage: String,
    pub dependencies: Vec<Dependency>,
    pub description: String,
}
