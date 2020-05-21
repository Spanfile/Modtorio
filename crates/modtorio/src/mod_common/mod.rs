mod dependency;
mod info;

use crate::{cache::models, util::HumanVersion, Cache, ModPortal};
use bytesize::ByteSize;
use chrono::Utc;
use info::Info;
use log::*;
use std::{fmt, path::Path};

pub use dependency::{Dependency, Requirement};
pub use info::Release;

pub struct Mod<'a> {
    info: Info,
    portal: &'a ModPortal,
    cache: &'a Cache,
}

#[derive(Debug)]
pub enum DownloadResult {
    New,
    Unchanged,
    Replaced {
        old_version: HumanVersion,
        old_archive: String,
    },
}

impl<'a> Mod<'a> {
    pub async fn from_zip<P>(
        path: P,
        portal: &'a ModPortal,
        cache: &'a Cache,
    ) -> anyhow::Result<Mod<'a>>
    where
        P: 'static + AsRef<Path> + Send,
    {
        let info = Info::from_zip(path).await?;

        Ok(Self {
            info,
            portal,
            cache,
        })
    }

    pub async fn from_portal(
        name: &str,
        portal: &'a ModPortal,
        cache: &'a Cache,
    ) -> anyhow::Result<Mod<'a>> {
        let info = Info::from_portal(name, portal).await?;

        Ok(Self {
            info,
            portal,
            cache,
        })
    }
}

impl<'a> Mod<'a> {
    pub async fn update_cache(&mut self) -> anyhow::Result<()> {
        if !self.info.is_portal_populated() {
            debug!(
                "Info not populated from portal before updating cache for '{}', populating...",
                self
            );

            self.fetch_portal_info().await?;
        }

        self.cache.set_factorio_mod(models::NewFactorioMod {
            name: self.info.name(),
            summary: self.info.summary(),
            last_updated: &Utc::now().to_string(),
        })?;

        for release in self.releases()? {
            self.cache.set_mod_release(models::NewModRelease {
                factorio_mod: self.info.name(),
                download_url: release.url()?,
                file_name: release.file_name(),
                released_on: &release.released_on().to_string(),
                version: &release.version().to_string(),
                sha1: release.sha1(),
                factorio_version: &release.factorio_version().to_string(),
            })?;

            self.cache.set_release_dependencies(
                &release
                    .dependencies()
                    .iter()
                    .map(|dependency| models::NewReleaseDependency {
                        release_mod_name: self.info.name(),
                        release_version: release.version().to_string(),
                        name: dependency.name(),
                        requirement: dependency.requirement() as i32,
                        version_req: dependency.version().map(|v| v.to_string()),
                    })
                    .collect::<Vec<models::NewReleaseDependency>>(),
            )?;
        }

        Ok(())
    }

    /// Fetch the latest info from portal
    pub async fn fetch_portal_info(&mut self) -> anyhow::Result<()> {
        self.info.populate_from_portal(self.portal).await
    }

    /// Load the potentially missing portal info by first reading it from cache, and then fetching
    /// from the mod portal if the cache has expired
    pub async fn load_portal_info(&mut self) -> anyhow::Result<()> {
        // if let Some(_cache_mod) = self.cache.get_mod(&self.info.name())? {
        //     // TODO: check expiry
        //     self.info.populate_from_cache(self.cache)?;
        //     return Ok(());
        // }

        self.fetch_portal_info().await
    }

    pub async fn download<P>(
        &mut self,
        version: Option<HumanVersion>,
        destination: P,
    ) -> anyhow::Result<DownloadResult>
    where
        P: AsRef<Path>,
    {
        let release = self.info.get_release(version)?;
        let (path, download_size) = self
            .portal
            .download_mod(release.url()?, destination)
            .await?;

        debug!(
            "{} ({} bytes) downloaded, populating info from {}",
            ByteSize::b(download_size as u64),
            download_size,
            path.display()
        );

        let old_version = self.own_version().ok();
        let old_archive = self.get_archive_filename()?;
        self.info.populate_from_zip(path).await?;

        if let Some(old_version) = old_version {
            if old_version == self.own_version()? {
                debug!("'{}' unchaged after download", self.name());
                Ok(DownloadResult::Unchanged)
            } else {
                debug!("'{}' changed from ver. {}", self.name(), old_version);
                Ok(DownloadResult::Replaced {
                    old_version,
                    old_archive,
                })
            }
        } else {
            debug!("'{}' newly downloaded", self.name());
            Ok(DownloadResult::New)
        }
    }
}

impl fmt::Display for Mod<'_> {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.write_fmt(format_args!(
            "'{}' ('{}') ver. {}",
            self.title(),
            self.name(),
            self.own_version()
                .map_or_else(|_| String::from("unknown"), |v| v.to_string())
        ))
    }
}

impl Mod<'_> {
    pub fn get_archive_filename(&self) -> anyhow::Result<String> {
        Ok(format!(
            "{}_{}.zip",
            self.info.name(),
            self.info.own_version()?
        ))
    }

    pub fn name(&self) -> &str {
        self.info.name()
    }

    pub fn title(&self) -> &str {
        self.info.title()
    }

    pub fn own_version(&self) -> anyhow::Result<HumanVersion> {
        self.info.own_version()
    }

    pub fn releases(&self) -> anyhow::Result<&Vec<Release>> {
        self.info.releases()
    }

    pub fn latest_release(&self) -> anyhow::Result<&Release> {
        self.info.get_release(None)
    }

    pub fn dependencies(&self) -> anyhow::Result<&[Dependency]> {
        self.info.dependencies()
    }
}
