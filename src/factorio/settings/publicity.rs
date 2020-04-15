use super::Limit;
use serde::{
    de::{self, MapAccess, Visitor},
    Deserialize,
};

#[derive(Debug, PartialEq)]
pub enum Credential {
    Password(String),
    Token(String),
}

#[derive(Debug, PartialEq)]
pub struct PublicVisibility {
    pub username: String,
    pub credential: Credential,
}

#[derive(Deserialize, Debug, PartialEq)]
pub struct PlayerLimit {
    pub max: Limit,
    pub ignore_for_returning: bool,
    pub autokick: Limit,
}

#[derive(Debug, PartialEq)]
pub struct Publicity {
    pub public: Option<PublicVisibility>,
    pub lan: bool,
    pub require_user_verification: bool,
    pub player_limit: PlayerLimit,
    pub password: String,
}

impl Default for Publicity {
    fn default() -> Self {
        Self {
            lan: true,
            require_user_verification: true,
            player_limit: PlayerLimit::default(),
            password: String::default(),
            public: Some(PublicVisibility::default()),
        }
    }
}

impl Default for PlayerLimit {
    fn default() -> Self {
        Self {
            max: Limit::Unlimited,
            ignore_for_returning: false,
            autokick: Limit::Limited(5),
        }
    }
}

impl Default for PublicVisibility {
    fn default() -> Self {
        Self {
            username: String::default(),
            credential: Credential::Password(String::default()),
        }
    }
}

impl<'de> Deserialize<'de> for Publicity {
    fn deserialize<D>(deserializer: D) -> Result<Self, D::Error>
    where
        D: serde::Deserializer<'de>,
    {
        #[derive(Debug, Deserialize)]
        #[serde(field_identifier, rename_all = "snake_case")]
        enum Field {
            Visibility,
            Username,
            Password,
            Token,
            RequireUserVerification,
            MaxPlayers,
            IgnorePlayerLimitForReturningPlayers,
            AfkAutokickInterval,
            GamePassword,
        }

        #[derive(Debug, Deserialize)]
        struct Visibility {
            public: bool,
            lan: bool,
        }

        struct PublicityVisitor;

        impl<'de> Visitor<'de> for PublicityVisitor {
            type Value = Publicity;

            fn expecting(&self, formatter: &mut std::fmt::Formatter) -> std::fmt::Result {
                formatter.write_str("publicity settings")
            }

            fn visit_map<V>(self, mut map: V) -> Result<Self::Value, V::Error>
            where
                V: MapAccess<'de>,
            {
                field_deserializers!(
                    map,
                    [visibility, Visibility, Visibility],
                    [username, String, Username],
                    [password, String, Password],
                    [token, String, Token],
                    [require_user_verification, bool, RequireUserVerification],
                    [max_players, Limit, MaxPlayers],
                    [
                        ignore_player_limit_for_returning_players,
                        bool,
                        IgnorePlayerLimitForReturningPlayers
                    ],
                    [afk_autokick_interval, Limit, AfkAutokickInterval],
                    [game_password, String, GamePassword]
                );

                Ok(Self::Value {
                    lan: visibility.lan,
                    public: if visibility.public {
                        Some(PublicVisibility {
                            username,
                            credential: if !token.is_empty() {
                                Credential::Token(token)
                            } else {
                                Credential::Password(password)
                            },
                        })
                    } else {
                        None
                    },
                    require_user_verification,
                    password: game_password,
                    player_limit: PlayerLimit {
                        max: max_players,
                        ignore_for_returning: ignore_player_limit_for_returning_players,
                        autokick: afk_autokick_interval,
                    },
                })
            }
        }

        const FIELDS: &'static [&'static str] = &[
            "visibility",
            "username",
            "password",
            "token",
            "require_user_verification",
            "max_players",
            "ignore_player_limit_for_returning_players",
            "afk_autokick_interval",
            "game_password",
        ];

        deserializer.deserialize_struct("Publicity", FIELDS, PublicityVisitor)
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use serde_json::from_str;

    #[test]
    fn deserialize() -> anyhow::Result<()> {
        let obj: Publicity = from_str(
            r#"{
    "visibility": {
        "public": true,
        "lan": true
    },
    "username": "test",
    "password": "test",
    "token": "test",
    "require_user_verification": true,
    "max_players": 8,
    "ignore_player_limit_for_returning_players": true,
    "afk_autokick_interval": 5,
    "game_password": "test"
}"#,
        )?;

        assert_eq!(
            obj.public,
            Some(PublicVisibility {
                username: String::from("test"),
                credential: Credential::Token(String::from("test"))
            })
        );
        assert_eq!(obj.lan, true);
        assert_eq!(obj.require_user_verification, true);
        assert_eq!(
            obj.player_limit,
            PlayerLimit {
                max: Limit::Limited(8),
                ignore_for_returning: true,
                autokick: Limit::Limited(5)
            }
        );
        assert_eq!(obj.password, "test");

        Ok(())
    }
}
