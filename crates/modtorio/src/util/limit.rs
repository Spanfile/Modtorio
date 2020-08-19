//! Provides the `Limit` struct.

use serde::{de, de::Visitor, Deserialize, Serialize};
use std::{fmt, fmt::Formatter};

/// Represents a limit that is either unbounded ([Unlimited](#variant.Unlimited)) or bounded by a
/// 64-bit unsigned integer ([Limited](#variant.Limited)).
///
/// Conversion to and from `u64` is provided, where 0 is seen as [Unlimited](#variant.Unlimited)
/// and every other value as [Limited](#variant.Limited).
#[derive(Debug, PartialEq, Clone, Copy)]
pub enum Limit {
    /// An unlimited limit
    Unlimited,
    /// A limited limit
    Limited(u64),
}

impl Default for Limit {
    fn default() -> Self {
        Self::Unlimited
    }
}

impl From<u64> for Limit {
    fn from(val: u64) -> Self {
        if val == 0 {
            Self::Unlimited
        } else {
            Self::Limited(val)
        }
    }
}

impl From<Limit> for u64 {
    fn from(val: Limit) -> Self {
        match val {
            Limit::Unlimited => 0,
            Limit::Limited(v) => v,
        }
    }
}

impl Serialize for Limit {
    fn serialize<S>(&self, serializer: S) -> Result<S::Ok, S::Error>
    where
        S: serde::Serializer,
    {
        serializer.serialize_u64((*self).into())
    }
}

impl<'de> Deserialize<'de> for Limit {
    fn deserialize<D>(deserializer: D) -> Result<Self, D::Error>
    where
        D: serde::Deserializer<'de>,
    {
        #[allow(clippy::missing_docs_in_private_items)]
        struct LimitVisitor;

        impl<'de> Visitor<'de> for LimitVisitor {
            type Value = Limit;

            fn expecting(&self, formatter: &mut Formatter) -> fmt::Result {
                formatter.write_str("limit integer")
            }

            fn visit_u8<E>(self, v: u8) -> Result<Self::Value, E>
            where
                E: de::Error,
            {
                Ok((v as u64).into())
            }

            fn visit_u16<E>(self, v: u16) -> Result<Self::Value, E>
            where
                E: de::Error,
            {
                Ok((v as u64).into())
            }

            fn visit_u32<E>(self, v: u32) -> Result<Self::Value, E>
            where
                E: de::Error,
            {
                Ok((v as u64).into())
            }

            fn visit_u64<E>(self, v: u64) -> Result<Self::Value, E>
            where
                E: de::Error,
            {
                Ok(v.into())
            }

            fn visit_i8<E>(self, v: i8) -> Result<Self::Value, E>
            where
                E: de::Error,
            {
                Ok((v as u64).into())
            }

            fn visit_i16<E>(self, v: i16) -> Result<Self::Value, E>
            where
                E: de::Error,
            {
                Ok((v as u64).into())
            }

            fn visit_i32<E>(self, v: i32) -> Result<Self::Value, E>
            where
                E: de::Error,
            {
                Ok((v as u64).into())
            }

            fn visit_i64<E>(self, v: i64) -> Result<Self::Value, E>
            where
                E: de::Error,
            {
                Ok((v as u64).into())
            }
        }

        deserializer.deserialize_u64(LimitVisitor)
    }
}
