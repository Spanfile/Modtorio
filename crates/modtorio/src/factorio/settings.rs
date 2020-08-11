//! Provides the [`ServerSettings`](ServerSettings) object used to interact with a server's
//! settings.

mod allow_commands;
mod autosave;
mod game_format;
mod information;
mod pause;
mod publicity;
mod rpc_format;
mod traffic;

use allow_commands::AllowCommands;
use autosave::Autosave;
use game_format::{GameFormatConversion, ServerSettingsGameFormat};
use information::Information;
use pause::Pause;
use publicity::Publicity;
pub use rpc_format::RpcFormatConversion;
use serde::{Deserialize, Serialize};
use traffic::Traffic;

/// Stores a server's settings in a structured manner.
#[derive(Deserialize, Serialize, Debug, Default)]
pub struct ServerSettings {
    /// Contains settings related to the server's public display.
    pub information: Information,
    /// Contains settings related to the servers publicity.
    pub publicity: Publicity,
    /// Contains settings related to the server's autosaving.
    pub autosave: Autosave,
    /// Contains settings related to the server's pausing.
    pub pause: Pause,
    /// Represents the `allow_commands` setting.
    pub allow_commands: AllowCommands,
    /// Contains settings related to the server's traffic use.
    pub traffic: Traffic,
}

#[allow(dead_code)]
impl ServerSettings {
    /// Returns a new `ServerSettings` by deserializing a given JSON string.
    pub fn from_game_json(json: &str) -> anyhow::Result<Self> {
        let game_format = serde_json::from_str(json)?;
        Ok(ServerSettings::from_game_format(&game_format)?)
    }

    /// Returns a string by serializing the `ServerSettings` object into the game's
    /// `server-settings.json` file format.
    pub fn to_game_json(&self) -> anyhow::Result<String> {
        let mut game_format = ServerSettingsGameFormat::default();
        self.to_game_format(&mut game_format)?;
        Ok(serde_json::to_string(&game_format)?)
    }
}

impl GameFormatConversion for ServerSettings {
    fn from_game_format(game_format: &ServerSettingsGameFormat) -> anyhow::Result<Self> {
        Ok(Self {
            information: Information::from_game_format(game_format)?,
            publicity: Publicity::from_game_format(game_format)?,
            autosave: Autosave::from_game_format(game_format)?,
            pause: Pause::from_game_format(game_format)?,
            allow_commands: AllowCommands::from_game_format(game_format)?,
            traffic: Traffic::from_game_format(game_format)?,
        })
    }

    fn to_game_format(&self, game_format: &mut ServerSettingsGameFormat) -> anyhow::Result<()> {
        self.information.to_game_format(game_format)?;
        self.publicity.to_game_format(game_format)?;
        self.autosave.to_game_format(game_format)?;
        self.pause.to_game_format(game_format)?;
        self.allow_commands.to_game_format(game_format)?;
        self.traffic.to_game_format(game_format)?;

        Ok(())
    }
}

impl RpcFormatConversion for ServerSettings {
    fn from_rpc_format(rpc_format: &rpc::ServerSettings) -> anyhow::Result<Self> {
        Ok(Self {
            information: Information::from_rpc_format(rpc_format)?,
            publicity: Publicity::from_rpc_format(rpc_format)?,
            autosave: Autosave::from_rpc_format(rpc_format)?,
            pause: Pause::from_rpc_format(rpc_format)?,
            allow_commands: AllowCommands::from_rpc_format(rpc_format)?,
            traffic: Traffic::from_rpc_format(rpc_format)?,
        })
    }
    fn to_rpc_format(&self, rpc_format: &mut rpc::ServerSettings) -> anyhow::Result<()> {
        self.information.to_rpc_format(rpc_format)?;
        self.publicity.to_rpc_format(rpc_format)?;
        self.autosave.to_rpc_format(rpc_format)?;
        self.pause.to_rpc_format(rpc_format)?;
        self.allow_commands.to_rpc_format(rpc_format)?;
        self.traffic.to_rpc_format(rpc_format)?;

        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::util::{Limit, Range};

    #[test]
    fn from_game_format() -> anyhow::Result<()> {
        let obj = ServerSettings::from_game_json(
            r#"{
  "name": "test",
  "description": "test",
  "tags": [
    "1",
    "2"
  ],
  "max_players": 0,
  "visibility": {
    "public": true,
    "lan": true
  },
  "username": "test",
  "password": "test",
  "token": "test",
  "game_password": "test",
  "require_user_verification": true,
  "max_upload_in_kilobytes_per_second": 0,
  "max_upload_slots": 5,
  "minimum_latency_in_ticks": 0,
  "ignore_player_limit_for_returning_players": false,
  "allow_commands": "admins-only",
  "autosave_interval": 5,
  "autosave_slots": 10,
  "afk_autokick_interval": 0,
  "auto_pause": true,
  "only_admins_can_pause_the_game": true,
  "autosave_only_on_server": true,
  "non_blocking_saving": true,
  "minimum_segment_size": 25,
  "minimum_segment_size_peer_count": 20,
  "maximum_segment_size": 100,
  "maximum_segment_size_peer_count": 10
}"#,
        )?;

        assert_eq!(
            obj.information,
            Information {
                name: String::from("test"),
                description: String::from("test"),
                tags: vec![String::from("1"), String::from("2")],
            }
        );

        assert_eq!(
            obj.publicity,
            Publicity {
                lan: true,
                public: Some(publicity::PublicVisibility {
                    username: String::from("test"),
                    credential: publicity::Credential::Token(String::from("test"))
                }),
                password: String::from("test"),
                player_limit: publicity::PlayerLimit {
                    max: Limit::Unlimited,
                    autokick: Limit::Unlimited,
                    ignore_for_returning: false,
                },
                require_user_verification: true,
            }
        );

        assert_eq!(
            obj.autosave,
            Autosave {
                interval: 5,
                slots: 10,
                only_on_server: true,
                non_blocking: true,
            }
        );

        assert_eq!(
            obj.pause,
            Pause {
                auto: true,
                only_admins: true,
            }
        );

        assert_eq!(obj.allow_commands, AllowCommands::AdminsOnly);

        assert_eq!(
            obj.traffic,
            Traffic {
                minimum_latency: 0,
                segment_size: traffic::SegmentSize {
                    size: Range { min: 25, max: 100 },
                    peer_count: Range { min: 20, max: 10 }
                },
                upload: traffic::Upload {
                    max: Limit::Unlimited,
                    slots: Limit::Limited(5)
                }
            }
        );

        Ok(())
    }
}
