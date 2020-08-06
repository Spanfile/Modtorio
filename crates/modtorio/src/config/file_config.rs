//! Provides the `FileConfig` object, used to access config values from a config file.

use super::Config;
use crate::util::LogLevel;
use serde::{Deserialize, Serialize};
use std::io::{Read, Write};

/// Contains the config values from a config file.
#[derive(Debug, Deserialize, Serialize, Default)]
pub struct FileConfig {
    /// General config options
    general: GeneralOptions,
    /// Cache config options
    cache: CacheOptions,
}

/// Contains the config values from the `[general]` section of a config file.
#[derive(Debug, Deserialize, Serialize, Default)]
pub struct GeneralOptions {
    /// The log level to use.
    log_level: LogLevel,
}

/// Contains the config values from the `[cache]` section of a config file.
#[derive(Debug, Deserialize, Serialize)]
pub struct CacheOptions {
    /// The program cache expiry in seconds.
    expiry: u64,
}

impl FileConfig {
    /// Writes a config file with all values set to their config defaults to a given writer.
    pub fn write_default_to_writer<W>(writer: &mut W) -> anyhow::Result<()>
    where
        W: Write,
    {
        let default = FileConfig::default();
        let serialised = toml::to_string(&default)?;

        write!(writer, "{}", serialised)?;
        Ok(())
    }

    /// Returns a new `FileConfig` built from a given config file reader.
    pub fn from_file<R>(file: &mut R) -> anyhow::Result<Self>
    where
        R: Read,
    {
        let mut file_contents = String::new();
        file.read_to_string(&mut file_contents)?;
        Ok(toml::from_str(&file_contents)?)
    }

    /// Applies the contained config values to a given `Config`, returning a new `Config` with the
    /// values set.
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
