use anyhow::Context;
use log::*;
use serde::Deserialize;
use serde_with::with_prefix;

const APP_PREFIX: &str = "MODTORIO_";

with_prefix!(prefix_portal "portal_");

#[derive(Debug, Deserialize)]
pub struct Config {
    #[serde(flatten, with = "prefix_portal")]
    pub portal: PortalConfig,
}

#[derive(Debug, Deserialize)]
pub struct PortalConfig {
    pub username: String,
    pub token: String,
}

impl Config {
    pub fn from_env() -> anyhow::Result<Config> {
        debug!("{:?}", util::dump_env_lines(APP_PREFIX));
        Ok(envy::prefixed(APP_PREFIX)
            .from_env::<Config>()
            .with_context(|| {
                format!(
                    "Failed to load Config from environment variables.\nConfig env:\n{}",
                    util::dump_env(APP_PREFIX)
                )
            })?)
    }
}
