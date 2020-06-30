use super::Dependency;
use crate::{
    cache::models,
    error::ModError,
    ext::{PathExt, ZipExt},
    mod_portal::ModPortal,
    util,
    util::HumanVersion,
    Cache,
};
use anyhow::ensure;
use chrono::{DateTime, Utc};
use log::*;
use serde::Deserialize;
use std::path::{Path, PathBuf};
use tokio::task;

#[derive(Debug)]
pub struct Info {
    name: String,
    versions: Option<Versions>,
    author: Author,
    display: Display,
    dependencies: Option<Vec<Dependency>>,
    releases: Option<Vec<Release>>,
    /* fields the portal API has but not represented here:
     * github_path, created_at, tag */
}

#[derive(Debug)]
pub struct Author {
    name: String,
    contact: Option<String>,
    homepage: Option<String>,
}

#[derive(Debug)]
pub struct Display {
    title: String,
    summary: Option<String>,
    description: String,
    changelog: Option<String>,
}

#[derive(Debug, Copy, Clone)]
pub struct Versions {
    own: HumanVersion,
    factorio: HumanVersion,
}

#[derive(Debug, Deserialize, Clone)]
pub struct Release {
    download_url: PathBuf,
    // this field isn't actually useful, since when downloading a mod from the portal, it redirects
    // to a location where the filename is in the URL and that's used instead
    // file_name: String,
    #[serde(rename = "released_at")]
    released_on: DateTime<Utc>,
    version: HumanVersion,
    sha1: String,
    #[serde(rename = "info_json")]
    info_object: ReleaseInfoObject,
}

#[derive(Debug, Deserialize, Clone)]
struct ReleaseInfoObject {
    factorio_version: HumanVersion,
    dependencies: Vec<Dependency>,
}

#[derive(Debug, Deserialize)]
struct ZipInfo {
    name: String,
    version: HumanVersion,
    factorio_version: HumanVersion,
    title: String,
    author: String,
    contact: Option<String>,
    homepage: Option<String>,
    description: String,
    #[serde(default = "default_dependencies")]
    dependencies: Vec<Dependency>,
}

#[derive(Debug, Deserialize)]
struct PortalInfo {
    name: Option<String>,
    owner: Option<String>,
    releases: Option<Vec<Release>>,
    summary: Option<String>,
    title: Option<String>,
    changelog: Option<String>,
    description: Option<String>,
    homepage: Option<String>,
}

// #[derive(Debug)]
// pub struct Tag {
//     id: u8,
//     name: String,
//     title: String,
//     description: String,
//     r#type: String,
// }

fn default_dependencies() -> Vec<Dependency> {
    vec!["base".parse().unwrap()]
}

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

fn compress_portal_info(info: PortalInfo) -> PortalInfo {
    if let Some(releases) = info.releases {
        let new_releases = releases
            .into_iter()
            .map(|release| Release {
                download_url: util::get_last_path_segment(release.download_url),
                ..release
            })
            .collect();

        PortalInfo {
            releases: Some(new_releases),
            ..info
        }
    } else {
        warn!("Missing releases in portal info when compressing");
        debug!("{:?}", info);

        info
    }
}

impl Info {
    pub async fn from_zip<P>(path: P) -> anyhow::Result<Self>
    where
        P: 'static + AsRef<Path> + Send,
    {
        let info = read_object_from_zip(path, "info.json").await?;
        // TODO: read changelog
        Ok(Self::from_zip_info(info, String::new()))
    }

    pub async fn from_portal(name: &str, portal: &ModPortal) -> anyhow::Result<Self> {
        let portal_info: PortalInfo = portal.fetch_mod(name).await?;
        let portal_info = compress_portal_info(portal_info);

        Ok(Self::from_portal_info(portal_info)?)
    }

    pub async fn from_cache(
        factorio_mod: models::FactorioMod,
        version: HumanVersion,
        cache: &Cache,
    ) -> anyhow::Result<Self> {
        trace!("Building info object from cached mod {:?}", factorio_mod);

        let name = factorio_mod.name;
        let mut releases = Vec::new();
        let mut this_release = None;

        for release in cache.get_mod_releases(name.clone()).await? {
            let mut dependencies = Vec::new();

            for cache_dep in cache
                .get_release_dependencies(name.clone(), release.version)
                .await?
            {
                dependencies.push(cache_dep.into());
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

    fn from_portal_info(info: PortalInfo) -> anyhow::Result<Self> {
        Ok(Self {
            name: info.name.ok_or(ModError::MissingField("name"))?,
            versions: None,
            author: Author {
                name: info.owner.unwrap_or_default(), // TODO: warn when default returned
                contact: None,
                homepage: info.homepage,
            },
            display: Display {
                title: info.title.ok_or(ModError::MissingField("name"))?,
                summary: info.summary,
                description: info.description.unwrap_or_default(),
                changelog: info.changelog,
            },
            dependencies: None,
            releases: info.releases,
        })
    }

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

    pub async fn populate_from_portal(&mut self, portal: &ModPortal) -> anyhow::Result<()> {
        trace!("Populating '{}' from portal", self.name);
        let info: PortalInfo = portal.fetch_mod(&self.name).await?;
        let info = compress_portal_info(info);

        // trace!("'{}' got PortalInfo: {:?}", self.name, info);
        self.display.summary = Some(info.summary.unwrap_or_default());
        self.releases = Some(info.releases.unwrap_or_default());

        Ok(())
    }

    pub async fn populate_with_cache_object(
        &mut self,
        cache: &'_ Cache,
        cache_mod: models::FactorioMod,
    ) -> anyhow::Result<()> {
        trace!("Mod '{}' got cached mod: {:?}", self.name, cache_mod);

        self.author.name = cache_mod.author;
        self.author.contact = cache_mod.contact;
        self.author.homepage = cache_mod.homepage;
        self.display.title = cache_mod.title;
        self.display.summary = cache_mod.summary;
        self.display.description = cache_mod.description;
        self.display.changelog = cache_mod.changelog;

        let mut releases = Vec::new();
        for release in cache.get_mod_releases(self.name.clone()).await? {
            trace!("Mod '{}' got cached release: {:?}", self.name, release);
            let mut dependencies = Vec::new();

            for cache_dep in cache
                .get_release_dependencies(self.name.clone(), release.version)
                .await?
            {
                dependencies.push(cache_dep.into());
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

    pub async fn populate_from_cache(&mut self, cache: &'_ Cache) -> anyhow::Result<()> {
        match cache.get_factorio_mod(self.name.clone()).await? {
            Some(cache_mod) => self.populate_with_cache_object(cache, cache_mod).await,
            None => {
                trace!(
                    "Mod '{}' not in cache while trying to load it from cache",
                    self.name
                );

                Err(ModError::ModNotInCache.into())
            }
        }
    }
}

impl Info {
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
    /// existing releases
    pub fn is_portal_populated(&self) -> bool {
        self.releases.is_some()
    }

    fn versions(&self) -> anyhow::Result<Versions> {
        Ok(self.versions.ok_or(ModError::MissingInfo)?)
    }

    pub fn name(&self) -> &str {
        &self.name
    }

    pub fn author(&self) -> &str {
        &self.author.name
    }

    pub fn contact(&self) -> Option<&str> {
        self.author.contact.as_deref()
    }

    pub fn homepage(&self) -> Option<&str> {
        self.author.homepage.as_deref()
    }

    pub fn title(&self) -> &str {
        &self.display.title
    }

    pub fn summary(&self) -> Option<&str> {
        self.display.summary.as_deref()
    }

    pub fn description(&self) -> &str {
        &self.display.description
    }

    pub fn changelog(&self) -> Option<&str> {
        self.display.changelog.as_deref()
    }

    pub fn own_version(&self) -> anyhow::Result<HumanVersion> {
        Ok(self.versions()?.own)
    }

    pub fn factorio_version(&self) -> anyhow::Result<HumanVersion> {
        Ok(self.versions()?.factorio)
    }

    pub fn releases(&self) -> anyhow::Result<Vec<Release>> {
        Ok(self
            .releases
            .as_ref()
            .cloned()
            .ok_or(ModError::MissingInfo)?)
    }

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

    pub fn dependencies(&self) -> anyhow::Result<Vec<Dependency>> {
        Ok(self
            .dependencies
            .as_ref()
            .cloned()
            .ok_or(ModError::MissingInfo)?)
    }
}

impl Release {
    pub fn url(&self) -> anyhow::Result<&str> {
        self.download_url.get_str()
    }

    pub fn version(&self) -> HumanVersion {
        self.version
    }

    pub fn factorio_version(&self) -> HumanVersion {
        self.info_object.factorio_version
    }

    pub fn released_on(&self) -> DateTime<Utc> {
        self.released_on
    }

    pub fn sha1(&self) -> &str {
        &self.sha1
    }

    pub fn dependencies(&self) -> Vec<&Dependency> {
        self.info_object.dependencies.iter().collect()
    }
}
