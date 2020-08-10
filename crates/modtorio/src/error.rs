//! Provides all error types the program uses.

use crate::{mod_common::Dependency, util::HumanVersion};
use std::path::PathBuf;
use thiserror::Error;

/// Represents all types of errors that can occur when interacting with the mod portal.
#[derive(Debug, Error)]
pub enum ModPortalError {
    /// The mod portal responded with an HTTP error status code.
    #[error("Portal returned status {0}")]
    ErrorStatus(reqwest::StatusCode),
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
    /// A portal-added mod doesn't have the mod same name as its corresponding mod archive.
    #[error("Mod name from zip does not match existing name: {zip} vs {existing}")]
    ZipNameMismatch {
        /// The mod name from its zip archive
        zip: String,
        /// The mod's existing name
        existing: String,
    },
    /// Returned when verifying a cached-loaded mod whose corresponding zip archive doesn't exist
    /// in the filesystem.
    #[error("Mod zip does not exist in filesystem: {0}")]
    MissingZip(PathBuf),
    /// Returned when:
    ///  * a downloaded mod archive's SHA1 checksum doesn't match its expected checksum from the
    ///    portal.
    ///  * a mod archive's checksum doesn't match the mod's cached archive checksum.
    #[error("Mod zip's checksum does not match expected: got {zip_checksum}, expected {expected}")]
    ZipChecksumMismatch {
        /// The zip archive's checksum.
        zip_checksum: String,
        /// The expected checksum.
        expected: String,
    },
    /// Returned when trying to load an uncached mod from the cache.
    #[error("Mod not in cache")]
    ModNotInCache,
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
    #[error("Missing critical field: {0}")]
    MissingField(&'static str),
}

/// Represents all types of errors that can occur when parsing dependency strings
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
    #[error(
        "Insufficient store file permissions ({path}): maximum {maximum:o}, actual {actual:o}"
    )]
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
    #[error("No listen addresses specified.")]
    NoListenAddresses,
}
