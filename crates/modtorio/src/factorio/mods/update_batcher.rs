#![allow(clippy::missing_docs_in_private_items)]

use crate::{
    error::UpdateBatcherError,
    mod_common::Mod,
    mod_portal::{ModPortal, PortalResult},
};
use log::*;
use std::{collections::HashMap, sync::Arc};

pub struct UpdateBatcher<'a> {
    portal: &'a ModPortal,
    mods: HashMap<String, Arc<Mod>>,
}

impl<'a> UpdateBatcher<'a> {
    pub fn new(portal: &'a ModPortal) -> Self {
        Self {
            portal,
            mods: HashMap::new(),
        }
    }

    pub async fn add_mod(&mut self, fact_mod: Arc<Mod>) {
        self.mods.insert(fact_mod.name().await, fact_mod);
    }

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
