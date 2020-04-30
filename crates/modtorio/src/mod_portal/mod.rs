mod portal_mod;

use crate::{
    config::Config,
    ext::{PathExt, ResponseExt},
};
use anyhow::{anyhow, ensure};
use log::*;
use portal_mod::PortalMod;
use reqwest::{Client, StatusCode};
use std::path::{Path, PathBuf};
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

    pub async fn download_mod<P: AsRef<Path>>(
        &self,
        title: &str,
        version: Option<HumanVersion>,
        directory: P,
    ) -> anyhow::Result<(PathBuf, usize)> {
        let url = Url::parse(SITE_ROOT)?.join(API_ROOT)?.join(title)?;
        let portal_mod: PortalMod = self.get_json(url).await?;

        let release = match version {
            Some(version) => portal_mod
                .releases
                .iter()
                .find(|r| r.version == version)
                .ok_or_else(|| {
                    anyhow!("mod {} doesn't have a release version {}", title, version)
                })?,
            None => {
                let release = portal_mod
                    .releases
                    .last()
                    .ok_or_else(|| anyhow!("mod {} doesn't have any releases", title))?;
                info!("Latest version of '{}': {}", title, release.version);

                release
            }
        };

        let download_url = Url::parse(SITE_ROOT)?.join(release.download_url.get_str()?)?;
        let mut response = self.get(download_url).await?;
        let status = response.status();

        ensure!(
            status == StatusCode::OK,
            anyhow!("download returned non-OK status code {}", status)
        );

        let dest_path = {
            let fname = response
                .url()
                .path_segments()
                .and_then(|segments| segments.last())
                .and_then(|name| if name.is_empty() { None } else { Some(name) })
                .unwrap_or("tmp.bin");
            directory.as_ref().join(fname)
        };

        debug!("Writing response to {}", dest_path.display());

        let mut dest = tokio::fs::File::create(&dest_path).await?;
        let written = response.to_writer(&mut dest).await?;

        Ok((dest_path, written))
    }
}

impl ModPortal {
    async fn get(&self, url: Url) -> anyhow::Result<reqwest::Response> {
        debug!("Mod portal GET URL: {}", url);
        let response = self
            .client
            .get(url.as_str())
            .query(&[
                ("username", self.credentials.username.as_str()),
                ("token", self.credentials.token.as_str()),
            ])
            .send()
            .await?;
        debug!("URL {} GET status: {}", url, response.status());
        Ok(response)
    }

    async fn get_json<T>(&self, url: Url) -> anyhow::Result<T>
    where
        T: serde::de::DeserializeOwned,
    {
        Ok(self.get(url).await?.json().await?)
    }
}
