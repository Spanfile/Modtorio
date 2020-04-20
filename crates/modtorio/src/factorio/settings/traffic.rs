use super::{Limit, Range};
use serde::{
    de::{self, MapAccess, Visitor},
    Deserialize,
};

#[derive(Deserialize, Debug, PartialEq)]
pub struct Upload {
    #[serde(rename = "max_upload_in_kilobytes_per_second")]
    pub max: Limit,
    #[serde(rename = "max_upload_slots")]
    pub slots: Limit,
}

#[derive(Debug, PartialEq)]
pub struct SegmentSize {
    pub size: Range,
    pub peer_count: Range,
}

#[derive(Deserialize, Debug, Default, PartialEq)]
pub struct Traffic {
    #[serde(flatten)]
    pub upload: Upload,
    #[serde(rename = "minimum_latency_in_ticks")]
    pub minimum_latency: u32,
    #[serde(flatten)]
    pub segment_size: SegmentSize,
}

impl Default for Upload {
    fn default() -> Self {
        Self {
            max: Limit::Unlimited,
            slots: Limit::Limited(5),
        }
    }
}

impl Default for SegmentSize {
    fn default() -> Self {
        Self {
            size: Range { min: 25, max: 100 },
            peer_count: Range { min: 20, max: 10 },
        }
    }
}

impl<'de> Deserialize<'de> for SegmentSize {
    fn deserialize<D>(deserializer: D) -> Result<Self, D::Error>
    where
        D: serde::Deserializer<'de>,
    {
        #[derive(Debug, Deserialize)]
        #[serde(field_identifier, rename_all = "snake_case")]
        enum Field {
            MinimumSegmentSize,
            MinimumSegmentSizePeerCount,
            MaximumSegmentSize,
            MaximumSegmentSizePeerCount,
        }

        struct SegmentSizeVisitor;

        impl<'de> Visitor<'de> for SegmentSizeVisitor {
            type Value = SegmentSize;

            fn expecting(&self, formatter: &mut std::fmt::Formatter) -> std::fmt::Result {
                formatter.write_str("segment size settings")
            }

            fn visit_map<V>(self, mut map: V) -> Result<Self::Value, V::Error>
            where
                V: MapAccess<'de>,
            {
                macros::field_deserializers!(
                    map,
                    [minimum_segment_size, u64, MinimumSegmentSize],
                    [maximum_segment_size, u64, MaximumSegmentSize],
                    [
                        minimum_segment_size_peer_count,
                        u64,
                        MinimumSegmentSizePeerCount
                    ],
                    [
                        maximum_segment_size_peer_count,
                        u64,
                        MaximumSegmentSizePeerCount
                    ]
                );

                Ok(Self::Value {
                    size: Range {
                        min: minimum_segment_size,
                        max: maximum_segment_size,
                    },
                    peer_count: Range {
                        min: minimum_segment_size_peer_count,
                        max: maximum_segment_size_peer_count,
                    },
                })
            }
        }

        const FIELDS: &[&str] = &[
            "minimum_segment_size",
            "minimum_segment_size_peer_count",
            "maximum_segment_size",
            "maximum_segment_size_peer_count",
        ];

        deserializer.deserialize_struct("SegmentSize", FIELDS, SegmentSizeVisitor)
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use serde_json::from_str;

    #[test]
    fn deserialize_segment_size() -> anyhow::Result<()> {
        let obj: SegmentSize = from_str(
            r#"{
    "minimum_segment_size": 25,
    "minimum_segment_size_peer_count": 20,
    "maximum_segment_size": 100,
    "maximum_segment_size_peer_count": 10
}"#,
        )?;

        assert_eq!(obj.size.min, 25);
        assert_eq!(obj.size.max, 100);
        assert_eq!(obj.peer_count.min, 20);
        assert_eq!(obj.peer_count.max, 10);

        Ok(())
    }
}
