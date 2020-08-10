//! Provides the [ModsBuilder](ModsBuilder) which is used to build a [Mods](super::Mods) object from
//! a game's mod root directory, optionally loading them from the program cache.

use super::Mods;
use crate::{
    config::Config,
    error::ModError,
    factorio::GameCacheId,
    mod_common::Mod,
    mod_portal::ModPortal,
    store::Store,
    util,
    util::{ext::PathExt, status},
};
use log::*;
use std::{
    collections::{hash_map::Entry, HashMap, HashSet},
    path::PathBuf,
    sync::Arc,
};

/// A glob string used to find zip archives (.zip extension).
const ZIP_GLOB: &str = "*.zip";

/// A builder used to build a game's mod collection from the game's mod directory.
///
/// If the given game is cached, the mods will be built from the cache and verified against their
/// corresponding mod zip archives. If a cached mod isn't found in the mod directory, it will be
/// ignored. Any mods in the mod directory that weren't cached (i.e. mods added externally) will be
/// loaded afterwards.
// TODO: doctests
pub struct ModsBuilder {
    /// The mods' root directory.
    directory: PathBuf,
    /// The cache ID of the game these mods belong to.
    game_cache_id: Option<GameCacheId>,
    /// A status update channel.
    prog_tx: Option<status::AsyncProgressChannel>,
}

impl<'a> ModsBuilder {
    /// Returns a new `ModsBuilder` with a given mod root directory. Doesn't have a game's cache ID
    /// set.
    pub fn root(directory: PathBuf) -> Self {
        ModsBuilder {
            directory,
            game_cache_id: None,
            prog_tx: None,
        }
    }

    /// Sets a the cache ID of the game to load mods from the program cache for.
    pub fn with_game_cache_id(self, game_cache_id: GameCacheId) -> Self {
        Self {
            game_cache_id: Some(game_cache_id),
            ..self
        }
    }

    pub fn with_status_updates(self, prog_tx: status::AsyncProgressChannel) -> Self {
        Self {
            prog_tx: Some(prog_tx),
            ..self
        }
    }

    /// Builds mods from the program cache with a given game cache ID.
    async fn build_mods_from_cache(
        &self,
        game_cache_id: GameCacheId,
        config: Arc<Config>,
        portal: Arc<ModPortal>,
        store: Arc<Store>,
    ) -> anyhow::Result<Vec<Mod>> {
        trace!("Building mods for cached game ID {}", game_cache_id);
        let mods = store.cache.get_mods_of_game(game_cache_id).await?;
        let mut created_mods = Vec::new();
        let mut mod_zips = HashSet::new();

        for game_mod in mods {
            status::send_status(
                self.prog_tx.clone(),
                status::indefinite(&format!(
                    "Loading mod from cache: {}",
                    game_mod.factorio_mod
                )),
            )
            .await?;

            let created_mod = match Mod::from_cache(
                &game_mod,
                &self.directory,
                Arc::clone(&config),
                Arc::clone(&portal),
                Arc::clone(&store),
            )
            .await
            {
                Ok(created_mod) => created_mod,
                Err(e) => {
                    error!(
                        "Cached mod '{}' for game ID {} failed to load: {}",
                        game_mod.factorio_mod, game_cache_id, e
                    );
                    continue;
                }
            };

            info!("Loaded mod {} from cache", created_mod.display().await);

            let mod_zip = created_mod.zip_path().await?.get_str()?.to_string();
            if !mod_zips.insert(mod_zip.clone()) {
                return Err(ModError::DuplicateMod(mod_zip).into());
            }

            created_mods.push(created_mod);
        }

        status::send_status(
            self.prog_tx.clone(),
            status::indefinite("Checking for non-cached mod zip archives..."),
        )
        .await?;
        debug!(
            "{} mods loaded from cache, checking for non-cached zips...",
            created_mods.len()
        );
        trace!("Mod zips: {:?}", mod_zips);

        let zips = self.directory.join(ZIP_GLOB);
        for entry in util::glob(&zips)? {
            let entry_file_name = entry.get_file_name()?;
            trace!("Checking if {} is loaded...", entry_file_name);

            if !mod_zips.contains(&entry_file_name) {
                warn!(
                    "Found non-cached mod from filesystem: {}, loading from zip...",
                    entry.display()
                );
                status::send_status(
                    self.prog_tx.clone(),
                    status::indefinite(&format!(
                        "Found non-cached mod zip archive: {}, loading...",
                        entry.display()
                    )),
                )
                .await?;

                let created_mod = match Mod::from_zip(
                    &entry,
                    Arc::clone(&config),
                    Arc::clone(&portal),
                    Arc::clone(&store),
                )
                .await
                {
                    Ok(created_mod) => created_mod,
                    Err(e) => {
                        error!("Zip mod '{}' failed to load: {}", entry.display(), e);
                        continue;
                    }
                };

                info!(
                    "Loaded non-cached mod {} from zip ({})",
                    created_mod.display().await,
                    entry.display()
                );
                created_mods.push(created_mod);
            }
        }

        Ok(created_mods)
    }

    /// Builds mods from the filesystem based on the builder's mod root directory.
    async fn build_mods_from_filesystem(
        &self,
        config: Arc<Config>,
        portal: Arc<ModPortal>,
        store: Arc<Store>,
    ) -> anyhow::Result<Vec<Mod>> {
        let zips = self.directory.join(ZIP_GLOB);
        trace!("Building mods from filesystem: {}", zips.display());
        let mut created_mods = Vec::new();

        for entry in util::glob(&zips)? {
            status::send_status(
                self.prog_tx.clone(),
                status::indefinite(&format!(
                    "Loading mod from zip archive: {}",
                    entry.display()
                )),
            )
            .await?;

            let created_mod = match Mod::from_zip(
                &entry,
                Arc::clone(&config),
                Arc::clone(&portal),
                Arc::clone(&store),
            )
            .await
            {
                Ok(created_mod) => created_mod,
                Err(e) => {
                    error!("Zip mod '{}' failed to load: {}", entry.display(), e);
                    continue;
                }
            };

            info!(
                "Loaded mod {} from zip ({})",
                created_mod.display().await,
                entry.display()
            );
            created_mods.push(created_mod);
        }

        Ok(created_mods)
    }

    /// Finalises the builder and returns a new `Mods` object.
    pub async fn build(
        self,
        config: Arc<Config>,
        portal: Arc<ModPortal>,
        store: Arc<Store>,
    ) -> anyhow::Result<Mods> {
        let built_mods = if let Some(game_cache_id) = self.game_cache_id {
            debug!(
                "Got cached game ID {}, loading mods from cache",
                game_cache_id
            );

            self.build_mods_from_cache(
                game_cache_id,
                Arc::clone(&config),
                Arc::clone(&portal),
                Arc::clone(&store),
            )
            .await?
        } else {
            debug!("No cached game, loading mods from filesystem");

            self.build_mods_from_filesystem(
                Arc::clone(&config),
                Arc::clone(&portal),
                Arc::clone(&store),
            )
            .await?
        };

        let mut mods = HashMap::new();

        for built_mod in built_mods {
            let mod_name = built_mod.name().await;

            match mods.entry(mod_name) {
                Entry::Occupied(mut entry) => {
                    let existing: &Arc<Mod> = entry.get();

                    warn!(
                        "Found duplicate '{}' (new {} vs existing {})",
                        entry.key(),
                        built_mod.own_version().await?,
                        existing.own_version().await?
                    );

                    let own_version = built_mod.own_version().await?;
                    let existing_version = existing.own_version().await?;
                    if own_version > existing_version {
                        entry.insert(Arc::new(built_mod));
                    }
                }
                Entry::Vacant(entry) => {
                    entry.insert(Arc::new(built_mod));
                }
            }
        }

        Ok(Mods {
            directory: self.directory,
            mods,
            config,
            portal,
            store,
        })
    }
}
