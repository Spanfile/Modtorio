use super::{Limit, Range};
use serde::Deserialize;

#[derive(Deserialize, Debug)]
struct Upload {
    max: Limit,
    slots: Limit,
}

#[derive(Deserialize, Debug)]
struct SegmentSize {
    size: Range,
    peer_count: Range,
}

#[derive(Deserialize, Debug, Default)]
pub struct Traffic {
    upload: Upload,
    minimum_latency: u32,
    segment_size: SegmentSize,
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
