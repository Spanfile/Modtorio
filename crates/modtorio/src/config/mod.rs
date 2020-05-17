mod log_level;

use crate::util;
use anyhow::Context;
use log::*;
pub use log_level::LogLevel;
use serde::Deserialize;
use serde_with::with_prefix;

const APP_PREFIX: &str = "MODTORIO_";

with_prefix!(prefix_portal "portal_");
with_prefix!(prefix_log "log_");
with_prefix!(prefix_cache "cache_");

#[derive(Debug, Deserialize)]
pub struct Config {
    #[serde(flatten, with = "prefix_portal")]
    pub portal: PortalConfig,
    #[serde(flatten, with = "prefix_log")]
    pub log: LogConfig,
    #[serde(flatten, with = "prefix_cache")]
    pub cache: CacheConfig,
}

#[derive(Debug, Deserialize)]
pub struct PortalConfig {
    pub username: String,
    pub token: String,
}

#[derive(Debug, Deserialize)]
pub struct LogConfig {
    #[serde(default)]
    pub level: LogLevel,
}

#[derive(Debug, Deserialize)]
pub struct CacheConfig {
    #[serde(default = "default_cache_expiry")]
    pub expiry: String,
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

fn default_cache_expiry() -> String {
    String::from("3600")
}
