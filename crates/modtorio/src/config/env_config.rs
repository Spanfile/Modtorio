//! Provides the `EnvConfig` object, used to access config values from the running program's
//! environment variables.

use super::{Config, ConfigSource};
use crate::{util, APP_PREFIX};
use anyhow::Context;
use serde::Deserialize;

/// Contains the config values from the running program's environment variables.
#[derive(Debug, Deserialize, Default)]
pub struct EnvConfig {
    /// Corresponds to the `MODTORIO_PORTAL_USERNAME` environment variable.
    pub portal_username: Option<String>,
    /// Corresponds to the `MODTORIO_PORTAL_TOKEN` environment variable.
    pub portal_token: Option<String>,
}

impl ConfigSource for EnvConfig {
    /// Applies the contained config values to a given `Config`, returning a new `Config` with the
    /// values set.
    // clippy complains that the config parameter should be taken by reference, but if it is the
    // ..config will fail
    #[allow(clippy::needless_pass_by_value)]
    fn apply_to_config(self, config: Config) -> Config {
        Config {
            portal_username: self.portal_username.unwrap_or(config.portal_username),
            portal_token: self.portal_token.unwrap_or(config.portal_token),
            ..config
        }
    }
}

impl EnvConfig {
    /// Returns a new `EnvConfig` built from the running program's environment variables.
    pub fn new() -> anyhow::Result<Self> {
        Ok(envy::prefixed(APP_PREFIX)
            .from_env::<Self>()
            .with_context(|| {
                format!(
                    "Failed to load config from environment variables:\n{}",
                    util::env::dump_string(APP_PREFIX)
                )
            })?)
    }
}

#[cfg(test)]
mod tests {
    use super::{super::SERIAL_MUTEX, *};
    use std::env;

    #[test]
    fn full() {
        let _s = SERIAL_MUTEX.lock().expect("failed to lock serial mutex");

        env::set_var("MODTORIO_PORTAL_USERNAME", "env_username");
        env::set_var("MODTORIO_PORTAL_TOKEN", "env_token");

        let config = EnvConfig::new().expect("failed to create EnvConfig");

        assert_eq!(config.portal_username, Some(String::from("env_username")));
        assert_eq!(config.portal_token, Some(String::from("env_token")));
    }

    #[test]
    fn default() {
        let _s = SERIAL_MUTEX.lock().expect("failed to lock serial mutex");

        env::remove_var("MODTORIO_PORTAL_USERNAME");
        env::remove_var("MODTORIO_PORTAL_TOKEN");

        let config = EnvConfig::new().expect("failed to create EnvConfig");

        assert_eq!(config.portal_username, None);
        assert_eq!(config.portal_token, None);
    }
}
