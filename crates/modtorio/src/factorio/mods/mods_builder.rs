//! Provides the [ModsBuilder](ModsBuilder) which is used to build a [Mods](super::Mods) object from
//! a game's mod root directory, optionally loading them from the program store.

use super::Mods;
use crate::{
    config::Config,
    error::ModError,
    factorio::GameStoreId,
    mod_common::Mod,
    mod_portal::ModPortal,
    store::Store,
    util,
    util::{async_status, ext::PathExt},
};
use async_status::{AsyncProgressChannel, AsyncProgressChannelExt};
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
/// If the given game is stored, the mods will be built from the store and verified against their
/// corresponding mod zip archives. If a stored mod isn't found in the mod directory, it will be
/// ignored. Any mods in the mod directory that weren't stored (i.e. mods added externally) will be
/// loaded afterwards.
pub struct ModsBuilder {
    /// The mods' root directory.
    directory: PathBuf,
    /// The store ID of the game these mods belong to.
    game_store_id: Option<GameStoreId>,
    /// A status update channel.
    prog_tx: Option<AsyncProgressChannel>,
}

impl<'a> ModsBuilder {
    /// Returns a new `ModsBuilder` with a given mod root directory. Doesn't have a game's store ID
    /// set.
    pub fn root(directory: PathBuf) -> Self {
        ModsBuilder {
            directory,
            game_store_id: None,
            prog_tx: None,
        }
    }

    /// Sets a the store ID of the game to load mods from the program store for.
    pub fn with_game_store_id(self, game_store_id: GameStoreId) -> Self {
        Self {
            game_store_id: Some(game_store_id),
            ..self
        }
    }

    /// Specifies an `AsyncProgressChannel` to use for status updates when building the mods.
    pub fn with_status_updates(self, prog_tx: AsyncProgressChannel) -> Self {
        Self {
            prog_tx: Some(prog_tx),
            ..self
        }
    }

    /// Builds mods from the program store with a given game store ID.
    async fn build_mods_from_store(
        &self,
        game_store_id: GameStoreId,
        config: Arc<Config>,
        portal: Arc<ModPortal>,
        store: Arc<Store>,
    ) -> anyhow::Result<Vec<Mod>> {
        trace!("Building mods for stored game ID {}", game_store_id);
        let mods = store.get_mods_of_game(game_store_id).await?;
        let max_mods = mods.len() as u32;
        let mut created_mods = Vec::new();
        let mut mod_zips = HashSet::new();

        for (index, game_mod) in mods.into_iter().enumerate() {
            self.prog_tx
                .send_status(async_status::definite(
                    &format!("Loading mod from store: {}", game_mod.factorio_mod),
                    index as u32,
                    max_mods,
                ))
                .await?;

            let created_mod = match Mod::from_store(
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
                        "Stored mod '{}' for game ID {} failed to load: {}",
                        game_mod.factorio_mod, game_store_id, e
                    );
                    continue;
                }
            };

            let mod_zip = created_mod.zip_path().await?.get_string()?;
            trace!("{} zip: {}", created_mod.name().await, mod_zip);
            if !mod_zips.insert(mod_zip.clone()) {
                return Err(ModError::DuplicateMod(mod_zip).into());
            }

            debug!("Loaded {} from store", created_mod.name().await);
            created_mods.push(created_mod);
        }

        self.prog_tx
            .send_status(async_status::indefinite("Checking for non-stored mod zip archives..."))
            .await?;
        debug!(
            "{} mods loaded from store, checking for non-stored zips...",
            created_mods.len()
        );
        trace!("Mod zips: {:?}", mod_zips);

        let zips = util::glob(&self.directory.join(ZIP_GLOB))?;
        let max_zips = zips.len() as u32;
        for (index, entry) in zips.into_iter().enumerate() {
            let entry_file_name = entry.get_file_name()?;
            trace!("Checking if {} is loaded...", entry_file_name);
            self.prog_tx
                .send_status(async_status::definite(
                    &format!("Checking if {} is loaded...", entry.display()),
                    index as u32,
                    max_zips,
                ))
                .await?;

            if !mod_zips.contains(&entry_file_name) {
                warn!(
                    "Found non-stored mod from filesystem: {}, loading from zip...",
                    entry.display()
                );
                self.prog_tx
                    .send_status(async_status::definite(
                        &format!("Loading {}...", entry.display()),
                        index as u32,
                        max_zips,
                    ))
                    .await?;

                let created_mod =
                    match Mod::from_zip(&entry, Arc::clone(&config), Arc::clone(&portal), Arc::clone(&store)).await {
                        Ok(created_mod) => created_mod,
                        Err(e) => {
                            error!("Zip mod '{}' failed to load: {}", entry.display(), e);
                            continue;
                        }
                    };

                debug!(
                    "Loaded non-stored mod {} from zip ({})",
                    created_mod.name().await,
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

        let entries = util::glob(&zips)?;
        let max_entries = entries.len() as u32;
        for (index, entry) in entries.iter().enumerate() {
            self.prog_tx
                .send_status(async_status::definite(
                    &format!("Loading mod from zip archive: {}", entry.display()),
                    index as u32,
                    max_entries,
                ))
                .await?;

            let created_mod =
                match Mod::from_zip(&entry, Arc::clone(&config), Arc::clone(&portal), Arc::clone(&store)).await {
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
    pub async fn build(self, config: Arc<Config>, portal: Arc<ModPortal>, store: Arc<Store>) -> anyhow::Result<Mods> {
        let built_mods = if let Some(game_store_id) = self.game_store_id {
            debug!("Got stored game ID {}, loading mods from store", game_store_id);

            self.build_mods_from_store(
                game_store_id,
                Arc::clone(&config),
                Arc::clone(&portal),
                Arc::clone(&store),
            )
            .await?
        } else {
            debug!("No stored game, loading mods from filesystem");

            self.build_mods_from_filesystem(Arc::clone(&config), Arc::clone(&portal), Arc::clone(&store))
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
