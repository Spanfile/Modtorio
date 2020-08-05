use super::{Config, APP_PREFIX};
use crate::util;
use anyhow::Context;
use serde::Deserialize;

#[derive(Debug, Deserialize, Default)]
pub struct Env {
    pub portal_username: String,
    pub portal_token: String,
}

impl Env {
    pub fn from_env() -> anyhow::Result<Self> {
        Ok(envy::prefixed(APP_PREFIX)
            .from_env::<Self>()
            .with_context(|| {
                format!(
                    "Failed to load config from environment variables:\n{}",
                    util::dump_env(APP_PREFIX)
                )
            })?)
    }

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
