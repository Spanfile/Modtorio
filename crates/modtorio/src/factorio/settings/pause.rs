//! Provides the [Pause](Pause) struct which contains a Factorio server's settings
//! related to pausing the game.

use super::{GameFormatConversion, ServerSettingsGameFormat};
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

impl GameFormatConversion for Pause {
    fn from_game_format(game_format: &ServerSettingsGameFormat) -> anyhow::Result<Self> {
        Ok(Self {
            auto: game_format.auto_pause,
            only_admins: game_format.only_admins_can_pause_the_game,
        })
    }

    fn to_game_format(&self, game_format: &mut ServerSettingsGameFormat) -> anyhow::Result<()> {
        game_format.auto_pause = self.auto;
        game_format.only_admins_can_pause_the_game = self.only_admins;

        Ok(())
    }
}
