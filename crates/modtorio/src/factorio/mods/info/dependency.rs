use anyhow::anyhow;
use lazy_static::lazy_static;
use regex::Regex;
use semver::VersionReq;
use serde::{de, de::Visitor, Deserialize};
use std::{fmt, str::FromStr};

#[derive(Debug)]
pub enum Requirement {
    Mandatory,
    Optional,
    OptionalHidden,
    Incompatible,
}

#[derive(Debug)]
pub struct Dependency {
    pub requirement: Requirement,
    pub name: String,
    pub version: VersionReq,
}

impl FromStr for Dependency {
    type Err = anyhow::Error;

    fn from_str(s: &std::primitive::str) -> Result<Self, Self::Err> {
        lazy_static! {
            static ref RE: Regex = Regex::new(r"(\?|!|\(\?\))? ?(\S*) ?(.+)?").unwrap();
        }
        let captures = RE
            .captures(s)
            .ok_or_else(|| anyhow!("dependency regex returned no captures"))?;

        let requirement = match captures.get(1).map(|c| c.as_str()) {
            Some("?") => Requirement::Optional,
            Some("!") => Requirement::Incompatible,
            Some("(?)") => Requirement::OptionalHidden,
            Some(_) => panic!("impossible case"),
            None => Requirement::Mandatory,
        };

        let name = captures
            .get(2)
            .ok_or_else(|| anyhow!("dependency regex didn't capture name"))?
            .as_str()
            .to_string();
        let version = captures
            .get(3)
            .map_or(VersionReq::parse("*"), |c| VersionReq::parse(c.as_str()))?;
        Ok(Dependency {
            requirement,
            name,
            version,
        })
    }
}

impl fmt::Display for Dependency {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self.requirement {
            Requirement::Optional => f.write_str("? ")?,
            Requirement::Incompatible => f.write_str("! ")?,
            Requirement::OptionalHidden => f.write_str("(?)")?,
            _ => {}
        }

        f.write_fmt(format_args!("{} ", &self.name))?;
        f.write_str(&self.version.to_string())
    }
}

impl<'de> Deserialize<'de> for Dependency {
    fn deserialize<D>(deserializer: D) -> Result<Self, D::Error>
    where
        D: serde::Deserializer<'de>,
    {
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
