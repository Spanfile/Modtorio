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
/// The default cache expiry time in seconds.
pub const DEFAULT_CACHE_EXPIRY: u64 = 3600;

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

#[cfg(test)]
mod tests {
    use super::*;
    use crate::ext::PathExt;
    use lazy_static::lazy_static;
    use std::{env, fs::File, io::Write, path::PathBuf, sync::Mutex};
    use tempfile::Builder;

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

    fn temp_config_file(contents: &str) -> (PathBuf, File) {
        let named = Builder::new()
            .prefix("modtorio")
            .suffix(".conf")
            .tempfile()
            .expect("failed to open tempfile");
        write!(&named, "{}", contents).expect("failed to write contents into tempfile");
        (
            named.path().to_path_buf(),
            named.reopen().expect("failed to reopen tempfile"),
        )
    }

    #[tokio::test]
    async fn full_config() {
        let _s = SERIAL_MUTEX.lock().expect("failed to lock serial mutex");

        let (conf_path, _f) = temp_config_file(
            r#"
            [general]
            log_level = "trace"
            [cache]
            expiry = 1337
        "#,
        );
        let opts = Opts::custom_args(&[
            "modtorio",
            "-c",
            conf_path.get_str().expect("failed to get tempfile path"),
            "--store",
            crate::store::MEMORY_STORE,
        ]);
        let store = Store::build(&opts).await.unwrap();

        env::set_var(MODTORIO_PORTAL_USERNAME, "username");
        env::set_var(MODTORIO_PORTAL_TOKEN, "token");

        println!("{:?}", util::env::dump_lines(crate::APP_PREFIX));
        let config = Config::build(&opts, &store).await.unwrap();
        println!("{:?}", config);

        assert_eq!(config.portal_username, "username");
        assert_eq!(config.portal_token, "token");
        assert_eq!(config.log_level, LogLevel::Trace);
        assert_eq!(config.cache_expiry, 1337);
    }

    // #[tokio::test]
    // async fn default_config() {
    //     let _s = SERIAL_MUTEX.lock().expect("failed to lock serial mutex");

    //     let opts = Opts::custom_args(&["--store", crate::store::MEMORY_STORE]);
    //     let store = Store::build(&opts).await.unwrap();

    //     // remove variables possibly left over from other tests
    //     env::remove_var(MODTORIO_PORTAL_USERNAME);
    //     env::remove_var(MODTORIO_PORTAL_TOKEN);

    //     println!("{:?}", util::env::dump_lines(crate::APP_PREFIX));
    //     let config = Config::build(&opts, &store).await.unwrap();
    //     println!("{:?}", config);

    //     assert_eq!(config.cache_expiry, DEFAULT_CACHE_EXPIRY);
    //     assert_eq!(config.log_level, LogLevel::default());
    // }
}
