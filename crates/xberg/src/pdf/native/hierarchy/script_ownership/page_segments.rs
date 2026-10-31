//! Private transport of exact native witnesses beside segment projections.

use crate::pdf::hierarchy::SegmentData;

use super::NativeScriptOwner;

#[derive(Debug, Clone)]
struct NativeSegment {
    original_index: usize,
    segment: SegmentData,
}

struct NativePageSegments {
    segments: Vec<NativeSegment>,
    script_owners: Vec<NativeScriptOwner>,
}

#[derive(Debug)]
struct SegmentScriptOwner {
    base_index: usize,
    script_index: usize,
    source: NativeScriptOwner,
}

impl NativePageSegments {
    fn remap_script_owners(&self, _retained_segments: &[NativeSegment]) -> Vec<SegmentScriptOwner> {
        Vec::new()
    }
}

#[cfg(test)]
mod tests;
