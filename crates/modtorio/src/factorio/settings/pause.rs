//! Provides the [Pause](Pause) struct which contains a Factorio server's settings
//! related to pausing the game.

use super::ServerSettingsGameFormat;
use crate::store::models::GameSettings;
use serde::{Deserialize, Serialize};

/// Contains a server's settings related to pausing the game.
#[derive(Deserialize, Serialize, Debug, PartialEq)]
pub struct Pause {
    /// Corresponds to the `auto_pause` field. Defaults to `true`.
    pub auto: bool,
    /// Corresponds to the `only_admins_can_pause_the_game` field. Defaults to `true`.
    pub only_admins: bool,
}

impl Default for Pause {
    fn default() -> Self {
        Self {
            auto: true,
            only_admins: true,
        }
    }
}

impl Pause {
    pub fn from_game_format(game_format: &ServerSettingsGameFormat) -> anyhow::Result<Self> {
        Ok(Self {
            auto: game_format.auto_pause,
            only_admins: game_format.only_admins_can_pause_the_game,
        })
    }

    pub fn to_game_format(&self, game_format: &mut ServerSettingsGameFormat) -> anyhow::Result<()> {
        game_format.auto_pause = self.auto;
        game_format.only_admins_can_pause_the_game = self.only_admins;

        Ok(())
    }

    pub fn from_store_format(store_format: &GameSettings) -> anyhow::Result<Self> {
        Ok(Self {
            auto: store_format.auto_pause != 0,
            only_admins: store_format.only_admins_can_pause_the_game != 0,
        })
    }

    pub fn to_store_format(&self, store_format: &mut GameSettings) -> anyhow::Result<()> {
        store_format.auto_pause = self.auto as i64;
        store_format.only_admins_can_pause_the_game = self.only_admins as i64;

        Ok(())
    }

    pub fn from_rpc_format(rpc_format: &rpc::ServerSettings) -> anyhow::Result<Self> {
        Ok(Self {
            auto: rpc_format.auto_pause,
            only_admins: rpc_format.only_admins_can_pause_the_game,
        })
    }

    pub fn to_rpc_format(&self, rpc_format: &mut rpc::ServerSettings) -> anyhow::Result<()> {
        rpc_format.auto_pause = self.auto;
        rpc_format.only_admins_can_pause_the_game = self.only_admins;

        Ok(())
    }
}
