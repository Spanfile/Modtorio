use super::{GameFormatConversion, Limit, ServerSettingsGameFormat};
use serde::{Deserialize, Serialize};

#[derive(Deserialize, Serialize, Debug, PartialEq, Clone)]
pub enum Credential {
    Password(String),
    Token(String),
}

#[derive(Deserialize, Serialize, Debug, PartialEq, Clone)]
pub struct PublicVisibility {
    pub username: String,
    pub credential: Credential,
}

#[derive(Deserialize, Serialize, Debug, PartialEq)]
pub struct PlayerLimit {
    pub max: Limit,
    pub ignore_for_returning: bool,
    pub autokick: Limit,
}

#[derive(Deserialize, Serialize, Debug, PartialEq)]
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

impl GameFormatConversion for Publicity {
    fn from_game_format(game_format: &ServerSettingsGameFormat) -> anyhow::Result<Self> {
        Ok(Self {
            public: if game_format.visibility.public {
                Some(PublicVisibility {
                    username: game_format.username.clone(),
                    credential: if !game_format.token.is_empty() {
                        Credential::Token(game_format.token.clone())
                    } else {
                        Credential::Password(game_format.password.clone())
                    },
                })
            } else {
                None
            },
            lan: game_format.visibility.lan,
            require_user_verification: game_format.require_user_verification,
            player_limit: PlayerLimit {
                max: Limit::from(game_format.max_players),
                ignore_for_returning: game_format.ignore_player_limit_for_returning_players,
                autokick: Limit::from(game_format.afk_autokick_interval),
            },
            password: game_format.game_password.clone(),
        })
    }

    fn to_game_format(&self, game_format: &mut ServerSettingsGameFormat) -> anyhow::Result<()> {
        game_format.visibility.public = self.public.is_some();
        game_format.visibility.lan = self.lan;

        if let Some(publicity) = self.public.clone() {
            game_format.username = publicity.username;
            match publicity.credential {
                Credential::Password(password) => game_format.password = password,
                Credential::Token(token) => game_format.token = token,
            }
        }

        game_format.require_user_verification = self.require_user_verification;
        game_format.max_players = self.player_limit.max.into();
        game_format.ignore_player_limit_for_returning_players =
            self.player_limit.ignore_for_returning;
        game_format.afk_autokick_interval = self.player_limit.autokick.into();
        game_format.game_password = self.password.clone();

        Ok(())
    }
}
