use serde::{
    de::{self, Visitor},
    Deserialize, Serialize,
};
use std::fmt;

#[derive(Debug, PartialEq)]
pub enum Limit {
    Unlimited,
    Limited(u64),
}

impl<'de> Deserialize<'de> for Limit {
    fn deserialize<D>(deserializer: D) -> Result<Self, D::Error>
    where
        D: serde::Deserializer<'de>,
    {
        struct LimitVisitor;

        impl<'de> Visitor<'de> for LimitVisitor {
            type Value = Limit;

            fn expecting(&self, formatter: &mut fmt::Formatter) -> fmt::Result {
                formatter.write_str("any positive integer or zero")
            }

            fn visit_u8<E>(self, v: u8) -> Result<Self::Value, E>
            where
                E: de::Error,
            {
                if v == 0 {
                    Ok(Limit::Unlimited)
                } else {
                    Ok(Limit::Limited(u64::from(v)))
                }
            }

            fn visit_u16<E>(self, v: u16) -> Result<Self::Value, E>
            where
                E: de::Error,
            {
                if v == 0 {
                    Ok(Limit::Unlimited)
                } else {
                    Ok(Limit::Limited(u64::from(v)))
                }
            }

            fn visit_u32<E>(self, v: u32) -> Result<Self::Value, E>
            where
                E: de::Error,
            {
                if v == 0 {
                    Ok(Limit::Unlimited)
                } else {
                    Ok(Limit::Limited(u64::from(v)))
                }
            }

            fn visit_u64<E>(self, v: u64) -> Result<Self::Value, E>
            where
                E: de::Error,
            {
                if v == 0 {
                    Ok(Limit::Unlimited)
                } else {
                    Ok(Limit::Limited(v))
                }
            }
        }

        deserializer.deserialize_u64(LimitVisitor)
    }
}

impl Serialize for Limit {
    fn serialize<S>(&self, serializer: S) -> Result<S::Ok, S::Error>
    where
        S: serde::Serializer,
    {
        serializer.serialize_u64(match self {
            Limit::Unlimited => 0,
            Limit::Limited(limit) => *limit,
        })
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use serde_json::{from_str, to_string};

    #[test]
    fn deserialize() -> anyhow::Result<()> {
        let unlimited: Limit = from_str("0")?;
        assert_eq!(unlimited, Limit::Unlimited);

        let limited: Limit = from_str("1")?;
        assert_eq!(limited, Limit::Limited(1));

        let limited: Limit = from_str(&u64::MAX.to_string())?;
        assert_eq!(limited, Limit::Limited(u64::MAX));

        Ok(())
    }

    #[test]
    fn serialize() -> anyhow::Result<()> {
        let unlimited = Limit::Unlimited;
        let limited = Limit::Limited(1);

        assert_eq!("0", to_string(&unlimited)?);
        assert_eq!("1", to_string(&limited)?);

        Ok(())
    }
}
