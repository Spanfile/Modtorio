use derive::Model;

/// A Factorio server's settings, including both settings from its `server-settings.json` file and its command line
/// parameters.
#[derive(Debug, Model)]
pub struct GameSettings {
    /// The game's ID these settings are for.
    #[index]
    pub game: i64,
    /// The `name` setting in `server-settings.json`.
    pub name: String,
    /// The `description` setting in `server-settings.json`.
    pub description: String,
    /// The `max_players` setting in `server-settings.json`.
    pub max_players: i64,
    /// The `visibility.public` boolean setting in `server-settings.json`.
    pub public_visibility: i64,
    /// The `visibility.lan` boolean setting in `server-settings.json`.
    pub lan_visibility: i64,
    /// The `username` setting in `server-settings.json`.
    pub username: String,
    /// The `password` setting in `server-settings.json`.
    pub password: String,
    /// The `token` setting in `server-settings.json`.
    pub token: String,
    /// The `game_password` setting in `server-settings.json`.
    pub game_password: String,
    /// The `require_user_verification` boolean setting in `server-settings.json`.
    pub require_user_verification: i64,
    /// The `max_upload_in_kilobytes_per_second` setting in `server-settings.json`.
    pub max_upload_in_kilobytes_per_second: i64,
    /// The `max_upload_slots` setting in `server-settings.json`.
    pub max_upload_slots: i64,
    /// The `minimum_latency_in_ticks` setting in `server-settings.json`.
    pub minimum_latency_in_ticks: i64,
    /// The `ignore_player_limit_for_returning_players` boolean setting in `server-settings.json`.
    pub ignore_player_limit_for_returning_players: i64,
    /// The `allow_commands` setting in `server-settings.json`.
    pub allow_commands: String,
    /// The `autosave_interval` setting in `server-settings.json`.
    pub autosave_interval: i64,
    /// The `autosave_slots` setting in `server-settings.json`.
    pub autosave_slots: i64,
    /// The `afk_autokick_interval` setting in `server-settings.json`.
    pub afk_autokick_interval: i64,
    /// The `auto_pause` boolean setting in `server-settings.json`.
    pub auto_pause: i64,
    /// The `only_admins_can_pause_the_game` boolean setting in `server-settings.json`.
    pub only_admins_can_pause_the_game: i64,
    /// The `autosave_only_on_server` boolean setting in `server-settings.json`.
    pub autosave_only_on_server: i64,
    /// The `non_blocking_saving` boolean setting in `server-settings.json`.
    pub non_blocking_saving: i64,
    /// The `minimum_segment_size` setting in `server-settings.json`.
    pub minimum_segment_size: i64,
    /// The `minimum_segment_size_peer_count` setting in `server-settings.json`.
    pub minimum_segment_size_peer_count: i64,
    /// The `maximum_segment_size` setting in `server-settings.json`.
    pub maximum_segment_size: i64,
    /// The `maximimum_segment_size_peer_count` setting in `server-settings.json`.
    pub maximimum_segment_size_peer_count: i64,
    /// The `--bind` command line parameter's bind address' IP version (either 4 or 6).
    pub bind_address_ip_version: i64,
    /// The `--bind` command line parameter's bind address (four bytes for v4, 16 bytes for v6).
    pub bind_address: Vec<u8>,
    /// The `--bind` command line parameter's bind port.
    pub bind_port: i64,
}
