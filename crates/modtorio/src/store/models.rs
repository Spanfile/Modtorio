//! Store models used to map rows in the store database tables to/from structs.
//!
//! Each model struct derives [`Model`], which provides functions to to build SQL queries
//! and parameters from them.
//!
//! [Model]: derive::Model

use crate::{
    factorio::GameStoreId,
    mod_common::Requirement,
    util::{HumanVersion, HumanVersionReq},
};
use chrono::{DateTime, Utc};
use derive::Model;

/// An instance of a Factorio game.
///
/// Uses the [`id`](#structfield.factorio_mod) field as index when querying the store database.
#[derive(Debug, PartialEq, Model)]
pub struct Game {
    /// The game's store ID.
    #[index]
    #[ignore_in_all_params]
    pub id: GameStoreId,
    /// The game's root directory's path.
    pub path: String,
}

/// An instance of a Factorio mod.
///
/// Uses the [`name`](#structfield.name) field as index when querying the store database.
#[derive(Debug, PartialEq, Model)]
pub struct FactorioMod {
    /// The mod's name.
    #[index]
    pub name: String,
    /// The mod's author.
    pub author: String,
    /// The mod author's optional contact information.
    pub contact: Option<String>,
    /// The mod's or its author's optional homepage.
    pub homepage: Option<String>,
    /// The mod's title.
    pub title: String,
    /// The mod's optional summary.
    pub summary: Option<String>,
    /// The mod's description.
    pub description: String,
    /// The mod's optional changelog.
    pub changelog: Option<String>,
    /// The timestamp when this mod was last updated in the store.
    pub last_updated: DateTime<Utc>,
}

/// A mapping of an instance of a Factorio game to an instance of a Factorio mod. Represents a
/// many-to-many relationship from [`Game`] to [`FactorioMod`].
///
/// Uses the [`game`](#structfield.game) field as an index when querying the store database.
///
/// [Game]: Game
/// [FactorioMod]: FactorioMod
#[derive(Debug, PartialEq, Model)]
pub struct GameMod {
    /// The game's store ID. Corresponds to the [id][Game#structfield.id] field of a [Game].
    ///
    /// [Game]: super::Game
    #[index]
    pub game: GameStoreId,
    /// The mod's name. Corresponds to the [`name`][FactorioMod#structfield.name] field of a
    /// [`FactorioMod`].
    ///
    /// [FactorioMod]: super::FactorioMod
    pub factorio_mod: String,
    /// The mod's release version. Corresponds to the [`version`][ModRelease#structfield.version]
    /// field of a [`ModRelease`].
    ///
    /// [ModRelease]: super::ModRelease
    pub mod_version: HumanVersion,
    /// The filesystem path of the mod's zip archive.
    pub mod_zip: String,
    /// The the mod's zip archive last modified time.
    pub zip_last_mtime: DateTime<Utc>,
}

/// An instance of a [`FactorioMod`'s](super::FactorioMod) release.
///
/// Uses the [`factorio_mod`](#structfield.factorio_mod) and [`version`](#structfield.version)
/// fields as indices when querying the store database.
#[derive(Debug, PartialEq, Model)]
pub struct ModRelease {
    /// The mod's name. Corresponds to the [`name`][FactorioMod#structfield.name] field of a
    /// [`FactorioMod`].
    ///
    /// [`FactorioMod`]: super::FactorioMod
    #[index]
    pub factorio_mod: String,
    /// The release's version.
    pub version: HumanVersion,
    /// The release's download URL.
    pub download_url: String,
    /// The timestamp when the mod was released.
    pub released_on: DateTime<Utc>,
    /// The mod zip archive's SHA1 checksum.
    pub sha1: String,
    /// The Factorio version this release is for.
    pub factorio_version: HumanVersion,
}

/// A [`ModRelease`'s](crate::store::models::ModRelease) dependency to another mod.
///
/// Uses the [`release_mod_name`](#structfield.release_mod_name) and
/// [`release_version`](#structfield.release_version) fields as indices when querying the store
/// database.
#[derive(Debug, PartialEq, Model)]
pub struct ReleaseDependency {
    /// The mod's name this dependency is of. Corresponds to the
    /// [`name`][FactorioMod#structfield.name] field of a [`FactorioMod`].
    ///
    /// [FactorioMod]: super::FactorioMod
    #[index]
    pub release_mod_name: String,
    /// The version of the release this dependency is of. Corresponds to the
    /// [`version`][ModRelease#structfield.version] of a [`ModRelease`].
    ///
    /// [ModRelease]: super::ModRelease
    #[index]
    pub release_version: HumanVersion,
    /// The name of the dependent mod.
    pub name: String,
    /// The requirement to this dependent mod.
    pub requirement: Requirement,
    /// The optional version requirement to this dependent mod.
    pub version_req: Option<HumanVersionReq>,
}

/// A Factorio server's settings, including both settings from its `server-settings.json` file and its command line
/// parameters.
#[derive(Debug, Model, Default)]
pub struct GameSettings {
    /// The game's ID these settings are for.
    #[index]
    pub game: GameStoreId,
    /// The `name` setting in `server-settings.json`.
    pub name: String,
    /// The `description` setting in `server-settings.json`.
    pub description: String,
    /// The `tags` setting in `server-settings.json`, combined into a single string.
    pub tags: String,
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
    /// The `maximum_segment_size_peer_count` setting in `server-settings.json`.
    pub maximum_segment_size_peer_count: i64,
    /// The `--bind` command line parameter's bind address' IP version (either 4 or 6).
    pub bind_address_ip_version: i64,
    /// The `--bind` command line parameter's bind address (four bytes for v4, 16 bytes for v6).
    pub bind_address: Vec<u8>,
    /// The `--bind` command line parameter's bind port.
    pub bind_port: i64,
}
