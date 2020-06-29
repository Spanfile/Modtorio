use super::Dependency;
use crate::{
    cache::models,
    ext::{PathExt, ZipExt},
    mod_portal::ModPortal,
    util,
    util::HumanVersion,
    Cache,
};
use anyhow::{anyhow, ensure};
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
    name: String,
    owner: String,
    releases: Vec<Release>,
    summary: String,
    title: String,
    changelog: Option<String>,
    description: String,
    homepage: String,
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
    P: 'static + AsRef<Path> + Send,
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
    let new_releases = info
        .releases
        .into_iter()
        .map(|release| Release {
            download_url: util::get_last_path_segment(release.download_url),
            ..release
        })
        .collect();

    PortalInfo {
        releases: new_releases,
        ..info
    }
}

impl Info {
    pub async fn from_zip<P>(path: P) -> anyhow::Result<Info>
    where
        P: 'static + AsRef<Path> + Send,
    {
        let info = read_object_from_zip(path, "info.json").await?;
        // TODO: read changelog
        Ok(Self::from_zip_info(info, String::new()))
    }

    pub async fn from_portal(name: &str, portal: &ModPortal) -> anyhow::Result<Info> {
        let portal_info: PortalInfo = portal.fetch_mod(name).await?;
        let portal_info = compress_portal_info(portal_info);

        Ok(Self::from_portal_info(portal_info))
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

    fn from_portal_info(info: PortalInfo) -> Self {
        Self {
            name: info.name,
            versions: None,
            author: Author {
                name: info.owner,
                contact: None,
                homepage: Some(info.homepage),
            },
            display: Display {
                title: info.title,
                summary: Some(info.summary),
                description: info.description,
                changelog: info.changelog,
            },
            dependencies: None,
            releases: Some(info.releases),
        }
    }

    pub async fn populate_from_zip<P>(&mut self, path: P) -> anyhow::Result<()>
    where
        P: 'static + AsRef<Path> + Send,
    {
        let info: ZipInfo = read_object_from_zip(path, "info.json").await?;
        ensure!(
            info.name == self.name,
            anyhow!(
                "Mod zip name doesn't match existing name ({} vs {})",
                info.name,
                self.name
            )
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

        trace!("'{}' got PortalInfo: {:?}", self.name, info);
        self.display.summary = Some(info.summary);
        self.releases = Some(info.releases);

        Ok(())
    }

    pub async fn populate_with_cache_object(
        &mut self,
        cache: &'_ Cache,
        cache_mod: models::FactorioMod,
    ) -> anyhow::Result<()> {
        trace!("Mod '{}' got cached mod: {:?}", self.name, cache_mod);
        self.display.summary = cache_mod.summary;

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

                Err(anyhow!("mod not in cache"))
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
        Ok(self
            .versions
            .ok_or_else(|| anyhow!("Missing version info (is the mod installed?)"))?)
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
            .ok_or_else(|| anyhow!("Missing releases (has the mod been fetched from portal?)"))?)
    }

    pub fn get_release(&self, version: Option<HumanVersion>) -> anyhow::Result<Release> {
        let releases = self.releases()?;

        match version {
            Some(version) => releases
                .iter()
                .find(|r| r.version == version)
                .cloned()
                .ok_or_else(|| {
                    anyhow!(
                        "Mod '{}' doesn't have a release version {}",
                        self.name,
                        version
                    )
                }),
            None => releases
                .iter()
                .last()
                .cloned()
                .ok_or_else(|| anyhow!("Mod '{}' has no releases", self.name)),
        }
    }

    pub fn dependencies(&self) -> anyhow::Result<Vec<Dependency>> {
        self.dependencies
            .as_ref()
            .cloned()
            .ok_or_else(|| anyhow!("Missing dependencies (has the mod been fetched from portal?)"))
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
