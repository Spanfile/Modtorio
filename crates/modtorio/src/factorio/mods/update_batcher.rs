//! Provides the `UpdateBatcher` which is used to update the portal info for multiple mods with a single request.

use crate::{
    error::UpdateBatcherError,
    mod_common::Mod,
    mod_portal::{ModPortal, PortalResult},
};
use log::*;
use std::{collections::HashMap, sync::Arc};

/// Used to update the portal info for multiple mods with a single request.
pub struct UpdateBatcher<'a> {
    /// The mod portal instance to use.
    portal: &'a ModPortal,
    /// The mods this batcher keeps track of.
    mods: HashMap<String, Arc<Mod>>,
}

impl<'a> UpdateBatcher<'a> {
    /// Returns a new `UpdateBatcher` which uses a given `ModPortal` instance.
    pub fn new(portal: &'a ModPortal) -> Self {
        Self {
            portal,
            mods: HashMap::new(),
        }
    }

    /// Adds a mod to this batcher's tracked mods.
    pub async fn add_mod(&mut self, fact_mod: Arc<Mod>) {
        self.mods.insert(fact_mod.name().await, fact_mod);
    }

    /// Fetches and applies the portal info for all tracked mods.
    pub async fn apply(&mut self) -> anyhow::Result<()> {
        let mut names = Vec::new();

        for name in self.mods.keys() {
            names.push(name.as_str());
        }

        trace!("Batching info update for mods: {:?}", names);
        let mod_infos: Vec<PortalResult> = self.portal.fetch_multiple_mods(&names).await?;

        for info in mod_infos {
            let name = info.name()?;
            if let Some(fact_mod) = self.mods.get(name) {
                fact_mod.apply_portal_info(info).await?;
            } else {
                return Err(UpdateBatcherError::UnknownModName(name.to_owned()).into());
            }
        }

        Ok(())
    }

    /// Consumes the batcher and returns all mods that can be updated.
    pub async fn get_updates(self) -> anyhow::Result<Vec<String>> {
        let mut updated_mods = Vec::new();

        for (name, fact_mod) in self.mods {
            let latest = fact_mod.latest_release().await?;
            let version = fact_mod.own_version().await?;

            if version < latest.version() {
                info!(
                    "Found newer version of {}: {} over {}, released on {}",
                    fact_mod.name().await,
                    latest.version(),
                    version,
                    latest.released_on()
                );
                updated_mods.push(name);
            }
        }

        Ok(updated_mods)
    }
}
