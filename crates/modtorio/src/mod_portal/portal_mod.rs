use anyhow::anyhow;
use chrono::{DateTime, Utc};
use serde::Deserialize;
use std::path::PathBuf;
use util::HumanVersion;

#[derive(Debug, Deserialize)]
pub struct PortalMod {
    pub downloads_count: u64,
    pub name: String,
    pub owner: String,
    releases: Vec<Release>,
    pub summary: String,
    pub title: String,
}

#[derive(Debug, Deserialize)]
pub struct Release {
    pub download_url: PathBuf,
    pub file_name: String,
    #[serde(rename = "released_at")]
    pub released_on: DateTime<Utc>,
    pub version: HumanVersion,
    pub sha1: String,
}

impl PortalMod {
    pub fn get_release(&self, version: Option<HumanVersion>) -> anyhow::Result<&Release> {
        match version {
            Some(version) => self
                .releases
                .iter()
                .find(|release| release.version == version)
                .ok_or_else(|| {
                    anyhow!(
                        "Mod '{}' doesn't have a release matching version {}",
                        self.title,
                        version
                    )
                }),
            None => self
                .releases
                .last()
                .ok_or_else(|| anyhow!("mod {} doesn't have any releases", self.title)),
        }
    }
}
