//! Provides the `ModList` object which is used to manage a server's `mod-list.json` file in its mods directory.

use crate::error::ModError;
use log::*;
use serde::{Deserialize, Serialize};
use std::{
    collections::HashMap,
    fs::File,
    io::BufReader,
    path::{Path, PathBuf},
};

/// The filename of the server's mod list.
const MOD_LIST_JSON_FILE: &str = "mod-list.json";
/// The name of the base mod.
const BASE_MOD: &str = "base";

/// Represents the server's `mod-list.json` file, used to enable/disable mods.
#[derive(Debug, Deserialize, Serialize)]
pub struct ModList {
    /// The path to the file this list is stored in.
    #[serde(skip)]
    path: PathBuf,
    /// The mods in this list.
    mods: Vec<ModListMod>,
}

/// Represents a single mod entry in the server's `mod-list.json` file.
#[derive(Debug, Deserialize, Serialize)]
struct ModListMod {
    /// The mod's name.
    name: String,
    /// Is the mod enabled or not.
    enabled: bool,
}

impl ModList {
    /// Loads a new `ModList` from the `mod-list.json` file in a given `mods/` directory.
    pub fn from_mods_directory<P>(directory: P) -> anyhow::Result<Self>
    where
        P: AsRef<Path>,
    {
        let mod_list_file = directory.as_ref().join(MOD_LIST_JSON_FILE);
        debug!("Loading mod list from {}", mod_list_file.display());

        let file = File::open(&mod_list_file)?;
        let reader = BufReader::new(file);

        let mut mod_list: ModList = serde_json::from_reader(reader)?;

        mod_list.path = mod_list_file;
        trace!("{:?}", mod_list);
        Ok(mod_list)
    }

    /// Saves this `ModList` to the same file it was originally loaded from.
    pub fn save(&self) -> anyhow::Result<()> {
        debug!("Saving mod list to {}", self.path.display());
        trace!("{:?}", self);

        let file = File::create(&self.path)?;
        serde_json::to_writer(file, self)?;
        Ok(())
    }

    /// Returns whether a given mod is enabled or not. If the mod doesn't exist, returns `false`.
    pub fn get_mod_enabled(&self, name: &str) -> bool {
        for list_mod in &self.mods {
            if list_mod.name == name {
                return list_mod.enabled;
            }
        }

        false
    }

    /// Sets a mod's enabled status.
    pub fn set_mod_enabled(&mut self, name: &str, enabled: bool) -> Result<(), ModError> {
        if name == BASE_MOD {
            if enabled {
                // the base mod is always enabled
                return Ok(());
            } else {
                return Err(ModError::CannotDisableBaseMod);
            }
        }

        for list_mod in self.mods.iter_mut() {
            if list_mod.name == name {
                list_mod.enabled = enabled;
                return Ok(());
            }
        }

        // the mod wasn't found, so just add a new one. where this is called in mods.rs ensures the mod being
        // enabled/disabled is already a managed one, and there's no harm in adding invalid entries
        self.mods.push(ModListMod {
            name: name.to_string(),
            enabled,
        });

        Ok(())
    }

    /// Returns a `HashMap<String, bool>` for all the mods and their enabled status.
    pub fn get_mods_enabled_status(&self) -> HashMap<String, bool> {
        self.mods.iter().map(|m| (m.name.to_string(), m.enabled)).collect()
    }
}
