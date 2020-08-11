//! Provides the [`ModPortal`](ModPortal) object to interact with the Factorio mod portal via HTTP.

use crate::{config::Config, error::ModPortalError, util::ext::ResponseExt};
use log::*;
use reqwest::Client;
use std::path::{Path, PathBuf};
use tempfile::tempfile;
use tokio::{fs, io};
use url::Url;

/// The user-agent used in all HTTP requests.
const USER_AGENT: &str = "modtorio";
/// The mod portal's site root.
const SITE_ROOT: &str = "https://mods.factorio.com";
/// The mod portal's download root.
const DOWNLOAD_ROOT: &str = "/download/";
/// The mod portal's API root.
const API_ROOT: &str = "/api/mods/";
/// The endpoint for requesting full mod information.
const FULL_ENDPOINT: &str = "full";

/// A username-token pair used to authenticate with the mod portal.
#[derive(Debug)]
struct Credentials {
    /// The username.
    username: String,
    /// The token used to authenticate.
    token: String,
}

/// The mod portal interface object.
#[derive(Debug)]
pub struct ModPortal {
    /// The authentication credentials.
    credentials: Credentials,
    /// The HTTP client.
    client: Client,
}

impl ModPortal {
    /// Returns a new `ModPortal` object with credentials from the given `Config` object.
    pub fn new(config: &Config) -> anyhow::Result<Self> {
        let client = Client::builder().user_agent(USER_AGENT).build()?;

        Ok(Self {
            credentials: Credentials {
                username: config.portal_username().to_owned(),
                token: config.portal_token().to_owned(),
            },
            client,
        })
    }

    /// Fetches information for a given mod based on its name.
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

    /// Downloads a given mod its zip archive URL to a temporary location and copies it to the final
    /// given location. Returns the final location's path and the zip archive's size in the
    /// filesystem.
    pub async fn download_mod<P>(&self, name: &str, url_path: &str, directory: P) -> anyhow::Result<(PathBuf, usize)>
    where
        P: AsRef<Path>,
    {
        let download_url = Url::parse(SITE_ROOT)?
            .join(DOWNLOAD_ROOT)?
            .join(&format!("{}/", name))?
            .join(url_path)?;
        debug!("Downloading mod from {}", download_url);

        let mut response = self.get(download_url).await?;

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
    /// GETs a given URL and returns the response. Will include the current mod portal credentials
    /// in the request query.
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

        let status = response.status();
        if status.is_success() {
            Ok(response)
        } else if status.is_client_error() {
            Err(ModPortalError::ClientError(status).into())
        } else if status.is_server_error() {
            Err(ModPortalError::ServerError(status).into())
        } else {
            Err(ModPortalError::UnexpectedStatus(status).into())
        }
    }

    /// GETs a given URL and returns the response as a string. Will include the current mod portal
    /// credentials in the request query.
    async fn get_string(&self, url: Url) -> anyhow::Result<String> {
        let response = self.get(url).await?;
        trace!("{:?}", response);
        Ok(response.text().await?)
    }

    /// GETs a given URL and returns the response as a object deserialized from JSON. Will include
    /// the current mod portal credentials in the request query.
    async fn get_json<T>(&self, url: Url) -> anyhow::Result<T>
    where
        T: serde::de::DeserializeOwned,
    {
        let response = self.get_string(url).await?;
        trace!("{}", response);
        Ok(serde_json::from_str(&response)?)
    }
}
