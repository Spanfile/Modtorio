mod dependency;
mod fact_mod;
mod portal_mod;

use serde::Deserialize;
use util::HumanVersion;

pub use dependency::{Dependency, Requirement};
pub use fact_mod::Mod;
pub use portal_mod::{PortalMod, Release};

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

#[derive(Debug, Deserialize)]
pub struct PortalInfo {
    pub factorio_version: HumanVersion,
    pub dependencies: Vec<Dependency>,
}

fn default_dependencies() -> Vec<Dependency> {
    vec!["base".parse().unwrap()]
}
