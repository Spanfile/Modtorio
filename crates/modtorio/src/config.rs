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
    // TODO: these with_prefix things break being able to serialise into anything but strings or
    // enums..?
    // #[serde(flatten, with = "prefix_cache")]
    // pub cache: CacheConfig,
    #[serde(default = "default_cache_expiry")]
    pub cache_expiry: u64,
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
    pub expiry: u64,
}

impl Config {
    pub fn from_env() -> anyhow::Result<Config> {
        Ok(envy::prefixed(APP_PREFIX)
            .from_env::<Config>()
            .with_context(|| {
                format!(
                    "Failed to load config from environment variables:\n{}",
                    util::dump_env(APP_PREFIX)
                )
            })?)
    }

    pub fn debug_values(&self) {
        debug!("{:?}", util::dump_env_lines(APP_PREFIX));
        debug!("{:?}", self);
    }
}

fn default_cache_expiry() -> u64 {
    3600
}

#[cfg(test)]
mod tests {
    use super::*;
    use lazy_static::lazy_static;
    use std::{env, sync::Mutex};

    // SO WHAT'S THIS THEN? when running tests with cargo, they all share the same set of
    // environment variables (cargo's) and cargo runs them all in parallel. this means the tests
    // *will* interfere with each other's environment variables. it'd be cool if each had their own
    // set but whatcha gonna do. SO TO FIX IT, you could just run all the test in series on a single
    // thread (cargo test -- --test-threads=1) but that fucks with other tests, slowing things down.
    // instead, all these tests lock this one dummy mutex when starting, and release it when done,
    // so these tests won't ever run in parallel but all other tests will.
    lazy_static! {
        static ref SERIAL_MUTEX: Mutex<()> = Mutex::new(());
    }

    const MODTORIO_LOG_LEVEL: &str = "MODTORIO_LOG_LEVEL";
    const MODTORIO_CACHE_EXPIRY: &str = "MODTORIO_CACHE_EXPIRY";
    const MODTORIO_PORTAL_USERNAME: &str = "MODTORIO_PORTAL_USERNAME";
    const MODTORIO_PORTAL_TOKEN: &str = "MODTORIO_PORTAL_TOKEN";

    #[test]
    fn config_from_env() {
        let _s = SERIAL_MUTEX.lock().expect("failed to lock serial mutex");

        env::set_var(MODTORIO_LOG_LEVEL, "trace");
        env::set_var(MODTORIO_CACHE_EXPIRY, "1");
        env::set_var(MODTORIO_PORTAL_USERNAME, "username");
        env::set_var(MODTORIO_PORTAL_TOKEN, "token");

        println!("{:?}", util::dump_env_lines(APP_PREFIX));
        let config = Config::from_env().unwrap();
        println!("{:?}", config);

        assert_eq!(config.log.level, LogLevel::Trace);
        assert_eq!(config.cache_expiry, 1);
        assert_eq!(config.portal.username, "username");
        assert_eq!(config.portal.token, "token");
    }

    #[test]
    fn default_config() {
        let _s = SERIAL_MUTEX.lock().expect("failed to lock serial mutex");

        // expliclitly unset variables we're expecting aren't set to make sure any values from other
        // tests aren't carried over
        env::remove_var(MODTORIO_LOG_LEVEL);
        env::remove_var(MODTORIO_CACHE_EXPIRY);

        env::set_var(MODTORIO_PORTAL_USERNAME, "value not needed in test");
        env::set_var(MODTORIO_PORTAL_TOKEN, "value not needed in test");

        println!("{:?}", util::dump_env_lines(APP_PREFIX));
        let config = Config::from_env().unwrap();
        println!("{:?}", config);

        assert_eq!(config.cache_expiry, default_cache_expiry());
        assert_eq!(config.log.level, LogLevel::default());
    }
}
