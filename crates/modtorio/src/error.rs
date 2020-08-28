//! Provides all error types the program uses.

use crate::{
    factorio::{ExecutionStatus, GameStoreId},
    mod_common::Dependency,
    util::HumanVersion,
};
use chrono::{DateTime, Utc};
use std::path::PathBuf;
use thiserror::Error;

/// Represents all types of errors that can occur when interacting with the mod portal.
#[derive(Debug, Error)]
pub enum ModPortalError {
    /// The mod portal responded with an HTTP client error status code.
    #[error("Portal returned client error status {0}")]
    ClientError(reqwest::StatusCode),
    /// The mod portal responded with an HTTP server error status code.
    #[error("Portal returned server error status {0}")]
    ServerError(reqwest::StatusCode),
    /// The mod portal responded with an unexpected HTTP error status code.
    #[error("Portal returned unexpected status {0}")]
    UnexpectedStatus(reqwest::StatusCode),
}

/// Represents all types of errors that can occur when transforming paths.
#[derive(Debug, Error)]
pub enum PathError {
    /// A given path doesn't have a file name when trying to extract the file name.
    #[error("Path doesn't have a filename")]
    NoFilename,
    /// A given path isn't valid Unicode when converting it (or part of it) into a
    /// `String` or a `&str`.
    #[error("Path isn't valid unicode")]
    InvalidUnicode,
}

/// Represents all types of errors that can occur when working with responses.
#[derive(Debug, Error)]
pub enum ResponseError {
    /// A given response's URL doesn't have a file name when trying to extract the
    /// file name.
    #[error("Response URL doesn't have a filename component")]
    NoFilename,
}

/// Represents all types of errors that can occur when working with zip files.
#[derive(Debug, Error)]
pub enum ZipError {
    /// Returned when trying to extract a non-existent file from a given zip archive.
    #[error("Zip file doesn't contain such file: {0}")]
    NoFile(String),
}

/// Represents all types of errors that can occur when working with game mods.
#[derive(Debug, Error)]
pub enum ModError {
    /// Returned when looking up a non-existent mod from a game's mods.
    #[error("No such mod: {0}")]
    NoSuchMod(String),
    /// Returned when there already exists a mod with the same name when loading mods from the
    /// filesystem (likely means there are multiple versions of the same mod).
    #[error("Duplicate mod: {0}")]
    DuplicateMod(String),
    /// Returned when a mod's dependency cannot be ensured (the mod is incompatible with some other
    /// installed mod).
    #[error("Cannot ensure dependency {dependency} of {mod_display}")]
    CannotEnsureDependency {
        /// The dependency that failed.
        dependency: Dependency,
        /// The mod's friendly display.
        mod_display: String,
    },
    /// A game's mod doesn't have its archive zip path set (it likely isn't installed).
    #[error("No zip path set (is the mod installed?)")]
    MissingZipPath,
    /// A game's mod doesn't have its archive zip last mtime set (it likely isn't installed).
    #[error("No zip last mtime set (is the mod installed?)")]
    MissingZipLastMtime,
    /// A portal-added mod doesn't have the mod same name as its corresponding mod archive.
    #[error("Mod name from zip does not match existing name: {zip} vs {existing}")]
    ZipNameMismatch {
        /// The mod name from its zip archive
        zip: String,
        /// The mod's existing name
        existing: String,
    },
    /// Returned when verifying a store-loaded mod whose corresponding zip archive doesn't exist
    /// in the filesystem.
    #[error("Mod zip does not exist in filesystem: {0}")]
    MissingZip(PathBuf),
    /// Returned when:
    ///  * a downloaded mod archive's SHA1 checksum doesn't match its expected checksum from the portal.
    ///  * a mod archive's checksum doesn't match the mod's stored archive checksum.
    #[error("Mod zip's checksum does not match expected: got {zip_checksum}, expected {expected}")]
    ZipChecksumMismatch {
        /// The zip archive's checksum.
        zip_checksum: String,
        /// The expected checksum.
        expected: String,
    },
    /// Returned when verifying a mod zip and its last modified time is later than expected (it was changed on the
    /// filesystem).
    #[error("Mod zip's last mtime later than expected: got {last_mtime}, expected {expected}")]
    ZipLastMtimeMismatch {
        /// The zip archive's last modified time.
        last_mtime: DateTime<Utc>,
        /// The expected last modified time.
        expected: DateTime<Utc>,
    },
    /// Returned when trying to load an unstored mod from the store.
    #[error("Mod not in store")]
    ModNotInStore,
    // TODO: separate this error into not-installed and not-fetched-from-portal
    /// A mod doesn't contain all info when expected, i.e. it hasn't been populated
    /// from the portal or the zip archive.
    #[error("Missing info (is the mod installed?)")]
    MissingInfo,
    /// Returned when searching for a non-existent mod release.
    #[error("No such release version: {0}")]
    NoSuchRelease(HumanVersion),
    /// A mod doesn't have any releases when searching for a certain release.
    #[error("No releases")]
    NoReleases,
    /// The mod portal's mod info response doesn't contain a critical field.
    #[error("Missing critical field in mod portal response: {0}")]
    MissingField(&'static str),
    /// A given info object's mod name doesn't match the mod where its being applied to.
    #[error("Mod name mismatch when applying info object. Own: {own}, given: {given}")]
    NameMismatch {
        /// The mod's name.
        own: String,
        /// The name in the info object.
        given: String,
    },
}

/// Represents all types of errors that can occur when parsing dependency strings.
#[derive(Debug, Error)]
pub enum DependencyParsingError {
    /// The dependency parser regex didn't capture anything from a given string.
    #[error("Regex returned no captures for dependency string: {0}")]
    NoRegexCaptures(String),
    /// The dependency parser regex didn't capture the dependency name from a given string.
    #[error("Regex did not capture name for dependency string: {0}")]
    NameNotCaptured(String),
    /// A given dependency string has an invalid requirement portion.
    #[error("Invalid requirement string: {0}")]
    InvalidRequirementString(String),
    /// A given dependency string has an invalid version requirement portion.
    #[error("Invalid version requirement string")]
    InvalidVersionRequirementString(#[from] HumanVersionError),
}

/// Represents all types of errors that can occur when parsing [`HumanVersion`s][HumanVersion].
///
/// [HumanVersion]: crate::util::HumanVersion
#[derive(Debug, Error)]
pub enum HumanVersionError {
    /// A component of a given version string isn't an integer.
    #[error(transparent)]
    ParsingError(#[from] std::num::ParseIntError),
    /// A critical component of a given version string is missing.
    #[error("Missing component")]
    MissingComponent,
    /// The version parser regex didn't capture anything from a given version requirement
    /// string.
    #[error("Regex returned no captures for version requirement string: {0}")]
    NoRegexCaptures(String),
    /// A given version dependency string doesn't contain the version comparator component.
    #[error("Missing version comparator in requirement string: {0}")]
    MissingComparator(String),
    /// A given version dependency string doesn't contain the version component.
    #[error("Missing version in requirement string: {0}")]
    MissingVersion(String),
}

/// Represents all types of errors that can occur when interacting with the [`program
/// store`](crate::store::Store);
#[derive(Debug, Error)]
pub enum StoreError {
    /// Returned when loading the program store database file and it has insufficient permissions.
    #[error("Insufficient store file permissions ({path}): maximum {maximum:o}, actual {actual:o}")]
    InsufficientFilePermissions {
        /// Path to the database file.
        path: String,
        /// The maximum required permissions.
        maximum: u32,
        /// The database file's actual permissions.
        actual: u32,
    },
}

/// Represesnts all types of errors that correspond to invalid configuration.
#[derive(Debug, Error)]
pub enum ConfigError {
    /// Returned when trying to run a Modtorio instance which has no RPC listen addresses specified
    /// in its config.
    #[error("No listen addresses specified")]
    NoListenAddresses,
}

/// Represents all types of errors that can occur in RPC calls.
#[derive(Debug, Error)]
pub enum RpcError {
    /// Returned when trying to interact with a non-existent game.
    #[error("No such game ID: {0}")]
    NoSuchServer(GameStoreId),
    /// Returned when trying to import a Factorio server instance which is already managed by the Modtorio instance.
    #[error("A game in the root directory '{0}' is already managed by this Modtorio instance")]
    GameAlreadyExists(PathBuf),
    /// Returned when trying to install a non-existent mod.
    #[error("No such mod: {0}")]
    NoSuchMod(String),
    /// Returned when asserting the Modtorio instance's status fails.
    #[error("Instance status assertion failed: wanted {wanted:?}, actual {actual:?}")]
    InvalidInstanceStatus {
        /// The wanted instance status.
        wanted: rpc::instance_status::Status,
        /// The actual instance status.
        actual: rpc::instance_status::Status,
    },
    /// Returned when trying to run an invalid command.
    #[error("No such command identifier: {0}")]
    NoSuchCommand(i32),
    /// Returned when the RPC request is missing a required field.
    #[error("Missing argument")]
    MissingArgument,
    /// Returned when an unknown or internal error occurred.
    #[error("An internal error occurred: {0}")]
    Internal(#[from] anyhow::Error),
}

impl From<RpcError> for tonic::Status {
    fn from(e: RpcError) -> Self {
        tonic::Status::from(&e)
    }
}

impl From<&RpcError> for tonic::Status {
    fn from(e: &RpcError) -> Self {
        match e {
            RpcError::Internal(int) => tonic::Status::internal(int.to_string()),
            RpcError::NoSuchMod(_)
            | RpcError::NoSuchServer(_)
            | RpcError::NoSuchCommand(_)
            | RpcError::MissingArgument => tonic::Status::invalid_argument(e.to_string()),
            RpcError::GameAlreadyExists(_) => tonic::Status::already_exists(e.to_string()),
            RpcError::InvalidInstanceStatus { .. } => tonic::Status::failed_precondition(e.to_string()),
        }
    }
}

/// Represents all types of errors that can occur when interacting with the Factorio server's executable.
#[derive(Debug, Error)]
pub enum ExecutableError {
    /// Returned when verifying a given executable isn't a valid Factorio server executable.
    #[error("The executable in '{path}' is not a valid Factorio executable: {source}")]
    InvalidExecutable {
        /// The path to the executable.
        path: PathBuf,
        /// The source for this error.
        #[source]
        source: anyhow::Error,
    },
    /// Returned when an executable terminated unsuccesfully (terminated by signal or returned a non-zero exit code).
    #[error("The executable terminated unsuccesfully: {exit_code:?}\nstdout: {stdout:?}\nstderr: {stderr:?}")]
    Unsuccesfull {
        /// The executable's exit code, if any.
        exit_code: Option<i32>,
        /// The executable's full standard output.
        stdout: Option<String>,
        /// The executable's full standard error.
        stderr: Option<String>,
    },
    /// Returned when trying to parse a version information string fails.
    #[error("Version information string failed to parse ({ver_str}): {source}")]
    InvalidVersionInformation {
        /// The invalid version information string.
        ver_str: String,
        /// The source for this error.
        #[source]
        source: anyhow::Error,
    },
    /// Returned when spawning a child process and trying to acquire its non-existent stdio handle.
    #[error("Child process did not have an stdio handle")]
    NoStdioHandle,
}

/// Represents all types of errors that can occur when loading or saving the server's settings.
#[derive(Debug, Error)]
pub enum SettingsError {
    /// A field has an unexpected value.
    #[error("Unexpected value in settings: {0}")]
    UnexpectedValue(String),
}

/// Represents all types of errors that can occur when using the update batcher.
#[derive(Debug, Error)]
pub enum UpdateBatcherError {
    /// The mod portal returned a mod with a name that wasn't originally requested for.
    #[error("Mod portal returned an unknown mod name: {0}")]
    UnknownModName(String),
}

/// Represents all types of errors that can occur with a Factorio server.
#[derive(Debug, Error)]
pub enum ServerError {
    /// Returned when trying to change a server's state and its current status is invalid given the change.
    #[error("Invalid game status: {0:?}")]
    InvalidGameStatus(ExecutionStatus),
    /// Returned when trying to access a server's store ID when it hasn't been added to the store yet.
    #[error("The game hasn't been added to the program store yet")]
    GameNotInStore,
    /// Returned when trying to add a new player with the same username as an already existing player.
    #[error("A player with the username '{0}' already exists in the server")]
    PlayerAlreadyExists(String),
    /// Returned when trying to access a player when no such player with the given username or peer ID exists.
    #[error("No player with the username or peer ID '{0}' exists")]
    NoSuchPlayer(String),
    /// Returned when updating a joining player and the last modified index does not point to a player.
    #[error("No last modified player (last modified index: {0}, player count: {1}). This is likely an internal bug.")]
    NoLastModifiedPlayer(usize, usize),
}

/// Represents the parser error for `GameEvent`.
#[derive(Debug, Error)]
pub enum GameEventError {
    /// Returned when a given string didn't match any `GameEvent` parser.
    #[error("The line '{0}' doesn't have a matching parser implemented")]
    NoParser(String),
    /// Returned when a given string failed to be parsed into a `GameEvent`.
    #[error("The line '{0}' failed to be parsed into a GameEvent")]
    FailedToParse(String),
}
