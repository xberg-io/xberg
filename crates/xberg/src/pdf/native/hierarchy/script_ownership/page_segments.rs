//! Private transport of exact native witnesses beside segment projections.

use std::collections::HashMap;

use crate::pdf::hierarchy::SegmentData;

use super::NativeScriptOwner;

#[derive(Debug, Clone)]
pub(crate) struct NativeSegment {
    pub(crate) original_index: usize,
    pub(crate) segment: SegmentData,
}

pub(crate) struct NativePageSegments {
    segments: Vec<NativeSegment>,
    script_owners: Vec<NativeScriptOwner>,
}

#[derive(Debug, Clone)]
pub(crate) struct SegmentScriptOwner {
    pub(crate) base_index: usize,
    pub(crate) script_index: usize,
    source: NativeScriptOwner,
}

impl SegmentScriptOwner {
    pub(crate) fn kind(&self) -> crate::types::document_structure::AnnotationKind {
        self.source.kind.clone()
    }

    pub(crate) fn byte_range(&self) -> std::ops::Range<usize> {
        self.source.script_byte_range.clone()
    }

    pub(crate) fn logical_baseline(&self) -> f32 {
        self.source.logical_baseline
    }
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
    pub(crate) fn segments(&self) -> impl Iterator<Item = &SegmentData> {
        self.segments.iter().map(|record| &record.segment)
    }

    pub(crate) fn retain_segments(&mut self, mut keep: impl FnMut(&SegmentData) -> bool) {
        self.segments.retain(|record| keep(&record.segment));
        self.script_owners = self
            .remap_script_owners(&self.segments)
            .into_iter()
            .map(|owner| owner.source)
            .collect();
    }

    pub(crate) fn projected_records(
        &self,
        segments: Vec<SegmentData>,
    ) -> std::result::Result<Vec<NativeSegment>, Vec<SegmentData>> {
        if segments.len() != self.segments.len() {
            return Err(segments);
        }
        Ok(self
            .segments
            .iter()
            .zip(segments)
            .map(|(source, segment)| NativeSegment {
                original_index: source.original_index,
                segment,
            })
            .collect())
    }

    pub(crate) fn from_final_spans(
        spans: Vec<xberg_native_pdf::layout::TextSpan>,
        mcid_roles: &HashMap<u32, Option<u8>>,
        artifacts: super::super::ArtifactRetention,
        script_geometry: super::super::ScriptGeometry,
    ) -> Self {
        let script_owners = super::collect_suffix_script_owners(&spans);
        let segments = project_native_segments(spans, mcid_roles, artifacts, script_geometry);
        let originals = Self {
            segments,
            script_owners,
        };
        let segments = super::super::dedupe_indexed_redrawn_segments(
            originals
                .segments
                .iter()
                .map(|record| (record.original_index, record.segment.clone())),
        )
        .into_iter()
        .map(|(original_index, segment)| NativeSegment {
            original_index,
            segment,
        })
        .collect::<Vec<_>>();
        // Validate against the full projection inventory before redraws collapse
        // ambiguous sources or merge styles into a surviving endpoint. ~keep
        let script_owners = originals
            .remap_script_owners(&segments)
            .into_iter()
            .map(|owner| owner.source)
            .collect();
        Self {
            segments,
            script_owners,
        }
    }

    pub(crate) fn remap_script_owners(&self, retained_segments: &[NativeSegment]) -> Vec<SegmentScriptOwner> {
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

fn project_native_segments(
    spans: Vec<xberg_native_pdf::layout::TextSpan>,
    mcid_roles: &HashMap<u32, Option<u8>>,
    artifacts: super::super::ArtifactRetention,
    script_geometry: super::super::ScriptGeometry,
) -> Vec<NativeSegment> {
    spans
        .into_iter()
        .enumerate()
        .filter(|(_, span)| {
            !span.text.trim().is_empty()
                && span
                    .artifact_type
                    .as_ref()
                    .is_none_or(|artifact| artifacts.keeps(artifact))
        })
        .map(|(original_index, span)| NativeSegment {
            original_index,
            segment: super::super::project_final_span(span, mcid_roles, script_geometry),
        })
        .collect()
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

pub(crate) fn extract_all_segment_carriers(
    document: &mut super::super::NativeDocument,
    artifacts: super::super::ArtifactRetention,
) -> crate::pdf::error::Result<(Vec<NativePageSegments>, bool)> {
    use super::super::ScriptGeometry;

    let (pages, used_tree) = super::super::extract_pages_with_structure_tree(
        document,
        artifacts,
        ScriptGeometry::Preserve,
        extract_page_carrier,
        |page| {
            page.segments
                .iter()
                .filter(|record| record.segment.assigned_role.is_some())
                .count()
        },
    )?;
    if used_tree && !pages.is_empty() {
        return Ok((pages, true));
    }
    let page_count = document.doc.page_count().map_err(|error| {
        crate::pdf::error::PdfError::TextExtractionFailed(format!(
            "xberg_native_pdf: failed to get page count: {error}"
        ))
    })?;
    let pages = (0..page_count)
        .map(|index| extract_page_carrier(document, index, &HashMap::new(), artifacts, ScriptGeometry::Preserve))
        .collect::<crate::pdf::error::Result<Vec<_>>>()?;
    Ok((pages, false))
}

fn extract_page_carrier(
    document: &mut super::super::NativeDocument,
    page_index: usize,
    roles: &HashMap<u32, Option<u8>>,
    artifacts: super::super::ArtifactRetention,
    geometry: super::super::ScriptGeometry,
) -> crate::pdf::error::Result<NativePageSegments> {
    let spans = super::super::extract_final_spans_from_page(document, page_index, geometry)?;
    Ok(NativePageSegments::from_final_spans(spans, roles, artifacts, geometry))
}

#[cfg(test)]
mod transport_tests;
