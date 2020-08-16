//! Provides the [`AllowCommands`](AllowCommands) enum which corresponds to the `allow_commands`
//! field.

use super::ServerSettingsGameFormat;
use crate::store::models::GameSettings;
use rpc::server_settings;
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

impl AllowCommands {
    pub fn from_game_format(game_format: &ServerSettingsGameFormat) -> anyhow::Result<Self> {
        Ok(match game_format.allow_commands.as_str() {
            YES_GAME_VALUE => AllowCommands::Yes,
            NO_GAME_VALUE => AllowCommands::No,
            ADMINS_ONLY_GAME_VALUE => AllowCommands::AdminsOnly,
            v => panic!("invalid allow_commands value in game format: {}", v),
        })
    }

    pub fn to_game_format(&self, game_format: &mut ServerSettingsGameFormat) -> anyhow::Result<()> {
        game_format.allow_commands = match self {
            Self::Yes => String::from(YES_GAME_VALUE),
            Self::No => String::from(NO_GAME_VALUE),
            Self::AdminsOnly => String::from(ADMINS_ONLY_GAME_VALUE),
        };
        Ok(())
    }

    pub fn from_store_format(store_format: &GameSettings) -> anyhow::Result<Self> {
        Ok(match store_format.allow_commands.as_str() {
            YES_GAME_VALUE => AllowCommands::Yes,
            NO_GAME_VALUE => AllowCommands::No,
            ADMINS_ONLY_GAME_VALUE => AllowCommands::AdminsOnly,
            v => panic!("invalid allow_commands value in store format: {}", v),
        })
    }

    pub fn to_store_format(&self, store_format: &mut GameSettings) -> anyhow::Result<()> {
        store_format.allow_commands = match self {
            Self::Yes => String::from(YES_GAME_VALUE),
            Self::No => String::from(NO_GAME_VALUE),
            Self::AdminsOnly => String::from(ADMINS_ONLY_GAME_VALUE),
        };
        Ok(())
    }

    pub fn from_rpc_format(rpc_format: &rpc::ServerSettings) -> anyhow::Result<Self> {
        // TODO: ugly integer match
        Ok(match rpc_format.allow_commands {
            0 => AllowCommands::Yes,
            1 => AllowCommands::No,
            2 => AllowCommands::AdminsOnly,
            i => panic!("invalid allow_commands value in RPC format: {}", i),
        })
    }

    pub fn to_rpc_format(&self, rpc_format: &mut rpc::ServerSettings) -> anyhow::Result<()> {
        rpc_format.allow_commands = match self {
            Self::Yes => server_settings::AllowCommands::Yes.into(),
            Self::No => server_settings::AllowCommands::No.into(),
            Self::AdminsOnly => server_settings::AllowCommands::AdminsOnly.into(),
        };
        Ok(())
    }
}
