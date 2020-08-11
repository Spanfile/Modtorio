//! Because not all versions are exactly semver. Humans can think of so many more credible formats
//! that look like semver but aren't. Provides [`HumanVersion`](HumanVersion) which represents a
//! single version (for example `1.0.0`) and [`HumanVersionReq`](HumanVersionReq) which represents a
//! version requirement (for example `>= 1.0.0`).

use crate::error::HumanVersionError;
use lazy_static::lazy_static;
use regex::Regex;
use rusqlite::{
    types::{self, ToSqlOutput},
    ToSql,
};
use serde::{de, de::Visitor, Deserialize};
use std::{fmt, fmt::Display, str::FromStr};
use types::{FromSql, FromSqlError, FromSqlResult, Value, ValueRef};

/// A version comparator.
#[derive(Debug, PartialEq, Copy, Clone)]
pub enum Comparator {
    /// The greater-or-equal `>=` comparator.
    GreaterOrEqual,
    /// The greater `>` comparator.
    Greater,
    /// The equal `=` comparator.
    Equal,
    /// The less `<` comparator.
    Less,
    /// The less-or-equal `<=` comparator.
    LessOrEqual,
}

/// A human-friendly semver-like version. Acts like semver, but is a lot more lenient on the exact
/// format of the version string.
///
/// Consists of three components, [`major`](#structfield.major), [`minor`](#structfield.minor) and
/// [`patch`](#structfield.patch). Each is a 64-bit unsigned integer.
///
/// # Parsing from a string
///
/// A `HumanVersion` can be parsed from a string in the form of `major.minor.patch`. The following
/// restrictions and allowances apply:
/// * The `major` component is required.
/// * The `minor` and `patch` components are optional. If they're missing, they're defaulted to `0`.
/// * Each component has to be in the range of `0` to [`u64::MAX`](std::u64::MAX).
/// * Each component may have an indefinite amount of leading zeroes.
///
/// Examples of valid human version strings:
/// * `1.0.0`
/// * `1.0`
/// * `1`
/// * `01.02.03`
#[derive(Debug, PartialEq, PartialOrd, Clone, Copy)]
pub struct HumanVersion {
    /// The major component.
    pub major: u64,
    /// The minor component.
    pub minor: u64,
    /// The patch component.
    pub patch: u64,
}

/// A human-friendly version requirement.
///
/// Consists of a `HumanVersion` component and a `Comparator`.
///
/// # Parsing from a string
///
/// A `HumanVersionReq` can be parsed from a string in the form of `comparator version`. The
/// following restrictions and allowances apply:
/// * The `comparator` is one of the possible [`Comparator`s](super::Comparator).
/// * The `version` is a valid [`HumanVersion`](super::HumanVersion).
/// * There may be zero or more white spaces between the `comparator` and the `version` components.
///
/// Examples of valid human version requirement strings:
/// * `>= 1.0.0`
/// * `< 1.0`
#[derive(Debug, PartialEq, Copy, Clone)]
pub struct HumanVersionReq {
    /// The version comparator.
    pub comparator: Comparator,
    /// The version to compare against.
    pub version: HumanVersion,
}

impl HumanVersion {
    /// Checks whether this version meets a `HumanVersionReq` requirement.
    pub fn meets(self, requirement: HumanVersionReq) -> bool {
        match requirement.comparator {
            Comparator::GreaterOrEqual => self >= requirement.version,
            Comparator::Greater => self > requirement.version,
            Comparator::Equal => self == requirement.version,
            Comparator::Less => self < requirement.version,
            Comparator::LessOrEqual => self <= requirement.version,
        }
    }
}

impl FromStr for HumanVersion {
    type Err = HumanVersionError;

    fn from_str(s: &str) -> Result<Self, Self::Err> {
        let args: Vec<&str> = s.split('.').collect();

        let major = args.get(0).ok_or(HumanVersionError::MissingComponent)?.parse::<u64>()?;
        let minor = args.get(1).map_or(Ok(0), |c| c.parse::<u64>())?;
        let patch = args.get(2).map_or(Ok(0), |c| c.parse::<u64>())?;

        Ok(Self { major, minor, patch })
    }
}

impl Display for HumanVersion {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.write_fmt(format_args!("{}.{}.{}", self.major, self.minor, self.patch))
    }
}

impl Display for Comparator {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Comparator::GreaterOrEqual => f.write_str(">="),
            Comparator::Greater => f.write_str(">"),
            Comparator::Equal => f.write_str("=="),
            Comparator::Less => f.write_str("<"),
            Comparator::LessOrEqual => f.write_str("<="),
        }
    }
}

impl Display for HumanVersionReq {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.write_fmt(format_args!("{} {}", self.comparator, self.version))
    }
}

impl FromStr for HumanVersionReq {
    type Err = HumanVersionError;

    fn from_str(s: &str) -> Result<Self, Self::Err> {
        lazy_static! {
            static ref RE: Regex = Regex::new(r"(>=|<=|>|=|<) ?(.*)").unwrap();
        }

        let captures = RE
            .captures(s)
            .ok_or_else(|| HumanVersionError::NoRegexCaptures(s.to_owned()))?;

        let comparator = match captures.get(1).map(|c| c.as_str()) {
            Some(">=") => Comparator::GreaterOrEqual,
            Some(">") => Comparator::Greater,
            Some("=") => Comparator::Equal,
            Some("<") => Comparator::Less,
            Some("<=") => Comparator::LessOrEqual,
            Some(c) => panic!("impossible case (regex returned {})", c),
            None => return Err(HumanVersionError::MissingComparator(s.to_owned())),
        };

        let version = captures
            .get(2)
            .ok_or_else(|| HumanVersionError::MissingVersion(s.to_owned()))?
            .as_str()
            .parse()?;

        Ok(Self { comparator, version })
    }
}

impl ToSql for HumanVersion {
    fn to_sql(&self) -> rusqlite::Result<ToSqlOutput<'_>> {
        Ok(ToSqlOutput::Owned(Value::Text(self.to_string())))
    }
}

impl FromSql for HumanVersion {
    fn column_result(value: ValueRef<'_>) -> FromSqlResult<Self> {
        match HumanVersion::from_str(value.as_str()?) {
            Ok(v) => Ok(v),
            Err(_) => Err(FromSqlError::InvalidType), // TODO: bad error type?
        }
    }
}

impl ToSql for HumanVersionReq {
    fn to_sql(&self) -> rusqlite::Result<ToSqlOutput<'_>> {
        Ok(ToSqlOutput::Owned(Value::Text(self.to_string())))
    }
}

impl FromSql for HumanVersionReq {
    fn column_result(value: ValueRef<'_>) -> FromSqlResult<Self> {
        match HumanVersionReq::from_str(value.as_str()?) {
            Ok(v) => Ok(v),
            Err(_) => Err(FromSqlError::InvalidType), // TODO: bad error type?
        }
    }
}

impl From<rpc::Version> for HumanVersion {
    fn from(rpc_version: rpc::Version) -> Self {
        Self {
            major: rpc_version.major,
            minor: rpc_version.minor,
            patch: rpc_version.patch,
        }
    }
}

impl Into<rpc::Version> for HumanVersion {
    fn into(self) -> rpc::Version {
        rpc::Version {
            major: self.major,
            minor: self.minor,
            patch: self.patch,
        }
    }
}

impl<'de> Deserialize<'de> for HumanVersion {
    fn deserialize<D>(deserializer: D) -> Result<Self, D::Error>
    where
        D: serde::Deserializer<'de>,
    {
        #[allow(clippy::missing_docs_in_private_items)]
        struct HumanVersionVisitor;

        impl<'de> Visitor<'de> for HumanVersionVisitor {
            type Value = HumanVersion;

            fn expecting(&self, formatter: &mut fmt::Formatter) -> fmt::Result {
                formatter.write_str("version string")
            }

            fn visit_str<E>(self, v: &str) -> Result<Self::Value, E>
            where
                E: de::Error,
            {
                v.parse::<Self::Value>()
                    .map_err(|_| de::Error::invalid_value(de::Unexpected::Str(v), &self))
            }
        }

        deserializer.deserialize_str(HumanVersionVisitor)
    }
}

impl<'de> Deserialize<'de> for HumanVersionReq {
    fn deserialize<D>(deserializer: D) -> Result<Self, D::Error>
    where
        D: serde::Deserializer<'de>,
    {
        #[allow(clippy::missing_docs_in_private_items)]
        struct HumanVersionReqVisitor;

        impl<'de> Visitor<'de> for HumanVersionReqVisitor {
            type Value = HumanVersionReq;

            fn expecting(&self, formatter: &mut fmt::Formatter) -> fmt::Result {
                formatter.write_str("version requirement string")
            }

            fn visit_str<E>(self, v: &str) -> Result<Self::Value, E>
            where
                E: de::Error,
            {
                v.parse::<Self::Value>()
                    .map_err(|_| de::Error::invalid_value(de::Unexpected::Str(v), &self))
            }
        }

        deserializer.deserialize_str(HumanVersionReqVisitor)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn parse_version() -> anyhow::Result<()> {
        assert_eq!(
            "1.2.3".parse::<HumanVersion>()?,
            HumanVersion {
                major: 1,
                minor: 2,
                patch: 3
            }
        );

        assert_eq!(
            "0.1.2".parse::<HumanVersion>()?,
            HumanVersion {
                major: 0,
                minor: 1,
                patch: 2
            }
        );

        assert_eq!(
            "1.2".parse::<HumanVersion>()?,
            HumanVersion {
                major: 1,
                minor: 2,
                patch: 0
            }
        );

        assert_eq!(
            "1".parse::<HumanVersion>()?,
            HumanVersion {
                major: 1,
                minor: 0,
                patch: 0
            }
        );

        assert_eq!(
            "01.00.00".parse::<HumanVersion>()?,
            HumanVersion {
                major: 1,
                minor: 0,
                patch: 0
            }
        );

        Ok(())
    }

    #[test]
    fn parse_version_req() -> anyhow::Result<()> {
        assert_eq!(
            ">= 1.0.0".parse::<HumanVersionReq>()?,
            HumanVersionReq {
                comparator: Comparator::GreaterOrEqual,
                version: HumanVersion {
                    major: 1,
                    minor: 0,
                    patch: 0,
                }
            }
        );

        assert_eq!(
            "> 1.0.0".parse::<HumanVersionReq>()?,
            HumanVersionReq {
                comparator: Comparator::Greater,
                version: HumanVersion {
                    major: 1,
                    minor: 0,
                    patch: 0,
                }
            }
        );

        assert_eq!(
            "= 1.0.0".parse::<HumanVersionReq>()?,
            HumanVersionReq {
                comparator: Comparator::Equal,
                version: HumanVersion {
                    major: 1,
                    minor: 0,
                    patch: 0,
                }
            }
        );

        assert_eq!(
            "< 1.0.0".parse::<HumanVersionReq>()?,
            HumanVersionReq {
                comparator: Comparator::Less,
                version: HumanVersion {
                    major: 1,
                    minor: 0,
                    patch: 0,
                }
            }
        );

        assert_eq!(
            "<= 1.0.0".parse::<HumanVersionReq>()?,
            HumanVersionReq {
                comparator: Comparator::LessOrEqual,
                version: HumanVersion {
                    major: 1,
                    minor: 0,
                    patch: 0,
                }
            }
        );

        Ok(())
    }

    #[test]
    fn compare_version() -> anyhow::Result<()> {
        assert!("1.0.0".parse::<HumanVersion>()? < "2.0.0".parse::<HumanVersion>()?);
        assert!("1.0.0".parse::<HumanVersion>()? < "1.1.0".parse::<HumanVersion>()?);
        assert!("1.0.0".parse::<HumanVersion>()? < "1.0.1".parse::<HumanVersion>()?);

        assert!("1.0.0".parse::<HumanVersion>()? > "0.1.0".parse::<HumanVersion>()?);
        assert!("1.0.0".parse::<HumanVersion>()? > "0.0.1".parse::<HumanVersion>()?);
        assert!("1.0.0".parse::<HumanVersion>()? == "1.0.0".parse::<HumanVersion>()?);

        Ok(())
    }
}
