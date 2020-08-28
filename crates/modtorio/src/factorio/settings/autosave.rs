//! Provides the [Autosave](Autosave) struct which contains a server's autosave settings.

use super::ServerSettingsGameFormat;
use crate::store::models::GameSettings;
use serde::{Deserialize, Serialize};

/// Contains a server's autosave settings.
#[derive(Deserialize, Serialize, Debug, PartialEq)]
pub struct Autosave {
    /// Corresponds to the `autosave_interval` field. Defaults to 5.
    pub interval: u64,
    /// Corresponds to the `autosave_slots` field. Defaults to 5.
    pub slots: u64,
    /// Corresponds to the `autosave_only_on_server` field. Defaults to `true`.
    pub only_on_server: bool,
    /// Corresponds to the `non_blocking_saving` field. Defaults to `false`.
    pub non_blocking: bool,
}

impl Default for Autosave {
    fn default() -> Self {
        Self {
            interval: 5,
            slots: 5,
            only_on_server: true,
            non_blocking: false,
        }
    }
}

impl Autosave {
    /// Returns a new `Autosave` from a given `ServerSettingsGameFormat`.
    pub fn from_game_format(game_format: &ServerSettingsGameFormat) -> Self {
        Self {
            interval: game_format.autosave_interval,
            slots: game_format.autosave_slots,
            only_on_server: game_format.autosave_only_on_server,
            non_blocking: game_format.non_blocking_saving,
        }
    }

    /// Modifies a given `ServerSettingsGameFormat` with this object's settings.
    pub fn to_game_format(&self, game_format: &mut ServerSettingsGameFormat) {
        game_format.autosave_interval = self.interval;
        game_format.autosave_slots = self.slots;
        game_format.autosave_only_on_server = self.only_on_server;
        game_format.non_blocking_saving = self.non_blocking;
    }

    /// Merges the settings from the server's JSON-setting file from another given `ServerSettings` object.
    pub fn merge_game_settings(&mut self, other: Self) {
        *self = other
    }

    /// Returns a new `Autosave` from a given `GameSettings`.
    pub fn from_store_format(store_format: &GameSettings) -> Self {
        Self {
            interval: store_format.autosave_interval as u64,
            slots: store_format.autosave_slots as u64,
            only_on_server: store_format.autosave_only_on_server != 0,
            non_blocking: store_format.non_blocking_saving != 0,
        }
    }

    /// Modifies a given `GameSettings` with this object's settings.
    pub fn to_store_format(&self, store_format: &mut GameSettings) {
        store_format.autosave_interval = self.interval as i64;
        store_format.autosave_slots = self.slots as i64;
        store_format.autosave_only_on_server = self.only_on_server as i64;
        store_format.non_blocking_saving = self.non_blocking as i64;
    }

    /// Mutates `self` with the value from a given RPC `ServerSettings` object.
    pub fn modify_self_with_rpc(&mut self, rpc_format: &rpc::ServerSettings) {
        self.interval = rpc_format.autosave_interval;
        self.slots = rpc_format.autosave_slots;
        self.only_on_server = rpc_format.autosave_only_on_server;
        self.non_blocking = rpc_format.non_blocking_saving;
    }

    /// Modifies a given `ServerSettings` with this object's settings.
    pub fn to_rpc_format(&self, rpc_format: &mut rpc::ServerSettings) {
        rpc_format.autosave_interval = self.interval;
        rpc_format.autosave_slots = self.slots;
        rpc_format.autosave_only_on_server = self.only_on_server;
        rpc_format.non_blocking_saving = self.non_blocking;
    }
}
