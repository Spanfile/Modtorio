//! The configuration framework for Modtorio.

mod env_config;
mod file_config;
mod store_config;

use crate::{store::Store, util};
use env_config::EnvConfig;
use file_config::FileConfig;
use serde::Deserialize;
use std::io::Read;
use store_config::StoreConfig;
use util::LogLevel;

pub const DEFAULT_CONFIG_FILE_LOCATION: &str = "modtorio.toml";
pub const DEFAULT_STORE_FILE_LOCATION: &str = "modtorio.db";
/// The default cache expiry time in seconds.
pub const DEFAULT_CACHE_EXPIRY: u64 = 3600;

#[derive(Debug, Deserialize, Default)]
pub struct Config {
    log_level: LogLevel,
    portal_username: String,
    portal_token: String,
    cache_expiry: u64,
}

pub struct Builder {
    config: Config,
}

impl Builder {
    pub fn new() -> Builder {
        Builder {
            config: Config::default(),
        }
    }

    fn get_env_config() -> anyhow::Result<EnvConfig> {
        if cfg!(debug_assertions) {
            dotenv::dotenv()?;
        }

        Ok(EnvConfig::from_env()?)
    }

    pub fn apply_config_file<R>(self, file: &mut R) -> anyhow::Result<Self>
    where
        R: Read,
    {
        let file_config = FileConfig::from_file(file)?;
        Ok(Builder {
            config: file_config.apply_to_config(self.config),
        })
    }

    pub fn apply_env(self) -> anyhow::Result<Self> {
        let env_config = Builder::get_env_config()?;
        Ok(Builder {
            config: env_config.apply_to_config(self.config),
        })
    }

    pub async fn apply_store(self, store: &Store) -> anyhow::Result<Self> {
        let store_config = StoreConfig::from_store(store).await?;
        Ok(Builder {
            config: store_config.apply_to_config(self.config),
        })
    }

    pub fn build(self) -> Config {
        self.config
    }
}

impl Config {
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

#[cfg(test)]
mod tests {
    use super::*;
    use crate::store::option;
    use lazy_static::lazy_static;
    use std::{
        env,
        fs::File,
        io::{Seek, SeekFrom, Write},
        sync::Mutex,
    };

    // when running tests with cargo, they all share the same set of environment variables (cargo's)
    // and cargo runs them all in parallel. this means the tests *will* interfere with each other's
    // environment variables. it'd be cool if each had their own set but whatcha gonna do. so to fix
    // it, you could just run all the test in series on a single thread (cargo test --
    // --test-threads=1) but that fucks with other tests, slowing things down. instead, all these
    // tests lock this one dummy mutex when starting, and release it when done, so these tests won't
    // ever run in parallel but all other tests will.
    lazy_static! {
        static ref SERIAL_MUTEX: Mutex<()> = Mutex::new(());
    }

    const MODTORIO_PORTAL_USERNAME: &str = "MODTORIO_PORTAL_USERNAME";
    const MODTORIO_PORTAL_TOKEN: &str = "MODTORIO_PORTAL_TOKEN";
    const ENV_USERNAME: &str = "env_username";
    const ENV_TOKEN: &str = "env_token";
    const STORE_USERNAME: &str = "store_username";
    const STORE_TOKEN: &str = "store_token";

    fn temp_config_file(contents: &str) -> File {
        let mut temp = tempfile::tempfile().expect("failed to open tempfile");
        write!(&temp, "{}", contents).expect("failed to write contents into tempfile");
        temp.seek(SeekFrom::Start(0))
            .expect("failed to seek tempfile back to start");
        temp
    }

    fn temp_env() {
        env::set_var(MODTORIO_PORTAL_USERNAME, ENV_USERNAME);
        env::set_var(MODTORIO_PORTAL_TOKEN, ENV_TOKEN);
    }

    async fn temp_store() -> Store {
        let store = Store::build(crate::store::MEMORY_STORE.into())
            .await
            .expect("failed to build store");
        store
            .set_option(option::Value {
                field: option::Field::PortalUsername,
                value: Some(String::from(STORE_USERNAME)),
            })
            .await
            .expect("failed to store portal username");
        store
            .set_option(option::Value {
                field: option::Field::PortalToken,
                value: Some(String::from(STORE_TOKEN)),
            })
            .await
            .expect("failed to store portal token");
        store
    }

    #[test]
    fn config_from_file() {
        let mut f = temp_config_file(
            r#"
            [general]
            log_level = "trace"
            [cache]
            expiry = 1337
        "#,
        );

        let config = Builder::new()
            .apply_config_file(&mut f)
            .expect("failed to apply config file to builder")
            .build();
        println!("{:?}", config);

        assert_eq!(config.log_level, LogLevel::Trace);
        assert_eq!(config.cache_expiry, 1337);
    }

    #[test]
    fn config_from_env() {
        let _s = SERIAL_MUTEX.lock().expect("failed to lock serial mutex");
        temp_env();

        println!("{:?}", util::env::dump_lines(crate::APP_PREFIX));
        let config = Builder::new()
            .apply_env()
            .expect("failed to apply env to builder")
            .build();
        println!("{:?}", config);

        assert_eq!(config.portal_username, ENV_USERNAME);
        assert_eq!(config.portal_token, ENV_TOKEN);
    }

    #[tokio::test]
    async fn config_from_store() {
        let store = temp_store().await;

        let config = Builder::new()
            .apply_store(&store)
            .await
            .expect("failed to apply store to builder")
            .build();
        println!("{:?}", config);

        assert_eq!(config.portal_username, STORE_USERNAME);
        assert_eq!(config.portal_token, STORE_TOKEN);
    }

    #[tokio::test]
    async fn full_config() {
        let _s = SERIAL_MUTEX.lock().expect("failed to lock serial mutex");

        let store = temp_store().await;
        let mut f = temp_config_file(
            r#"
            [general]
            log_level = "trace"
            [cache]
            expiry = 1337
        "#,
        );
        temp_env();

        let config = Builder::new()
            .apply_config_file(&mut f)
            .expect("failed to apply config file")
            .apply_store(&store)
            .await
            .expect("failed to apply store to builder")
            .apply_env()
            .expect("failed to apply env")
            .build();
        println!("{:?}", config);

        assert_eq!(config.log_level, LogLevel::Trace);
        assert_eq!(config.cache_expiry, 1337);
        assert_eq!(config.portal_username, ENV_USERNAME);
        assert_eq!(config.portal_token, ENV_TOKEN);
    }
}
