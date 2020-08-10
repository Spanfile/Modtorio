//! Provides the `StoreConfig` object, used to access config values from the program store.

use super::Config;
use crate::store;
use serde::Deserialize;

/// Contains the config values from the program store.
#[derive(Debug, Deserialize, Default)]
pub struct StoreConfig {
    /// Corresponds to the `Field::PortalUsername` option.
    pub portal_username: Option<String>,
    /// Corresponds to the `Field::PortalToken` option.
    pub portal_token: Option<String>,
}

impl StoreConfig {
    /// Returns a new `StoreConfig` built from a given program store.
    pub async fn from_store(store: &store::Store) -> anyhow::Result<Self> {
        let portal_username = store
            .get_option(store::option::Field::PortalUsername)
            .await?
            .and_then(|v| v.take_value());
        let portal_token = store
            .get_option(store::option::Field::PortalToken)
            .await?
            .and_then(|v| v.take_value());

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
            portal_username: self.portal_username.unwrap_or(config.portal_username),
            portal_token: self.portal_token.unwrap_or(config.portal_token),
            ..config
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::{store, store::option};

    #[tokio::test]
    async fn full() {
        let store = store::Builder::from_location(crate::store::MEMORY_STORE.into())
            .build()
            .await
            .expect("failed to build store");
        store
            .set_option(option::Value::new(
                option::Field::PortalUsername,
                Some(String::from("store_username")),
            ))
            .await
            .expect("failed to store portal username");
        store
            .set_option(option::Value::new(
                option::Field::PortalToken,
                Some(String::from("store_token")),
            ))
            .await
            .expect("failed to store portal token");

        let config = StoreConfig::from_store(&store)
            .await
            .expect("failed to create StoreConfig");

        assert_eq!(config.portal_username, Some(String::from("store_username")));
        assert_eq!(config.portal_token, Some(String::from("store_token")));
    }

    #[tokio::test]
    async fn default() {
        let store = store::Builder::from_location(crate::store::MEMORY_STORE.into())
            .build()
            .await
            .expect("failed to build store");

        let config = StoreConfig::from_store(&store)
            .await
            .expect("failed to create StoreConfig");

        assert_eq!(config.portal_username, None);
        assert_eq!(config.portal_token, None);
    }
}
