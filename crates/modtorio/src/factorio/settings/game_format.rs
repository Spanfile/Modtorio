use serde::{Deserialize, Serialize};

pub trait GameFormatConversion
where
    Self: Sized,
{
    fn from_game_format(game_format: &ServerSettingsGameFormat) -> anyhow::Result<Self>;
    fn to_game_format(&self, game_format: &mut ServerSettingsGameFormat) -> anyhow::Result<()>;
}

#[derive(Debug, Deserialize, Serialize, Default)]
pub struct ServerSettingsGameFormat {
    pub name: String,
    pub description: String,
    pub tags: Vec<String>,
    pub max_players: u64,
    pub visibility: VisibilityGameFormat,
    pub username: String,
    pub password: String,
    pub token: String,
    pub game_password: String,
    pub require_user_verification: bool,
    pub max_upload_in_kilobytes_per_second: u64,
    pub max_upload_slots: u64,
    pub minimum_latency_in_ticks: u64,
    pub ignore_player_limit_for_returning_players: bool,
    pub allow_commands: String,
    pub autosave_interval: u64,
    pub autosave_slots: u64,
    pub afk_autokick_interval: u64,
    pub auto_pause: bool,
    pub only_admins_can_pause_the_game: bool,
    pub autosave_only_on_server: bool,
    pub non_blocking_saving: bool,
    pub minimum_segment_size: u64,
    pub minimum_segment_size_peer_count: u64,
    pub maximum_segment_size: u64,
    pub maximum_segment_size_peer_count: u64,
}

#[derive(Debug, Deserialize, Serialize, Default)]
pub struct VisibilityGameFormat {
    pub public: bool,
    pub lan: bool,
}
