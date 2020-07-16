use super::Mods;
use crate::{
    cache::Cache, config::Config, error::ModError, ext::PathExt, factorio::GameCacheId,
    mod_common::Mod, mod_portal::ModPortal, util,
};
use log::*;
use std::{
    collections::{hash_map::Entry, HashMap, HashSet},
    path::PathBuf,
    sync::Arc,
};

const ZIP_GLOB: &str = "*.zip";

pub struct ModsBuilder {
    directory: PathBuf,
    game_cache_id: Option<GameCacheId>,
}

impl<'a> ModsBuilder {
    pub fn root(directory: PathBuf) -> Self {
        ModsBuilder {
            directory,
            game_cache_id: None,
        }
    }

    pub fn with_game_cache_id(self, game_cache_id: GameCacheId) -> Self {
        Self {
            game_cache_id: Some(game_cache_id),
            ..self
        }
    }

    async fn build_mods_from_cache(
        &self,
        game_cache_id: GameCacheId,
        config: Arc<Config>,
        portal: Arc<ModPortal>,
        cache: Arc<Cache>,
    ) -> anyhow::Result<Vec<Mod>> {
        trace!("Building mods for cached game ID {}", game_cache_id);
        let mods = cache.get_mods_of_game(game_cache_id).await?;
        let mut created_mods = Vec::new();
        let mut mod_zips = HashSet::new();

        for game_mod in mods {
            let created_mod = match Mod::from_cache(
                &game_mod,
                &self.directory,
                Arc::clone(&config),
                Arc::clone(&portal),
                Arc::clone(&cache),
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

                let created_mod = match Mod::from_zip(
                    &entry,
                    Arc::clone(&config),
                    Arc::clone(&portal),
                    Arc::clone(&cache),
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

    async fn build_mods_from_filesystem(
        &self,
        config: Arc<Config>,
        portal: Arc<ModPortal>,
        cache: Arc<Cache>,
    ) -> anyhow::Result<Vec<Mod>> {
        let zips = self.directory.join(ZIP_GLOB);
        trace!("Building mods from filesystem: {}", zips.display());
        let mut created_mods = Vec::new();

        for entry in util::glob(&zips)? {
            let created_mod = match Mod::from_zip(
                &entry,
                Arc::clone(&config),
                Arc::clone(&portal),
                Arc::clone(&cache),
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

    pub async fn build(
        self,
        config: Arc<Config>,
        portal: Arc<ModPortal>,
        cache: Arc<Cache>,
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
                Arc::clone(&cache),
            )
            .await?
        } else {
            debug!("No cached game, loading mods from filesystem");

            self.build_mods_from_filesystem(
                Arc::clone(&config),
                Arc::clone(&portal),
                Arc::clone(&cache),
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
            cache,
        })
    }
}
