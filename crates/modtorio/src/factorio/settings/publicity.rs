//! Provides the object which corresponds to a Factorio server's settings about publicity.

use super::{GameFormatConversion, ServerSettingsGameFormat};
use crate::util::Limit;
use serde::{Deserialize, Serialize};

/// Represents the `password` and `token` fields.
#[derive(Deserialize, Serialize, Debug, PartialEq, Clone)]
pub enum Credential {
    Password(String),
    Token(String),
}

/// Represents the combination of the factorio.com login credential settings (`username` and either
/// credential) together with `visibility.public` being `true`.
#[derive(Deserialize, Serialize, Debug, PartialEq, Clone)]
pub struct PublicVisibility {
    pub username: String,
    pub credential: Credential,
}

/// Contains a server's player limit settings.
#[derive(Deserialize, Serialize, Debug, PartialEq)]
pub struct PlayerLimit {
    /// Corresponds to the `max_players` field. Defaults to `Limit::Unlimited` (value of 0 in
    /// `server-settings.json`).
    pub max: Limit,
    /// Corresponds to the `ignore_player_limit_for_returning_players` field. Defaults to `false`.
    pub ignore_for_returning: bool,
    /// Corresponds to the `afk_autokick_interval` field. Defaults to `Limit::Limited(5)` (value of
    /// 5 in `server-settings.json`).
    pub autokick: Limit,
}

/// Contains a server's publicity settings.
#[derive(Deserialize, Serialize, Debug, PartialEq)]
pub struct Publicity {
    /// Corresponds to the `visiblity.public` field. Defaults to
    /// `Some(PublicVisiblity::default())`. A value of `Some(...)` corresponds to
    /// `visibility.public` being `true` and `None` corresponds to the field being `false`.
    pub public: Option<PublicVisibility>,
    /// Corresponds to the `visibility.lan` field. Defaults to `true`.
    pub lan: bool,
    /// Corresponds to the `require_user_verification` field. Defaults to `true`.
    pub require_user_verification: bool,
    /// Corresponds to the player limit fields. Defaults to `PlayerLimit`'s default.
    pub player_limit: PlayerLimit,
    /// Corresponds to the `game_password` field. Defaults to an empty string.
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
                    credential: if game_format.token.is_empty() {
                        Credential::Password(game_format.password.clone())
                    } else {
                        Credential::Token(game_format.token.clone())
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
