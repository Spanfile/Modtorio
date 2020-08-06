use super::Config;
use crate::{opts::Opts, util::LogLevel};
use serde::{Deserialize, Serialize};

#[derive(Debug, Deserialize, Serialize, Default)]
pub struct OptsConfig {
    log_level: Option<LogLevel>,
    cache_expiry: Option<u64>,
}

impl OptsConfig {
    pub fn from_opts(opts: &Opts) -> Self {
        Self {
            log_level: opts.log_level,
            cache_expiry: opts.cache_expiry,
        }
    }

    pub fn apply_to_config(self, config: Config) -> Config {
        Config {
            log_level: self.log_level.unwrap_or(config.log_level),
            cache_expiry: self.cache_expiry.unwrap_or(config.cache_expiry),
            ..config
        }
    }
}
