use super::{GameFormatConversion, ServerSettingsGameFormat};
use serde::{Deserialize, Serialize};

#[derive(Deserialize, Serialize, Debug, PartialEq)]
pub struct Autosave {
    pub interval: u64,
    pub slots: u64,
    pub only_on_server: bool,
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

impl GameFormatConversion for Autosave {
    fn from_game_format(game_format: &ServerSettingsGameFormat) -> anyhow::Result<Self> {
        Ok(Self {
            interval: game_format.autosave_interval,
            slots: game_format.autosave_slots,
            only_on_server: game_format.autosave_only_on_server,
            non_blocking: game_format.non_blocking_saving,
        })
    }

    fn to_game_format(&self, game_format: &mut ServerSettingsGameFormat) -> anyhow::Result<()> {
        game_format.autosave_interval = self.interval;
        game_format.autosave_slots = self.slots;
        game_format.autosave_only_on_server = self.only_on_server;
        game_format.non_blocking_saving = self.non_blocking;

        Ok(())
    }
}
