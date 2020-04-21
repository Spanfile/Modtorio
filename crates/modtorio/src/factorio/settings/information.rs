use super::{GameFormatConversion, ServerSettingsGameFormat};
use serde::{Deserialize, Serialize};

#[derive(Deserialize, Serialize, Debug, PartialEq)]
pub struct Information {
    pub name: String,
    pub description: String,
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

impl GameFormatConversion for Information {
    fn from_game_format(game_format: &ServerSettingsGameFormat) -> anyhow::Result<Self> {
        Ok(Self {
            name: game_format.name.clone(),
            description: game_format.description.clone(),
            tags: game_format.tags.clone(),
        })
    }

    fn to_game_format(&self, game_format: &mut ServerSettingsGameFormat) -> anyhow::Result<()> {
        game_format.name = self.name.clone();
        game_format.description = self.description.clone();
        game_format.tags = self.tags.clone();

        Ok(())
    }
}
