use crate::{
    config::Config,
    ext::{PathExt, ResponseExt},
    mod_common::PortalMod,
};
use anyhow::{anyhow, ensure};
use log::*;
use reqwest::{Client, StatusCode};
use std::path::{Path, PathBuf};
use tempfile::tempfile;
use tokio::{fs, io};
use url::Url;

const USER_AGENT: &str = "modtorio";
const SITE_ROOT: &str = "https://mods.factorio.com";
const API_ROOT: &str = "/api/mods/";
const FULL_ENDPOINT: &str = "full";

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

    pub async fn fetch_mod(&self, name: &str) -> anyhow::Result<PortalMod> {
        let url = Url::parse(SITE_ROOT)?
            .join(API_ROOT)?
            .join(&format!("{}/", name))?
            .join(FULL_ENDPOINT)?;
        let portal_mod: PortalMod = self.get_json(url).await?;

        Ok(portal_mod)
    }

    pub async fn download_mod<P>(
        &self,
        portal_mod: &PortalMod,
        directory: P,
    ) -> anyhow::Result<(PathBuf, usize)>
    where
        P: AsRef<Path>,
    {
        let release = portal_mod.get_release(None)?;
        let download_url = Url::parse(SITE_ROOT)?.join(release.download_url.get_str()?)?;
        debug!(
            "Download mod '{}' ver. {} from URL {}",
            portal_mod.name, release.version, download_url
        );

        let mut response = self.get(download_url).await?;
        let status = response.status();

        ensure!(
            status == StatusCode::OK,
            anyhow!("download returned non-OK status code {}", status)
        );

        let mut temp = fs::File::from_std(tempfile()?);
        let written = response.to_writer(&mut temp).await?;

        let dest_path = directory.as_ref().join(response.url_file_name()?);
        debug!(
            "'{}' downloaded to tempfile, copying to destination ({})...",
            portal_mod.title,
            dest_path.display()
        );

        let mut dest = fs::File::create(&dest_path).await?;
        temp.seek(std::io::SeekFrom::Start(0)).await?;
        io::copy(&mut temp, &mut dest).await?;

        Ok((dest_path, written))
    }
}

impl ModPortal {
    async fn get(&self, url: Url) -> anyhow::Result<reqwest::Response> {
        let response = self
            .client
            .get(url.as_str())
            .query(&[
                ("username", self.credentials.username.as_str()),
                ("token", self.credentials.token.as_str()),
            ])
            .send()
            .await?;

        Ok(response)
    }

    async fn get_json<T>(&self, url: Url) -> anyhow::Result<T>
    where
        T: serde::de::DeserializeOwned,
    {
        Ok(self.get(url).await?.json().await?)
    }
}
