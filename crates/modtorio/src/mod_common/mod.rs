mod dependency;
mod info;

use crate::{cache::models, util::HumanVersion, Cache, ModPortal};
use bytesize::ByteSize;
use chrono::Utc;
use info::Info;
use log::*;
use std::{
    path::{Path, PathBuf},
    sync::Arc,
};
use tokio::sync::Mutex;

pub use dependency::{Dependency, Requirement};
pub use info::Release;

pub struct Mod {
    info: Mutex<Info>,
    portal: Arc<ModPortal>,
    cache: Arc<Cache>,
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

impl Mod {
    pub async fn from_zip<P>(
        path: P,
        portal: Arc<ModPortal>,
        cache: Arc<Cache>,
    ) -> anyhow::Result<Mod>
    where
        P: 'static + AsRef<Path> + Send,
    {
        let info = Mutex::new(Info::from_zip(path).await?);

        Ok(Self {
            info,
            portal,
            cache,
        })
    }

    pub async fn from_portal(
        name: &str,
        portal: Arc<ModPortal>,
        cache: Arc<Cache>,
    ) -> anyhow::Result<Mod> {
        let info = Mutex::new(Info::from_portal(name, portal.as_ref()).await?);

        Ok(Self {
            info,
            portal,
            cache,
        })
    }
}

impl Mod {
    pub async fn update_cache(&self) -> anyhow::Result<()> {
        if !self.is_portal_populated().await {
            debug!(
                "Info not populated from portal before updating cache for '{}', populating...",
                self.name().await
            );

            self.fetch_portal_info().await?;
        }

        let new_factorio_mod = models::NewFactorioMod {
            name: self.name().await,
            summary: self.summary().await,
            last_updated: Utc::now().to_string(),
        };
        trace!("'{}' cached mod: {:?}", self.name().await, new_factorio_mod);
        self.cache.set_factorio_mod(new_factorio_mod).await?;

        for release in self.releases().await? {
            let new_mod_release = models::NewModRelease {
                factorio_mod: self.name().await,
                download_url: release.url()?.to_string(),
                file_name: release.file_name().to_string(),
                released_on: release.released_on().to_string(),
                version: release.version().to_string(),
                sha1: release.sha1().to_string(),
                factorio_version: release.factorio_version().to_string(),
            };
            trace!(
                "'{}'s cached release {}: {:?}",
                self.name().await,
                release.version(),
                new_mod_release
            );
            self.cache.set_mod_release(new_mod_release).await?;

            let mut new_release_dependencies = Vec::new();
            for dependency in release.dependencies().into_iter() {
                new_release_dependencies.push(models::NewReleaseDependency {
                    release_mod_name: self.name().await,
                    release_version: release.version().to_string(),
                    name: dependency.name().to_string(),
                    requirement: dependency.requirement() as i32,
                    version_req: dependency.version().map(|v| v.to_string()),
                });
            }

            trace!(
                "'{}'s release {}'s cached dependencies: {:?}",
                self.name().await,
                release.version(),
                new_release_dependencies
            );
            self.cache
                .set_release_dependencies(new_release_dependencies)
                .await?;
        }

        Ok(())
    }

    /// Fetch the latest info from portal
    pub async fn fetch_portal_info(&self) -> anyhow::Result<()> {
        let mut info = self.info.lock().await;
        info.populate_from_portal(self.portal.as_ref()).await
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
        &self,
        version: Option<HumanVersion>,
        destination: P,
    ) -> anyhow::Result<DownloadResult>
    where
        P: AsRef<Path>,
    {
        let release = self.latest_release().await?;
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

        let old_version = self.own_version().await.ok();
        let old_archive = self.get_archive_filename().await?;
        self.populate_info_from_zip(path).await?;

        if let Some(old_version) = old_version {
            if old_version == self.own_version().await? {
                debug!("'{}' unchaged after download", self.name().await);
                Ok(DownloadResult::Unchanged)
            } else {
                debug!("'{}' changed from ver. {}", self.name().await, old_version);
                Ok(DownloadResult::Replaced {
                    old_version,
                    old_archive,
                })
            }
        } else {
            debug!("'{}' newly downloaded", self.name().await);
            Ok(DownloadResult::New)
        }
    }
}

impl Mod {
    async fn populate_info_from_zip(&self, path: PathBuf) -> anyhow::Result<()> {
        self.info.lock().await.populate_from_zip(path).await
    }

    async fn is_portal_populated(&self) -> bool {
        self.info.lock().await.is_portal_populated()
    }

    pub async fn display(&self) -> String {
        self.info.lock().await.display()
    }

    pub async fn get_archive_filename(&self) -> anyhow::Result<String> {
        let info = self.info.lock().await;
        Ok(format!("{}_{}.zip", info.name(), info.own_version()?))
    }

    pub async fn name(&self) -> String {
        let info = self.info.lock().await;
        info.name().to_string()
    }

    pub async fn title(&self) -> String {
        let info = self.info.lock().await;
        info.title().to_string()
    }

    pub async fn summary(&self) -> Option<String> {
        let info = self.info.lock().await;
        info.summary().map(|s| s.to_string())
    }

    pub async fn own_version(&self) -> anyhow::Result<HumanVersion> {
        let info = self.info.lock().await;
        info.own_version()
    }

    pub async fn releases(&self) -> anyhow::Result<Vec<Release>> {
        let info = self.info.lock().await;
        info.releases()
    }

    pub async fn latest_release(&self) -> anyhow::Result<Release> {
        let info = self.info.lock().await;
        info.get_release(None)
    }

    pub async fn dependencies(&self) -> anyhow::Result<Vec<Dependency>> {
        let info = self.info.lock().await;
        info.dependencies()
    }
}
