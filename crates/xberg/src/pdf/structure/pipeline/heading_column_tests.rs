use super::{
    BODY_SIZE_BOLD_ALIGNMENT_TOLERANCE_EM, MIN_BODY_SIZE_BOLD_SIGNALS, PdfParagraph, SegmentData,
    is_body_size_bold_heading_candidate, page_column_corridor_for_segments, paragraph_text_raw,
    promote_repeated_body_size_bold_headings, shares_same_text_row,
};
use crate::pdf::structure::types::PdfLine;

const BODY_FONT: f32 = 10.0;
const BOLD_LEAD: &str = "A highlighted result continues through the remaining";
const HEADING: &str = "Genuine repeated section heading";

fn source_line(text: &str, bold: bool, bbox: (f32, f32, f32, f32)) -> PdfLine {
    let (left, bottom, right, top) = bbox;
    PdfLine {
        segments: vec![SegmentData {
            text: text.to_owned(),
            x: left,
            y: bottom,
            width: right - left,
            height: top - bottom,
            font_size: BODY_FONT,
            is_bold: bold,
            is_italic: false,
            is_monospace: false,
            baseline_y: bottom,
            rotation_degrees: 0.0,
            assigned_role: None,
        }],
        native_source_origin: None,
        native_scripts: Vec::new(),
        baseline_y: bottom,
        dominant_font_size: BODY_FONT,
        is_bold: bold,
        is_monospace: false,
    }
}

fn paragraph(text: &str, bold: bool, bbox: (f32, f32, f32, f32)) -> PdfParagraph {
    let lines = vec![source_line(text, bold, bbox)];
    let word_count = PdfParagraph::compute_word_count("", &lines);
    PdfParagraph {
        text: String::new(),
        lines,
        dominant_font_size: BODY_FONT,
        heading_level: None,
        is_bold: bold,
        is_list_item: false,
        is_code_block: false,
        is_formula: false,
        is_page_furniture: false,
        layout_class: None,
        layout_region_path: None,
        caption_for: None,
        block_bbox: Some(bbox),
        word_count,
    }
}

fn two_column_pages(offset: f32, genuine_heading: bool) -> Vec<Vec<PdfParagraph>> {
    (0..MIN_BODY_SIZE_BOLD_SIGNALS)
        .map(|_| {
            let candidate = paragraph(
                if genuine_heading { HEADING } else { BOLD_LEAD },
                true,
                (60.0 + offset, 380.0, 300.0 + offset, 392.0),
            );
            let foreign = paragraph(
                if genuine_heading {
                    "Foreign column emphasized statement"
                } else {
                    "A separate column contains its own ordinary discussion without completing the highlighted left column sentence."
                },
                genuine_heading,
                (330.0 + offset, 370.0, 570.0 + offset, 385.0),
            );
            let continuation = paragraph(
                if genuine_heading {
                    "The genuine section opens substantive ordinary body content here."
                } else {
                    "argument."
                },
                false,
                (50.0 + offset, 368.0, 290.0 + offset, 378.0),
            );
            vec![candidate, foreign, continuation]
        })
        .collect()
}

fn assert_two_column_input(pages: &[Vec<PdfParagraph>], offset: f32) {
    assert_eq!(pages.len(), MIN_BODY_SIZE_BOLD_SIGNALS);
    for page in pages {
        assert_eq!(page.len(), 3);
        assert!(is_body_size_bold_heading_candidate(&page[0], BODY_FONT));
        assert!(!shares_same_text_row(&page[0], &page[1], true));
        assert_eq!(
            page_column_corridor_for_segments(page.iter().flat_map(|p| &p.lines).flat_map(|line| &line.segments)),
            Some((300.0 + offset, 330.0 + offset)),
            "source geometry must establish the existing column corridor before promotion"
        );
    }
}

fn promote_and_check_sources(pages: &mut [Vec<PdfParagraph>], expected: Option<u8>) {
    let source = |pages: &[Vec<PdfParagraph>]| {
        pages
            .iter()
            .flatten()
            .map(|p| {
                (
                    paragraph_text_raw(p),
                    format!("{:?}", p.lines),
                    p.block_bbox,
                    p.word_count,
                )
            })
            .collect::<Vec<_>>()
    };
    let before = source(pages);
    assert!(!before.is_empty());
    promote_repeated_body_size_bold_headings(pages, Some(BODY_FONT));
    assert_eq!(
        source(pages),
        before,
        "classification must preserve source text, geometry and paragraph count"
    );
    assert_eq!(
        pages.iter().map(|page| page[0].heading_level).collect::<Vec<_>>(),
        vec![expected; MIN_BODY_SIZE_BOLD_SIGNALS],
        "heading evidence must come from the candidate's own column"
    );
    assert!(
        pages
            .iter()
            .all(|page| page.iter().skip(1).all(|p| p.heading_level.is_none()))
    );
}

#[test]
fn foreign_column_body_must_not_promote_a_wrapped_bold_body_line() {
    let mut pages = two_column_pages(0.0, false);
    assert_two_column_input(&pages, 0.0);
    promote_and_check_sources(&mut pages, None);
}

#[test]
fn foreign_column_signal_must_not_hide_a_genuine_same_column_heading() {
    let mut pages = two_column_pages(0.0, true);
    assert_two_column_input(&pages, 0.0);
    promote_and_check_sources(&mut pages, Some(3));
}

#[test]
fn foreign_column_heading_decisions_are_translation_invariant() {
    for offset in [-420.0, 2400.0] {
        let mut pages = two_column_pages(offset, false);
        assert_two_column_input(&pages, offset);
        promote_and_check_sources(&mut pages, None);
    }
}

fn same_column_pages(offset: f32, lead: &str, tail: &str, indent: f32) -> Vec<Vec<PdfParagraph>> {
    (0..MIN_BODY_SIZE_BOLD_SIGNALS)
        .map(|_| {
            vec![
                paragraph(lead, true, (60.0 + offset, 380.0, 300.0 + offset, 392.0)),
                paragraph(tail, false, (60.0 + offset + indent, 350.0, 310.0 + offset, 360.0)),
            ]
        })
        .collect()
}

#[test]
fn genuine_same_column_headings_remain_valid_after_translation() {
    for offset in [0.0, 720.0] {
        let mut pages = same_column_pages(
            offset,
            HEADING,
            "This section opens an ordinary substantive body paragraph.",
            0.0,
        );
        assert!(
            pages
                .iter()
                .all(|page| is_body_size_bold_heading_candidate(&page[0], BODY_FONT))
        );
        promote_and_check_sources(&mut pages, Some(3));
    }
}

#[test]
fn a_short_same_column_continuation_keeps_a_bold_body_line_as_body() {
    let mut pages = same_column_pages(0.0, BOLD_LEAD, "argument.", 0.0);
    assert!(
        pages
            .iter()
            .all(|page| is_body_size_bold_heading_candidate(&page[0], BODY_FONT))
    );
    promote_and_check_sources(&mut pages, None);
}

#[test]
fn genuine_outdented_headings_keep_the_existing_alignment_policy() {
    let indent = BODY_FONT * BODY_SIZE_BOLD_ALIGNMENT_TOLERANCE_EM + BODY_FONT;
    let mut pages = same_column_pages(
        0.0,
        HEADING,
        "This section opens an ordinary substantive body paragraph.",
        indent,
    );
    assert!(
        pages
            .iter()
            .all(|page| is_body_size_bold_heading_candidate(&page[0], BODY_FONT))
    );
    promote_and_check_sources(&mut pages, Some(3));
}
