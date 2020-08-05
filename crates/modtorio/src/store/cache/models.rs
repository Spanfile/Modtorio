//! Cache models used to map rows in the cache database tables to/from structs.
//!
//! Each model struct derives [`Model`], which provides functions to to build SQL queries
//! and parameters from them.
//!
//! [Model]: derive::Model

use crate::{
    factorio::GameCacheId,
    mod_common::Requirement,
    util::{HumanVersion, HumanVersionReq},
};
use chrono::{DateTime, Utc};
use derive::Model;

/// An instance of a Factorio game.
///
/// Uses the [`id`](#structfield.factorio_mod) field as index when querying the cache database.
#[derive(Debug, PartialEq, Model)]
pub struct Game {
    /// The game's cache ID.
    #[index]
    #[ignore_in_all_params]
    pub id: GameCacheId,
    /// The game's root directory's path.
    pub path: String,
}

/// An instance of a Factorio mod.
///
/// Uses the [`name`](#structfield.name) field as index when querying the cache database.
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
    /// The timestamp when this mod was last updated in the cache.
    pub last_updated: DateTime<Utc>,
}

/// A mapping of an instance of a Factorio game to an instance of a Factorio mod. Represents a
/// many-to-many relationship from [`Game`] to [`FactorioMod`].
///
/// Uses the [`game`](#structfield.game) field as an index when querying the cache database.
///
/// [Game]: Game
/// [FactorioMod]: FactorioMod
#[derive(Debug, PartialEq, Model)]
pub struct GameMod {
    /// The game's cache ID. Corresponds to the [id][Game#structfield.id] field of a [Game].
    ///
    /// [Game]: super::Game
    #[index]
    pub game: GameCacheId,
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
    /// The checksum of the mod's zip archive.
    pub zip_checksum: String,
}

/// An instance of a [`FactorioMod`'s](super::FactorioMod) release.
///
/// Uses the [`factorio_mod`](#structfield.factorio_mod) and [`version`](#structfield.version)
/// fields as indices when querying the cache database.
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

/// A [`ModRelease`'s](crate::cache::models::ModRelease) dependency to another mod.
///
/// Uses the [`release_mod_name`](#structfield.release_mod_name) and
/// [`release_version`](#structfield.release_version) fields as indices when querying the cache
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
