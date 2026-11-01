use super::super::line_tests::fingerprint;
use super::*;
use sha2::{Digest, Sha256};

struct BoundaryCase {
    segments: Vec<SegmentData>,
    evidence: NativeSegmentEvidence,
    gaps: Vec<f32>,
}

impl BoundaryCase {
    fn new(segments: Vec<SegmentData>, evidence: NativeSegmentEvidence) -> Self {
        assert!(!segments.is_empty());
        assert!(evidence.origin.is_some());
        assert!(
            !evidence.owners.is_empty(),
            "actual unrelated native ownership activates calibration"
        );
        let gaps = super::super::super::compute_paragraph_gap_ys(&segments);
        Self {
            segments,
            evidence,
            gaps,
        }
    }

    fn ordered(&self) -> (Vec<SegmentData>, NativeScriptIndex) {
        let evidence = NativeSegmentEvidence {
            origin: self.evidence.origin,
            original_indices: self.evidence.original_indices.clone(),
            owners: self.evidence.owners.clone(),
        };
        super::super::order_native_paragraph_segments(self.segments.clone(), evidence, &self.gaps)
    }

    fn assert_joined(&self, upper_id: usize, lower_id: usize) {
        let (ordered, index) = self.ordered();
        let upper = source_position(&index, upper_id);
        let lower = source_position(&index, lower_id);
        let labels = index
            .ordinary_groups
            .as_ref()
            .expect("actual original and repaired source groups");
        assert_ne!(labels.original[upper], labels.original[lower]);
        assert_ne!(labels.repaired[upper], labels.repaired[lower]);
        assert!(!index.by_script.contains_key(&upper) && !index.by_script.contains_key(&lower));
        assert!(
            !index
                .continuation_gaps
                .as_ref()
                .unwrap()
                .crosses_rows(&ordered[upper], &ordered[lower])
        );
        let grouped = super::super::super::blocks_to_paragraphs_with_native_scripts(
            ordered.clone(),
            &[],
            &self.gaps,
            Some(&index),
        );
        let first = paragraph_with(&grouped, &ordered[upper]);
        let next = paragraph_with(&grouped, &ordered[lower]);
        assert_ne!(first, next, "initial real source groups retain the boundary");
        assert_ordinary_room(&grouped[first], &grouped[next], &ordered[upper], &ordered[lower]);
        let mut generic = vec![grouped[first].clone(), grouped[next].clone()];
        crate::pdf::structure::paragraphs::merge_continuation_paragraphs(&mut generic);
        assert_eq!(
            generic.len(),
            1,
            "the unchanged generic merger admits the actual endpoint pair"
        );
        let output = self.output();
        assert_eq!(
            paragraph_with(&output, &ordered[upper]),
            paragraph_with(&output, &ordered[lower]),
            "the real source continuation must survive the final native gap merger"
        );
    }
    fn output(&self) -> Vec<PdfParagraph> {
        let evidence = NativeSegmentEvidence {
            origin: self.evidence.origin,
            original_indices: self.evidence.original_indices.clone(),
            owners: self.evidence.owners.clone(),
        };
        super::super::super::segments_to_paragraphs_with_native_scripts(
            self.segments.clone(),
            &[],
            &self.gaps,
            &Default::default(),
            Some(evidence),
        )
    }
}

fn assert_ordinary_room(previous: &PdfParagraph, next: &PdfParagraph, upper: &SegmentData, lower: &SegmentData) {
    let rows = [previous.lines.last().unwrap(), next.lines.first().unwrap()];
    for row in rows {
        assert!(row.valid_native_scripts().is_empty());
        assert!(!crate::pdf::native::table::is_dense_numeric_row(
            row.segments.iter().flat_map(|segment| segment.text.split_whitespace())
        ));
    }
    let right = rows[1]
        .segments
        .iter()
        .map(|segment| segment.upright_advance_extent().1)
        .fold(f32::NEG_INFINITY, f32::max);
    assert!(!super::super::super::heading_fills_column(upper, right));
    assert!(!super::super::super::heading_line_ran_out_of_room(upper, lower, right));
}

fn assert_band_edges(ordered: &[SegmentData], index: &NativeScriptIndex, first: usize, last: usize, anchor: f32) {
    let ordinary = ordered
        .iter()
        .enumerate()
        .filter(|(i, _)| !index.by_script.contains_key(i))
        .map(|(_, segment)| segment);
    let bands = super::super::super::paragraph_line_bands(ordinary);
    let rows: Vec<_> = bands
        .iter()
        .filter(|band| {
            fingerprint(band.source.first) == fingerprint(&ordered[source_position(index, first)])
                && fingerprint(band.source.last) == fingerprint(&ordered[source_position(index, last)])
        })
        .collect();
    assert_eq!(rows.len(), 1, "actual complete ordinary source-band membership");
    assert_eq!(
        rows[0].anchor_y, anchor,
        "actual anchor generating the original gap vote"
    );
}

fn source_case(name: &str, expected_sha: &str) -> BoundaryCase {
    let path = std::path::Path::new(env!("CARGO_MANIFEST_DIR"))
        .join("../../test_documents/pdf/regression_pages")
        .join(format!("{name}.pdf"));
    let bytes = std::fs::read(&path).unwrap_or_else(|error| panic!("mandatory source {}: {error}", path.display()));
    assert!(bytes.starts_with(b"%PDF-"));
    assert_eq!(hex::encode(Sha256::digest(&bytes)), expected_sha);
    let mut document = crate::pdf::native::NativeDocument::open_bytes(&bytes).unwrap();
    let (mut pages, used_tree) =
        crate::pdf::native::hierarchy::script_ownership::page_segments::extract_all_segment_carriers(
            &mut document,
            ArtifactRetention::default(),
        )
        .unwrap();
    assert!(
        !used_tree,
        "same actual native heuristic source route as the public regression"
    );
    assert_eq!(pages.len(), 1);
    let page = pages.pop().unwrap();
    let segments = page.segments().cloned().collect();
    let (segments, evidence) = prepare_segments(segments, page, &[]);
    BoundaryCase::new(segments, evidence.expect("real native source witnesses"))
}

fn source_position(index: &NativeScriptIndex, original: usize) -> usize {
    let positions: Vec<_> = index
        .original_indices
        .iter()
        .enumerate()
        .filter(|(_, source)| **source == original)
        .map(|(position, _)| position)
        .collect();
    assert_eq!(positions.len(), 1, "unique actual original source ordinal {original}");
    positions[0]
}

fn paragraph_with(paragraphs: &[PdfParagraph], segment: &SegmentData) -> usize {
    let expected = fingerprint(segment);
    let matches: Vec<_> = paragraphs
        .iter()
        .enumerate()
        .flat_map(|(index, paragraph)| {
            paragraph
                .lines
                .iter()
                .flat_map(|line| &line.segments)
                .filter(|current| fingerprint(current) == expected)
                .map(move |_| index)
        })
        .collect();
    assert_eq!(
        matches.len(),
        1,
        "actual target source record remains unchanged exactly once"
    );
    matches[0]
}

fn assert_gap_evidence(case: &BoundaryCase, upper_id: usize, lower_id: usize, endpoint_vote: bool) {
    let (ordered, index) = case.ordered();
    let upper = &ordered[source_position(&index, upper_id)];
    let lower = &ordered[source_position(&index, lower_id)];
    let ordinary: Vec<_> = ordered
        .iter()
        .enumerate()
        .filter(|(i, _)| !index.by_script.contains_key(i))
        .map(|(_, segment)| segment)
        .collect();
    assert!(super::super::super::page_column_corridor_for_segments(ordinary.iter().copied()).is_none());
    let bands = super::super::super::paragraph_line_bands(ordered.iter());
    let height = super::super::super::median_line_height(&bands).unwrap();
    let leading = super::super::super::body_leading(&bands, height);
    let local = super::super::super::paragraph_line_bands(ordinary.into_iter());
    let local_gaps = super::super::super::paragraph_gaps_with_leading(&local, height, leading);
    for gaps in [&case.gaps, &local_gaps] {
        assert!(
            gaps.iter()
                .any(|gap| *gap < upper.baseline_y && *gap > lower.baseline_y)
        );
    }
    let pair = super::super::super::paragraph_line_bands([upper, lower].into_iter());
    let actual_pair_gaps = super::super::super::paragraph_gaps_with_leading(&pair, height, leading);
    assert_eq!(
        !actual_pair_gaps.is_empty(),
        endpoint_vote,
        "original thresholds applied to the ACTUAL endpoint pair"
    );
}

#[test]
fn should_decline_a_page_gap_witness_anchored_by_a_foreign_column_2275() {
    let case = source_case(
        "openstax-introductory-business-statistics-1e-p362",
        "dcf927a2900537d64df1b3484af3894d88c38fdbbc05aa82957d228c431016c4",
    );
    let (ordered, index) = case.ordered();
    let upper = &ordered[source_position(&index, 159)];
    let lower = &ordered[source_position(&index, 160)];
    let foreign = &ordered[source_position(&index, 227)];
    assert_eq!(upper.text, "n");
    assert!(upper.is_italic);
    assert!(lower.text.starts_with("how far away"));
    assert_eq!(foreign.text, "Proportion");
    assert!(foreign.x > lower.upright_advance_extent().1);
    assert!(foreign.baseline_y > upper.baseline_y && upper.baseline_y > lower.baseline_y);
    assert_band_edges(&ordered, &index, 159, 227, foreign.baseline_y);
    assert_gap_evidence(&case, 159, 160, false);
    assert!(!crate::pdf::native::table::is_dense_numeric_row(
        lower.text.split_whitespace()
    ));
    assert!(!super::super::super::heading_line_ran_out_of_room(
        upper,
        lower,
        lower.upright_advance_extent().1
    ));
    case.assert_joined(159, 160);
}

#[test]
fn should_decline_a_boundary_claim_from_an_incomplete_physical_source_row_2275() {
    let case = source_case(
        "gutenberg-38769-p74",
        "8843c419ae6e36aa019af4b6a1155bf798fd589463df572b188f88bc402f6b77",
    );
    let (ordered, index) = case.ordered();
    let upper_position = source_position(&index, 179);
    let lower_position = source_position(&index, 180);
    let ordinal_position = source_position(&index, 184);
    let upper = &ordered[upper_position];
    let lower = &ordered[lower_position];
    let ordinal = &ordered[ordinal_position];
    let tail = &ordered[source_position(&index, 187)];
    assert_eq!(upper.text, "m");
    assert_eq!(ordinal.text, "th");
    assert_eq!(tail.text, ", whose");
    assert!(lower.text.starts_with("coefficients are rational functions"));
    assert!(
        !index.by_script.contains_key(&ordinal_position),
        "actual raised ordinal is unowned"
    );
    assert!(ordinal_position > lower_position && source_position(&index, 185) > lower_position);
    assert!(ordinal.baseline_y > upper.baseline_y && ordinal.font_size < upper.font_size);
    assert_eq!(
        tail.baseline_y, upper.baseline_y,
        "delayed tail belongs to the same physical source row"
    );
    assert!(tail.upright_advance_extent().1 > upper.upright_advance_extent().1);
    let grouped =
        super::super::super::blocks_to_paragraphs_with_native_scripts(ordered.clone(), &[], &case.gaps, Some(&index));
    let paragraph = &grouped[paragraph_with(&grouped, upper)];
    assert_eq!(
        fingerprint(paragraph.lines.last().unwrap().segments.last().unwrap()),
        fingerprint(upper)
    );
    assert_band_edges(&ordered, &index, 176, 187, ordinal.baseline_y);
    assert_gap_evidence(&case, 179, 180, false);
    case.assert_joined(179, 180);
}

#[test]
fn should_keep_a_same_scope_ragged_wrap_with_a_short_next_word_2275() {
    let mut spans = unrelated_power();
    spans.extend([
        span("a", 80.0, 750.0, 8.0, 12.0),
        span("b", 80.0, 742.0, 8.0, 12.0),
        span("ordinary source prose continues", 80.0, 700.0, 120.0, 12.0),
        span("and the ordinary account continues here", 80.0, 680.8, 180.0, 12.0),
    ]);
    let expected = spans.len();
    let (segments, evidence) = carrier(spans);
    assert_eq!(segments.len(), expected);
    let case = BoundaryCase::new(segments, evidence);
    let upper = &case.segments[5];
    let lower = &case.segments[6];
    assert_eq!(upper.text, "ordinary source prose continues");
    assert_eq!(lower.text, "and the ordinary account continues here");
    let right = lower.upright_advance_extent().1;
    assert!(!super::super::super::heading_fills_column(upper, right));
    assert!(!super::super::super::heading_line_ran_out_of_room(upper, lower, right));
    for row in [upper, lower] {
        assert!(!crate::pdf::native::table::is_dense_numeric_row(
            row.text.split_whitespace()
        ));
    }
    let upper_id = case.evidence.original_indices[5];
    let lower_id = case.evidence.original_indices[6];
    assert_gap_evidence(&case, upper_id, lower_id, true);
    case.assert_joined(upper_id, lower_id);
}
