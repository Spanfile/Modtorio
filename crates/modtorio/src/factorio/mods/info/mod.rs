mod dependency;

use dependency::Dependency;
use serde::Deserialize;

#[derive(Debug, Deserialize)]
pub struct Info {
    pub name: String,
    // TODO: the semver crate is cool and all but it errors if the parse input isn't 100% up to
    // spec. user input sure as fuck is not up to spec
    pub version: String,
    pub factorio_version: String,
    pub title: String,
    pub author: String,
    #[serde(default)]
    pub contact: String,
    #[serde(default)]
    pub homepage: String,
    #[serde(default = "default_dependencies")]
    pub dependencies: Vec<Dependency>,
    #[serde(default)]
    pub description: String,
}

fn default_dependencies() -> Vec<Dependency> {
    vec!["base".parse().unwrap()]
}
