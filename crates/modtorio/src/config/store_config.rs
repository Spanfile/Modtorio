use super::Config;
use crate::store;
use serde::Deserialize;

#[derive(Debug, Deserialize, Default)]
pub struct StoreConfig {
    pub portal_username: String,
    pub portal_token: String,
}

impl StoreConfig {
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
