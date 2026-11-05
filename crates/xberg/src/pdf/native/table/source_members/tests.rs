use super::*;
use ahash::AHashSet;
use xberg_native_pdf::geometry::Rect;
use xberg_native_pdf::structure::table_extractor::TableRow;

const FONT: f32 = 8.966_376;

fn glyph(text: &str, x: f32, y: f32, width: f32) -> SegmentData {
    SegmentData {
        text: text.into(),
        x,
        y,
        width,
        height: FONT,
        font_size: FONT,
        is_bold: false,
        is_italic: false,
        is_monospace: false,
        baseline_y: y,
        rotation_degrees: 0.0,
        assigned_role: None,
    }
}

fn tick(x: f32, y: f32, last: &str) -> Vec<SegmentData> {
    vec![
        glyph("0", x, y, 4.106_600_8),
        glyph(".", x + 4.112_152, y, 2.429_888_2),
        glyph(last, x + 6.550_903_3, y, 4.106_600_8),
    ]
}

fn span(text: &str, y: f32) -> TextSpan {
    TextSpan {
        text: text.into(),
        bbox: Rect {
            x: 444.646_36,
            y,
            width: 10.657_501,
            height: 8.966_37,
        },
        font_size: FONT,
        ..Default::default()
    }
}

fn native_cell() -> TableCell {
    TableCell {
        text: "0.6\n0.4".into(),
        spans: vec![span("0.6", 663.616_94), span("0.4", 633.722_05)],
        colspan: 1,
        rowspan: 1,
        mcids: vec![],
        bbox: Some(Rect {
            x: 444.646_36,
            y: 633.722_05,
            width: 10.657_501,
            height: 38.861_267,
        }),
        is_header: false,
    }
}

fn fixture() -> (Vec<SegmentData>, TableSources, Table) {
    let native = NativeTable {
        rows: vec![TableRow {
            cells: vec![native_cell()],
            is_header: false,
        }],
        has_header: false,
        col_count: 1,
        bbox: None,
    };
    let table = Table {
        cells: vec![vec!["0.6 0.4".into()]],
        markdown: "|0.6 0.4|".into(),
        page_number: 1,
        bounding_box: Some(BoundingBox {
            x0: 440.0,
            y0: 630.0,
            x1: 460.0,
            y1: 675.0,
        }),
        ..Default::default()
    };
    let source = capture(&native, &table, None).unwrap();
    let mut segments = tick(350.686_37, 645.023_44, "6");
    segments.extend(tick(444.646_36, 663.616_94, "6"));
    segments.extend(tick(444.646_36, 633.722_05, "4"));
    (segments, source, table)
}

fn owned(segments: Vec<SegmentData>, source: &TableSources, table: Table) -> AHashSet<SegmentKey> {
    project(&[segments], &[source], &[table])
        .remove(&0)
        .unwrap_or_default()
        .contained
        .into_iter()
        .flatten()
        .collect()
}

#[test]
fn exact_native_cell_runs_own_right_members_without_borrowing_equal_left_tick() {
    let (segments, source, table) = fixture();
    let keys = owned(segments.clone(), &source, table);
    assert_eq!(keys.len(), 6);
    assert!(
        segments[..3]
            .iter()
            .all(|segment| !keys.contains(&SegmentKey::from(segment)))
    );
    assert!(
        segments[3..]
            .iter()
            .all(|segment| keys.contains(&SegmentKey::from(segment)))
    );
    let mut reversed = segments;
    reversed.reverse();
    assert_eq!(
        keys,
        owned(
            reversed,
            &source,
            Table {
                cells: source.cells.clone(),
                markdown: source.markdown.clone(),
                page_number: source.page,
                bounding_box: Some(BoundingBox {
                    x0: 440.0,
                    y0: 630.0,
                    x1: 460.0,
                    y1: 675.0
                }),
                ..Default::default()
            }
        )
    );
}

#[test]
fn selective_ownership_preserves_all_nine_visible_glyphs_without_orphan_zero() {
    let (segments, source, table) = fixture();
    let keys = owned(segments.clone(), &source, table.clone());
    let kept = segments
        .iter()
        .filter(|segment| !keys.contains(&SegmentKey::from(*segment)))
        .map(|segment| segment.text.as_str())
        .collect::<String>();
    assert_eq!(kept, "0.6");
    let text = kept + &table.cells[0][0];
    for (glyph, expected) in [('0', 3), ('.', 3), ('6', 2), ('4', 1)] {
        assert_eq!(
            text.matches(glyph).count(),
            expected,
            "complete affected source inventory for {glyph}"
        );
    }
    let mixed_paragraph = "0.6 0".to_owned() + &table.cells[0][0];
    assert_eq!(
        mixed_paragraph.matches('0').count(),
        4,
        "whole-paragraph retention duplicates the owned zero"
    );
}

#[test]
fn changed_or_ambiguous_source_membership_abstains_for_the_entire_cell() {
    for mutation in 0..8 {
        let (mut segments, source, table) = fixture();
        match mutation {
            0 => {
                segments[6].text = "9".into();
            }
            1 => {
                segments.remove(7);
            }
            2 => segments.push(segments[6].clone()),
            3 => {
                segments[6].font_size += 1.0;
            }
            4 => {
                segments[6].rotation_degrees = 90.0;
            }
            5 => {
                segments[6].baseline_y += 1.0;
            }
            6 => {
                segments[6].x = f32::NAN;
            }
            _ => {
                segments[6].width = 0.0;
            }
        }
        assert!(
            owned(segments, &source, table).is_empty(),
            "unsupported or ambiguous source case {mutation}"
        );
    }
}

#[test]
fn partially_overlapping_or_extra_visible_source_glyphs_prevent_complete_run_proof() {
    for extra in [glyph("9", 445.0, 634.0, 1.0), glyph("9", 443.0, 633.722_05, 4.0)] {
        let (mut segments, source, table) = fixture();
        segments.push(extra);
        assert!(owned(segments, &source, table).is_empty());
    }
}

#[test]
fn changed_emitted_identity_cannot_authorize_source_removal() {
    for mutation in 0..4 {
        let (segments, source, mut table) = fixture();
        match mutation {
            0 => table.page_number = 2,
            1 => table.cells[0][0] = "0.6".into(),
            2 => table.markdown.clear(),
            _ => table.bounding_box.as_mut().unwrap().x0 += 1.0,
        }
        assert!(owned(segments, &source, table).is_empty());
    }
}

#[test]
fn conflicting_table_or_cell_claims_do_not_choose_an_arbitrary_owner() {
    let (segments, mut source, table) = fixture();
    assert!(
        project(
            std::slice::from_ref(&segments),
            &[&source],
            &[table.clone(), table.clone()]
        )
        .is_empty()
    );
    source.members.push(source.members[0].clone());
    assert!(owned(segments, &source, table).is_empty());
}

#[test]
fn unavailable_rewritten_or_spanning_native_membership_is_not_captured() {
    for mutation in 0..5 {
        let mut cell = native_cell();
        match mutation {
            0 => cell.spans.clear(),
            1 => cell.bbox = None,
            2 => cell.rowspan = 2,
            3 => cell.spans[0].rotation_degrees = 90.0,
            _ => cell.spans[0].text_rise = 1.0,
        }
        assert!(cell_members(&cell, "0.6 0.4").is_none());
    }
    assert!(
        cell_members(&native_cell(), "0.6").is_none(),
        "rewritten cell cannot carry the complete runs"
    );
}

#[test]
fn source_limits_abstain_without_selecting_partial_proofs() {
    let (segments, source, table) = fixture();
    let mut at_limit = vec![segments[0].clone(); projection::MAX_PAGE_SEGMENTS - segments.len()];
    at_limit.extend(segments.clone());
    assert_eq!(
        owned(at_limit.clone(), &source, table.clone()).len(),
        6,
        "the exact segment cap still permits proof"
    );
    at_limit.push(segments[0].clone());
    assert!(owned(at_limit, &source, table.clone()).is_empty());
    let mut excessive_text = segments;
    excessive_text.push(glyph(&"x".repeat(projection::MAX_PROOF_WORK), 10.0, 10.0, 1.0));
    assert!(
        owned(excessive_text, &source, table).is_empty(),
        "text budget exhaustion discards all claims"
    );
}

#[test]
fn source_capture_refuses_unbounded_metadata_or_native_run_lists() {
    let (_, _, mut table) = fixture();
    table.markdown = "x".repeat(MAX_TABLE_TEXT_BYTES + 1);
    assert!(table_text_size(&table).is_none());
    table.markdown.clear();
    table.cells = vec![vec![String::new(); MAX_TABLE_CELLS + 1]];
    assert!(table_text_size(&table).is_none());
    let mut native = NativeTable {
        rows: vec![TableRow {
            cells: vec![native_cell()],
            is_header: false,
        }],
        ..Default::default()
    };
    native.rows[0].cells[0].spans = vec![span("0.4", 633.722_05); MAX_TABLE_RUNS + 1];
    assert!(source_size(&native).is_none());
    native.rows[0].cells = (0..5)
        .map(|_| {
            let mut cell = native_cell();
            cell.colspan = 1000;
            cell
        })
        .collect();
    assert!(
        source_size(&native).is_none(),
        "span-grid workspace is bounded before projection"
    );
}

#[cfg(feature = "tokio-runtime")]
fn reference_projection() -> (Vec<SegmentData>, AHashSet<SegmentKey>) {
    use sha2::{Digest, Sha256};
    let path = std::env::var_os("XBERG_2443_SOURCE")
        .map(std::path::PathBuf::from)
        .unwrap_or_else(|| {
            std::path::Path::new(env!("CARGO_MANIFEST_DIR"))
                .join("../../test_documents/pdf/regression_pages/arxiv-2302.00681v1-p3.pdf")
        });
    let bytes = std::fs::read(&path).unwrap_or_else(|error| panic!("required source {}: {error}", path.display()));
    assert_eq!(
        hex::encode(Sha256::digest(&bytes)),
        "1508aa9a0e12e8aad8bbc594a99ded16ce7517518e2689fc6002e59263c3c005"
    );
    let mut doc = crate::pdf::native::NativeDocument::open_bytes(&bytes).unwrap();
    let tables = crate::pdf::native::table::extract_ruled_tables(&doc).unwrap().tables;
    assert_eq!(
        tables.len(),
        1,
        "the actual Native detector remains the source of this cell"
    );
    let sources: Vec<_> = tables
        .iter()
        .filter_map(|table| table.source_members.as_ref())
        .collect();
    assert_eq!(sources.len(), 1, "Native cell membership was retained");
    let (pages, _) = crate::pdf::native::hierarchy::script_evidence::extract_all_segment_carriers(
        &mut doc,
        crate::pdf::native::hierarchy::ArtifactRetention::default(),
    )
    .unwrap();
    assert_eq!(pages.len(), 1);
    let segments = pages.into_iter().next().unwrap().segments;
    let emitted: Vec<_> = tables.iter().map(|table| table.table.clone()).collect();
    let keys = project(std::slice::from_ref(&segments), &sources, &emitted)
        .remove(&0)
        .expect("actual source membership proof");
    (segments, keys.contained.into_iter().flatten().collect())
}

#[cfg(feature = "tokio-runtime")]
#[test]
fn actual_native_membership_selects_the_right_runs_and_preserves_all_left_glyphs() {
    let (segments, keys) = reference_projection();
    for (text, left, baseline, expected) in [
        ("0.6", 350.686_37f32, 645.023_44f32, false),
        ("0.6", 444.646_36, 663.616_94, true),
        ("0.4", 444.646_36, 633.722_05, true),
    ] {
        let mut glyphs: Vec<_> = segments
            .iter()
            .filter(|segment| segment.y.to_bits() == baseline.to_bits() && segment.x >= left && segment.x < left + 12.0)
            .collect();
        glyphs.sort_by(|left, right| left.x.total_cmp(&right.x));
        assert_eq!(glyphs.len(), 3);
        assert_eq!(
            glyphs.iter().map(|segment| segment.text.as_str()).collect::<String>(),
            text
        );
        assert!(
            glyphs
                .iter()
                .all(|segment| keys.contains(&SegmentKey::from(*segment)) == expected)
        );
    }
}

#[test]
fn a_synthetic_run_union_cannot_authorize_a_raised_source_member() {
    let (mut segments, mut source, table) = fixture();
    segments[7].y += 1.0;
    segments[7].baseline_y = segments[7].y;
    source.members[0][1].bounds.top += 1.0;
    assert!(
        owned(segments, &source, table).is_empty(),
        "a complete box union does not establish a common source baseline"
    );
}

#[test]
fn explicit_source_whitespace_cannot_be_borrowed_by_a_single_native_word() {
    let (mut segments, source, table) = fixture();
    segments.push(glyph(" ", 448.754, 633.722_05, 0.003));
    assert!(
        owned(segments, &source, table).is_empty(),
        "explicit source word boundaries must not disappear during run matching"
    );
}

#[test]
fn a_cell_run_outside_the_emitted_table_cannot_authorize_source_removal() {
    let (segments, _, mut table) = fixture();
    let native = NativeTable {
        rows: vec![TableRow {
            cells: vec![native_cell()],
            is_header: false,
        }],
        ..Default::default()
    };
    let run_left = f64::from(native.rows[0].cells[0].spans[0].bbox.x);
    table.bounding_box.as_mut().unwrap().x0 = run_left;
    let source = capture(&native, &table, None).expect("exact boundary contact qualifies");
    assert_eq!(owned(segments.clone(), &source, table.clone()).len(), 6);

    for left in [run_left + 1e-8, 456.0] {
        table.bounding_box.as_mut().unwrap().x0 = left;
        let keys = capture(&native, &table, None)
            .map(|source| owned(segments.clone(), &source, table.clone()))
            .unwrap_or_default();
        assert!(keys.is_empty(), "source run crosses emitted table edge {left}");
    }
}

#[test]
fn recovered_line_certifies_complete_split_words_and_rejects_changed_boundaries() {
    let mut cell = native_cell();
    cell.spans = [("Technology", 20., 45.), (" Limited", 70., 30.)]
        .into_iter()
        .map(|(text, x, width)| TextSpan {
            text: text.into(),
            bbox: Rect::new(x, 100., width, FONT),
            font_size: FONT,
            ..Default::default()
        })
        .collect();
    cell.bbox = Some(Rect::new(10., 90., 100., 30.));
    let text = "Technology Limited";
    let lines = recovered_cell_lines(&cell, text).expect("exact original words in the emitted line");
    let (_, mut source, mut table) = fixture();
    table.cells = vec![vec![text.into()]];
    table.markdown = format!("| {text} |");
    table.bounding_box = Some(BoundingBox {
        x0: 10.,
        y0: 90.,
        x1: 110.,
        y1: 120.,
    });
    source.bbox = table_bounds(table.bounding_box).unwrap();
    source.cells = table.cells.clone();
    source.markdown = table.markdown.clone();
    source.members.clear();
    source.recovered_lines = lines;
    let exact = vec![glyph("T", 20., 100., 5.), glyph("echnology Limited", 25., 100., 75.)];
    let projected = |spans: Vec<SegmentData>| {
        project(&[spans], &[&source], std::slice::from_ref(&table))
            .remove(&0)
            .unwrap_or_default()
            .recovered_lines
    };
    assert_eq!(
        projected(exact.clone()),
        exact.iter().map(SegmentKey::from).collect::<Vec<_>>()
    );

    let mut different_words = exact.clone();
    different_words[1].text = "echnologyLimited".into();
    assert!(projected(different_words).is_empty());
    let mut crossing = exact.clone();
    crossing.push(glyph("Outside", 99., 100., 10.));
    assert!(projected(crossing).is_empty());
    let mut tagged = exact;
    tagged[0].assigned_role = Some(2);
    assert!(projected(tagged).is_empty());
}
