//! The configuration framework for Modtorio.

mod env_config;
mod file_config;
mod store_config;

use crate::{error::ConfigError, opts::Opts, store::Store, util};
use env_config::EnvConfig;
use file_config::FileConfig;
use serde::Deserialize;
use std::path::Path;
use store_config::StoreConfig;
use util::LogLevel;

pub const DEFAULT_CONFIG_FILE_LOCATION: &str = "modtorio.toml";
pub const DEFAULT_STORE_FILE_LOCATION: &str = "modtorio.db";

#[derive(Debug, Deserialize, Default)]
pub struct Config {
    log_level: LogLevel,
    portal_username: String,
    portal_token: String,
    cache_expiry: u64,
}

impl Config {
    fn get_file_config<P>(path: P) -> anyhow::Result<FileConfig>
    where
        P: AsRef<Path>,
    {
        match FileConfig::from_config_file(path.as_ref()) {
            Ok(file) => Ok(file),
            Err(e) => {
                if let Some(ConfigError::ConfigFileDoesNotExist(_)) = e.downcast_ref() {
                    FileConfig::new_config_file(path.as_ref())?;
                    FileConfig::from_config_file(path.as_ref())
                } else {
                    Err(e)
                }
            }
        }
    }

    fn get_env_config() -> anyhow::Result<EnvConfig> {
        if cfg!(debug_assertions) {
            dotenv::dotenv()?;
        }

        Ok(EnvConfig::from_env()?)
    }

    async fn get_store_config(store: &Store) -> anyhow::Result<StoreConfig> {
        Ok(StoreConfig::from_store(store).await?)
    }

    pub async fn build(opts: &Opts, store: &Store) -> anyhow::Result<Self> {
        let config = Config::default();

        let file = Config::get_file_config(&opts.config)?;
        let config = file.apply_to_config(config);

        let store = Config::get_store_config(store).await?;
        let config = store.apply_to_config(config);

        let config = if opts.no_env {
            config
        } else {
            let env = Config::get_env_config()?;
            env.apply_to_config(config)
        };

        Ok(config)
    }

    pub fn log_level(&self) -> LogLevel {
        self.log_level
    }

    pub fn portal_username(&self) -> &str {
        &self.portal_username
    }

    pub fn portal_token(&self) -> &str {
        &self.portal_token
    }

    pub fn cache_expiry(&self) -> u64 {
        self.cache_expiry
    }
}

// #[cfg(test)]
// mod tests {
//     use super::*;
//     use lazy_static::lazy_static;
//     use std::{env, sync::Mutex};

//     // SO WHAT'S THIS THEN? when running tests with cargo, they all share the same set of
//     // environment variables (cargo's) and cargo runs them all in parallel. this means the tests
//     // *will* interfere with each other's environment variables. it'd be cool if each had their
// own     // set but whatcha gonna do. SO TO FIX IT, you could just run all the test in series on a
// single     // thread (cargo test -- --test-threads=1) but that fucks with other tests, slowing
// things down.     // instead, all these tests lock this one dummy mutex when starting, and release
// it when done,     // so these tests won't ever run in parallel but all other tests will.
//     lazy_static! {
//         static ref SERIAL_MUTEX: Mutex<()> = Mutex::new(());
//     }

//     const MODTORIO_LOG_LEVEL: &str = "MODTORIO_LOG_LEVEL";
//     const MODTORIO_CACHE_EXPIRY: &str = "MODTORIO_CACHE_EXPIRY";
//     const MODTORIO_PORTAL_USERNAME: &str = "MODTORIO_PORTAL_USERNAME";
//     const MODTORIO_PORTAL_TOKEN: &str = "MODTORIO_PORTAL_TOKEN";

//     #[test]
//     fn config_from_env() {
//         let _s = SERIAL_MUTEX.lock().expect("failed to lock serial mutex");

//         env::set_var(MODTORIO_LOG_LEVEL, "trace");
//         env::set_var(MODTORIO_CACHE_EXPIRY, "1");
//         env::set_var(MODTORIO_PORTAL_USERNAME, "username");
//         env::set_var(MODTORIO_PORTAL_TOKEN, "token");

//         println!("{:?}", util::dump_env_lines(APP_PREFIX));
//         let config = Config::from_env().unwrap();
//         println!("{:?}", config);

//         assert_eq!(config.log.level, LogLevel::Trace);
//         assert_eq!(config.cache_expiry, 1);
//         assert_eq!(config.portal.username, "username");
//         assert_eq!(config.portal.token, "token");
//     }

//     #[test]
//     fn default_config() {
//         let _s = SERIAL_MUTEX.lock().expect("failed to lock serial mutex");

//         // expliclitly unset variables we're expecting aren't set to make sure any values from
// other         // tests aren't carried over
//         env::remove_var(MODTORIO_LOG_LEVEL);
//         env::remove_var(MODTORIO_CACHE_EXPIRY);

//         env::set_var(MODTORIO_PORTAL_USERNAME, "value not needed in test");
//         env::set_var(MODTORIO_PORTAL_TOKEN, "value not needed in test");

//         println!("{:?}", util::dump_env_lines(APP_PREFIX));
//         let config = Config::from_env().unwrap();
//         println!("{:?}", config);

//         assert_eq!(config.cache_expiry, default_cache_expiry());
//         assert_eq!(config.log.level, LogLevel::default());
//     }
// }
