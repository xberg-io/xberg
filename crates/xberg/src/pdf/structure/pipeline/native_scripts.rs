//! Preserve page-local source identities through native table filtering.

use crate::pdf::hierarchy::SegmentData;
use crate::pdf::native::hierarchy::script_ownership::page_segments::{NativePageSegments, SegmentScriptOwner};

use super::{TableCoverage, filter_segments_by_table_bboxes, segment_survives_table_filter};

pub(super) struct NativeSegmentEvidence {
    pub(super) original_indices: Vec<usize>,
    pub(super) owners: Vec<SegmentScriptOwner>,
}

pub(super) fn prepare_segments(
    segments: Vec<SegmentData>,
    page: NativePageSegments,
    tables: &[TableCoverage],
) -> (Vec<SegmentData>, Option<NativeSegmentEvidence>) {
    let mut records = match page.projected_records(segments) {
        Ok(records) => records,
        Err(segments) => return (filter_segments_by_table_bboxes(segments, tables), None),
    };
    records.retain(|record| segment_survives_table_filter(&record.segment, tables));
    let owners = page.remap_script_owners(&records);
    let (original_indices, segments) = records
        .into_iter()
        .map(|record| (record.original_index, record.segment))
        .unzip();
    (
        segments,
        Some(NativeSegmentEvidence {
            original_indices,
            owners,
        }),
    )
}

pub(super) struct NativeScriptIndex {
    original_indices: Vec<usize>,
    by_script: std::collections::HashMap<usize, SegmentScriptOwner>,
}

impl NativeScriptIndex {
    pub(super) fn base_for(&self, index: usize) -> Option<usize> {
        self.by_script.get(&index).map(|owner| owner.base_index)
    }

    pub(super) fn row_for(&self, index: usize) -> usize {
        self.base_for(index).unwrap_or(index)
    }

    pub(super) fn subset(&self, indices: &[usize]) -> NativeSegmentEvidence {
        let positions: std::collections::HashMap<_, _> = indices
            .iter()
            .enumerate()
            .map(|(local, &global)| (global, local))
            .collect();
        let owners = indices
            .iter()
            .filter_map(|index| {
                let mut owner = self.by_script.get(index)?.clone();
                owner.base_index = *positions.get(&owner.base_index)?;
                owner.script_index = *positions.get(&owner.script_index)?;
                Some(owner)
            })
            .collect();
        NativeSegmentEvidence {
            original_indices: indices
                .iter()
                .filter_map(|&index| self.original_indices.get(index).copied())
                .collect(),
            owners,
        }
    }
}

pub(super) fn order_native_segments(
    segments: Vec<SegmentData>,
    evidence: NativeSegmentEvidence,
) -> (Vec<SegmentData>, NativeScriptIndex) {
    if evidence.original_indices.len() != segments.len() {
        return (
            super::order_segments_in_reading_frames(segments),
            NativeScriptIndex {
                original_indices: Vec::new(),
                by_script: std::collections::HashMap::new(),
            },
        );
    }
    let refs: Vec<_> = segments.iter().collect();
    let (order, owners) = plan_native_order(&refs, &evidence);
    let original_indices = order.iter().map(|&index| evidence.original_indices[index]).collect();
    let mut slots: Vec<_> = segments.into_iter().map(Some).collect();
    let segments = order
        .into_iter()
        .map(|index| slots[index].take().expect("source permutation visits each slot once"))
        .collect();
    let by_script = owners.into_iter().map(|owner| (owner.script_index, owner)).collect();
    (
        segments,
        NativeScriptIndex {
            original_indices,
            by_script,
        },
    )
}

fn valid_owners<'a>(segments: &[&SegmentData], evidence: &'a NativeSegmentEvidence) -> Vec<&'a SegmentScriptOwner> {
    if evidence.original_indices.len() != segments.len() {
        return Vec::new();
    }
    let mut endpoints = std::collections::HashMap::new();
    evidence
        .owners
        .iter()
        .filter(|owner| {
            [(owner.base_index, false), (owner.script_index, true)]
                .into_iter()
                .all(|(index, script)| {
                    *endpoints
                        .entry((index, owner.original_index(script)))
                        .or_insert_with(|| {
                            segments.get(index).is_some_and(|segment| {
                                evidence.original_indices[index] == owner.original_index(script)
                                    && owner.endpoint_matches(segment, script)
                            })
                        })
                })
        })
        .collect()
}

fn plan_native_order(
    segments: &[&SegmentData],
    evidence: &NativeSegmentEvidence,
) -> (Vec<usize>, Vec<SegmentScriptOwner>) {
    let ordered = super::order_segment_records((0..segments.len()).collect(), |index| segments[*index]);
    let owners = valid_owners(segments, evidence);
    let scripts: std::collections::HashSet<_> = owners.iter().map(|owner| owner.script_index).collect();
    let mut script_counts = std::collections::HashMap::new();
    for owner in &owners {
        *script_counts.entry(owner.script_index).or_insert(0_usize) += 1;
    }
    // A nested script chain needs a stronger ownership representation. Keep its
    // original order instead of claiming a partially supported attachment. ~keep
    let owners: Vec<_> = owners
        .into_iter()
        .filter(|owner| !scripts.contains(&owner.base_index) && script_counts[&owner.script_index] == 1)
        .collect();
    let order = place_suffixes_after_owners(&ordered, &owners);
    if order.len() != segments.len()
        || order.iter().copied().collect::<std::collections::HashSet<_>>().len() != segments.len()
    {
        return (ordered, Vec::new());
    }
    let positions: std::collections::HashMap<_, _> = order
        .iter()
        .enumerate()
        .map(|(position, &index)| (index, position))
        .collect();
    let owners = owners
        .into_iter()
        .map(|owner| {
            let mut owner = owner.clone();
            owner.base_index = positions[&owner.base_index];
            owner.script_index = positions[&owner.script_index];
            owner
        })
        .collect();
    (order, owners)
}

fn place_suffixes_after_owners(ordered: &[usize], owners: &[&SegmentScriptOwner]) -> Vec<usize> {
    let mut children: std::collections::HashMap<usize, Vec<usize>> = std::collections::HashMap::new();
    let mut held = std::collections::HashSet::new();
    for owner in owners {
        children.entry(owner.base_index).or_default().push(owner.script_index);
        held.insert(owner.script_index);
    }
    let mut order = Vec::with_capacity(ordered.len());
    for &index in ordered {
        if !held.contains(&index) {
            order.push(index);
            order.extend(children.remove(&index).unwrap_or_default());
        }
    }
    order
}

pub(super) fn reconstruct_native_lines(
    segments: &[&SegmentData],
    evidence: &NativeSegmentEvidence,
) -> Vec<crate::pdf::structure::types::PdfLine> {
    let (order, owners) = plan_native_order(segments, evidence);
    let ordered: Vec<_> = order.iter().map(|&index| segments[index]).collect();
    let by_script: std::collections::HashMap<_, _> =
        owners.into_iter().map(|owner| (owner.script_index, owner)).collect();
    let mut lines = Vec::new();
    let mut current: Vec<(usize, SegmentData)> = Vec::new();
    let mut baseline = 0.0_f32;
    let mut rotation = 0.0_f32;
    let mut scale = 0.0_f32;
    for (index, segment) in ordered.iter().enumerate() {
        let owner = by_script.get(&index);
        let logical_baseline = owner.map_or_else(|| segment.upright_baseline(), SegmentScriptOwner::logical_baseline);
        let logical_scale = owner.map_or(segment.font_size.max(segment.height).abs(), |owner| {
            let base = ordered[owner.base_index];
            base.font_size.max(base.height).abs()
        });
        let tolerance =
            (scale.max(logical_scale) * super::LINE_TOLERANCE_SCALE_FACTOR).min(super::MAX_LINE_TOLERANCE_PT);
        if !current.is_empty()
            && ((segment.rotation_degrees - rotation).abs() > f32::EPSILON
                || (logical_baseline - baseline).abs() > tolerance)
        {
            lines.push(finish_native_line(std::mem::take(&mut current), baseline, &by_script));
        }
        if current.is_empty() {
            baseline = logical_baseline;
            rotation = segment.rotation_degrees;
            scale = logical_scale;
        } else {
            scale = scale.max(logical_scale);
        }
        current.push((index, (*segment).clone()));
    }
    if !current.is_empty() {
        lines.push(finish_native_line(current, baseline, &by_script));
    }
    lines
}

fn finish_native_line(
    mut records: Vec<(usize, SegmentData)>,
    baseline: f32,
    owners: &std::collections::HashMap<usize, SegmentScriptOwner>,
) -> crate::pdf::structure::types::PdfLine {
    if !records.iter().any(|(_, segment)| {
        segment
            .text
            .chars()
            .any(|character| xberg_native_pdf::text::is_rtl_text(character as u32))
    }) {
        records.sort_by(|left, right| {
            left.1
                .upright_advance_extent()
                .0
                .total_cmp(&right.1.upright_advance_extent().0)
        });
    }
    let positions: std::collections::HashMap<_, _> = records
        .iter()
        .enumerate()
        .map(|(local, (source, _))| (*source, local))
        .collect();
    let native_scripts = records
        .iter()
        .filter_map(|(index, _)| {
            let mut owner = owners.get(index)?.clone();
            owner.base_index = *positions.get(&owner.base_index)?;
            owner.script_index = *positions.get(&owner.script_index)?;
            Some(owner)
        })
        .collect();
    let mut line = super::make_pdf_line(records.into_iter().map(|(_, segment)| segment).collect(), baseline);
    line.native_scripts = native_scripts;
    line
}

#[cfg(test)]
mod line_tests;

#[cfg(test)]
mod tests {
    use xberg_native_pdf::geometry::Rect;
    use xberg_native_pdf::layout::TextSpan;
    use xberg_native_pdf::structure::McidScope;

    use crate::pdf::native::hierarchy::{ArtifactRetention, ScriptGeometry};

    use super::*;

    fn page() -> NativePageSegments {
        let span = |text: &str, x, y, width, font_size| TextSpan {
            text: text.to_owned(),
            bbox: Rect::new(x, y, width, font_size),
            font_size,
            mcid_scope: Some(McidScope::Page(0)),
            ..Default::default()
        };
        NativePageSegments::from_final_spans(
            vec![
                span(" ", 50.0, 700.0, 4.0, 12.0),
                span("q", 100.0, 700.0, 10.0, 12.0),
                span("2 ", 110.0, 704.0, 4.0, 8.0),
            ],
            &Default::default(),
            ArtifactRetention::default(),
            ScriptGeometry::Preserve,
        )
    }

    fn table(x0: f64, x1: f64, text: &str) -> TableCoverage {
        TableCoverage::from_rows(
            crate::types::BoundingBox {
                x0,
                y0: 700.0,
                x1,
                y1: 712.0,
            },
            [vec![text]],
        )
    }

    #[test]
    fn should_keep_source_ordinals_and_glyph_geometry_through_native_table_filter_2275() {
        let page = page();
        let segments = page.segments().cloned().collect();
        let (segments, evidence) = prepare_segments(segments, page, &[]);
        let evidence = evidence.expect("native source evidence");
        assert_eq!(evidence.original_indices, [1, 2]);
        assert_eq!(evidence.owners.len(), 1);
        assert_eq!((evidence.owners[0].base_index, evidence.owners[0].script_index), (0, 1));
        assert_eq!(segments[1].baseline_y, 704.0);
        assert_eq!(segments[1].text, "2 ");
    }

    #[test]
    fn should_drop_script_edge_when_table_owns_either_endpoint_2275() {
        for (coverage, expected_source, expected_text) in
            [(table(100.0, 110.0, "q"), 2, "2 "), (table(110.0, 114.0, "2"), 1, "q")]
        {
            let page = page();
            let segments = page.segments().cloned().collect();
            let (segments, evidence) = prepare_segments(segments, page, &[coverage]);
            let evidence = evidence.expect("native source evidence");
            assert_eq!(segments.len(), 1);
            assert_eq!(segments[0].text, expected_text);
            assert_eq!(evidence.original_indices, [expected_source]);
            assert!(evidence.owners.is_empty());
        }
    }

    #[test]
    fn should_keep_unrepresented_prose_and_its_native_script_proof_inside_table_bbox_2275() {
        let page = page();
        let segments = page.segments().cloned().collect();
        let (segments, evidence) = prepare_segments(segments, page, &[table(100.0, 114.0, "unrelated")]);
        assert_eq!(segments.len(), 2);
        assert_eq!(evidence.unwrap().owners.len(), 1);
    }

    #[test]
    fn should_omit_proof_after_projection_mutation_without_discarding_current_text_2275() {
        let page = page();
        let mut segments: Vec<_> = page.segments().cloned().collect();
        segments[0].assigned_role = Some(2);
        let (segments, evidence) = prepare_segments(segments, page, &[]);
        assert_eq!(segments.len(), 2);
        assert_eq!(segments[0].assigned_role, Some(2));
        assert!(evidence.unwrap().owners.is_empty());
    }

    #[test]
    fn should_fail_closed_on_missing_source_slot_without_truncating_projection_2275() {
        let page = page();
        let mut segments: Vec<_> = page.segments().cloned().collect();
        segments.pop();
        let (segments, evidence) = prepare_segments(segments, page, &[]);
        assert_eq!(segments.len(), 1);
        assert_eq!(segments[0].text, "q");
        assert!(evidence.is_none());
    }
}
