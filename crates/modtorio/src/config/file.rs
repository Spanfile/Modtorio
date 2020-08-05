//! Provides the [File](File) object, which represents Modtorio's config file.

use super::Config;
use crate::{error::ConfigError, ext::PathExt, util::LogLevel};
use serde::{Deserialize, Serialize};
use std::{fs, path::Path};

/// The default cache expiry time in seconds.
const DEFAULT_CACHE_EXPIRY: u64 = 3600;

#[derive(Debug, Deserialize, Serialize, Default)]
pub struct File {
    general: GeneralOptions,
    cache: CacheOptions,
}

#[derive(Debug, Deserialize, Serialize, Default)]
pub struct GeneralOptions {
    log_level: LogLevel,
}

#[derive(Debug, Deserialize, Serialize)]
pub struct CacheOptions {
    expiry: u64,
}

impl File {
    /// Attempts to create a new `File` object from a given path to a config file.
    ///
    /// Returns [`ConfigError::ConfigFileDoesNotExist`](crate::error::ConfigError::
    /// ConfigFileDoesNotExist) if the given path does not exist.
    pub fn from_config_file<P>(path: P) -> anyhow::Result<Self>
    where
        P: AsRef<Path>,
    {
        if path.as_ref().exists() {
            let file_contents = fs::read_to_string(path)?;
            Ok(toml::from_str(&file_contents)?)
        } else {
            Err(ConfigError::ConfigFileDoesNotExist(path.as_ref().get_str()?.to_string()).into())
        }
    }

    /// Attempts to create a new `File` object from a given path to a config file. Will overwrite an
    /// existing file.
    pub fn new_config_file<P>(path: P) -> anyhow::Result<Self>
    where
        P: AsRef<Path>,
    {
        unimplemented!()
    }

    pub fn apply_to_config(self, config: Config) -> Config {
        Config {
            log_level: self.general.log_level,
            cache_expiry: self.cache.expiry,
            ..config
        }
    }
}

impl Default for CacheOptions {
    fn default() -> Self {
        Self {
            expiry: DEFAULT_CACHE_EXPIRY,
        }
    }
}
