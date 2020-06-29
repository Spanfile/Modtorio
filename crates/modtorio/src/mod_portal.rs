use crate::{config::Config, ext::ResponseExt};
use anyhow::{anyhow, ensure};
use log::*;
use reqwest::{Client, StatusCode};
use std::path::{Path, PathBuf};
use tempfile::tempfile;
use tokio::{fs, io};
use url::Url;

const USER_AGENT: &str = "modtorio";
const SITE_ROOT: &str = "https://mods.factorio.com";
const DOWNLOAD_ROOT: &str = "/download/";
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

    pub async fn fetch_mod<T>(&self, name: &str) -> anyhow::Result<T>
    where
        T: serde::de::DeserializeOwned,
    {
        let url = Url::parse(SITE_ROOT)?
            .join(API_ROOT)?
            .join(&format!("{}/", name))?
            .join(FULL_ENDPOINT)?;
        debug!("Fetching mod info from {}", url);

        Ok(self.get_json(url).await?)
    }

    pub async fn download_mod<P>(
        &self,
        name: &str,
        url_path: &str,
        directory: P,
    ) -> anyhow::Result<(PathBuf, usize)>
    where
        P: AsRef<Path>,
    {
        let download_url = Url::parse(SITE_ROOT)?
            .join(DOWNLOAD_ROOT)?
            .join(&format!("{}/", name))?
            .join(url_path)?;
        debug!("Downloading mod from {}", download_url);

        let mut response = self.get(download_url).await?;
        let status = response.status();

        ensure!(
            status == StatusCode::OK,
            anyhow!("download returned non-OK status code {}", status)
        );

        let mut temp = fs::File::from_std(tempfile()?);
        let written = response.to_writer(&mut temp).await?;

        let filename = response.url_file_name()?;
        let dest_path = directory.as_ref().join(filename);
        debug!(
            "'{}' downloaded to tempfile, copying to destination ({})...",
            filename,
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

    async fn get_string(&self, url: Url) -> anyhow::Result<String> {
        Ok(self.get(url).await?.text().await?)
    }

    async fn get_json<T>(&self, url: Url) -> anyhow::Result<T>
    where
        T: serde::de::DeserializeOwned,
    {
        let response = self.get_string(url).await?;
        trace!("{}", response);
        Ok(serde_json::from_str(&response)?)
    }
}
