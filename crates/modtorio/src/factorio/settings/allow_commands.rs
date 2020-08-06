//! Provides the [`AllowCommands`](AllowCommands) enum which corresponds to the `allow_commands`
//! field.

use super::{GameFormatConversion, ServerSettingsGameFormat};
use serde::{Deserialize, Serialize};

/// The string-value for the [`Yes`](./enum.AllowCommands.html#variant.Yes) variant
const YES_GAME_VALUE: &str = "true";
/// The string-value for the [`No`](./enum.AllowCommands.html#variant.No) variant
const NO_GAME_VALUE: &str = "false";
/// The string-value for the [`AdminsOnly`](./enum.AllowCommands.html#variant.AdminsOnly) variant
const ADMINS_ONLY_GAME_VALUE: &str = "admins-only";

/// Represents the `allow_commands` field.
///
/// Defaults to `AdminsOnly`.
#[derive(Deserialize, Serialize, Debug, PartialEq)]
pub enum AllowCommands {
    /// Represents the `true` value for the setting.
    Yes,
    /// Represents the `false` value for the setting.
    No,
    /// Represents the `admins-only` for the setting.
    AdminsOnly,
}

impl Default for AllowCommands {
    fn default() -> Self {
        AllowCommands::AdminsOnly
    }
}

impl GameFormatConversion for AllowCommands {
    fn from_game_format(game_format: &ServerSettingsGameFormat) -> anyhow::Result<Self> {
        Ok(match game_format.allow_commands.as_str() {
            YES_GAME_VALUE => AllowCommands::Yes,
            NO_GAME_VALUE => AllowCommands::No,
            ADMINS_ONLY_GAME_VALUE => AllowCommands::AdminsOnly,
            v => panic!("invalid allow_commands value in game format: {}", v),
        })
    }

    fn to_game_format(&self, game_format: &mut ServerSettingsGameFormat) -> anyhow::Result<()> {
        game_format.allow_commands = match self {
            Self::Yes => String::from(YES_GAME_VALUE),
            Self::No => String::from(NO_GAME_VALUE),
            Self::AdminsOnly => String::from(ADMINS_ONLY_GAME_VALUE),
        };
        Ok(())
    }
}
