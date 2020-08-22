//! Provides the [Information](Information) struct which contains a server's settings related to its
//! public display.

use super::ServerSettingsGameFormat;
use crate::store::models::GameSettings;
use serde::{Deserialize, Serialize};

/// The splitter sequence used to separate server tags in the store database column.
const TAGS_SPLITTER: &str = ",";

/// Contains a server's settings related to its public display.
#[derive(Deserialize, Serialize, Debug, PartialEq)]
pub struct Information {
    /// Corresponds to the `name` field. Defaults to `A Factorio server`.
    pub name: String,
    /// Corresponds to the `description` field. Defaults to `A Factorio server`.
    pub description: String,
    /// Corresponds to the `tags` field. Defaults to an empty vector.
    pub tags: Vec<String>,
}

impl Default for Information {
    fn default() -> Self {
        Self {
            name: String::from("A Factorio server"),
            description: String::from("A Factorio server"),
            tags: Vec::new(),
        }
    }
}

impl Information {
    /// Returns a new `Information` from a given `ServerSettingsGameFormat`.
    pub fn from_game_format(game_format: &ServerSettingsGameFormat) -> Self {
        Self {
            name: game_format.name.clone(),
            description: game_format.description.clone(),
            tags: game_format.tags.clone(),
        }
    }

    /// Modifies a given `ServerSettingsGameFormat` with this object's settings.
    pub fn to_game_format(&self, game_format: &mut ServerSettingsGameFormat) {
        game_format.name = self.name.clone();
        game_format.description = self.description.clone();
        game_format.tags = self.tags.clone();
    }

    /// Returns a new `Information` from a given `GameSettings`.
    pub fn from_store_format(store_format: &GameSettings) -> Self {
        Self {
            name: store_format.name.clone(),
            description: store_format.description.clone(),
            tags: store_format.tags.split(TAGS_SPLITTER).map(str::to_string).collect(),
        }
    }

    /// Modifies a given `GameSettings` with this object's settings.
    pub fn to_store_format(&self, store_format: &mut GameSettings) {
        store_format.name = self.name.clone();
        store_format.description = self.description.clone();
        store_format.tags = self.tags.clone().join(TAGS_SPLITTER);
    }

    /// Mutates `self` with the value from a given RPC `ServerSettings` object.
    pub fn modify_self_with_rpc(&mut self, rpc_format: &rpc::ServerSettings) {
        self.name = rpc_format.name.clone();
        self.description = rpc_format.description.clone();
        self.tags = rpc_format.tags.clone();
    }

    /// Modifies a given `ServerSettings` with this object's settings.
    pub fn to_rpc_format(&self, rpc_format: &mut rpc::ServerSettings) {
        rpc_format.name = self.name.clone();
        rpc_format.description = self.description.clone();
        rpc_format.tags = self.tags.clone();
    }
}
