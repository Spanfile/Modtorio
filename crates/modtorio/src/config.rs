//! Provides the [`Config`](Config) object, a collection of configuration values for the program.

mod log_level;

use crate::util;
use anyhow::Context;
use log::*;
pub use log_level::LogLevel;
use serde::Deserialize;
use serde_with::with_prefix;

/// The prefix used with every environment value related to the program configuration.
const APP_PREFIX: &str = "MODTORIO_";

// TODO: hide these from the docs. a simple #[doc(hidden)] doesn't seem to work
with_prefix!(prefix_portal "portal_");
with_prefix!(prefix_log "log_");
with_prefix!(prefix_cache "cache_");

/// A collection of configuration values.
///
/// A `Config` is built from the environment variables using the [`from_env`](#method.from_env)
/// function.
#[derive(Debug, Deserialize)]
pub struct Config {
    /// Configuration values related to the mod portal.
    #[serde(flatten, with = "prefix_portal")]
    pub portal: PortalConfig,
    /// Configuration values related to logging.
    #[serde(flatten, with = "prefix_log")]
    pub log: LogConfig,
    // TODO: these with_prefix things break being able to serialise into anything but strings or
    // enums..?
    // #[serde(flatten, with = "prefix_cache")]
    // pub cache: CacheConfig,
    /// The cache entry expiry in seconds. Defaults to 3600 seconds (1 hour).
    #[serde(default = "default_cache_expiry")]
    pub cache_expiry: u64,
}

/// Configuration values related to the mod portal.
#[derive(Debug, Deserialize)]
pub struct PortalConfig {
    /// The username to use with the mod portal.
    pub username: String,
    /// The token to use with the mod portal.
    pub token: String,
}

/// Configuration values related logging.
#[derive(Debug, Deserialize)]
pub struct LogConfig {
    /// The log level to use.
    #[serde(default)]
    pub level: LogLevel,
}

/// Configuration values related to the program cache.
#[derive(Debug, Deserialize)]
pub struct CacheConfig {
    /// The cache entry expiry in seconds. Defaults to 3600 seconds (1 hour).
    #[serde(default = "default_cache_expiry")]
    pub expiry: u64,
}

impl Config {
    /// Builds a new `Config` object from the current environment variables prefixed by
    /// [`APP_PREFIX`](./constant.APP_PREFIX.html). Returns an error if the config object fails to
    /// be deserialized from the environment variables.
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

    /// Prints debug information about the environment variables and self.
    pub fn debug_values(&self) {
        debug!("{:?}", util::dump_env_lines(APP_PREFIX));
        debug!("{:?}", self);
    }
}

#[doc(hidden)]
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
