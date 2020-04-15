use super::{Limit, Range};
use serde::Deserialize;

#[derive(Deserialize)]
struct Upload {
    max: Limit,
    slots: Limit,
}

#[derive(Deserialize)]
struct SegmentSize {
    size: Range,
    peer_count: Range,
}

#[derive(Deserialize)]
pub struct Traffic {
    upload: Upload,
    minimum_latency: u32,
    segment_size: SegmentSize,
}
