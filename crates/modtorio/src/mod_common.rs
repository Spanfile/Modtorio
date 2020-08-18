//! Provides the [`Mod`](Mod) object and various tools to work with Factorio mods.

mod dependency;
mod info;

use crate::{
    error::ModError,
    mod_portal::PortalResult,
    store::{models, Store},
    util::{self, file, HumanVersion},
    Config, ModPortal,
};
use bytesize::ByteSize;
use chrono::{DateTime, Utc};
use info::Info;
use log::*;
use std::{
    path::{Path, PathBuf},
    sync::Arc,
    time::Duration,
};
use tokio::{sync::RwLock, task};
use util::ext::PathExt;

pub use dependency::{Dependency, Requirement};
pub use info::Release;

/// A Factorio mod.
///
/// Consists of a combination of mod information from both a mod zip archive and the mod portal.
/// Provides functions to build, download and update mods from the [program store](crate::store),
/// zip archives and the [mod portal](crate::mod_portal).
pub struct Mod {
    /// The mod's info.
    info: RwLock<Info>,
    /// Reference to the program config.
    config: Arc<Config>,
    /// Reference to the mod portal.
    portal: Arc<ModPortal>,
    /// Reference to the program store.
    store: Arc<Store>,
    /// Path to the installed zip archive.
    zip_path: Arc<RwLock<Option<PathBuf>>>,
    /// The installed zip archive's last modified time.
    zip_last_mtime: Arc<RwLock<Option<DateTime<Utc>>>>,
}

/// The result of a mod zip archive download.
#[derive(Debug)]
pub enum DownloadResult {
    /// The downloaded archive is entirely new.
    New,
    /// The downloaded archive is the same as its previous install.
    Unchanged,
    /// The downloaded archive replaced an older install.
    Replaced {
        /// The older install's version.
        old_version: HumanVersion,
        /// The older install's archive's filesystem path.
        old_archive: String,
    },
}

/// The available checksum algorithms.
enum ChecksumAlgorithm {
    /// The `BLAKE2b`-algorith.
    BLAKE2b,
    /// The `SHA1`-algorithm.
    SHA1,
}

/// The default checksum algorithm to use to verifying stored mods.
const STORE_ZIP_CHECKSUM_ALGO: ChecksumAlgorithm = ChecksumAlgorithm::BLAKE2b;
/// The algorithm used to verify downloaded mods from the mod portal. This is dictated by what the
/// mod portal returns as a checksum.
const DOWNLOADED_ZIP_CHECKSUM_ALGO: ChecksumAlgorithm = ChecksumAlgorithm::SHA1;

/// Calculates a mod zip archive's checksum using the given checksum algorithm and returns it as a
/// `Result<String>`.
///
/// The checksum is calculated in a blocking thread with `task::spawn_blocking`. The function will
/// return an error if:
/// * The task spawning fails
/// * The checksum function fails
async fn calculate_zip_checksum<P>(algorithm: ChecksumAlgorithm, zip: P) -> anyhow::Result<String>
where
    P: AsRef<Path>,
{
    let path = zip.as_ref().to_owned();
    let result = match algorithm {
        ChecksumAlgorithm::BLAKE2b => {
            task::spawn_blocking(move || -> anyhow::Result<String> { util::checksum::blake2b_file(path) })
        }
        ChecksumAlgorithm::SHA1 => {
            task::spawn_blocking(move || -> anyhow::Result<String> { util::checksum::sha1_file(path) })
        }
    }
    .await?;

    let result = result?;
    trace!("Calculated zip checksum ({}): {}", zip.as_ref().display(), result);
    Ok(result)
}

/// Verifies that a given `GameMod`'s corresponding zip archive exists and is valid.
///
/// Returns `Ok(())` if the zip archive passes the validity checks, otherwise returns an `Err()`
/// containing a variant of `ModError` that describes which part of the check failed.
///
/// The validity check will:
/// * Ensure the zip archive's checksum matches what is stored
async fn verify_zip<P>(game_mod: &models::GameMod, mods_root_path: P) -> anyhow::Result<()>
where
    P: AsRef<Path>,
{
    let zip_path = mods_root_path.as_ref().join(&game_mod.mod_zip);
    if !zip_path.exists() {
        return Err(ModError::MissingZip(zip_path).into());
    }

    trace!(
        "Verifying mod zip ({}) last mtime (expecting {})...",
        zip_path.display(),
        game_mod.zip_last_mtime
    );
    let existing_zip_last_mtime = file::get_last_mtime(&zip_path)?;

    if existing_zip_last_mtime > game_mod.zip_last_mtime {
        return Err(ModError::ZipLastMtimeMismatch {
            last_mtime: existing_zip_last_mtime,
            expected: game_mod.zip_last_mtime,
        }
        .into());
    }
    Ok(())
}

impl Mod {
    /// Builds a new mod from a given stored `GameMod` and mod root directory.
    pub async fn from_store<P>(
        game_mod: &models::GameMod,
        mods_root_path: P,
        config: Arc<Config>,
        portal: Arc<ModPortal>,
        store: Arc<Store>,
    ) -> anyhow::Result<Mod>
    where
        P: AsRef<Path>,
    {
        debug!("Creating mod from stored: {:?}", game_mod);
        let factorio_mod = store
            .get_factorio_mod(game_mod.factorio_mod.clone())
            .await?
            .ok_or(ModError::ModNotInStore)?;
        let info = RwLock::new(Info::from_store(factorio_mod, game_mod.mod_version, store.as_ref()).await?);

        debug!(
            "Verifying mod '{}' zip ({}) against store...",
            game_mod.factorio_mod, game_mod.mod_zip
        );

        let zip_path = PathBuf::from(&game_mod.mod_zip);
        if let Err(e) = verify_zip(&game_mod, &mods_root_path).await {
            if let Some(ModError::ZipLastMtimeMismatch { last_mtime, expected }) = e.downcast_ref() {
                warn!(
                    "Stored mod '{}' failed to load: zip archive changed on filesystem after storing (last mtime: {}, \
                     stored mtime: {}). Ignoring store and loading mod from zip.",
                    game_mod.factorio_mod, last_mtime, expected
                );
                Ok(Self::from_zip(mods_root_path.as_ref().join(&zip_path), config, portal, store).await?)
            } else {
                Err(e)
            }
        } else {
            Ok(Self {
                info,
                config,
                portal,
                store,
                zip_path: Arc::new(RwLock::new(Some(zip_path))),
                zip_last_mtime: Arc::new(RwLock::new(Some(game_mod.zip_last_mtime))),
            })
        }
    }

    /// Builds a new mod from a given path to a mod zip archive.
    pub async fn from_zip<P>(
        path: P,
        config: Arc<Config>,
        portal: Arc<ModPortal>,
        store: Arc<Store>,
    ) -> anyhow::Result<Mod>
    where
        P: AsRef<Path>,
    {
        debug!("Creating mod from zip: '{}'", path.as_ref().display());
        let info = RwLock::new(Info::from_zip(path.as_ref().to_owned()).await?);
        let zip_path = PathBuf::from(path.as_ref().get_file_name()?);
        let zip_last_mtime = file::get_last_mtime(&path)?;

        Ok(Self {
            info,
            config,
            portal,
            store,
            zip_path: Arc::new(RwLock::new(Some(zip_path))),
            zip_last_mtime: Arc::new(RwLock::new(Some(zip_last_mtime))),
        })
    }

    /// Builds a new mod from the mod portal.
    pub async fn from_portal(
        name: &str,
        config: Arc<Config>,
        portal: Arc<ModPortal>,
        store: Arc<Store>,
    ) -> anyhow::Result<Mod> {
        debug!("Creating mod from portal: '{}'", name);
        let info = RwLock::new(Info::from_portal(name, portal.as_ref()).await?);

        Ok(Self {
            info,
            config,
            portal,
            store,
            zip_path: Arc::new(RwLock::new(None)),
            zip_last_mtime: Arc::new(RwLock::new(None)),
        })
    }
}

impl Mod {
    /// Updates the mod's store
    pub async fn update_store(&self) -> anyhow::Result<()> {
        trace!("Updating store for '{}'", self.name().await);

        if !self.is_portal_populated().await {
            debug!(
                "Info not populated from portal before updating store for '{}', populating...",
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

        // trace!("'{}' stored mod: {:?}", self.name().await, new_factorio_mod);
        self.store.set_factorio_mod(new_factorio_mod).await?;

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
            //     "'{}'s stored release {}: {:?}",
            //     self.name().await,
            //     release.version(),
            //     new_mod_release
            // );
            self.store.set_mod_release(new_mod_release).await?;

            let mut new_release_dependencies = Vec::new();
            for dependency in release.dependencies() {
                new_release_dependencies.push(models::ReleaseDependency {
                    release_mod_name: self.name().await,
                    release_version: release.version(),
                    name: dependency.name().to_string(),
                    requirement: dependency.requirement(),
                    version_req: dependency.version(),
                });
            }

            // trace!(
            //     "'{}'s release {}'s stored dependencies: {:?}",
            //     self.name().await,
            //     release.version(),
            //     new_release_dependencies
            // );
            self.store.set_release_dependencies(new_release_dependencies).await?;
        }

        Ok(())
    }

    /// Fetch the latest info from portal
    pub async fn fetch_portal_info(&self) -> anyhow::Result<()> {
        trace!("Fetcing portal info for '{}'", self.name().await);

        let mut info = self.info.write().await;
        info.populate_from_portal(self.portal.as_ref()).await
    }

    /// Fetch the latest info from store
    pub async fn fetch_store_info(&self) -> anyhow::Result<()> {
        trace!("Fetcing store info for '{}'", self.name().await);

        let mut info = self.info.write().await;
        info.populate_from_store(self.store.as_ref()).await
    }

    /// Applies a given prefetched portal info object.
    pub async fn apply_portal_info(&self, portal_info: PortalResult) -> anyhow::Result<()> {
        if self.name().await != portal_info.name()? {
            return Err(ModError::NameMismatch {
                own: self.name().await,
                given: portal_info.name()?.to_owned(),
            }
            .into());
        }

        let mut info = self.info.write().await;
        info.populate_with_portal_object(portal_info)
    }

    /// Load the potentially missing portal info by first reading it from store, and then fetching
    /// from the mod portal if the store has expired
    pub async fn ensure_portal_info(&self) -> anyhow::Result<()> {
        trace!("Ensuring info for '{}'", self.name().await);

        if let Some(store_mod) = self.store.get_factorio_mod(self.name().await).await? {
            let time_since_updated = Utc::now() - store_mod.last_updated;
            let expired = time_since_updated.to_std()? > Duration::from_secs(self.config.store_expiry());

            trace!(
                "Ensuring mod '{}' has portal info. Got stored mod: {:?}. Expired: {} (configured expiry {} seconds)",
                self.name().await,
                store_mod,
                expired,
                self.config.store_expiry(),
            );

            if !expired {
                let mut info = self.info.write().await;
                info.populate_with_store_object(self.store.as_ref(), store_mod).await?;

                return Ok(());
            }
        }

        // TODO: update the store here?
        self.fetch_portal_info().await
    }

    /// Download a certain version of the mod. If no version is given, downloads the latest version.
    pub async fn download<P>(&self, version: Option<HumanVersion>, destination: P) -> anyhow::Result<DownloadResult>
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
            "{} ({} bytes) downloaded, validating checksum...",
            ByteSize::b(download_size as u64),
            download_size,
        );

        let download_checksum = calculate_zip_checksum(DOWNLOADED_ZIP_CHECKSUM_ALGO, &path).await?;
        let checksums_match = download_checksum == release.sha1();
        trace!(
            "Got downloaded zip checksum: {} (matches: {})",
            download_checksum,
            checksums_match
        );

        if !checksums_match {
            return Err(ModError::ZipChecksumMismatch {
                zip_checksum: download_checksum,
                expected: release.sha1().to_owned(),
            }
            .into());
        }

        let old_version = self.own_version().await.ok();
        let old_archive = self.zip_path().await.ok();
        self.populate_info_from_zip(path).await?;

        if let (Some(old_version), Some(old_archive)) = (old_version, old_archive) {
            if old_version == self.own_version().await? {
                debug!("'{}' unchaged after download", self.name().await);
                Ok(DownloadResult::Unchanged)
            } else {
                debug!("'{}' changed from ver. {}", self.name().await, old_version);

                let old_archive = old_archive.display().to_string();
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

#[allow(dead_code)]
impl Mod {
    /// Populates the mod's info from a given mod zip archive.
    async fn populate_info_from_zip(&self, path: PathBuf) -> anyhow::Result<()> {
        *self.zip_path.write().await = Some(path.clone());
        self.info.write().await.populate_from_zip(path).await?;
        Ok(())
    }

    /// Returns whether the mod has its portal info populated or not.
    async fn is_portal_populated(&self) -> bool {
        self.info.read().await.is_portal_populated()
    }

    /// Returns a user-friendly display of the mod.
    pub async fn display(&self) -> String {
        self.info.read().await.display()
    }

    /// Returns the path of the mod's zip archive relative to the server's mods directory. Returns
    /// `ModError::MissingZipPath` if the path isn't set.
    pub async fn zip_path(&self) -> anyhow::Result<PathBuf> {
        Ok(self.zip_path.read().await.clone().ok_or(ModError::MissingZipPath)?)
    }

    /// Returns the mod zip archive's checksum if set. If not set, will calculate the checksum, set
    /// it and return it. Returns `ModError::MissingZipPath` if the path isn't set.
    pub async fn get_zip_checksum(&self) -> anyhow::Result<String> {
        let checksum = calculate_zip_checksum(STORE_ZIP_CHECKSUM_ALGO, self.zip_path().await?).await?;

        trace!(
            "Calculated zip checksum for mod '{}' ({}): {}",
            self.title().await,
            self.zip_path().await?.display(),
            checksum
        );
        Ok(checksum)
    }

    /// Returns the mod zip archive's last mtime. Returns `ModError::MissingZipLastMtime` if the last mtime isn't set
    /// (the mod isn't installed).
    pub async fn get_zip_last_mtime(&self) -> anyhow::Result<DateTime<Utc>> {
        if let Some(last_mtime) = *self.zip_last_mtime.read().await {
            Ok(last_mtime)
        } else {
            Err(ModError::MissingZipLastMtime.into())
        }
    }

    /// Returns the mod's name.
    pub async fn name(&self) -> String {
        let info = self.info.read().await;
        info.name().to_string()
    }

    /// Returns the mod's author.
    pub async fn author(&self) -> String {
        let info = self.info.read().await;
        info.author().to_string()
    }

    /// Returns the mod author's contact.
    pub async fn contact(&self) -> Option<String> {
        let info = self.info.read().await;
        info.contact().map(std::string::ToString::to_string)
    }

    /// Returns the mod's author's homepage.
    pub async fn homepage(&self) -> Option<String> {
        let info = self.info.read().await;
        info.homepage().map(std::string::ToString::to_string)
    }

    /// Returns the mod's title.
    pub async fn title(&self) -> String {
        let info = self.info.read().await;
        info.title().to_string()
    }

    /// Returns the mod's summary, if any.
    pub async fn summary(&self) -> Option<String> {
        let info = self.info.read().await;
        info.summary().map(std::string::ToString::to_string)
    }

    /// Returns the mod's description.
    pub async fn description(&self) -> String {
        let info = self.info.read().await;
        info.description().to_string()
    }

    /// Returns the mod's changelog, if any.
    pub async fn changelog(&self) -> Option<String> {
        let info = self.info.read().await;
        info.changelog().map(std::string::ToString::to_string)
    }

    /// Returns the mod's version.
    pub async fn own_version(&self) -> anyhow::Result<HumanVersion> {
        let info = self.info.read().await;
        info.own_version()
    }

    /// Returns the version of Factorio the mod is for.
    pub async fn factorio_version(&self) -> anyhow::Result<HumanVersion> {
        let info = self.info.read().await;
        info.factorio_version()
    }

    /// Returns the mod's releases.
    pub async fn releases(&self) -> anyhow::Result<Vec<Release>> {
        let info = self.info.read().await;
        info.releases()
    }

    /// Returns a release with a given version.
    pub async fn get_release(&self, version: HumanVersion) -> anyhow::Result<Release> {
        let info = self.info.read().await;
        info.get_release(Some(version))
    }

    /// Returns the mod's latest release.
    pub async fn latest_release(&self) -> anyhow::Result<Release> {
        let info = self.info.read().await;
        info.get_release(None)
    }

    /// Returns the mod's dependencies on other mods.
    pub async fn dependencies(&self) -> anyhow::Result<Vec<Dependency>> {
        let info = self.info.read().await;
        info.dependencies()
    }
}
