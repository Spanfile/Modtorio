//! Provides the `FileConfig` object, used to access config values from a config file.

use super::{Config, ConfigSource, DEFAULT_STORE_EXPIRY};
use crate::util::LogLevel;
use common::net::NetAddress;
use serde::{Deserialize, Serialize};
use std::io::{Read, Write};

/// Contains the config values from a config file.
#[derive(Debug, Deserialize, Serialize, Default)]
pub struct FileConfig {
    /// Debug config options
    #[serde(default)]
    debug: DebugOptions,
    /// Store config options
    #[serde(default)]
    store: StoreOptions,
    /// Network config options
    network: NetworkOptions,
}

/// Contains the config values from the `[general]` section of a config file.
#[derive(Debug, Deserialize, Serialize, Default)]
pub struct DebugOptions {
    /// The log level to use.
    log_level: LogLevel,
}

// TODO: does this have to be its own thing or just stick it in debug options?
/// Contains the config values from the `[store]` section of a config file.
#[derive(Debug, Deserialize, Serialize)]
pub struct StoreOptions {
    /// The program store expiry in seconds.
    expiry: u64,
}

/// Contains the config values from the `[network]` section of a config file.
#[derive(Debug, Deserialize, Serialize, Default)]
pub struct NetworkOptions {
    /// The server listen addresses
    listen: Vec<NetAddress>,
}

impl ConfigSource for FileConfig {
    /// Applies the contained config values to a given `Config`, returning a new `Config` with the
    /// values set.
    fn apply_to_config(self, config: Config) -> Config {
        Config {
            log_level: self.debug.log_level,
            store_expiry: self.store.expiry,
            listen: self.network.listen,
            ..config
        }
    }
}

impl FileConfig {
    /// Returns a new `FileConfig` built from a given config file reader.
    pub fn new<R>(file: &mut R) -> anyhow::Result<Self>
    where
        R: Read,
    {
        let mut file_contents = String::new();
        file.read_to_string(&mut file_contents)?;
        Ok(toml::from_str(&file_contents)?)
    }

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
}

impl Default for StoreOptions {
    fn default() -> Self {
        Self {
            expiry: DEFAULT_STORE_EXPIRY,
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::{io::Cursor, path::PathBuf};

    #[test]
    fn full() {
        let contents = String::from(
            r#"[debug]
log_level = "trace"
[store]
expiry = 60
[network]
listen = ["0.0.0.0:1337", "unix:/temp/path"]"#,
        );
        let mut contents = Cursor::new(contents.into_bytes());
        let config = FileConfig::new(&mut contents).expect("failed to create FileConfig");

        assert_eq!(config.debug.log_level, LogLevel::Trace);
        assert_eq!(config.store.expiry, 60);
        assert_eq!(
            config.network.listen,
            vec![
                NetAddress::TCP(std::net::SocketAddr::new(
                    std::net::IpAddr::V4(std::net::Ipv4Addr::new(0, 0, 0, 0)),
                    1337
                )),
                NetAddress::Unix(PathBuf::from("/temp/path")),
            ]
        )
    }

    #[test]
    fn required() {
        let contents = String::new();
        let mut contents = Cursor::new(contents.into_bytes());

        assert!(FileConfig::new(&mut contents).is_err());
    }

    #[test]
    fn default() {
        let contents = String::from(
            r#"[network]
listen = ["0.0.0.0:1337"]"#,
        );
        let mut contents = Cursor::new(contents.into_bytes());
        let config = FileConfig::new(&mut contents).expect("failed to create FileConfig");

        assert_eq!(config.debug.log_level, LogLevel::default());
        assert_eq!(config.store.expiry, DEFAULT_STORE_EXPIRY);
    }
}
