//! Private transport of exact native witnesses beside segment projections.

use std::collections::HashMap;

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

#[derive(PartialEq, Eq, Hash)]
struct SegmentProjection<'a> {
    text: &'a str,
    geometry: [u32; 7],
    style: [bool; 3],
    assigned_role: Option<u8>,
}

impl<'a> From<&'a SegmentData> for SegmentProjection<'a> {
    fn from(segment: &'a SegmentData) -> Self {
        Self {
            text: &segment.text,
            geometry: [
                segment.x,
                segment.y,
                segment.width,
                segment.height,
                segment.font_size,
                segment.baseline_y,
                segment.rotation_degrees,
            ]
            .map(f32::to_bits),
            style: [segment.is_bold, segment.is_italic, segment.is_monospace],
            assigned_role: segment.assigned_role,
        }
    }
}

impl NativePageSegments {
    fn remap_script_owners(&self, retained_segments: &[NativeSegment]) -> Vec<SegmentScriptOwner> {
        let originals = self.unique_originals();
        let survivors = index_survivors(retained_segments, &originals);
        self.script_owners
            .iter()
            .filter_map(|owner| {
                let base_index = *survivors.get(&owner.base_witness.original_index)?.as_ref()?;
                let script_index = *survivors.get(&owner.script_witness.original_index)?.as_ref()?;
                (base_index != script_index).then(|| SegmentScriptOwner {
                    base_index,
                    script_index,
                    source: owner.clone(),
                })
            })
            .collect()
    }

    fn unique_originals(&self) -> HashMap<usize, Option<&NativeSegment>> {
        let mut projections = HashMap::new();
        let mut originals: HashMap<usize, Option<&NativeSegment>> = HashMap::new();
        for record in &self.segments {
            *projections
                .entry(SegmentProjection::from(&record.segment))
                .or_insert(0_usize) += 1;
            originals
                .entry(record.original_index)
                .and_modify(|entry| *entry = None)
                .or_insert(Some(record));
        }
        // Freeze projection ambiguity before filtering, even when the colliding
        // native witnesses came from different marked-content scopes. ~keep
        for original in originals.values_mut() {
            if original.is_some_and(|record| projections.get(&SegmentProjection::from(&record.segment)) != Some(&1)) {
                *original = None;
            }
        }
        originals
    }
}

fn index_survivors(
    retained: &[NativeSegment],
    originals: &HashMap<usize, Option<&NativeSegment>>,
) -> HashMap<usize, Option<usize>> {
    let mut survivors = HashMap::new();
    for (index, record) in retained.iter().enumerate() {
        let original = originals.get(&record.original_index).and_then(Option::as_ref);
        let mapped = original
            .is_some_and(|original| {
                SegmentProjection::from(&original.segment) == SegmentProjection::from(&record.segment)
            })
            .then_some(index);
        survivors
            .entry(record.original_index)
            .and_modify(|entry| *entry = None)
            .or_insert(mapped);
    }
    survivors
}

#[cfg(test)]
mod tests;
