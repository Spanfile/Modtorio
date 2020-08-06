//! Provides the [File](File) object, which represents Modtorio's config file.

use super::Config;
use crate::util::LogLevel;
use log::*;
use serde::{Deserialize, Serialize};
use std::io::{Read, Write};

#[derive(Debug, Deserialize, Serialize, Default)]
pub struct FileConfig {
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

impl FileConfig {
    pub fn write_default_to_writer<W>(writer: &mut W) -> anyhow::Result<()>
    where
        W: Write,
    {
        let default = FileConfig::default();
        let serialised = toml::to_string(&default)?;

        debug!("Default config file:\n{}", serialised);
        write!(writer, "{}", serialised)?;
        Ok(())
    }

    /// Attempts to create a new `File` object from a given path to a config file.
    ///
    /// Returns [`ConfigError::ConfigFileDoesNotExist`](crate::error::ConfigError::
    /// ConfigFileDoesNotExist) if the given path does not exist.
    pub fn from_file<R>(file: &mut R) -> anyhow::Result<Self>
    where
        R: Read,
    {
        let mut file_contents = String::new();
        file.read_to_string(&mut file_contents)?;
        Ok(toml::from_str(&file_contents)?)
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
            expiry: super::DEFAULT_CACHE_EXPIRY,
        }
    }
}
