//! Provides the [`ServerSettingsGameFormat`](ServerSettingsGameFormat) struct and
//! [`GameFormatConversion`](GameFormatConversion) trait used to translate a Factorio server's
//! `server-settings.json` into Modtorio's [`ServerSettings`](super::ServerSettings) and vice versa.

use serde::{Deserialize, Serialize};

/// Defines the functions used to convert a value into the kind used in a `server-settings.json`
/// file and vice versa.
pub trait GameFormatConversion
where
    Self: Sized,
{
    /// Creates a new instance of `Self` from a given `ServerSettingsGameFormat` struct.
    fn from_game_format(game_format: &ServerSettingsGameFormat) -> anyhow::Result<Self>;
    /// Modifies an existing `ServerSettingsGameFormat` struct with self's own settings.
    fn to_game_format(&self, game_format: &mut ServerSettingsGameFormat) -> anyhow::Result<()>;
}

/// Stores a server's settings in the same structure as its `server-settings.json` file.
#[derive(Debug, Deserialize, Serialize, Default)]
pub struct ServerSettingsGameFormat {
    /// Corresponds to the `name` setting.
    pub name: String,
    /// Corresponds to the `description` setting.
    pub description: String,
    /// Corresponds to the `tags` setting.
    pub tags: Vec<String>,
    /// Corresponds to the `max_players` setting.
    pub max_players: u64,
    /// Corresponds to the `visibility` setting object.
    pub visibility: VisibilityGameFormat,
    /// Corresponds to the `username` setting.
    pub username: String,
    /// Corresponds to the `password` setting.
    pub password: String,
    /// Corresponds to the `token` setting.
    pub token: String,
    /// Corresponds to the `game_password` setting.
    pub game_password: String,
    /// Corresponds to the `require_user_verification` setting.
    pub require_user_verification: bool,
    /// Corresponds to the `max_upload_in_kilobytes_per_second` setting.
    pub max_upload_in_kilobytes_per_second: u64,
    /// Corresponds to the `max_upload_slots` setting.
    pub max_upload_slots: u64,
    /// Corresponds to the `minimum_latency_in_ticks` setting.
    pub minimum_latency_in_ticks: u64,
    /// Corresponds to the `ignore_player_limit_for_returning_players` setting.
    pub ignore_player_limit_for_returning_players: bool,
    /// Corresponds to the `allow_commands` setting.
    pub allow_commands: String,
    /// Corresponds to the `autosave_interval` setting.
    pub autosave_interval: u64,
    /// Corresponds to the `autosave_slots` setting.
    pub autosave_slots: u64,
    /// Corresponds to the `afk_autokick_interval` setting.
    pub afk_autokick_interval: u64,
    /// Corresponds to the `auto_pause` setting.
    pub auto_pause: bool,
    /// Corresponds to the `only_admins_can_pause_the_game` setting.
    pub only_admins_can_pause_the_game: bool,
    /// Corresponds to the `autosave_only_on_server` setting.
    pub autosave_only_on_server: bool,
    /// Corresponds to the `non_blocking_saving` setting.
    pub non_blocking_saving: bool,
    /// Corresponds to the `minimum_segment_size` setting.
    pub minimum_segment_size: u64,
    /// Corresponds to the `minimum_segment_size_peer_count` setting.
    pub minimum_segment_size_peer_count: u64,
    /// Corresponds to the `maximum_segment_size` setting.
    pub maximum_segment_size: u64,
    /// Corresponds to the `maximum_segment_size_peer_count` setting.
    pub maximum_segment_size_peer_count: u64,
}

/// Corresponds to the `visibility` setting object.
#[derive(Debug, Deserialize, Serialize, Default)]
pub struct VisibilityGameFormat {
    /// Corresponds to the `public` setting.
    pub public: bool,
    /// Corresponds to the `lan` setting.
    pub lan: bool,
}
