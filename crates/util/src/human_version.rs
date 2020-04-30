use anyhow::anyhow;
use lazy_static::lazy_static;
use regex::Regex;
use serde::{de, de::Visitor, Deserialize};
use std::{fmt, fmt::Display, str::FromStr};

#[derive(Debug, PartialEq)]
pub enum Comparator {
    GreaterOrEqual,
    Greater,
    Equal,
    Less,
    LessOrEqual,
}

#[derive(Debug, PartialEq, PartialOrd, Clone, Copy)]
pub struct HumanVersion {
    pub major: u64,
    pub minor: u64,
    pub patch: u64,
}

#[derive(Debug, PartialEq)]
pub struct HumanVersionReq {
    pub comparator: Comparator,
    pub version: HumanVersion,
}

impl FromStr for HumanVersion {
    type Err = anyhow::Error;

    fn from_str(s: &str) -> Result<Self, Self::Err> {
        let args: Vec<&str> = s.split('.').collect();

        let major = args
            .get(0)
            .map(|c| c.parse::<u64>())
            .ok_or_else(|| anyhow!("couldn't parse first argument as u64"))??;

        let minor = args.get(1).map_or(Ok(0), |c| c.parse::<u64>())?;
        let patch = args.get(2).map_or(Ok(0), |c| c.parse::<u64>())?;

        Ok(Self {
            major,
            minor,
            patch,
        })
    }
}

impl Display for HumanVersion {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.write_fmt(format_args!("{}.{}.{}", self.major, self.minor, self.patch))
    }
}

impl FromStr for HumanVersionReq {
    type Err = anyhow::Error;

    fn from_str(s: &str) -> Result<Self, Self::Err> {
        lazy_static! {
            static ref RE: Regex = Regex::new(r"(>=|<=|>|=|<) ?(.*)").unwrap();
        }

        let captures = RE
            .captures(s)
            .ok_or_else(|| anyhow!("version requirement regex returned no captures"))?;

        let comparator = match captures.get(1).map(|c| c.as_str()) {
            Some(">=") => Comparator::GreaterOrEqual,
            Some(">") => Comparator::Greater,
            Some("=") => Comparator::Equal,
            Some("<") => Comparator::Less,
            Some("<=") => Comparator::LessOrEqual,
            Some(c) => panic!("impossible case (regex returned {})", c),
            None => return Err(anyhow!("no comparator in input")),
        };

        let version = captures
            .get(2)
            .ok_or_else(|| anyhow!("no version in input"))?
            .as_str()
            .parse()?;

        Ok(Self {
            comparator,
            version,
        })
    }
}

impl<'de> Deserialize<'de> for HumanVersion {
    fn deserialize<D>(deserializer: D) -> Result<Self, D::Error>
    where
        D: serde::Deserializer<'de>,
    {
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
