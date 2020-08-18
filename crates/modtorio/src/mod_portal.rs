//! Provides the [`ModPortal`](ModPortal) object to interact with the Factorio mod portal via HTTP.

use crate::{
    config::Config,
    error::{ModError, ModPortalError},
    mod_common::Release,
    util::{self, ext::ResponseExt},
};
use log::*;
use reqwest::Client;
use serde::Deserialize;
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
const API_ROOT: &str = "/api/mods";
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

/// Represents a single mods full information from the mod portal.
#[derive(Debug, Deserialize)]
pub struct PortalResult {
    /// The mod's name.
    name: Option<String>,
    /// The mod's author.
    ///
    /// This field is equal to the `author` field in other mod information structs.
    owner: Option<String>,
    /// The mod's releases.
    releases: Option<Vec<Release>>,
    /// The mod's summary.
    summary: Option<String>,
    /// The mod's title.
    title: Option<String>,
    /// The mod's changelog.
    changelog: Option<String>,
    /// The mod's description.
    description: Option<String>,
    /// The mod author's homepage.
    homepage: Option<String>,
}

/// Represents the result to querying for multiple mods.
#[derive(Debug, Deserialize)]
struct ModList {
    /// The pagination, if any.
    pagination: Option<Pagination>,
    /// The individual mod results.
    results: Vec<PortalResult>,
}

/// Represents the pagination from querying for multiple mods.
#[derive(Debug, Deserialize)]
struct Pagination {
    /// Total number of mods returned.
    count: i32,
    /// The current page number.
    page: i32,
    /// The total number of pages.
    page_count: i32,
    /// The number of results per page.
    page_size: i32,
    /// Links to other pages.
    links: Links,
}

/// Represents the links from the pagination from querying for multiple mods.
#[derive(Debug, Deserialize)]
struct Links {
    /// Link to the first page, or `None` if this is the first page.
    first: Option<String>,
    /// Link to the last page, or `None` if this is the last page.
    last: Option<String>,
    /// Link to the first page, or `None` if this is the first page.
    prev: Option<String>,
    /// Link to the last page, or `None` if this is the last page.
    next: Option<String>,
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
    pub async fn fetch_mod(&self, name: &str) -> anyhow::Result<PortalResult> {
        let url = Url::parse(SITE_ROOT)?
            .join(&format!("{}/", API_ROOT))?
            .join(&format!("{}/", name))?
            .join(FULL_ENDPOINT)?;
        debug!("Fetching mod info from {}", url);

        Ok(self.get_json(url).await?)
    }

    /// Fetches information for multiple mods based on their names.
    pub async fn fetch_multiple_mods(&self, names: &[&str]) -> anyhow::Result<Vec<PortalResult>> {
        let mut mods = Vec::new();
        let mut current_page = 1;

        loop {
            let mut url = Url::parse(SITE_ROOT)?.join(API_ROOT)?;
            url.query_pairs_mut()
                .append_pair("full", "True")
                .append_pair("page_size", "max") // TODO: make this a config option
                .append_pair("namelist", &names.join(","))
                .append_pair("page", &current_page.to_string());
            debug!("Fetching mod list from {} for {} mods", url, names.len());

            let mut mod_list: ModList = self.get_json(url).await?;
            debug!(
                "Got mod list response. Mod count in this response: {}. Pagination: {:?}",
                mod_list.results.len(),
                mod_list.pagination
            );
            trace!("Got mod list: {:?}", mod_list);

            mods.append(&mut mod_list.results);

            if mods.len() < names.len() {
                current_page += 1;
                continue;
            }

            break;
        }

        Ok(mods)
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
            debug!("Caught erroneus response. Body: {:?}", response.text().await);
            Err(ModPortalError::ClientError(status).into())
        } else if status.is_server_error() {
            debug!("Caught erroneus response. Body: {:?}", response.text().await);
            Err(ModPortalError::ServerError(status).into())
        } else {
            debug!("Caught erroneus response. Body: {:?}", response.text().await);
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

impl PortalResult {
    /// Returns the result's name or `ModError::MissingField` if it was missing from the portal response.
    pub fn name(&self) -> anyhow::Result<&str> {
        self.name
            .as_deref()
            .ok_or_else(|| ModError::MissingField("name").into())
    }

    /// Returns the result's title or `ModError::MissingField` if it was missing from the portal response.
    pub fn title(&self) -> anyhow::Result<&str> {
        self.title
            .as_deref()
            .ok_or_else(|| ModError::MissingField("title").into())
    }

    /// Returns the result's releases or `ModError::MissingField` if it was missing from the portal response.
    pub fn releases(&self) -> anyhow::Result<&[Release]> {
        self.releases
            .as_deref()
            .ok_or_else(|| ModError::MissingField("releases").into())
    }

    /// Immutably borrows the result's releases or `ModError::MissingField` if it was missing from the portal response.
    fn releases_mut(&mut self) -> anyhow::Result<&mut [Release]> {
        self.releases
            .as_deref_mut()
            .ok_or_else(|| ModError::MissingField("releases").into())
    }

    /// Consumes the resulta and returns the result's releases or `ModError::MissingField` if it was missing from the
    /// portal response.
    pub fn into_releases(self) -> anyhow::Result<Vec<Release>> {
        self.releases.ok_or_else(|| ModError::MissingField("releases").into())
    }

    /// Returns the result's owner or the default string if it was missing from the portal response.
    pub fn owner(&self) -> &str {
        self.owner.as_deref().unwrap_or_else(|| {
            debug!("Missing required but not critical field 'owner' in mod portal result");
            Default::default()
        })
    }

    /// Returns the result's description or the default string if it was missing from the portal response.
    pub fn description(&self) -> &str {
        self.owner.as_deref().unwrap_or_else(|| {
            debug!("Missing required but not critical field 'description' in mod portal result");
            Default::default()
        })
    }

    /// Returns the result's homepage if it was present in the portal response.
    pub fn homepage(&self) -> Option<&str> {
        self.homepage.as_deref()
    }

    /// Returns the result's summary if it was present in the portal response.
    pub fn summary(&self) -> Option<&str> {
        self.summary.as_deref()
    }

    /// Returns the result's changelog if it was present in the portal response.
    pub fn changelog(&self) -> Option<&str> {
        self.changelog.as_deref()
    }

    /// Removes redundant information from an info object returned by the mod portal.
    ///
    /// The function will:
    /// * Remove all but the last path segment from each release's download URL. The other components are always the
    ///   same and thus, can be derived when required.
    pub fn compress(&mut self) -> anyhow::Result<()> {
        for release in self.releases_mut()? {
            let url = release.url_mut();
            *url = util::get_last_path_segment(&url);
        }

        Ok(())
    }
}
