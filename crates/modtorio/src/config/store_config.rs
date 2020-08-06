//! Provides the `StoreConfig` object, used to access config values from the program store.

use super::Config;
use crate::store;
use serde::Deserialize;

/// Contains the config values from the program store.
#[derive(Debug, Deserialize, Default)]
pub struct StoreConfig {
    /// Corresponds to the `Field::PortalUsername` option.
    pub portal_username: String,
    /// Corresponds to the `Field::PortalToken` option.
    pub portal_token: String,
}

impl StoreConfig {
    /// Returns a new `StoreConfig` built from a given program store.
    pub async fn from_store(store: &store::Store) -> anyhow::Result<Self> {
        let portal_username = store
            .get_option(store::option::Field::PortalUsername)
            .await?
            .and_then(|v| v.take_value())
            .unwrap_or_else(String::new);
        let portal_token = store
            .get_option(store::option::Field::PortalToken)
            .await?
            .and_then(|v| v.take_value())
            .unwrap_or_else(String::new);

        Ok(Self {
            portal_username,
            portal_token,
        })
    }

    /// Applies the contained config values to a given `Config`, returning a new `Config` with the
    /// values set.
    // clippy complains that the config parameter should be taken by reference, but if it is the
    // '..config' bit will fail
    #[allow(clippy::needless_pass_by_value)]
    pub fn apply_to_config(self, config: Config) -> Config {
        Config {
            portal_username: self.portal_username,
            portal_token: self.portal_token,
            ..config
        }
    }
}
