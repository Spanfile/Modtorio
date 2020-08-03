//! Provides the [Information](Information) struct which contains a server's settings related to its
//! public display.

use super::{GameFormatConversion, ServerSettingsGameFormat};
use serde::{Deserialize, Serialize};

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
