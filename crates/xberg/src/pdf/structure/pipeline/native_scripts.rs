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
