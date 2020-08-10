//! Provides the `OptsConfig` object, used to access config values from the running program's
//! command line arguments.

use super::{Config, ConfigSource};
use crate::{opts::Opts, util::LogLevel};
use serde::{Deserialize, Serialize};

/// Contains the config values from the running program's command line arguments.
#[derive(Debug, Deserialize, Serialize, Default)]
pub struct OptsConfig {
    /// Corresponds to the `--log-level` option.
    log_level: Option<LogLevel>,
    /// Corresponds to the `--cache-expiry` option.
    cache_expiry: Option<u64>,
}

impl ConfigSource for OptsConfig {
    /// Applies the contained config values to a given `Config`, returning a new `Config` with the
    /// values set.
    fn apply_to_config(self, config: Config) -> Config {
        Config {
            log_level: self.log_level.unwrap_or(config.log_level),
            cache_expiry: self.cache_expiry.unwrap_or(config.cache_expiry),
            ..config
        }
    }
}

impl OptsConfig {
    /// Returns a new `EnvConfig` built from a given `Opts` object.
    pub fn new(opts: &Opts) -> Self {
        Self {
            log_level: opts.log_level,
            cache_expiry: opts.cache_expiry,
        }
    }
}
