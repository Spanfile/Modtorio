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
    pub homepage: String,
    pub dependencies: Vec<Dependency>,
    pub description: String,
}
