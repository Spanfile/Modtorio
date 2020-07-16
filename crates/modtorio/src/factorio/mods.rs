mod mods_builder;

use super::GameCacheId;
use crate::{
    cache::models,
    error::ModError,
    ext::PathExt,
    mod_common::{DownloadResult, Mod, Requirement},
    util::HumanVersion,
    Cache, Config, ModPortal,
};
pub use mods_builder::ModsBuilder;

use log::*;
use std::{
    collections::{hash_map::Entry, HashMap},
    path::PathBuf,
    sync::Arc,
};
use tokio::{fs, sync::Mutex};

pub struct Mods {
    directory: PathBuf,
    mods: HashMap<String, Arc<Mod>>,
    config: Arc<Config>,
    portal: Arc<ModPortal>,
    cache: Arc<Cache>,
}

impl Mods {
    pub fn count(&self) -> usize {
        self.mods.len()
    }

    #[allow(dead_code)]
    pub async fn update_cache(&self, game_id: GameCacheId) -> anyhow::Result<()> {
        debug!("Updating cached mods for game {}", game_id);
        let new_game_mods = Mutex::new(Vec::new());

        for fact_mod in self.mods.values() {
            let mod_name = fact_mod.name().await;
            let mod_display = fact_mod.display().await;

            match fact_mod.update_cache().await {
                Ok(()) => debug!("Updated Factorio mod cache for {}", mod_name),
                Err(e) => {
                    error!("Cache update for {} failed: {}", mod_display, e);
                    continue;
                }
            }

            debug!("Updating game '{}' cached mod '{}'...", game_id, mod_name);

            let factorio_mod = mod_name.clone();
            let mod_version = fact_mod.own_version().await?;
            let mod_zip = fact_mod.zip_path().await?.get_file_name()?;
            let zip_checksum = fact_mod.get_zip_checksum().await?;

            let cache_game_mod = models::GameMod {
                game: game_id,
                factorio_mod,
                mod_version,
                mod_zip,
                zip_checksum,
            };
            // trace!(
            //     "{}'s cached mod {}: {:?}",
            //     game_id,
            //     mod_name,
            //     cache_game_mod
            // );

            new_game_mods.lock().await.push(cache_game_mod);
            info!("Updated cache for {}", mod_display);
        }

        self.cache
            .set_mods_of_game(new_game_mods.into_inner())
            .await?;
        info!("Updated game ID {}'s cached mods", game_id);

        Ok(())
    }

    pub async fn add_from_portal(
        &mut self,
        name: &str,
        version: Option<HumanVersion>,
    ) -> anyhow::Result<()> {
        if let Some(version) = version {
            info!("Adding '{}' ver. {:?}", name, version);
        } else {
            info!("Adding latest '{}'", name);
        }

        let new_mod = self.add_or_update_in_place(name, version).await?;
        info!("Added {}", new_mod.display().await);
        Ok(())
    }

    #[allow(dead_code)]
    pub async fn update(&mut self) -> anyhow::Result<()> {
        info!("Checking for mod updates...");

        let mut updates = Vec::new();
        for m in self.mods.values_mut() {
            info!("Checking for updates to {}...", m.display().await);

            m.ensure_portal_info().await?;
            let release = m.latest_release().await?;

            if m.own_version().await? < release.version() {
                debug!(
                    "Found newer version of '{}': {} (over {}) released on {}",
                    m.title().await,
                    release.version(),
                    m.own_version().await?,
                    release.released_on()
                );

                updates.push(m.name().await.to_owned());
            } else {
                debug!("{} is up to date", m.display().await);
            }
        }

        info!("Found {} updates", updates.len());
        if !updates.is_empty() {
            debug!("{:?}", updates)
        };

        for update in &updates {
            self.add_or_update_in_place(update, None).await?;
        }

        Ok(())
    }

    #[allow(dead_code)]
    pub async fn ensure_dependencies(&mut self) -> anyhow::Result<()> {
        info!("Ensuring mod dependencies are met...");

        let mut missing: Vec<String> = Vec::new();

        for fact_mod in self.mods.values() {
            info!(
                "Ensuring '{}'s dependencies are met...",
                fact_mod.title().await
            );
            missing.extend(
                self.ensure_single_dependencies(fact_mod)
                    .await?
                    .into_iter()
                    .map(|m| m),
            );
        }

        if !missing.is_empty() {
            info!(
                "Found {} missing mod dependencies, installing",
                missing.len()
            );

            for miss in &missing {
                self.add_from_portal(&miss, None).await?;
            }
        } else {
            info!("All mod dependencies met");
        }

        Ok(())
    }
}

impl Mods {
    fn get_mod(&self, name: &str) -> anyhow::Result<&Mod> {
        Ok(self
            .mods
            .get(name)
            .ok_or_else(|| ModError::NoSuchMod(name.to_owned()))?)
    }

    async fn add_or_update_in_place(
        &mut self,
        name: &str,
        version: Option<HumanVersion>,
    ) -> anyhow::Result<&Mod> {
        match self.mods.entry(name.to_owned()) {
            Entry::Occupied(entry) => {
                let existing_mod = entry.into_mut();
                info!("Downloading {}...", existing_mod.display().await);

                match existing_mod.download(version, &self.directory).await? {
                    DownloadResult::New => info!("{} added", existing_mod.display().await),
                    DownloadResult::Unchanged => {
                        info!("{} unchanged", existing_mod.display().await)
                    }
                    DownloadResult::Replaced {
                        old_version,
                        old_archive,
                    } => {
                        debug!("Removing old mod archive {}", old_archive);
                        fs::remove_file(old_archive).await?;

                        info!(
                            "{} replaced from ver. {}",
                            existing_mod.display().await,
                            old_version
                        );
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
                        Arc::clone(&self.cache),
                    )
                    .await?,
                );

                new_mod.download(version, &self.directory).await?;
                Ok(entry.insert(new_mod))
            }
        }
    }

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
                    match self.get_mod(dep.name()) {
                        Ok(required_mod) => {
                            let required_version = required_mod.own_version().await?;

                            match dep.version() {
                                Some(version_req) if !required_version.meets(version_req) => {
                                    debug!(
                                        "Dependency {} of '{}' not met: version requirement \
                                         mismatch (found {})",
                                        dep, target_name, required_version
                                    );
                                    missing.push(dep.name().to_string());
                                }
                                _ => debug!(
                                    "Dependency {} of '{}' met (found {})",
                                    dep, target_name, required_version
                                ),
                            }
                        }
                        Err(_) => {
                            debug!(
                                "Dependency {} of '{}' not met: required mod not found",
                                dep, target_name
                            );

                            // TODO: resolve version
                            missing.push(dep.name().to_string());
                        }
                    }
                }
                Requirement::Incompatible => match self.get_mod(dep.name()) {
                    Ok(_) => {
                        return Err(ModError::CannotEnsureDependency {
                            dependency: dep,
                            mod_display: target_mod.display().await,
                        }
                        .into());
                    }
                    Err(_) => debug!("Dependency {} of '{}' met", dep, target_name),
                },
                _ => (),
            }
        }

        Ok(missing)
    }
}
