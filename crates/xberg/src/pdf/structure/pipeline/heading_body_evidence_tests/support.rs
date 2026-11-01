use super::super::{
    body_size_heading_eligibility, heading_body::has_recurring_formula_cells, is_explicit_numbered_body_size_section,
    promote_repeated_body_size_bold_headings,
};
use super::source_geometry::{canonical_body, source_digest};
use super::{
    MAX_OTHER_HEADING_RATIO, MIN_BODY_SIZE_BOLD_SIGNALS, PdfParagraph, assert_valid_source, fixtures,
    is_body_size_bold_heading_candidate, line, paragraph, paragraph_text_raw, source_fingerprint,
};

const SOURCE_BODY_FONT: f32 = f32::from_bits(0x411ae205);
const AUXILIARY_RECORDS_SHA256: &str = "e488879e1e0ad6d1607a3a3b5c1aa6181dc94c0e7d858ad1e336dd5c1f8fbf3e";

fn distribution() -> PdfParagraph {
    let mut result = paragraph(
        vec![line(
            f32::from_bits(0x4432dbb6),
            f32::from_bits(0x41200000),
            true,
            &[
                (
                    "9.3",
                    f32::from_bits(0x42900000),
                    f32::from_bits(0x415e3d72),
                    f32::from_bits(0x41200000),
                    true,
                    false,
                ),
                (
                    "Distribution Needed for Hypothesis Testing",
                    f32::from_bits(0x42b15c02),
                    f32::from_bits(0x434d5b85),
                    f32::from_bits(0x41200000),
                    true,
                    false,
                ),
            ],
        )],
        f32::from_bits(0x41200000),
        true,
    );
    result.block_bbox = Some((
        f32::from_bits(0x42900000),
        f32::from_bits(0x4432dbb6),
        f32::from_bits(0x439304c3),
        f32::from_bits(0x44355bb6),
    ));
    assert_eq!(result.word_count, 6);
    result
}

fn practice() -> PdfParagraph {
    let mut result = paragraph(
        vec![line(
            f32::from_bits(0x44052a9f),
            f32::from_bits(0x41600000),
            true,
            &[(
                "PRACTICE",
                f32::from_bits(0x42900000),
                f32::from_bits(0x428efefa),
                f32::from_bits(0x41600000),
                true,
                false,
            )],
        )],
        f32::from_bits(0x41600000),
        true,
    );
    result.block_bbox = Some((
        f32::from_bits(0x42900000),
        f32::from_bits(0x44052a9f),
        f32::from_bits(0x430f7f7d),
        f32::from_bits(0x4408aa9f),
    ));
    result.heading_level = Some(1);
    assert_eq!(result.word_count, 1);
    result
}

fn hypotheses() -> PdfParagraph {
    let mut result = paragraph(
        vec![line(
            f32::from_bits(0x43fb9394),
            f32::from_bits(0x41200000),
            true,
            &[
                (
                    "9.1",
                    f32::from_bits(0x42900000),
                    f32::from_bits(0x415e3d72),
                    f32::from_bits(0x41200000),
                    true,
                    false,
                ),
                (
                    "Null and Alternative Hypotheses",
                    f32::from_bits(0x42b15c02),
                    f32::from_bits(0x43193ae0),
                    f32::from_bits(0x41200000),
                    true,
                    false,
                ),
            ],
        )],
        f32::from_bits(0x41200000),
        true,
    );
    result.block_bbox = Some((
        f32::from_bits(0x42900000),
        f32::from_bits(0x43fb9394),
        f32::from_bits(0x4371e8e1),
        f32::from_bits(0x440049ca),
    ));
    assert_eq!(result.word_count, 5);
    result
}

fn exercise_opening_line() -> super::PdfLine {
    line(
        f32::from_bits(0x43f27cec),
        f32::from_bits(0x41180000),
        false,
        &[
            (
                "1.",
                f32::from_bits(0x42900000),
                f32::from_bits(0x40fd3b66),
                f32::from_bits(0x41180000),
                true,
                false,
            ),
            (
                concat!(
                    "You are testing that the mean speed of your cable Internet connection ",
                    "is more than three Megabits per second. What is"
                ),
                f32::from_bits(0x42a60831),
                f32::from_bits(0x43e47ccc),
                f32::from_bits(0x41180000),
                false,
                false,
            ),
        ],
    )
}

fn exercise() -> PdfParagraph {
    let mut result = paragraph(
        vec![
            exercise_opening_line(),
            line(
                f32::from_bits(0x43ecbe75),
                f32::from_bits(0x41180000),
                false,
                &[(
                    "the random variable? Describe in words.",
                    f32::from_bits(0x42900000),
                    f32::from_bits(0x4319edb3),
                    f32::from_bits(0x41180000),
                    false,
                    false,
                )],
            ),
        ],
        f32::from_bits(0x41180000),
        false,
    );
    result.block_bbox = Some((
        f32::from_bits(0x42900000),
        f32::from_bits(0x43ecbe75),
        f32::from_bits(0x4406ff6c),
        f32::from_bits(0x43f73cec),
    ));
    assert_eq!(result.word_count, 28);
    result
}

fn source_page(include_table_candidate: bool) -> Vec<Vec<PdfParagraph>> {
    let measured = [distribution(), practice(), hypotheses(), exercise()];
    assert_eq!(
        measured
            .iter()
            .flat_map(|p| &p.lines)
            .map(|l| l.segments.len())
            .sum::<usize>(),
        8
    );
    assert_eq!(
        source_digest(measured.iter().flat_map(|p| &p.lines).flat_map(|l| &l.segments)),
        AUXILIARY_RECORDS_SHA256
    );
    let [distribution, practice, hypotheses, exercise] = measured;
    let header = fixtures::two_cell_header();
    let body = canonical_body();
    assert!(has_recurring_formula_cells(&header, &body));
    let mut page = vec![distribution];
    if include_table_candidate {
        page.push(header);
    }
    page.extend([body, practice, hypotheses, exercise]);
    vec![page]
}

fn assert_support_inputs(pages: &[Vec<PdfParagraph>], expected_candidates: usize, expected_records: usize) {
    assert_eq!((MIN_BODY_SIZE_BOLD_SIGNALS, MAX_OTHER_HEADING_RATIO), (3, 2));
    assert_valid_source(pages);
    let page = &pages[0];
    assert_eq!(pages.len(), 1);
    assert_eq!(page.len(), expected_candidates + 3);
    assert_eq!(
        page.iter()
            .flat_map(|p| &p.lines)
            .map(|l| l.segments.len())
            .sum::<usize>(),
        expected_records
    );
    assert_eq!(
        page.iter()
            .filter(|p| is_body_size_bold_heading_candidate(p, SOURCE_BODY_FONT))
            .count(),
        expected_candidates
    );
    let existing: Vec<_> = page.iter().filter(|p| p.heading_level.is_some()).collect();
    assert_eq!(existing.len(), 1);
    assert_eq!(
        (paragraph_text_raw(existing[0]).trim(), existing[0].heading_level),
        ("PRACTICE", Some(1))
    );
    let eligible = body_size_heading_eligibility(pages, SOURCE_BODY_FONT);
    for index in [0, page.len() - 2] {
        assert!(is_explicit_numbered_body_size_section(
            paragraph_text_raw(&page[index]).trim()
        ));
        assert!(
            eligible[0][index],
            "the genuine numbered title remains locally eligible"
        );
    }
}

fn promote_source_page(pages: &mut [Vec<PdfParagraph>]) {
    let before = source_fingerprint(pages);
    assert!(!before.is_empty());
    promote_repeated_body_size_bold_headings(pages, Some(SOURCE_BODY_FONT));
    assert_eq!(source_fingerprint(pages), before);
    let practice = pages[0]
        .iter()
        .find(|p| paragraph_text_raw(p).trim() == "PRACTICE")
        .unwrap();
    assert_eq!(practice.heading_level, Some(1));
}

#[test]
fn should_keep_genuine_headings_when_table_style_support_is_not_emitted_2275() {
    let mut pages = source_page(true);
    assert_support_inputs(&pages, 3, 36);
    promote_source_page(&mut pages);
    assert_eq!(
        pages[0][0].heading_level,
        Some(3),
        "the genuine distribution section must remain a heading"
    );
    assert_eq!(
        pages[0][4].heading_level,
        Some(3),
        "the genuine hypothesis section must remain a heading"
    );
    assert_eq!(
        pages[0][1].heading_level, None,
        "the measured table header must remain body text"
    );
}

#[test]
fn should_not_promote_numbered_titles_without_three_style_support_candidates_2275() {
    let mut pages = source_page(false);
    assert_support_inputs(&pages, 2, 34);
    promote_source_page(&mut pages);
    assert_eq!(pages[0][0].heading_level, None);
    assert_eq!(pages[0][3].heading_level, None);
}
