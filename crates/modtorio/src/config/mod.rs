mod log_level;

use crate::util;
use anyhow::Context;
use log::*;
pub use log_level::LogLevel;
use serde::Deserialize;
use serde_with::with_prefix;

const APP_PREFIX: &str = "MODTORIO_";

with_prefix!(prefix_portal "portal_");

#[derive(Debug, Deserialize)]
pub struct Config {
    #[serde(flatten, with = "prefix_portal")]
    pub portal: PortalConfig,
    #[serde(default)]
    pub log_level: LogLevel,
}

#[derive(Debug, Deserialize)]
pub struct PortalConfig {
    pub username: String,
    pub token: String,
}

impl Config {
    pub fn from_env() -> anyhow::Result<Config> {
        Ok(envy::prefixed(APP_PREFIX)
            .from_env::<Config>()
            .with_context(|| {
                format!(
                    "Failed to load Config from environment variables.\nConfig env:\n{}",
                    util::dump_env(APP_PREFIX)
                )
            })?)
    }

    pub fn debug_values(&self) {
        debug!("{:?}", util::dump_env_lines(APP_PREFIX));
        debug!("{:?}", self);
    }
}
