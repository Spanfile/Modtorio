//! Provides the [`Dependency`](Dependency) object which is used to model a [`Mod`](super::Mod)'s
//! depdendency on another mod.

use crate::{error::DependencyParsingError, store::models, util::HumanVersionReq};
use lazy_static::lazy_static;
use regex::Regex;
use rusqlite::{
    types::{FromSql, FromSqlError, FromSqlResult, ToSqlOutput, Value, ValueRef},
    ToSql,
};
use serde::{de, de::Visitor, Deserialize};
use std::{fmt, str::FromStr};

#[doc(hidden)]
const DEPENDENCY_PARSER_REGEX: &str = r"(\?|!|\(\?\))? ?([^>=<]+)( ?[>=<]{1,2} ?[\d\.]*)?$";

/// A dependency requirement level.
#[derive(Debug, PartialEq, Copy, Clone)]
pub enum Requirement {
    /// The mandatory requirement (an empty string; no requirement specified).
    Mandatory = 0,
    /// The optional requirement `?`
    Optional,
    /// The optional and hidden requirement `(?)`
    OptionalHidden,
    /// The incompatible requirement `!`
    Incompatible,
}

/// A [`Mod`'s](super::Mod) dependency on another.
///
/// Consists of a requirement, the dependent mod's name and its optional version requirement.
///
/// # Parsing from a string
///
/// A `Dependency` can be parsed from a string in the form of `requirement name
/// version-requirement`. The following restrictions and allowances apply:
/// * The `requirement` component can be one of `?` (optional), `(?)` (optional hidden), `!` (incompatible) or empty, in
///   which case it is assumed to be mandatory.
/// * The `name` component is required.
/// * The `version-requirement` is optional. It is a [`HumanVersionReq`](crate::util::HumanVersionReq).
///
/// Examples of valid dependency strings:
/// * A mandatory dependency: `cool-mod >= 1.0.0`.
/// * An optional dependency without a specific version requirement: `?cool-mod`.
/// * An incompatible mod without a specific version requirement: `!evil-mod`.
#[derive(Debug, PartialEq, Clone)]
pub struct Dependency {
    /// The dependeny's requirement.
    requirement: Requirement,
    /// The name of the dependent mod.
    name: String,
    /// The optional version requirement to the dependent mod.
    version: Option<HumanVersionReq>,
}

impl Dependency {
    /// Returns the dependent mod's name.
    pub fn name(&self) -> &str {
        &self.name
    }

    /// Returns the dependency requirement.
    pub fn requirement(&self) -> Requirement {
        self.requirement
    }

    /// Returns the dependent mod's version, if any.
    pub fn version(&self) -> Option<HumanVersionReq> {
        self.version
    }
}

impl FromStr for Dependency {
    type Err = DependencyParsingError;

    fn from_str(s: &str) -> Result<Self, Self::Err> {
        lazy_static! {
            static ref RE: Regex = Regex::new(DEPENDENCY_PARSER_REGEX).unwrap();
        }

        let captures = RE
            .captures(s)
            .ok_or_else(|| DependencyParsingError::NoRegexCaptures(s.to_owned()))?;

        let requirement = captures
            .get(1)
            .map(|c| c.as_str())
            .map_or(Ok(Requirement::Mandatory), str::parse)?;

        let name = captures
            .get(2)
            .ok_or_else(|| DependencyParsingError::NameNotCaptured(s.to_owned()))?
            .as_str()
            .trim()
            .to_string();

        let version = captures
            .get(3)
            .map_or(Ok(None), |c| c.as_str().parse::<HumanVersionReq>().map(Some))?;

        Ok(Dependency {
            requirement,
            name,
            version,
        })
    }
}

impl FromStr for Requirement {
    type Err = DependencyParsingError;

    fn from_str(s: &str) -> Result<Self, Self::Err> {
        match s {
            "?" => Ok(Requirement::Optional),
            "!" => Ok(Requirement::Incompatible),
            "(?)" => Ok(Requirement::OptionalHidden),
            "" => Ok(Requirement::Mandatory),
            _ => Err(DependencyParsingError::InvalidRequirementString(s.to_owned())),
        }
    }
}

impl fmt::Display for Requirement {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Requirement::Optional => f.write_str("?"),
            Requirement::Incompatible => f.write_str("!"),
            Requirement::OptionalHidden => f.write_str("(?)"),
            Requirement::Mandatory => f.write_str(""),
        }
    }
}

impl fmt::Display for Dependency {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.write_fmt(format_args!("{} {} ", self.requirement, &self.name))?;
        if let Some(version) = self.version {
            f.write_fmt(format_args!("{}", version))?;
        }

        Ok(())
    }
}

impl ToSql for Requirement {
    fn to_sql(&self) -> rusqlite::Result<ToSqlOutput<'_>> {
        Ok(ToSqlOutput::Owned(Value::Text(self.to_string())))
    }
}

impl FromSql for Requirement {
    fn column_result(value: ValueRef<'_>) -> FromSqlResult<Self> {
        match Requirement::from_str(value.as_str()?) {
            Ok(v) => Ok(v),
            Err(_) => Err(FromSqlError::InvalidType), // TODO: bad error type?
        }
    }
}

impl From<models::ReleaseDependency> for Dependency {
    fn from(dep: models::ReleaseDependency) -> Self {
        Self {
            requirement: dep.requirement,
            name: dep.name,
            version: dep.version_req,
        }
    }
}

impl<'de> Deserialize<'de> for Dependency {
    fn deserialize<D>(deserializer: D) -> Result<Self, D::Error>
    where
        D: serde::Deserializer<'de>,
    {
        #[allow(clippy::missing_docs_in_private_items)]
        struct DependencyVisitor;

        impl<'de> Visitor<'de> for DependencyVisitor {
            type Value = Dependency;

            fn expecting(&self, formatter: &mut fmt::Formatter) -> fmt::Result {
                formatter.write_str("dependency string")
            }

            fn visit_str<E>(self, v: &str) -> Result<Self::Value, E>
            where
                E: de::Error,
            {
                v.parse::<Self::Value>()
                    .map_err(|_| de::Error::invalid_value(de::Unexpected::Str(v), &self))
            }
        }

        deserializer.deserialize_str(DependencyVisitor)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn parse_str() -> anyhow::Result<()> {
        assert_eq!(
            "base".parse::<Dependency>()?,
            Dependency {
                requirement: Requirement::Mandatory,
                name: String::from("base"),
                version: None,
            }
        );

        assert_eq!(
            "base >= 0.18.0".parse::<Dependency>()?,
            Dependency {
                requirement: Requirement::Mandatory,
                name: String::from("base"),
                version: Some(">= 0.18.0".parse().unwrap()),
            }
        );

        assert_eq!(
            "base >= 0.18.00".parse::<Dependency>()?,
            Dependency {
                requirement: Requirement::Mandatory,
                name: String::from("base"),
                version: Some(">= 0.18.00".parse().unwrap()),
            }
        );

        assert_eq!(
            "!base".parse::<Dependency>()?,
            Dependency {
                requirement: Requirement::Incompatible,
                name: String::from("base"),
                version: None,
            }
        );

        assert_eq!(
            "?base".parse::<Dependency>()?,
            Dependency {
                requirement: Requirement::Optional,
                name: String::from("base"),
                version: None,
            }
        );

        assert_eq!(
            "(?)base".parse::<Dependency>()?,
            Dependency {
                requirement: Requirement::OptionalHidden,
                name: String::from("base"),
                version: None,
            }
        );

        Ok(())
    }
}
