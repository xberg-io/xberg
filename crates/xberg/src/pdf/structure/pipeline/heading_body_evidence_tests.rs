use super::{
    FULL_LINE_FRACTION, MAX_OTHER_HEADING_RATIO, MIN_BODY_SIZE_BOLD_SIGNALS, PdfParagraph, SegmentData,
    body_size_heading_eligibility, heading_context::HeadingContext, heading_fills_column,
    is_body_size_bold_heading_candidate, paragraph_text_raw, promote_repeated_body_size_bold_headings,
    shares_same_text_row, spans_visual_line_break,
};
use crate::pdf::native::table::is_dense_numeric_row;
use crate::pdf::structure::{paragraphs::source_rows_continue, types::PdfLine};

mod fixtures;

const BODY_FONT: f32 = 10.0;
type Glyph<'a> = (&'a str, f32, f32, f32, bool, bool);

fn line(baseline: f32, font: f32, bold: bool, glyphs: &[Glyph<'_>]) -> PdfLine {
    PdfLine {
        segments: glyphs
            .iter()
            .map(|&(text, x, width, font_size, is_bold, is_italic)| SegmentData {
                text: text.to_owned(),
                x,
                y: baseline,
                width,
                height: font_size,
                font_size,
                is_bold,
                is_italic,
                is_monospace: false,
                baseline_y: baseline,
                rotation_degrees: 0.0,
                assigned_role: None,
            })
            .collect(),
        native_source_origin: None,
        native_scripts: Vec::new(),
        baseline_y: baseline,
        dominant_font_size: font,
        is_bold: bold,
        is_monospace: false,
    }
}

fn paragraph(lines: Vec<PdfLine>, font: f32, bold: bool) -> PdfParagraph {
    let bounds = lines.iter().flat_map(|line| &line.segments).fold(
        (f32::INFINITY, f32::INFINITY, f32::NEG_INFINITY, f32::NEG_INFINITY),
        |(left, bottom, right, top), span| {
            (
                left.min(span.x),
                bottom.min(span.y),
                right.max(span.x + span.width),
                top.max(span.y + span.height),
            )
        },
    );
    PdfParagraph {
        word_count: PdfParagraph::compute_word_count("", &lines),
        text: String::new(),
        lines,
        dominant_font_size: font,
        heading_level: None,
        is_bold: bold,
        is_list_item: false,
        is_code_block: false,
        is_formula: false,
        is_page_furniture: false,
        layout_class: None,
        layout_region_path: None,
        caption_for: None,
        block_bbox: Some(bounds),
    }
}

fn plain(text: &str, bold: bool, x: f32, y: f32, width: f32) -> PdfParagraph {
    paragraph(
        vec![line(y, BODY_FONT, bold, &[(text, x, width, BODY_FONT, bold, false)])],
        BODY_FONT,
        bold,
    )
}

fn active_pages(target_page: Vec<PdfParagraph>) -> Vec<Vec<PdfParagraph>> {
    assert_eq!((MIN_BODY_SIZE_BOLD_SIGNALS, MAX_OTHER_HEADING_RATIO), (3, 2));
    let mut pages = vec![target_page];
    for _ in 0..MIN_BODY_SIZE_BOLD_SIGNALS {
        pages.push(vec![
            plain("Independent genuine section heading", true, 60.0, 400.0, 220.0),
            plain(
                "This section opens substantive ordinary explanatory body content.",
                false,
                60.0,
                375.0,
                240.0,
            ),
        ]);
    }
    let mut prior_heading = plain("Already classified section", true, 60.0, 340.0, 200.0);
    prior_heading.heading_level = Some(2);
    pages.last_mut().unwrap().push(prior_heading);
    pages
}

fn assert_local_following(page: &[PdfParagraph], target: usize, expected: usize) {
    let mut context = HeadingContext::new(page);
    for (index, paragraph) in page.iter().enumerate().skip(target + 1).rev() {
        context.observe_content(index, 0, paragraph);
    }
    let global = &page[target + 1];
    let selected = context.following(target, 0, Some((0, global))).unwrap();
    assert_eq!(selected.0, 0);
    assert!(
        std::ptr::eq(selected.1, &page[expected]),
        "actual local source context must be selected"
    );
    assert!(
        !std::ptr::eq(selected.1, global),
        "foreign global following must be distinct"
    );
    assert!(!shares_same_text_row(&page[target], global, true));
}

fn source_fingerprint(pages: &[Vec<PdfParagraph>]) -> Vec<String> {
    pages
        .iter()
        .flatten()
        .map(|paragraph| {
            format!(
                "{:?}",
                (
                    paragraph_text_raw(paragraph),
                    &paragraph.lines,
                    paragraph.block_bbox,
                    paragraph.word_count,
                    paragraph.is_bold,
                    paragraph.is_formula,
                    paragraph.is_list_item,
                )
            )
        })
        .collect()
}

fn assert_valid_source(pages: &[Vec<PdfParagraph>]) {
    assert!(pages.iter().all(|page| !page.is_empty()));
    for paragraph in pages.iter().flatten() {
        assert!(paragraph.word_count > 0);
        assert!(paragraph.block_bbox.is_some_and(super::bbox_coordinates_are_finite));
        for span in paragraph.lines.iter().flat_map(|line| &line.segments) {
            assert!(!span.text.trim().is_empty());
            assert!(span.is_unrotated() && span.assigned_role.is_none());
            assert!(span.x.is_finite() && span.y.is_finite() && span.baseline_y.is_finite());
            assert!(span.width.is_finite() && span.width > 0.0 && span.font_size.is_finite() && span.font_size > 0.0);
        }
    }
}

fn promote_with_active_gate(pages: &mut [Vec<PdfParagraph>], target: usize, expected: Option<u8>) {
    assert_valid_source(pages);
    assert!(is_body_size_bold_heading_candidate(&pages[0][target], BODY_FONT));
    let eligibility = body_size_heading_eligibility(pages, BODY_FONT);
    let signals = eligibility.iter().flatten().filter(|&&eligible| eligible).count();
    let existing = pages
        .iter()
        .flatten()
        .filter(|paragraph| paragraph.heading_level.is_some())
        .count();
    assert_eq!(existing, 1);
    assert!(signals >= MIN_BODY_SIZE_BOLD_SIGNALS && signals > existing * MAX_OTHER_HEADING_RATIO);
    assert!(eligibility.iter().skip(1).all(|page| page[0]));
    let before = source_fingerprint(pages);
    assert!(!before.is_empty());
    promote_repeated_body_size_bold_headings(pages, Some(BODY_FONT));
    assert_eq!(
        source_fingerprint(pages),
        before,
        "classification must leave native records and text unchanged"
    );
    assert!(
        pages.iter().skip(1).all(|page| page[0].heading_level == Some(3)),
        "independent real signals must promote"
    );
    assert_eq!(pages.last().unwrap().last().unwrap().heading_level, Some(2));
    assert_eq!(
        pages[0][target].heading_level, expected,
        "target eligibility={}, signals={signals}, other headings={existing}",
        eligibility[0][target]
    );
}

#[test]
fn should_not_promote_a_proven_same_column_hyphen_continuation_2275() {
    let mut pages = active_pages(vec![
        fixtures::hyphen_candidate(),
        fixtures::foreign_bold_body(),
        fixtures::hyphen_body(),
    ]);
    assert_local_following(&pages[0], 0, 2);
    let (candidate, following) = (&pages[0][0], &pages[0][2]);
    assert_eq!(
        (candidate.word_count, following.word_count, following.lines.len()),
        (9, 13, 2)
    );
    let trailing = candidate.lines[0].segments.last().unwrap();
    let leading = &following.lines[0].segments[0];
    assert_eq!(
        (trailing.text.as_str(), leading.text.as_str()),
        ("specified-", "performance")
    );
    assert!(source_rows_continue(trailing, leading));
    assert!(spans_visual_line_break(trailing, leading));
    let right = following.block_bbox.unwrap().2;
    assert!(heading_fills_column(trailing, right));
    assert!(trailing.x + trailing.width >= right * FULL_LINE_FRACTION);
    assert!(leading.text.chars().next().unwrap().is_lowercase());
    promote_with_active_gate(&mut pages, 0, None);
}

#[test]
fn should_not_treat_aligned_numeric_formula_table_rows_as_a_prose_opener_2275() {
    let mut page = vec![
        fixtures::existing_section(),
        fixtures::two_cell_header(),
        plain("Sample Size", true, 357.195984, 721.182922, 58.300003),
        fixtures::formula_rows(),
        plain(
            "An independent right column continues its ordinary explanatory discussion.",
            false,
            315.0,
            580.0,
            225.0,
        ),
    ];
    assert_local_following(&page, 1, 3);
    assert_eq!(page[1].lines[0].segments.len(), 2);
    assert_eq!((page[3].word_count, page[3].lines.len()), (30, 8));
    assert_eq!(
        page[3].lines.iter().map(|line| line.segments.len()).collect::<Vec<_>>(),
        vec![1, 5, 3, 4, 1, 5, 3, 4]
    );
    assert_eq!(page[3].lines[1].segments[0].text, "< 30");
    assert_eq!(page[3].lines[5].segments[0].text, "< 30");
    assert_eq!(
        (page[3].lines[1].segments[0].x, page[3].lines[5].segments[0].x),
        (137.748992, 137.748001),
    );
    assert!(page[3].lines[1].segments[1].x > page[1].lines[0].segments[1].x);
    assert!(!is_dense_numeric_row(
        page[3]
            .lines
            .iter()
            .flat_map(|line| &line.segments)
            .map(|span| span.text.as_str())
    ));
    assert!(
        page.iter()
            .all(|paragraph| !paragraph.is_formula && !paragraph.is_list_item && paragraph.layout_class.is_none())
    );
    let mut pages = active_pages(std::mem::take(&mut page));
    promote_with_active_gate(&mut pages, 1, None);
}

#[test]
fn should_promote_a_genuine_bold_heading_before_quantitative_prose_2275() {
    let mut body = plain(
        "In 2024 the sample contained 30 observations and the fitted model used x = 2.",
        false,
        60.0,
        360.0,
        240.0,
    );
    body.lines.extend(
        plain(
            "The measured distance was 5 meters and confidence increased after another independent experiment.",
            false,
            60.0,
            348.0,
            240.0,
        )
        .lines,
    );
    body = paragraph(body.lines, BODY_FONT, false);
    assert_eq!((body.word_count, body.lines.len()), (28, 2));
    assert!(!body.is_formula);
    assert!(!is_dense_numeric_row(paragraph_text_raw(&body).split_whitespace()));
    let mut pages = active_pages(vec![
        plain("Measured annual performance", true, 60.0, 380.0, 180.0),
        body,
    ]);
    promote_with_active_gate(&mut pages, 0, Some(3));
}

#[test]
fn should_keep_a_genuine_heading_when_hyphen_continuation_evidence_is_absent_2275() {
    let candidate = plain("Closed-loop performance study", true, 311.999329, 317.759888, 160.0);
    let mut pages = active_pages(vec![candidate, fixtures::foreign_bold_body(), fixtures::hyphen_body()]);
    assert_local_following(&pages[0], 0, 2);
    let trailing = pages[0][0].lines[0].segments.last().unwrap();
    let leading = &pages[0][2].lines[0].segments[0];
    assert!(trailing.text.contains('-') && !trailing.text.ends_with('-'));
    assert!(spans_visual_line_break(trailing, leading) && source_rows_continue(trailing, leading));
    assert!(leading.text.chars().next().unwrap().is_lowercase());
    assert!(!heading_fills_column(trailing, pages[0][2].block_bbox.unwrap().2));
    promote_with_active_gate(&mut pages, 0, Some(3));
}

mod preservation;

mod source_geometry;
