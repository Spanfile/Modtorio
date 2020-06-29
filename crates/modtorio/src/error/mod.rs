use crate::{mod_common::Dependency, util::HumanVersion};
use thiserror::Error;

#[derive(Debug, Error)]
pub enum ModPortalError {
    #[error("Download returned status {0}")]
    ErrorStatus(reqwest::StatusCode),
}

#[derive(Debug, Error)]
pub enum PathError {
    #[error("Path doesn't have a filename")]
    NoFilename,
    #[error("Path isn't valid unicode")]
    InvalidUnicode,
}

#[derive(Debug, Error)]
pub enum ResponseError {
    #[error("Response URL doesn't have a filename component")]
    NoFilename,
}

#[derive(Debug, Error)]
pub enum ZipError<'a> {
    #[error("Zip file doesn't contain such file: {0}")]
    NoFile(&'a str),
}

#[derive(Debug, Error)]
pub enum ModError {
    #[error("No such mod: {0}")]
    NoSuchMod(String),
    #[error("Cannot ensure dependency {dependency} of {mod_display}")]
    CannotEnsureDependency {
        dependency: Dependency,
        mod_display: String,
    },
    #[error("No zip path set")]
    MissingZipPath,
    #[error("Mod zip name does not match existing name: {zip} vs {existing}")]
    ZipNameMismatch { zip: String, existing: String },
    #[error("Mod not in cache")]
    ModNotInCache,
    #[error("Missing info (ist the mod installed?)")]
    MissingInfo,
    #[error("No such release version: {0}")]
    NoSuchRelease(HumanVersion),
    #[error("No releases")]
    NoReleases,
}

#[derive(Debug, Error)]
pub enum DependencyParsingError {
    #[error("Regex returned no captures for dependency string: {0}")]
    NoRegexCaptures(String),
    #[error("Regex did not capture name for dependency string: {0}")]
    NameNotCaptured(String),
    #[error("Invalid requirement string: {0}")]
    InvalidRequirementString(String),
    #[error("Invalid version requirement string")]
    InvalidVersionRequirementString(#[from] HumanVersionError),
}

#[derive(Debug, Error)]
pub enum HumanVersionError {
    #[error(transparent)]
    ParsingError(#[from] std::num::ParseIntError),
    #[error("Missing component")]
    MissingComponent,
    #[error("Regex returned no captures for version requirement string: {0}")]
    NoRegexCaptures(String),
    #[error("Missing version comparator in requirement string: {0}")]
    MissingComparator(String),
    #[error("Missing version in requirement string: {0}")]
    MissingVersion(String),
}
