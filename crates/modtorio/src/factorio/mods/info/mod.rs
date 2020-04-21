mod dependency;

use dependency::Dependency;
use serde::Deserialize;
use util::HumanVersion;

#[derive(Debug, Deserialize)]
pub struct Info {
    pub name: String,
    pub version: HumanVersion,
    pub factorio_version: HumanVersion,
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
