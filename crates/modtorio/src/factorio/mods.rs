//! Provides the [`Mods`](Mods) object used to interact with the mods installed in a Factorio
//! server.

mod mod_list;
mod mods_builder;
mod update_batcher;

use super::GameStoreId;
use crate::{
    error::ModError,
    mod_common::{DownloadResult, Mod, Requirement},
    store::{models, Store},
    util::{async_status, ext::PathExt, HumanVersion},
    Config, ModPortal,
};
use async_status::{AsyncProgressChannel, AsyncProgressChannelExt};
use log::*;
use mod_list::ModList;
pub use mods_builder::ModsBuilder;
use std::{
    collections::{hash_map::Entry, HashMap},
    path::PathBuf,
    sync::Arc,
};
use tokio::{fs, sync::Mutex};
use update_batcher::UpdateBatcher;

// TODO: function to scan and remove duplicate mods

/// A collection of installed mods in a Factorio server.
pub struct Mods {
    /// The mod's root directory.
    directory: PathBuf,
    /// The collection of installed mods.
    mods: HashMap<String, Arc<Mod>>,
    /// Reference to the program config.
    config: Arc<Config>,
    /// Reference to the mod portal.
    portal: Arc<ModPortal>,
    /// Reference to the program store.
    store: Arc<Store>,
}

impl Mods {
    /// Returns how many mods there currently are. This counts all the mods, including ones that
    /// might not be installed yet.
    pub fn count(&self) -> usize {
        self.mods.len()
    }

    /// Updates the store for all current mods. This includes updating both the mod information and
    /// the game-to-mod mapping.
    #[allow(dead_code)]
    pub async fn update_store(
        &self,
        game_id: GameStoreId,
        skip_info_update: bool,
        prog_tx: Option<AsyncProgressChannel>,
    ) -> anyhow::Result<()> {
        debug!("Updating stored mods for game {}", game_id);
        let new_game_mods = Mutex::new(Vec::new());

        if !skip_info_update {
            prog_tx
                .send_status(async_status::indefinite("Updating mod infos..."))
                .await?;
            let mut update_batcher = UpdateBatcher::new(self.portal.as_ref());
            for fact_mod in self.mods.values() {
                update_batcher.add_mod(Arc::clone(fact_mod)).await;
            }

            debug!("Update batcher built, applying");
            update_batcher.apply().await?;
        }

        prog_tx
            .send_status(async_status::indefinite("Updating mod store..."))
            .await?;

        for fact_mod in self.mods.values() {
            let mod_name = fact_mod.name().await;
            let mod_display = fact_mod.display().await;

            match fact_mod.update_store(skip_info_update).await {
                Ok(()) => debug!("Updated mod store for {}", mod_name),
                Err(e) => {
                    error!("Store update for {} failed: {}", mod_display, e);
                    continue;
                }
            }

            let factorio_mod = mod_name.clone();
            let mod_version = fact_mod.own_version().await?;
            let mod_zip = fact_mod.zip_path().await?.get_string()?;
            let zip_last_mtime = fact_mod.get_zip_last_mtime().await?;

            let store_game_mod = models::GameMod {
                game: game_id,
                factorio_mod,
                mod_version,
                mod_zip,
                zip_last_mtime,
            };
            // trace!(
            //     "{}'s stored mod {}: {:?}",
            //     game_id,
            //     mod_name,
            //     store_game_mod
            // );

            new_game_mods.lock().await.push(store_game_mod);
        }

        self.store.remove_mods_of_game(game_id).await?;
        self.store.set_mods_of_game(new_game_mods.into_inner()).await?;
        info!("Updated game ID {}'s stored mods", game_id);

        Ok(())
    }

    /// Adds and installs a new mod with a given name from the portal. Optionally a wanted version
    /// can be supplied. If no wanted version is supplied, the latest version is installed.
    pub async fn add_from_portal(
        &mut self,
        name: &str,
        version: Option<HumanVersion>,
        prog_tx: Option<AsyncProgressChannel>,
    ) -> anyhow::Result<()> {
        if let Some(version) = version {
            info!("Adding '{}' ver. {:?}", name, version);
        } else {
            info!("Adding latest '{}'", name);
        }

        prog_tx
            .send_status(async_status::indefinite(&format!("Installing {}...", name)))
            .await?;

        let new_mod = self.add_or_update_in_place(name, version).await?;
        info!("Added {}", new_mod.display().await);
        Ok(())
    }

    /// Removes a mod by its name.
    pub async fn remove(&mut self, name: &str) -> anyhow::Result<()> {
        match self.mods.entry(name.to_string()) {
            Entry::Occupied(entry) => {
                let fact_mod = entry.get();
                debug!("Removing '{}' zip archive", fact_mod.name().await);
                fs::remove_file(self.directory.join(fact_mod.zip_path().await?)).await?;
                entry.remove();
                Ok(())
            }
            Entry::Vacant(_) => Err(ModError::NoSuchMod(name.to_string()).into()),
        }
    }

    /// Returns whether a mod is enabled or not.
    pub fn get_mod_enabled(&self, name: &str) -> anyhow::Result<bool> {
        if self.mods.contains_key(name) {
            let mod_list = ModList::from_mods_directory(&self.directory)?;
            Ok(mod_list.get_mod_enabled(name))
        } else {
            Err(ModError::NoSuchMod(name.to_string()).into())
        }
    }

    /// Enables a mod by its name.
    pub fn set_mod_enabled(&self, name: &str, enabled: bool) -> anyhow::Result<()> {
        if self.mods.contains_key(name) {
            debug!("Setting mod '{}' enabled: {}", name, enabled);

            let mut mod_list = ModList::from_mods_directory(&self.directory)?;
            mod_list.set_mod_enabled(name, enabled);
            mod_list.save()?;

            Ok(())
        } else {
            Err(ModError::NoSuchMod(name.to_string()).into())
        }
    }

    /// Returns a `HashMap<String, bool>` for all the mods and their enabled status. This collection is loaded directly
    /// from the `mod-list.json` file, so it may not contain all managed mods, or it may contain non-managed mods.
    pub fn get_mods_enabled_status(&self) -> anyhow::Result<HashMap<String, bool>> {
        let mod_list = ModList::from_mods_directory(&self.directory)?;
        Ok(mod_list.get_mods_enabled_status())
    }

    /// Updates the portal info for all mods and downloads their most recent version if the
    /// currently installed version is older.
    #[allow(dead_code)]
    pub async fn update(&mut self, prog_tx: Option<AsyncProgressChannel>) -> anyhow::Result<()> {
        info!("Checking for mod updates...");
        prog_tx
            .send_status(async_status::indefinite("Checking for mod updates..."))
            .await?;

        let mut update_batcher = UpdateBatcher::new(self.portal.as_ref());

        for m in self.mods.values() {
            update_batcher.add_mod(Arc::clone(m)).await;
        }

        debug!("Update batcher built, applying...");
        update_batcher.apply().await?;

        let updates = update_batcher.get_updates().await?;
        info!("Found {} updates", updates.len());
        prog_tx
            .send_status(async_status::indefinite(&format!("Found {} updates", updates.len())))
            .await?;

        if !updates.is_empty() {
            debug!("{:?}", updates)
        };

        let max_updates = updates.len() as u32;
        for (index, update) in updates.iter().enumerate() {
            info!("Updating {}...", update);
            prog_tx
                .send_status(async_status::definite(
                    &format!("Updating {}...", update),
                    index as u32,
                    max_updates,
                ))
                .await?;

            self.add_or_update_in_place(update, None).await?;
        }

        Ok(())
    }

    /// Tries to ensure all mod dependencies are met by installing any missing mods or mods that
    /// don't meet a dependency's version requirement. If a mod is incompatible with another
    /// installed mod, the ensuring will fail with
    /// [`ModError::CannotEnsureDependency`][CannotEnsureDependency].
    ///
    /// [CannotEnsureDependency]: crate::error::ModError::CannotEnsureDependency
    #[allow(dead_code)]
    pub async fn ensure_dependencies(&mut self, prog_tx: Option<AsyncProgressChannel>) -> anyhow::Result<()> {
        info!("Ensuring mod dependencies are met...");

        let mut missing: Vec<String> = Vec::new();

        let mods = self.mods.values();
        let max_mods = mods.len() as u32;
        for (index, fact_mod) in mods.into_iter().enumerate() {
            debug!("Ensuring '{}'s dependencies are met...", fact_mod.name().await);
            prog_tx
                .send_status(async_status::definite(
                    &format!("Ensuring '{}'s dependencies are met...", fact_mod.title().await),
                    index as u32,
                    max_mods,
                ))
                .await?;

            missing.extend(self.ensure_single_dependencies(fact_mod).await?.into_iter());
        }

        if missing.is_empty() {
            info!("All mod dependencies met");
        } else {
            info!("Found {} missing mod dependencies, installing", missing.len());

            let max_missing = missing.len() as u32;
            for (index, miss) in missing.iter().enumerate() {
                prog_tx
                    .send_status(async_status::definite(
                        &format!("Installing missing mod '{}'...", miss),
                        index as u32,
                        max_missing,
                    ))
                    .await?;

                self.add_from_portal(&miss, None, None).await?;
            }
        }

        Ok(())
    }
}

impl Mods {
    /// Retrieves a currently installed mod based on its name. Returns
    /// [`Err(ModError::NoSuchMod)`][NoSuchMod] if there is no mod with such name.
    ///
    /// [NoSuchMod]: crate::error::ModError::NoSuchMod
    fn get_mod(&self, name: &str) -> anyhow::Result<&Mod> {
        Ok(self
            .mods
            .get(name)
            .ok_or_else(|| ModError::NoSuchMod(name.to_owned()))?)
    }

    /// Given a mod name and an optional version, this function will redownload the mod if it's
    /// already installed or download it new if it doesn't.
    ///
    /// If a version is given, that version will be downloaded if it exists. If no version is given,
    /// the latest release of the mod will be downloaded.
    ///
    /// If an already installed mod is redownloaded and its version is higher than earlier, the old
    /// mod archive will be removed.
    async fn add_or_update_in_place(&mut self, name: &str, version: Option<HumanVersion>) -> anyhow::Result<&Mod> {
        match self.mods.entry(name.to_owned()) {
            Entry::Occupied(entry) => {
                let existing_mod = entry.into_mut();
                let existing_mod_display = existing_mod.display().await;

                info!("Downloading {}...", existing_mod_display);

                match existing_mod.download(version, &self.directory).await? {
                    DownloadResult::New => info!("{} added", existing_mod_display),
                    DownloadResult::Unchanged => info!("{} unchanged", existing_mod_display),
                    DownloadResult::Replaced {
                        old_version,
                        old_archive,
                    } => {
                        let old_archive = self.directory.join(old_archive);
                        debug!("Removing old mod archive {}", old_archive.display());
                        fs::remove_file(old_archive).await?;

                        // do .display() again since it has changed
                        info!("{} replaced from ver. {}", existing_mod.display().await, old_version);
                    }
                }

                Ok(existing_mod)
            }
            Entry::Vacant(entry) => {
                let new_mod = Arc::new(
                    Mod::from_portal(
                        name,
                        Arc::clone(&self.config),
                        Arc::clone(&self.portal),
                        Arc::clone(&self.store),
                    )
                    .await?,
                );

                info!("Downloading {}...", name);

                new_mod.download(version, &self.directory).await?;
                Ok(entry.insert(new_mod))
            }
        }
    }

    /// Given a reference to an installed mod, tries to ensure its dependencies are met. Returns a
    /// vector of mod names that are missing or don't meet a dependency's version requirement
    /// and should be installed to meet the mod's dependencies. If the mod is incompatible with
    /// another installed mod, the function will return an error
    /// `ModError::CannotEnsureDependency`.
    async fn ensure_single_dependencies(&self, target_mod: &Mod) -> anyhow::Result<Vec<String>> {
        let mut missing = Vec::new();
        let target_name = target_mod.name().await;

        for dep in target_mod.dependencies().await? {
            trace!("Ensuring dependency {:?}", dep);
            if dep.name() == "base" {
                continue;
            }

            match dep.requirement() {
                Requirement::Mandatory => {
                    if let Ok(required_mod) = self.get_mod(dep.name()) {
                        let required_version = required_mod.own_version().await?;

                        match dep.version() {
                            Some(version_req) if !required_version.meets(version_req) => {
                                debug!(
                                    "Dependency {} of '{}' not met: version requirement mismatch (found {})",
                                    dep, target_name, required_version
                                );
                                missing.push(dep.name().to_string());
                            }
                            _ => debug!(
                                "Dependency {} of '{}' met (found {})",
                                dep, target_name, required_version
                            ),
                        }
                    } else {
                        debug!(
                            "Dependency {} of '{}' not met: required mod not found",
                            dep, target_name
                        );

                        // TODO: resolve version
                        missing.push(dep.name().to_string());
                    }
                }
                Requirement::Incompatible => {
                    if self.get_mod(dep.name()).is_ok() {
                        return Err(ModError::CannotEnsureDependency {
                            dependency: dep,
                            mod_display: target_mod.display().await,
                        }
                        .into());
                    } else {
                        debug!("Dependency {} of '{}' met", dep, target_name);
                    }
                }
                _ => (),
            }
        }

        Ok(missing)
    }
}
