mod portal_mod;

use crate::config::Config;

#[derive(Debug)]
struct Credentials {
    username: String,
    token: String,
}

#[derive(Debug)]
pub struct ModPortal {
    credentials: Credentials,
}

impl ModPortal {
    pub fn new(config: &Config) -> anyhow::Result<Self> {
        Ok(Self {
            credentials: Credentials {
                username: config.portal.username.clone(),
                token: config.portal.token.clone(),
            },
        })
    }
}
