mod dependency;
mod info;

use crate::{
    cache::models,
    error::ModError,
    util::{self, HumanVersion},
    Cache, Config, ModPortal,
};
use bytesize::ByteSize;
use chrono::Utc;
use info::Info;
use log::*;
use std::{
    path::{Path, PathBuf},
    sync::Arc,
    time::Duration,
};
use tokio::{sync::Mutex, task};

pub use dependency::{Dependency, Requirement};
pub use info::Release;

pub struct Mod {
    info: Mutex<Info>,
    config: Arc<Config>,
    portal: Arc<ModPortal>,
    cache: Arc<Cache>,
    zip_path: Arc<Mutex<Option<PathBuf>>>,
    zip_checksum: Arc<Mutex<Option<String>>>,
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

async fn calculate_zip_checksum<P>(zip: P) -> anyhow::Result<String>
where
    P: AsRef<Path>,
{
    let path = zip.as_ref().to_owned();
    let result = task::spawn_blocking(move || -> anyhow::Result<String> {
        util::checksum::blake2b_file(path)
    })
    .await?;

    let result = result?;
    trace!(
        "Calculated zip checksum ({}): {}",
        zip.as_ref().display(),
        result
    );
    Ok(result)
}

async fn verify_zip(cached_mod: &models::GameMod) -> anyhow::Result<()> {
    let zip_path = PathBuf::from(&cached_mod.mod_zip);
    if !zip_path.exists() {
        return Err(ModError::MissingZip(zip_path).into());
    }

    trace!(
        "Verifying mod zip ({}) checksum (expecting {})...",
        zip_path.display(),
        cached_mod.zip_checksum
    );
    let existing_zip_checksum = calculate_zip_checksum(&zip_path).await?;
    if existing_zip_checksum != cached_mod.zip_checksum {
        return Err(ModError::ZipChecksumMismatch {
            zip_checksum: cached_mod.zip_checksum.clone(),
            expected: existing_zip_checksum,
        }
        .into());
    }
    Ok(())
}

impl Mod {
    pub async fn from_cache(
        game_mod: &models::GameMod,
        config: Arc<Config>,
        portal: Arc<ModPortal>,
        cache: Arc<Cache>,
    ) -> anyhow::Result<Mod> {
        debug!("Creating mod from cached: {:?}", game_mod);
        let factorio_mod = cache
            .get_factorio_mod(game_mod.factorio_mod.clone())
            .await?
            .ok_or(ModError::ModNotInCache)?;
        let info =
            Mutex::new(Info::from_cache(factorio_mod, game_mod.mod_version, cache.as_ref()).await?);

        debug!(
            "Verifying mod '{}' zip ({}) against cache...",
            game_mod.factorio_mod, game_mod.mod_zip
        );
        verify_zip(&game_mod).await?;

        Ok(Self {
            info,
            config,
            portal,
            cache,
            zip_path: Arc::new(Mutex::new(Some(PathBuf::from(game_mod.mod_zip.clone())))),
            zip_checksum: Arc::new(Mutex::new(Some(game_mod.zip_checksum.clone()))),
        })
    }

    pub async fn from_zip<P>(
        path: P,
        config: Arc<Config>,
        portal: Arc<ModPortal>,
        cache: Arc<Cache>,
    ) -> anyhow::Result<Mod>
    where
        P: AsRef<Path>,
    {
        debug!("Creating mod from zip: '{}'", path.as_ref().display());
        let info = Mutex::new(Info::from_zip(path.as_ref().to_owned()).await?);
        let zip_checksum = calculate_zip_checksum(&path).await?;

        Ok(Self {
            info,
            config,
            portal,
            cache,
            zip_path: Arc::new(Mutex::new(Some(path.as_ref().to_owned()))),
            zip_checksum: Arc::new(Mutex::new(Some(zip_checksum))),
        })
    }

    pub async fn from_portal(
        name: &str,
        config: Arc<Config>,
        portal: Arc<ModPortal>,
        cache: Arc<Cache>,
    ) -> anyhow::Result<Mod> {
        debug!("Creating mod from portal: '{}'", name);
        let info = Mutex::new(Info::from_portal(name, portal.as_ref()).await?);

        Ok(Self {
            info,
            config,
            portal,
            cache,
            zip_path: Arc::new(Mutex::new(None)),
            zip_checksum: Arc::new(Mutex::new(None)),
        })
    }
}

impl Mod {
    pub async fn update_cache(&self) -> anyhow::Result<()> {
        trace!("Updating cache for '{}'", self.name().await);

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
        let new_factorio_mod = models::FactorioMod {
            name,
            author,
            contact,
            homepage,
            title,
            summary,
            description,
            changelog,
            last_updated: Utc::now(),
        };

        // trace!("'{}' cached mod: {:?}", self.name().await, new_factorio_mod);
        self.cache.set_factorio_mod(new_factorio_mod).await?;

        for release in self.releases().await? {
            let new_mod_release = models::ModRelease {
                factorio_mod: self.name().await,
                download_url: release.url()?.to_string(),
                released_on: release.released_on(),
                version: release.version(),
                sha1: release.sha1().to_string(),
                factorio_version: release.factorio_version(),
            };
            // trace!(
            //     "'{}'s cached release {}: {:?}",
            //     self.name().await,
            //     release.version(),
            //     new_mod_release
            // );
            self.cache.set_mod_release(new_mod_release).await?;

            let mut new_release_dependencies = Vec::new();
            for dependency in release.dependencies().into_iter() {
                new_release_dependencies.push(models::ReleaseDependency {
                    release_mod_name: self.name().await,
                    release_version: release.version(),
                    name: dependency.name().to_string(),
                    requirement: dependency.requirement(),
                    version_req: dependency.version(),
                });
            }

            // trace!(
            //     "'{}'s release {}'s cached dependencies: {:?}",
            //     self.name().await,
            //     release.version(),
            //     new_release_dependencies
            // );
            self.cache
                .set_release_dependencies(new_release_dependencies)
                .await?;
        }

        Ok(())
    }

    /// Fetch the latest info from portal
    pub async fn fetch_portal_info(&self) -> anyhow::Result<()> {
        trace!("Fetcing portal info for '{}'", self.name().await);

        let mut info = self.info.lock().await;
        info.populate_from_portal(self.portal.as_ref()).await
    }

    /// Fetch the latest info from cache
    pub async fn fetch_cache_info(&self) -> anyhow::Result<()> {
        trace!("Fetcing cache info for '{}'", self.name().await);

        let mut info = self.info.lock().await;
        info.populate_from_cache(self.cache.as_ref()).await
    }

    /// Load the potentially missing portal info by first reading it from cache, and then fetching
    /// from the mod portal if the cache has expired
    pub async fn ensure_portal_info(&self) -> anyhow::Result<()> {
        trace!("Ensuring info for '{}'", self.name().await);

        if let Some(cache_mod) = self.cache.get_factorio_mod(self.name().await).await? {
            let time_since_updated = Utc::now() - cache_mod.last_updated;
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
        trace!(
            "Downloading version {:?} of '{}' to {}",
            version,
            self.name().await,
            destination.as_ref().display()
        );

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
        let old_archive = self.zip_path().await?.display().to_string();
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
        *self.zip_path.lock().await = Some(path.clone());
        self.info.lock().await.populate_from_zip(path).await?;
        Ok(())
    }

    async fn is_portal_populated(&self) -> bool {
        self.info.lock().await.is_portal_populated()
    }

    pub async fn display(&self) -> String {
        self.info.lock().await.display()
    }

    pub async fn zip_path(&self) -> anyhow::Result<PathBuf> {
        Ok(self
            .zip_path
            .lock()
            .await
            .clone()
            .ok_or(ModError::MissingZipPath)?)
    }

    pub async fn get_zip_checksum(&self) -> anyhow::Result<String> {
        if let Some(checksum) = self.zip_checksum.lock().await.as_ref() {
            return Ok(checksum.to_owned());
        }

        let checksum = calculate_zip_checksum(self.zip_path().await?).await?;
        *self.zip_checksum.lock().await = Some(checksum.clone());

        trace!(
            "Calculated zip checksum for mod '{}' ({}): {}",
            self.title().await,
            self.zip_path().await?.display(),
            checksum
        );
        Ok(checksum)
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
