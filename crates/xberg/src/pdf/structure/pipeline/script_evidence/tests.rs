use super::*;
use crate::pdf::native::hierarchy::{ArtifactRetention, script_evidence::NativePageSegments};
use xberg_native_pdf::{geometry::Rect, layout::TextSpan};

fn source(text: &str, x: f32, y: f32, width: f32, size: f32) -> TextSpan {
    TextSpan {
        text: text.to_owned(),
        bbox: Rect::new(x, y, width, size),
        font_size: size,
        ..Default::default()
    }
}

fn captured(spans: Vec<TextSpan>) -> NativePageSegments {
    NativePageSegments::from_final_spans(spans, &std::collections::HashMap::new(), ArtifactRetention::default())
}

fn paragraphs(page: &NativePageSegments, segments: Vec<SegmentData>) -> Vec<PdfParagraph> {
    native_paragraphs(segments, page, &[], &[]).expect("native source projection")
}

#[test]
fn should_group_proved_suffix_without_changing_raw_coordinates_or_other_paragraphs() {
    let page = captured(vec![
        source("1", 140.0, 704.5, 4.0, 8.0),
        source("Author", 100.0, 700.0, 40.0, 12.0),
        source("Independent paragraph.", 100.0, 630.0, 120.0, 12.0),
    ]);
    let gaps = super::super::compute_paragraph_gap_ys(&page.segments);
    let ordinary = super::super::blocks_to_paragraphs(page.segments.clone(), &[], &gaps);
    assert_eq!(ordinary.len(), 3, "production gaps preserve the independent paragraph");
    let result = native_paragraphs(page.segments.clone(), &page, &[], &gaps).expect("proved native suffix");
    assert_eq!(result.len(), 2);
    assert_eq!(result[0].lines.len(), 1);
    assert_eq!(
        result[0].lines[0]
            .segments
            .iter()
            .map(|s| s.text.as_str())
            .collect::<Vec<_>>(),
        ["Author", "1"]
    );
    assert_eq!(
        SegmentKey::from(&result[0].lines[0].segments[1]),
        SegmentKey::from(&page.segments[0])
    );
    let document =
        crate::pdf::structure::assembly::assemble_internal_document(vec![result], &[], None, &[], &Default::default());
    assert_eq!(document.elements.len(), 2);
    assert_eq!(document.elements[0].text, "Author1");
    assert_eq!(document.elements[0].annotations.len(), 1);
    assert_eq!(document.elements[0].annotations[0].start, 6);
    assert_eq!(document.elements[0].annotations[0].end, 7);
    assert_eq!(document.elements[1].text, "Independent paragraph.");
}

#[test]
fn should_leave_prefix_order_and_paragraph_partition_unchanged() {
    let page = captured(vec![
        source("1", 90.0, 704.5, 4.0, 8.0),
        source("University", 95.0, 700.0, 70.0, 12.0),
    ]);
    let legacy = super::super::blocks_to_paragraphs(page.segments.clone(), &[], &[]);
    assert!(native_paragraphs(page.segments.clone(), &page, &[], &[]).is_none());
    assert_eq!(legacy.len(), 2);
    assert_eq!(legacy[0].lines[0].segments[0].text, "1");
    assert_eq!(legacy[1].lines[0].segments[0].text, "University");
}

#[test]
fn should_reject_removed_table_endpoint_and_mutated_projection() {
    let page = captured(vec![
        source("Author", 100.0, 700.0, 40.0, 12.0),
        source("1", 140.0, 704.5, 4.0, 8.0),
    ]);
    assert!(native_paragraphs(page.segments[1..].to_vec(), &page, &[], &[]).is_none());
    let mut changed = page.segments.clone();
    changed[0].assigned_role = Some(2);
    assert!(native_paragraphs(changed, &page, &[], &[]).is_none());
}

#[test]
fn should_leave_existing_in_paragraph_scripts_in_legacy_lane() {
    let page = captured(vec![
        source("variable", 100.0, 700.0, 40.0, 12.0),
        source("2", 140.0, 703.0, 4.0, 8.0),
    ]);
    assert!(native_paragraphs(page.segments.clone(), &page, &[], &[]).is_none());
    let legacy = super::super::blocks_to_paragraphs(page.segments, &[], &[]);
    let document =
        crate::pdf::structure::assembly::assemble_internal_document(vec![legacy], &[], None, &[], &Default::default());
    assert_eq!(document.elements.len(), 1);
    assert_eq!(document.elements[0].text, "variable2");
    assert_eq!(document.elements[0].annotations.len(), 1);
}

#[test]
fn should_preserve_untouched_paragraph_when_its_subset_creates_a_column_corridor() {
    let mut left = source("Left prose remains together", 50.0, 650.0, 60.0, 12.0);
    left.font_weight = xberg_native_pdf::layout::FontWeight::Bold;
    let mut right = source("with its second line.", 140.0, 638.0, 60.0, 12.0);
    right.font_weight = xberg_native_pdf::layout::FontWeight::Bold;
    let page = captured(vec![
        source("1", 140.0, 704.5, 4.0, 8.0),
        source("Author", 100.0, 700.0, 40.0, 12.0),
        left,
        right,
    ]);
    assert_eq!(super::super::page_column_corridor(&page.segments), None);
    let ordinary = super::super::blocks_to_paragraphs(page.segments.clone(), &[], &[]);
    assert_eq!(
        ordinary.len(),
        3,
        "the original page has marker, author and one unrelated paragraph"
    );
    let original = &ordinary[2];
    assert_eq!(original.lines.len(), 2);
    let subset: Vec<_> = original
        .lines
        .iter()
        .flat_map(|line| line.segments.iter().cloned())
        .collect();
    assert_eq!(super::super::page_column_corridor(&subset), Some((110.0, 140.0)));

    let result = paragraphs(&page, page.segments.clone());
    assert_eq!(
        result[0].lines[0].native_scripts.len(),
        1,
        "a proved suffix must activate the native lane"
    );
    let untouched: Vec<_> =
        result
            .iter()
            .filter(|paragraph| {
                paragraph.lines.iter().flat_map(|line| &line.segments).any(|segment| {
                    segment.text == "Left prose remains together" || segment.text == "with its second line."
                })
            })
            .collect();
    assert_eq!(
        untouched.len(),
        1,
        "an unrelated original paragraph must not be reconstructed as separate columns"
    );
    assert_eq!(
        format!("{:?}", untouched[0]),
        format!("{original:?}"),
        "retain every original paragraph and line field"
    );
}

#[test]
fn should_attach_after_refinement_without_changing_existing_line_or_paragraph_semantics() {
    let page = captured(vec![
        source("1", 140.0, 704.5, 4.0, 8.0),
        source("Author", 100.0, 700.0, 40.0, 12.0),
    ]);
    let mut result = super::super::blocks_to_paragraphs(page.segments.clone(), &[], &[]);
    assert_eq!(result.len(), 2);
    result[1].text = "Author".to_owned();
    result[1].heading_level = Some(2);
    result[1].is_list_item = true;
    result[1].is_bold = true;
    result[1].block_bbox = Some((100.0, 700.0, 140.0, 712.0));
    let original = result[1].clone();
    assert!(associate_surviving_suffixes(&mut result, &page));
    assert_eq!(result.len(), 1);
    assert_eq!(result[0].heading_level, original.heading_level);
    assert_eq!(result[0].is_list_item, original.is_list_item);
    assert_eq!(result[0].is_bold, original.is_bold);
    assert_eq!(result[0].dominant_font_size, original.dominant_font_size);
    assert_eq!(result[0].lines[0].baseline_y, original.lines[0].baseline_y);
    assert_eq!(
        result[0].lines[0].dominant_font_size,
        original.lines[0].dominant_font_size
    );
    assert_eq!(result[0].lines[0].is_bold, original.lines[0].is_bold);
    assert_eq!(result[0].lines[0].is_monospace, original.lines[0].is_monospace);
    assert!(result[0].text.is_empty(), "assembly must read the updated records");
    assert_eq!(result[0].word_count, 2);
    assert_eq!(result[0].lines[0].valid_native_scripts().len(), 1);
    assert_eq!(result[0].block_bbox, Some((100.0, 700.0, 144.0, 712.5)));
}

#[test]
fn should_reject_complete_run_when_later_member_has_incompatible_furniture() {
    let page = captured(vec![
        source("1", 140.0, 704.5, 4.0, 8.0),
        source("2", 144.0, 704.5, 4.0, 8.0),
        source("Author", 100.0, 700.0, 40.0, 12.0),
    ]);
    let mut result = super::super::blocks_to_paragraphs(page.segments.clone(), &[], &[]);
    let marker = result[0].lines[0].segments.pop().expect("later marker");
    assert_eq!(marker.text, "2");
    let mut separate = result[0].clone();
    separate.lines[0].segments = vec![marker];
    separate.is_page_furniture = true;
    result.push(separate);
    let original = format!("{result:?}");
    assert!(!associate_surviving_suffixes(&mut result, &page));
    assert_eq!(format!("{result:?}"), original);
}

#[test]
fn should_reject_surviving_endpoints_in_distinct_layout_ancestry() {
    use crate::pdf::structure::types::{LayoutRegionPath, LayoutRegionTag};
    let page = captured(vec![
        source("1", 140.0, 704.5, 4.0, 8.0),
        source("Author", 100.0, 700.0, 40.0, 12.0),
    ]);
    let mut result = super::super::blocks_to_paragraphs(page.segments.clone(), &[], &[]);
    result[0].layout_region_path = Some(LayoutRegionPath {
        root: LayoutRegionTag {
            id: 1,
            class_name: None,
        },
        child: None,
    });
    let original = format!("{result:?}");
    assert!(!associate_surviving_suffixes(&mut result, &page));
    assert_eq!(format!("{result:?}"), original);
}

#[test]
fn should_preserve_existing_subscript_and_utf8_offsets_after_inserting_a_long_native_run() {
    use crate::types::document_structure::AnnotationKind;
    let page = captured(vec![
        source("1", 140.0, 704.5, 0.25, 8.0),
        source(",", 140.25, 704.5, 0.25, 8.0),
        source("2", 140.5, 704.5, 0.25, 8.0),
        source("†", 140.75, 704.5, 0.25, 8.0),
        source("d", 100.0, 700.0, 40.0, 12.0),
        source("2", 136.0, 697.0, 4.0, 8.0),
    ]);
    assert_eq!(
        page.valid_owners(&page.segments).len(),
        4,
        "the complete raised run is proved"
    );
    let mut ordinary = super::super::blocks_to_paragraphs(page.segments.clone(), &[], &[]);
    let baseline = crate::pdf::structure::assembly::assemble_internal_document(
        vec![ordinary.clone()],
        &[],
        None,
        &[],
        &Default::default(),
    );
    let base = baseline
        .elements
        .iter()
        .find(|element| element.text == "d2")
        .expect("original base and subscript");
    let original: Vec<_> = base
        .annotations
        .iter()
        .filter(|annotation| matches!(annotation.kind, AnnotationKind::Subscript))
        .collect();
    assert_eq!(original.len(), 1);
    assert_eq!((original[0].start, original[0].end), (1, 2));
    let unchanged = format!("{ordinary:?}");
    let associated = associate_surviving_suffixes(&mut ordinary, &page);
    if !associated {
        assert_eq!(
            format!("{ordinary:?}"),
            unchanged,
            "rejected transfer must leave every record and field intact"
        );
    }
    let result = crate::pdf::structure::assembly::assemble_internal_document(
        vec![ordinary],
        &[],
        None,
        &[],
        &Default::default(),
    );
    let base = result
        .elements
        .iter()
        .find(|element| element.text.starts_with('d'))
        .expect("surviving base");
    let expected = match base.text.as_str() {
        "d2" => (1, 2),
        "d1,2†2" => (7, 8),
        "d1,2† 2" => (8, 9),
        other => panic!("unexpected base projection: {other:?}"),
    };
    let retained: Vec<_> = base
        .annotations
        .iter()
        .filter(|annotation| matches!(annotation.kind, AnnotationKind::Subscript))
        .collect();
    assert_eq!(retained.len(), 1, "native transfer must retain the existing subscript");
    assert_eq!((retained[0].start, retained[0].end), expected);
    assert_eq!(&base.text[expected.0 as usize..expected.1 as usize], "2");
}

#[test]
fn should_preserve_existing_subscript_when_a_native_suffix_enters_its_lookback() {
    use crate::types::document_structure::AnnotationKind;

    let mut subscript = source("q", 145.5, 704.05, 1.0, 6.0);
    subscript.bbox.height = 8.0;
    let page = captured(vec![
        source("é", 135.0, 706.5, 9.5, 12.0),
        source("x", 100.0, 700.0, 40.0, 12.0),
        subscript,
        source("ω", 140.0, 704.5, 4.0, 8.0),
    ]);
    assert_eq!(page.valid_owners(&page.segments).len(), 1);

    let mut base = super::super::blocks_to_paragraphs(vec![page.segments[0].clone()], &[], &[]).remove(0);
    let mut next_line = base.lines[0].clone();
    next_line.baseline_y = 700.0;
    next_line.segments = page.segments[1..3].to_vec();
    base.lines.push(next_line);
    let mut suffix = base.clone();
    suffix.lines.truncate(1);
    suffix.lines[0].baseline_y = 704.5;
    suffix.lines[0].segments = vec![page.segments[3].clone()];
    let mut paragraphs = vec![base, suffix];
    let assemble = |paragraphs| {
        crate::pdf::structure::assembly::assemble_internal_document(
            vec![paragraphs],
            &[],
            None,
            &[],
            &Default::default(),
        )
    };
    let original = assemble(paragraphs.clone());
    let original_base = &original.elements[0];
    assert_eq!(
        original_base
            .annotations
            .iter()
            .filter(|annotation| annotation.kind == AnnotationKind::Subscript)
            .map(|annotation| &original_base.text[annotation.start as usize..annotation.end as usize])
            .collect::<Vec<_>>(),
        ["q"],
        "the legacy base has a genuine subscript before transfer",
    );

    let untouched = format!("{paragraphs:?}");
    assert!(
        !associate_surviving_suffixes(&mut paragraphs, &page),
        "a same-kind replacement base must not erase the existing annotation",
    );
    assert_eq!(format!("{paragraphs:?}"), untouched);
    let after = assemble(paragraphs);
    let base = &after.elements[0];
    assert_eq!(base.text, original_base.text);
    assert_eq!(
        base.annotations
            .iter()
            .filter(|annotation| annotation.kind == AnnotationKind::Subscript)
            .map(|annotation| &base.text[annotation.start as usize..annotation.end as usize])
            .collect::<Vec<_>>(),
        ["q"],
        "a same-kind replacement base must not erase the existing annotation",
    );
}
