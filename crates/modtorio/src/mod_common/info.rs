//! Provides structured objects of a mod's metadata information, both from the mod zip
//! archive and the mod portal.

use super::Dependency;
use crate::{
    error::ModError,
    mod_portal::{ModPortal, PortalResult},
    store::{models, Store},
    util::{
        ext::{PathExt, ZipExt},
        HumanVersion,
    },
};
use anyhow::ensure;
use chrono::{DateTime, Utc};
use log::*;
use serde::Deserialize;
use std::path::{Path, PathBuf};
use tokio::task;

/// A mod's metadata, both from the mod zip and optionally from the mod portal.
#[derive(Debug)]
pub struct Info {
    /// The mod's name.
    name: String,
    /// The mod's own version and its required Factorio version. Will only exist once a release of
    /// the mod is installed and the info is populated from the mod zip archive.
    versions: Option<Versions>,
    /// Information about the mod's author.
    author: Author,
    /// The mod's human-friendly display information.
    display: Display,
    /// The mod's dependencies on other mods. Will only exist once a release of the mod is
    /// installed and the info is populated from the mod zip archive.
    dependencies: Option<Vec<Dependency>>,
    /// The mod's releases. Will only exist once the info has been populated from the mod portal.
    releases: Option<Vec<Release>>,
    /* fields the portal API has but not represented here:
     * github_path, created_at, tag */
}

/// A mod author's information.
#[derive(Debug)]
pub struct Author {
    /// The author's name.
    name: String,
    /// The author's contact information, if given.
    contact: Option<String>,
    /// The author's homepage, if given.
    homepage: Option<String>,
}

/// A mod's human-friendly display information.
#[derive(Debug)]
pub struct Display {
    /// The mod's title.
    title: String,
    /// The mod's summary. May only exist once the info has been populated from the mod portal.
    summary: Option<String>,
    /// The mod's description.
    description: String,
    /// The mod's changelog, if given.
    changelog: Option<String>,
}

/// A mod's version information.
#[derive(Debug, Copy, Clone)]
pub struct Versions {
    /// The mod's currently installed version.
    own: HumanVersion,
    /// The currently installed mod's required Factorio version.
    factorio: HumanVersion,
}

/// A mod's release, retrieved from the mod portal.
#[derive(Debug, Deserialize, Clone)]
pub struct Release {
    /// The release's download URL.
    download_url: PathBuf,
    // this field isn't actually useful, since when downloading a mod from the portal, it redirects
    // to a location where the filename is in the URL and that's used instead
    // file_name: String,
    /// The timestamp when the release was published.
    #[serde(rename = "released_at")]
    released_on: DateTime<Utc>,
    /// The releases' version.
    version: HumanVersion,
    /// The release's mod zip archive's SHA1 checksum.
    sha1: String,
    /// Additional information about the release.
    #[serde(rename = "info_json")]
    info_object: ReleaseInfoObject,
}

/// Additional information about a Factorio mod's release.
///
/// The mod portal derives this field's contents from the `info.json` file inside the mod zip
/// archive.
#[derive(Debug, Deserialize, Clone)]
struct ReleaseInfoObject {
    /// The mod release's required Factorio version.
    factorio_version: HumanVersion,
    /// The mod release's dependencies on other mods, if any.
    dependencies: Vec<Dependency>,
}

/// A model of the `info.json` file inside every mod zip archive.
#[derive(Debug, Deserialize)]
struct ZipInfo {
    /// The mod's name.
    name: String,
    /// The mod's version.
    version: HumanVersion,
    /// The mod's required Factorio version.
    factorio_version: HumanVersion,
    /// The mod's title.
    title: String,
    /// The mod's author.
    author: String,
    /// The mod author's contact information, if given.
    contact: Option<String>,
    /// The mod author's homepage, if given.
    homepage: Option<String>,
    /// The mod's description.
    description: String,
    /// The mod's dependencies on other mods, if any. If the corresponding `info.json` file doesn't
    /// list any dependencies, defaults to a mandatory requirement on any version of `base`.
    #[serde(default = "default_dependencies")]
    dependencies: Vec<Dependency>,
}

// #[derive(Debug)]
// pub struct Tag {
//     id: u8,
//     name: String,
//     title: String,
//     description: String,
//     r#type: String,
// }

/// Returns a mandatory requirement of any version of `base`.
#[doc(hidden)]
fn default_dependencies() -> Vec<Dependency> {
    vec!["base".parse().unwrap()]
}

// TODO: generalise this
/// Reads a single file anywhere from a given zip archive based on its filename.
async fn read_object_from_zip<P, T>(path: P, name: &'static str) -> anyhow::Result<T>
where
    P: 'static + AsRef<Path> + Send, // TODO: these sorts of requirements are a bit icky
    T: 'static + serde::de::DeserializeOwned + Send,
{
    task::spawn_blocking(move || -> anyhow::Result<T> {
        let zipfile = std::fs::File::open(path)?;
        let mut archive = zip::ZipArchive::new(zipfile)?;

        let obj = serde_json::from_reader(archive.find_file(name)?)?;
        Ok(obj)
    })
    .await?
}

impl Info {
    /// Builds an info object from a given mod zip archive.
    pub async fn from_zip<P>(path: P) -> anyhow::Result<Self>
    where
        P: 'static + AsRef<Path> + Send,
    {
        let info = read_object_from_zip(path, "info.json").await?;
        // TODO: read changelog
        Ok(Self::from_zip_info(info, String::new()))
    }

    /// Fetches and builds an info object from the mod portal based on a given mod's name.
    pub async fn from_portal(name: &str, portal: &ModPortal) -> anyhow::Result<Self> {
        let mut portal_info: PortalResult = portal.fetch_mod(name).await?;
        portal_info.compress()?;

        Ok(Self::from_portal_info(&portal_info)?)
    }

    /// Builds an info object from the program store based on a stored mod and its wanted version.
    pub async fn from_store(
        factorio_mod: models::FactorioMod,
        version: HumanVersion,
        store: &Store,
    ) -> anyhow::Result<Self> {
        trace!("Building info object from stored mod {:?}", factorio_mod);

        let name = factorio_mod.name;
        let mut releases = Vec::new();
        let mut this_release = None;

        for release in store.get_mod_releases(name.clone()).await? {
            let mut dependencies = Vec::new();

            for store_dep in store.get_release_dependencies(name.clone(), release.version).await? {
                dependencies.push(store_dep.into());
            }

            releases.push(Release {
                download_url: PathBuf::from(release.download_url.clone()),
                released_on: release.released_on,
                version: release.version,
                sha1: release.sha1.clone(),
                info_object: ReleaseInfoObject {
                    factorio_version: release.factorio_version,
                    dependencies,
                },
            });

            if release.version == version {
                // store the index of this wanted version so it can be referenced later
                // required in order to get the release's dependencies into this info object
                this_release = Some(releases.len() - 1);
            }
        }

        let this_release = &releases[this_release.ok_or(ModError::NoSuchRelease(version))?];

        Ok(Self {
            name,
            versions: Some(Versions {
                own: this_release.version,
                factorio: this_release.info_object.factorio_version,
            }),
            author: Author {
                name: factorio_mod.author,
                contact: factorio_mod.contact,
                homepage: factorio_mod.homepage,
            },
            display: Display {
                title: factorio_mod.title,
                summary: factorio_mod.summary,
                description: factorio_mod.description,
                changelog: factorio_mod.changelog,
            },
            dependencies: Some(this_release.info_object.dependencies.clone()),
            releases: Some(releases),
        })
    }

    /// Converts the information from a mod zip archive (a `ZipInfo` and the changelog) into an info
    /// object.
    fn from_zip_info(info: ZipInfo, changelog: String) -> Self {
        Self {
            name: info.name,
            versions: Some(Versions {
                own: info.version,
                factorio: info.factorio_version,
            }),
            author: Author {
                name: info.author,
                contact: info.contact,
                homepage: info.homepage,
            },
            display: Display {
                title: info.title,
                summary: None,
                description: info.description,
                changelog: Some(changelog),
            },
            dependencies: Some(info.dependencies),
            releases: None,
        }
    }

    /// Converts the information from the mod portal info an info object.
    ///
    /// Returns an error if some of the required fields are missing.
    fn from_portal_info(info: &PortalResult) -> anyhow::Result<Self> {
        Ok(Self {
            name: info.name()?.to_owned(),
            versions: None,
            author: Author {
                name: info.owner().to_owned(),
                contact: None,
                homepage: info.homepage().map(str::to_owned),
            },
            display: Display {
                title: info.title()?.to_owned(),
                summary: info.summary().map(str::to_owned),
                description: info.description().to_owned(),
                changelog: info.changelog().map(str::to_owned),
            },
            dependencies: None,
            releases: Some(info.releases()?.to_owned()),
        })
    }

    /// Populates an existing info object from an existing mod zip archive.
    pub async fn populate_from_zip<P>(&mut self, path: P) -> anyhow::Result<()>
    where
        P: 'static + AsRef<Path> + Send,
    {
        let info: ZipInfo = read_object_from_zip(path, "info.json").await?;
        ensure!(
            info.name == self.name,
            ModError::ZipNameMismatch {
                zip: info.name,
                existing: self.name.clone()
            }
        );

        self.versions = Some(Versions {
            own: info.version,
            factorio: info.factorio_version,
        });
        self.author.contact = info.contact;
        self.dependencies = Some(info.dependencies);

        Ok(())
    }

    /// Populates an existing info object by fetching the mod's information from the mod portal.
    pub async fn populate_from_portal(&mut self, portal: &ModPortal) -> anyhow::Result<()> {
        trace!("Populating '{}' from portal", self.name);
        let info: PortalResult = portal.fetch_mod(&self.name).await?;
        self.populate_with_portal_object(info)
    }

    /// Populates an existing info object with a given `PortalInfo` object.
    pub fn populate_with_portal_object(&mut self, mut info: PortalResult) -> anyhow::Result<()> {
        trace!("Populating '{}' with portal object", self.name);
        info.compress()?;

        self.display.summary = info.summary().map(str::to_owned);
        self.releases = Some(info.into_releases()?);

        Ok(())
    }

    /// Populates an existing info object from the program store based on a given stored mod.
    pub async fn populate_with_store_object(
        &mut self,
        store: &'_ Store,
        store_mod: models::FactorioMod,
    ) -> anyhow::Result<()> {
        trace!("Mod '{}' got stored mod: {:?}", self.name, store_mod);

        self.author.name = store_mod.author;
        self.author.contact = store_mod.contact;
        self.author.homepage = store_mod.homepage;
        self.display.title = store_mod.title;
        self.display.summary = store_mod.summary;
        self.display.description = store_mod.description;
        self.display.changelog = store_mod.changelog;

        let mut releases = Vec::new();
        for release in store.get_mod_releases(self.name.clone()).await? {
            trace!("Mod '{}' got stored release: {:?}", self.name, release);
            let mut dependencies = Vec::new();

            for store_dep in store
                .get_release_dependencies(self.name.clone(), release.version)
                .await?
            {
                dependencies.push(store_dep.into());
            }

            releases.push(Release {
                download_url: PathBuf::from(release.download_url),
                released_on: release.released_on,
                version: release.version,
                sha1: release.sha1,
                info_object: ReleaseInfoObject {
                    factorio_version: release.factorio_version,
                    dependencies,
                },
            });
        }

        self.releases = Some(releases);
        Ok(())
    }

    /// Populates an existing info object from the program stored based on the mod's named in the
    /// info object.
    #[allow(dead_code)]
    pub async fn populate_from_store(&mut self, store: &'_ Store) -> anyhow::Result<()> {
        if let Some(store_mod) = store.get_factorio_mod(self.name.clone()).await? {
            self.populate_with_store_object(store, store_mod).await
        } else {
            trace!("Mod '{}' not in store while trying to load it from store", self.name);

            Err(ModError::ModNotInStore.into())
        }
    }
}

impl Info {
    /// Returns a human-friendly display of the info object.
    pub fn display(&self) -> String {
        format!(
            "'{}' ('{}') ver. {}",
            self.display.title,
            self.name,
            self.versions
                .map_or_else(|| String::from("unknown"), |v| v.own.to_string())
        )
    }

    // TODO this is a bad method
    /// Determines if the info has been populated from the mod portal, based on if there are
    /// existing releases.
    pub fn is_portal_populated(&self) -> bool {
        self.releases.is_some()
    }

    /// Returns an info object's `Versions` if they exist, otherwise returns the error
    /// `ModError::MissingInfo`.
    fn versions(&self) -> anyhow::Result<Versions> {
        Ok(self.versions.ok_or(ModError::MissingInfo)?)
    }

    /// Returns the mod's name.
    pub fn name(&self) -> &str {
        &self.name
    }

    /// Returns the mod's author.
    pub fn author(&self) -> &str {
        &self.author.name
    }

    /// Returns the mod author's contact information, if any.
    pub fn contact(&self) -> Option<&str> {
        self.author.contact.as_deref()
    }

    /// Returns the mod author's homepage, if any.
    pub fn homepage(&self) -> Option<&str> {
        self.author.homepage.as_deref()
    }

    /// Returns the mod's title.
    pub fn title(&self) -> &str {
        &self.display.title
    }

    /// Returns the mod's summary, if any.
    pub fn summary(&self) -> Option<&str> {
        self.display.summary.as_deref()
    }

    /// Returns the mod's description.
    pub fn description(&self) -> &str {
        &self.display.description
    }

    /// Returns the mod's changelog, if any.
    pub fn changelog(&self) -> Option<&str> {
        self.display.changelog.as_deref()
    }

    /// Returns the mod's installed version, or an error if mod isn't installed
    /// (`ModError::MissingInfo`).
    pub fn own_version(&self) -> anyhow::Result<HumanVersion> {
        Ok(self.versions()?.own)
    }

    /// Returns the mod's required Factorio version, or an error if mod isn't installed
    /// (`ModError::MissingInfo`).
    pub fn factorio_version(&self) -> anyhow::Result<HumanVersion> {
        Ok(self.versions()?.factorio)
    }

    /// Returns the mod's releases, or an error if the mod's info hasn't been fetched from the mod
    /// portal (`ModError::MissingInfo`).
    pub fn releases(&self) -> anyhow::Result<Vec<Release>> {
        Ok(self.releases.as_ref().cloned().ok_or(ModError::MissingInfo)?)
    }

    /// Returns a release with a certain version, or the latest version if no wanted version is
    /// specified. Returns an error if a release with the wanted version doesn't exist
    /// (`ModError::NoSuchRelease`), or if there aren't any releases (`ModError::NoReleases`).
    pub fn get_release(&self, version: Option<HumanVersion>) -> anyhow::Result<Release> {
        let releases = self.releases()?;

        match version {
            Some(version) => releases
                .iter()
                .find(|r| r.version == version)
                .cloned()
                .ok_or_else(|| ModError::NoSuchRelease(version).into()),
            None => releases
                .iter()
                .last()
                .cloned()
                .ok_or_else(|| ModError::NoReleases.into()),
        }
    }

    /// Returns the mod's dependencies on other mods, or an error if the mod isn't installed or its
    /// info hasn't been fetched from the mod portal (`ModError::MissingInfo`).
    pub fn dependencies(&self) -> anyhow::Result<Vec<Dependency>> {
        Ok(self.dependencies.as_ref().cloned().ok_or(ModError::MissingInfo)?)
    }
}

impl Release {
    /// Returns the release's download URL, or an error if the URL contains invalid Unicode.
    pub fn url(&self) -> anyhow::Result<&str> {
        self.download_url.get_str()
    }

    /// Mutably borrows this release's URL. Primarily used when "compressing" the URL.
    pub fn url_mut(&mut self) -> &mut PathBuf {
        &mut self.download_url
    }

    /// Returns the release's version.
    pub fn version(&self) -> HumanVersion {
        self.version
    }

    /// Returns the release's required Factorio version.
    pub fn factorio_version(&self) -> HumanVersion {
        self.info_object.factorio_version
    }

    /// Returns the timestamp when the release was published.
    pub fn released_on(&self) -> DateTime<Utc> {
        self.released_on
    }

    /// Returns the SHA1 checksum of the release's corresponding mod zip archive.
    pub fn sha1(&self) -> &str {
        &self.sha1
    }

    /// Returns the release's dependencies on other mods.
    pub fn dependencies(&self) -> Vec<&Dependency> {
        self.info_object.dependencies.iter().collect()
    }
}
