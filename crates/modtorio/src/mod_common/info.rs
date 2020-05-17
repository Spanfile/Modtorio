use super::Dependency;
use crate::{
    ext::{PathExt, ZipExt},
    mod_portal::ModPortal,
    util::HumanVersion,
};
use anyhow::{anyhow, ensure};
use chrono::{DateTime, Utc};
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

#[derive(Debug, Deserialize)]
pub struct Release {
    download_url: PathBuf,
    file_name: String,
    #[serde(rename = "released_at")]
    released_on: DateTime<Utc>,
    version: HumanVersion,
    sha1: String,
    #[serde(rename = "info_json")]
    info_object: ReleaseInfoObject,
}

#[derive(Debug, Deserialize)]
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
        let info: PortalInfo = portal.fetch_mod(&self.name).await?;

        self.display.summary = Some(info.summary);
        self.releases = Some(info.releases);

        Ok(())
    }
}

impl Info {
    fn versions(&self) -> anyhow::Result<Versions> {
        Ok(self
            .versions
            .ok_or_else(|| anyhow!("Missing version info (is the mod installed?)"))?)
    }

    pub fn name(&self) -> &str {
        &self.name
    }

    pub fn title(&self) -> &str {
        &self.display.title
    }

    pub fn own_version(&self) -> anyhow::Result<HumanVersion> {
        Ok(self.versions()?.own)
    }

    pub fn get_release(&self, version: Option<HumanVersion>) -> anyhow::Result<&Release> {
        let releases = self
            .releases
            .as_ref()
            .ok_or_else(|| anyhow!("Missing releases (has the mod been fetched from portal?)"))?;

        match version {
            Some(version) => releases
                .iter()
                .find(|r| r.version == version)
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
                .ok_or_else(|| anyhow!("Mod '{}' has no releases", self.name)),
        }
    }

    pub fn dependencies(&self) -> anyhow::Result<&[Dependency]> {
        self.dependencies
            .as_deref()
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

    pub fn released_on(&self) -> DateTime<Utc> {
        self.released_on
    }
}
