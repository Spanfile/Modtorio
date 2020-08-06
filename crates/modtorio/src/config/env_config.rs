//! Provides the `EnvConfig` object, used to access config values from the running program's
//! environment variables.

use super::Config;
use crate::{util, APP_PREFIX};
use anyhow::Context;
use serde::Deserialize;

/// Contains the config values from the running program's environment variables.
#[derive(Debug, Deserialize, Default)]
pub struct EnvConfig {
    /// Corresponds to the `MODTORIO_PORTAL_USERNAME` environment variable.
    pub portal_username: String,
    /// Corresponds to the `MODTORIO_PORTAL_TOKEN` environment variable.
    pub portal_token: String,
}

impl EnvConfig {
    /// Returns a new `EnvConfig` built from the running program's environment variables.
    pub fn from_env() -> anyhow::Result<Self> {
        Ok(envy::prefixed(APP_PREFIX)
            .from_env::<Self>()
            .with_context(|| {
                format!(
                    "Failed to load config from environment variables:\n{}",
                    util::env::dump_string(APP_PREFIX)
                )
            })?)
    }

    /// Applies the contained config values to a given `Config`, returning a new `Config` with the
    /// values set.
    // clippy complains that the config parameter should be taken by reference, but if it is the
    // ..config will fail
    #[allow(clippy::needless_pass_by_value)]
    pub fn apply_to_config(self, config: Config) -> Config {
        Config {
            portal_username: self.portal_username,
            portal_token: self.portal_token,
            ..config
        }
    }
}
