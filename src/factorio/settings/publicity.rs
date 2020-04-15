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
struct PlayerLimit {
    max: Limit,
    ignore_for_returning: bool,
    autokick: Limit,
}

#[derive(Debug)]
pub struct Publicity {
    public: Option<PublicVisibility>,
    lan: bool,
    require_user_verification: bool,
    player_limit: PlayerLimit,
    password: String,
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
                #[derive(Debug, Deserialize)]
                #[serde(field_identifier, rename_all = "lowercase")]
                enum VisibilityField {
                    Public,
                    LAN,
                }

                let mut visibility: Option<Visibility> = None;
                let mut username = None;
                let mut password = None;
                let mut token: Option<String> = None;
                let mut require_user_verification = None;
                let mut max_players = None;
                let mut ignore_player_limit_for_returning_players = None;
                let mut afk_autokick_interval = None;
                let mut game_password = None;

                while let Some(key) = map.next_key()? {
                    match key {
                        Field::Visibility => {
                            if visibility.is_some() {
                                return Err(de::Error::duplicate_field("visibility"));
                            }
                            visibility = Some(map.next_value()?);
                        }
                        Field::Username => {
                            if username.is_some() {
                                return Err(de::Error::duplicate_field("username"));
                            }
                            username = Some(map.next_value()?);
                        }
                        Field::Password => {
                            if password.is_some() {
                                return Err(de::Error::duplicate_field("password"));
                            }
                            password = Some(map.next_value()?);
                        }
                        Field::Token => {
                            if token.is_some() {
                                return Err(de::Error::duplicate_field("token"));
                            }
                            token = Some(map.next_value()?);
                        }
                        Field::RequireUserVerification => {
                            if require_user_verification.is_some() {
                                return Err(de::Error::duplicate_field(
                                    "require_user_verification",
                                ));
                            }
                            require_user_verification = Some(map.next_value()?);
                        }
                        Field::MaxPlayers => {
                            if max_players.is_some() {
                                return Err(de::Error::duplicate_field("max_players"));
                            }
                            max_players = Some(map.next_value()?);
                        }
                        Field::IgnorePlayerLimitForReturningPlayers => {
                            if ignore_player_limit_for_returning_players.is_some() {
                                return Err(de::Error::duplicate_field(
                                    "ignore_player_limit_for_returning_players",
                                ));
                            }
                            ignore_player_limit_for_returning_players = Some(map.next_value()?);
                        }
                        Field::AfkAutokickInterval => {
                            if afk_autokick_interval.is_some() {
                                return Err(de::Error::duplicate_field("afk_autokick_interval"));
                            }
                            afk_autokick_interval = Some(map.next_value()?);
                        }
                        Field::GamePassword => {
                            if game_password.is_some() {
                                return Err(de::Error::duplicate_field("game_password"));
                            }
                            game_password = Some(map.next_value()?);
                        }
                    }
                }

                let visibility =
                    visibility.ok_or_else(|| de::Error::missing_field("visibility"))?;
                let username = username.ok_or_else(|| de::Error::missing_field("username"))?;
                let password = password.ok_or_else(|| de::Error::missing_field("password"))?;
                let token = token.ok_or_else(|| de::Error::missing_field("token"))?;
                let require_user_verification = require_user_verification
                    .ok_or_else(|| de::Error::missing_field("require_user_verification"))?;
                let max_players =
                    max_players.ok_or_else(|| de::Error::missing_field("max_players"))?;
                let ignore_player_limit_for_returning_players =
                    ignore_player_limit_for_returning_players.ok_or_else(|| {
                        de::Error::missing_field("ignore_player_limit_for_returning_players")
                    })?;
                let afk_autokick_interval = afk_autokick_interval
                    .ok_or_else(|| de::Error::missing_field("afk_autokick_interval"))?;
                let game_password =
                    game_password.ok_or_else(|| de::Error::missing_field("game_password"))?;

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
