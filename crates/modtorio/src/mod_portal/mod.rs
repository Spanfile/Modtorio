mod portal_mod;

use crate::config::Config;
use anyhow::{anyhow, ensure};
use bytes::Bytes;
use ext::PathExt;
use log::*;
use portal_mod::PortalMod;
use reqwest::{Client, StatusCode};
use url::Url;
use util::HumanVersion;

const USER_AGENT: &str = "modtorio";
const SITE_ROOT: &str = "https://mods.factorio.com";
const API_ROOT: &str = "/api/mods/";

#[derive(Debug)]
struct Credentials {
    username: String,
    token: String,
}

#[derive(Debug)]
pub struct ModPortal {
    credentials: Credentials,
    client: Client,
}

impl ModPortal {
    pub fn new(config: &Config) -> anyhow::Result<Self> {
        let client = Client::builder().user_agent(USER_AGENT).build()?;

        Ok(Self {
            credentials: Credentials {
                username: config.portal.username.clone(),
                token: config.portal.token.clone(),
            },
            client,
        })
    }

    pub async fn download_mod(
        &self,
        title: &str,
        version: Option<HumanVersion>,
    ) -> anyhow::Result<Bytes> {
        let url = Url::parse(SITE_ROOT)?.join(API_ROOT)?.join(title)?;

        debug!("Mod GET URL: {}", url);

        let portal_mod: PortalMod = self.client.get(url.as_str()).send().await?.json().await?;

        let release = match version {
            Some(version) => portal_mod
                .releases
                .iter()
                .find(|r| r.version == version)
                .ok_or_else(|| {
                    anyhow!("mod {} doesn't have a release version {}", title, version)
                })?,
            None => portal_mod
                .releases
                .first()
                .ok_or_else(|| anyhow!("mod {} doesn't have any releases", title))?,
        };

        let download_url = Url::parse(SITE_ROOT)?.join(release.download_url.get_str()?)?;

        debug!("Mod download GET URL: {}", download_url);

        let response = self.client.get(download_url.as_str()).send().await?;
        let status = response.status();

        debug!("Mod download response status: {}", status);
        ensure!(
            status == StatusCode::OK,
            anyhow!("download returned non-OK status code {}", status)
        );

        Ok(response.bytes().await?)
    }
}
