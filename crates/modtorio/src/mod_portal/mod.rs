mod portal_mod;

use crate::{
    config::Config,
    ext::{PathExt, ResponseExt},
};
use anyhow::{anyhow, ensure};
use log::*;
use portal_mod::{PortalMod, Release};
use reqwest::{Client, StatusCode};
use std::path::{Path, PathBuf};
use tempfile::tempfile;
use tokio::{fs, io};
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
        let release = self.get_specific_release_or_latest(title, version).await?;
        info!("Downloading '{}' ver. {}", title, release.version);

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
                .ok_or_else(|| anyhow!(""))?;

            directory.as_ref().join(fname)
        };

        debug!(
            "'{}' downloading to tempfile. Destination {}",
            title,
            dest_path.display()
        );

        let mut temp = fs::File::from_std(tempfile()?);
        let written = response.to_writer(&mut temp).await?;

        debug!(
            "'{}' downloaded to tempfile, copying to destination ({})...",
            title,
            dest_path.display()
        );

        let mut dest = fs::File::create(&dest_path).await?;
        temp.seek(std::io::SeekFrom::Start(0)).await?;
        io::copy(&mut temp, &mut dest).await?;

        Ok((dest_path, written))
    }

    pub async fn latest_release(&self, title: &str) -> anyhow::Result<Release> {
        Ok(self.get_specific_release_or_latest(title, None).await?)
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

    async fn get_specific_release_or_latest(
        &self,
        title: &str,
        version: Option<HumanVersion>,
    ) -> anyhow::Result<Release> {
        let url = Url::parse(SITE_ROOT)?.join(API_ROOT)?.join(title)?;
        let mut portal_mod: PortalMod = self.get_json(url).await?;

        let release = match version {
            Some(version) => {
                let mut valid_releases: Vec<Release> = portal_mod
                    .releases
                    .drain_filter(|r| r.version == version)
                    .collect();

                ensure!(
                    !valid_releases.is_empty(),
                    anyhow!("mod {} has no release ver. {}", title, version)
                );

                ensure!(
                    valid_releases.len() == 1,
                    anyhow!("mod {} has multiple releases with ver. {}", title, version)
                );

                valid_releases.remove(0)
            }
            None => {
                ensure!(
                    !portal_mod.releases.is_empty(),
                    anyhow!("mod {} doesn't have any releases", title)
                );
                portal_mod.releases.remove(portal_mod.releases.len() - 1)
            }
        };

        Ok(release)
    }
}
