mod autosave;
mod information;
mod limit;
mod pause;
mod publicity;
mod traffic;

use limit::Limit;
use serde::{de, de::Visitor, Deserialize, Deserializer};

#[derive(Deserialize, Debug, PartialEq)]
enum AllowCommands {
    Yes,
    No,
    AdminsOnly,
}

#[derive(Deserialize, Debug, PartialEq)]
pub struct Range {
    pub min: u64,
    pub max: u64,
}

#[derive(Deserialize, Debug, Default)]
pub struct ServerSettings {
    #[serde(flatten)]
    information: information::Information,
    #[serde(flatten)]
    publicity: publicity::Publicity,
    #[serde(flatten)]
    autosave: autosave::Autosave,
    #[serde(flatten)]
    pause: pause::Pause,
    #[serde(deserialize_with = "allow_commands_deserialize")]
    allow_commands: AllowCommands,
    #[serde(flatten)]
    traffic: traffic::Traffic,
}

impl Default for AllowCommands {
    fn default() -> Self {
        Self::AdminsOnly
    }
}

fn allow_commands_deserialize<'de, D>(deserializer: D) -> Result<AllowCommands, D::Error>
where
    D: Deserializer<'de>,
{
    struct AllowCommandsVisitor;

    impl<'de> Visitor<'de> for AllowCommandsVisitor {
        type Value = AllowCommands;

        fn expecting(&self, formatter: &mut std::fmt::Formatter) -> std::fmt::Result {
            formatter.write_str("boolean or the string 'admins-only'")
        }

        fn visit_str<E>(self, value: &str) -> Result<Self::Value, E>
        where
            E: de::Error,
        {
            if value == "admins-only" {
                Ok(Self::Value::AdminsOnly)
            } else {
                Err(de::Error::invalid_value(de::Unexpected::Str(value), &self))
            }
        }

        fn visit_bool<E>(self, value: bool) -> Result<Self::Value, E>
        where
            E: de::Error,
        {
            Ok(if value {
                Self::Value::Yes
            } else {
                Self::Value::No
            })
        }
    }

    deserializer.deserialize_any(AllowCommandsVisitor)
}

impl ServerSettings {
    pub fn from_json(json: &str) -> anyhow::Result<Self> {
        Ok(serde_json::from_str(json)?)
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use serde_json::from_str;

    #[test]
    fn deserialize() -> anyhow::Result<()> {
        let obj: ServerSettings = from_str(
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
            information::Information {
                name: String::from("test"),
                description: String::from("test"),
                tags: vec![String::from("1"), String::from("2")],
            }
        );

        assert_eq!(
            obj.publicity,
            publicity::Publicity {
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
            autosave::Autosave {
                interval: 5,
                slots: 10,
                only_on_server: true,
                non_blocking: true,
            }
        );

        assert_eq!(
            obj.pause,
            pause::Pause {
                auto: true,
                only_admins: true,
            }
        );

        assert_eq!(obj.allow_commands, AllowCommands::AdminsOnly);

        assert_eq!(
            obj.traffic,
            traffic::Traffic {
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
