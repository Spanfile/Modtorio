use serde::Deserialize;
use std::path::PathBuf;
use util::HumanVersion;

#[derive(Debug, Deserialize)]
pub struct PortalMod {
    pub downloads_count: u64,
    pub name: String,
    pub owner: String,
    pub releases: Vec<Release>,
    pub summary: String,
    pub title: String,
}

#[derive(Debug, Deserialize)]
pub struct Release {
    pub download_url: PathBuf,
    pub file_name: String,
    pub released_at: String, // TODO: turn into datetime (it's ISO 8601)
    pub version: HumanVersion,
    pub sha1: String,
}
