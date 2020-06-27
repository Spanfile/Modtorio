mod dependency;
mod info;

use crate::{cache::models, util::HumanVersion, Cache, Config, ModPortal};
use bytesize::ByteSize;
use chrono::{DateTime, Utc};
use info::Info;
use log::*;
use std::{
    path::{Path, PathBuf},
    sync::Arc,
    time::Duration,
};
use tokio::sync::Mutex;

pub use dependency::{Dependency, Requirement};
pub use info::Release;

pub struct Mod {
    info: Mutex<Info>,
    config: Arc<Config>,
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
        config: Arc<Config>,
        portal: Arc<ModPortal>,
        cache: Arc<Cache>,
    ) -> anyhow::Result<Mod>
    where
        P: 'static + AsRef<Path> + Send,
    {
        let info = Mutex::new(Info::from_zip(path).await?);

        Ok(Self {
            info,
            config,
            portal,
            cache,
        })
    }

    pub async fn from_portal(
        name: &str,
        config: Arc<Config>,
        portal: Arc<ModPortal>,
        cache: Arc<Cache>,
    ) -> anyhow::Result<Mod> {
        let info = Mutex::new(Info::from_portal(name, portal.as_ref()).await?);

        Ok(Self {
            info,
            config,
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

        let name = self.name().await;
        let author = self.author().await;
        let contact = self.contact().await;
        let homepage = self.homepage().await;
        let title = self.title().await;
        let summary = self.summary().await;
        let description = self.description().await;
        let changelog = self.changelog().await;
        let version = self.own_version().await?.to_string();
        let factorio_version = self.factorio_version().await?.to_string();

        let new_factorio_mod = models::FactorioMod {
            name,
            author,
            contact,
            homepage,
            title,
            summary,
            description,
            changelog,
            version,
            factorio_version,
            last_updated: Utc::now().to_string(),
        };

        trace!("'{}' cached mod: {:?}", self.name().await, new_factorio_mod);
        self.cache.set_factorio_mod(new_factorio_mod).await?;

        for release in self.releases().await? {
            let new_mod_release = models::ModRelease {
                factorio_mod: self.name().await,
                download_url: release.url()?.to_string(),
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
                new_release_dependencies.push(models::ReleaseDependency {
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

    /// Fetch the latest info from cache
    pub async fn fetch_cache_info(&self) -> anyhow::Result<()> {
        let mut info = self.info.lock().await;
        info.populate_from_cache(self.cache.as_ref()).await
    }

    /// Load the potentially missing portal info by first reading it from cache, and then fetching
    /// from the mod portal if the cache has expired
    pub async fn ensure_portal_info(&self) -> anyhow::Result<()> {
        if let Some(cache_mod) = self.cache.get_factorio_mod(self.name().await).await? {
            let last_updated = cache_mod.last_updated.parse::<DateTime<Utc>>()?;
            let time_since_updated = Utc::now() - last_updated;
            let expired =
                time_since_updated.to_std()? > Duration::from_secs(self.config.cache_expiry);

            trace!(
                "Ensuring mod '{}' has portal info. Got cached mod: {:?}. Expired: {} (configured \
                 expiry {} seconds)",
                self.name().await,
                cache_mod,
                expired,
                self.config.cache_expiry,
            );

            if !expired {
                let mut info = self.info.lock().await;
                info.populate_with_cache_object(self.cache.as_ref(), cache_mod)
                    .await?;

                return Ok(());
            }
        }

        // TODO: update the cache here?
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
        let release = if let Some(version) = version {
            self.get_release(version).await?
        } else {
            self.latest_release().await?
        };

        let (path, download_size) = self
            .portal
            .download_mod(&self.name().await, release.url()?, destination)
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

    pub async fn author(&self) -> String {
        let info = self.info.lock().await;
        info.author().to_string()
    }

    pub async fn contact(&self) -> Option<String> {
        let info = self.info.lock().await;
        info.contact().map(|c| c.to_string())
    }

    pub async fn homepage(&self) -> Option<String> {
        let info = self.info.lock().await;
        info.homepage().map(|c| c.to_string())
    }

    pub async fn title(&self) -> String {
        let info = self.info.lock().await;
        info.title().to_string()
    }

    pub async fn summary(&self) -> Option<String> {
        let info = self.info.lock().await;
        info.summary().map(|s| s.to_string())
    }

    pub async fn description(&self) -> String {
        let info = self.info.lock().await;
        info.description().to_string()
    }

    pub async fn changelog(&self) -> Option<String> {
        let info = self.info.lock().await;
        info.changelog().map(|s| s.to_string())
    }

    pub async fn own_version(&self) -> anyhow::Result<HumanVersion> {
        let info = self.info.lock().await;
        info.own_version()
    }

    pub async fn factorio_version(&self) -> anyhow::Result<HumanVersion> {
        let info = self.info.lock().await;
        info.factorio_version()
    }

    pub async fn releases(&self) -> anyhow::Result<Vec<Release>> {
        let info = self.info.lock().await;
        info.releases()
    }

    pub async fn get_release(&self, version: HumanVersion) -> anyhow::Result<Release> {
        let info = self.info.lock().await;
        info.get_release(Some(version))
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
