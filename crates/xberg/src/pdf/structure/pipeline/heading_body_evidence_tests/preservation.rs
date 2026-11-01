use super::{
    BODY_FONT, Glyph, PdfLine, PdfParagraph, active_pages, assert_local_following, fixtures, line, paragraph,
    paragraph_text_raw, plain, promote_with_active_gate, spans_visual_line_break,
};
use crate::pdf::structure::layout_classify::has_formula_evidence;

fn candidate() -> PdfParagraph {
    let mut candidate = fixtures::two_cell_header();
    candidate.lines[0].segments[0].text = "Response model".to_owned();
    candidate.lines[0].segments[1].text = "Annual estimate".to_owned();
    candidate.word_count = PdfParagraph::compute_word_count("", &candidate.lines);
    assert_eq!((candidate.word_count, candidate.lines[0].segments.len()), (4, 2));
    candidate
}

fn source_page(body: PdfParagraph) -> Vec<PdfParagraph> {
    vec![
        fixtures::existing_section(),
        candidate(),
        plain("Foreign label", true, 357.195984, 721.182922, 58.300003),
        body,
        plain(
            "An independent right column continues its ordinary explanatory discussion.",
            false,
            315.0,
            580.0,
            225.0,
        ),
    ]
}

fn source_formula_rows(body: &PdfParagraph) -> Vec<usize> {
    body.lines
        .iter()
        .enumerate()
        .filter_map(|(index, line)| {
            let text = line
                .segments
                .iter()
                .map(|span| span.text.as_str())
                .collect::<Vec<_>>()
                .join(" ");
            has_formula_evidence(&text).then_some(index)
        })
        .collect()
}

fn assert_continuous_rows(body: &PdfParagraph) {
    let candidate = candidate();
    let left_end = candidate.lines[0].segments[0].x + candidate.lines[0].segments[0].width;
    let right_start = candidate.lines[0].segments[1].x;
    let gap = right_start - left_end;
    assert!(gap > 0.0);
    for row in &body.lines {
        assert!(row.segments.first().unwrap().x < left_end);
        let last = row.segments.last().unwrap();
        assert!(last.x + last.width > right_start);
        assert!(
            row.segments
                .windows(2)
                .all(|pair| pair[1].x - pair[0].x - pair[0].width < gap)
        );
    }
}

fn promote_genuine(body: PdfParagraph) {
    assert!(!body.is_formula && !body.is_list_item && body.layout_class.is_none());
    let mut pages = active_pages(source_page(body));
    assert_local_following(&pages[0], 1, 3);
    promote_with_active_gate(&mut pages, 1, Some(3));
}

#[test]
fn should_keep_a_two_span_heading_before_continuous_math_rich_prose_2275() {
    let body = paragraph(
        vec![
            plain(
                "The model uses a = b + c and d = e in this comparison.",
                false,
                114.196,
                663.124,
                130.0,
            )
            .lines
            .remove(0),
            plain(
                "Its estimate follows f = g - h and i = j + k during validation.",
                false,
                114.196,
                651.124,
                130.0,
            )
            .lines
            .remove(0),
        ],
        BODY_FONT,
        false,
    );
    assert_eq!(source_formula_rows(&body), vec![0, 1]);
    assert_continuous_rows(&body);
    promote_genuine(body);
}

fn unicode_row(texts: &[(&str, f32)], baseline: f32) -> PdfLine {
    let mut x = 114.196;
    let glyphs: Vec<Glyph<'_>> = texts
        .iter()
        .map(|&(text, width)| {
            let glyph = (text, x, width, BODY_FONT, false, false);
            x += width + 2.0;
            glyph
        })
        .collect();
    line(baseline, BODY_FONT, false, &glyphs)
}

#[test]
fn should_keep_a_two_span_heading_before_continuous_greek_and_cjk_prose_2275() {
    let body = paragraph(
        vec![
            unicode_row(
                &[
                    ("παρατήρηση", 36.0),
                    ("α", 6.0),
                    ("=", 5.0),
                    ("β", 6.0),
                    ("+", 5.0),
                    ("γ", 6.0),
                    ("=", 5.0),
                    ("δ", 6.0),
                    ("σταθερή", 28.0),
                ],
                663.124,
            ),
            unicode_row(
                &[
                    ("观测结果", 20.0),
                    ("α", 6.0),
                    ("=", 5.0),
                    ("β", 6.0),
                    ("+", 5.0),
                    ("γ", 6.0),
                    ("=", 5.0),
                    ("δ", 6.0),
                    ("稳定结论", 20.0),
                ],
                651.124,
            ),
        ],
        BODY_FONT,
        false,
    );
    assert_eq!(source_formula_rows(&body), vec![0, 1]);
    assert!(paragraph_text_raw(&body).contains("παρατήρηση") && paragraph_text_raw(&body).contains("观测结果"));
    assert!(body.lines.iter().all(|row| row.segments.len() == 9));
    assert_continuous_rows(&body);
    promote_genuine(body);
}

#[test]
fn should_keep_a_two_span_heading_before_two_single_cell_equations_2275() {
    let body = paragraph(
        vec![
            line(
                663.124,
                BODY_FONT,
                false,
                &[("u = v", 196.0, 40.0, BODY_FONT, false, false)],
            ),
            line(
                651.124,
                BODY_FONT,
                false,
                &[("x = y", 196.0, 40.0, BODY_FONT, false, false)],
            ),
        ],
        BODY_FONT,
        false,
    );
    assert_eq!((body.word_count, body.lines.len()), (6, 2));
    assert_eq!(source_formula_rows(&body), vec![0, 1]);
    let right = candidate().lines[0].segments[1].x;
    assert!(
        body.lines
            .iter()
            .flat_map(|row| &row.segments)
            .all(|span| span.x >= right)
    );
    assert!(spans_visual_line_break(
        &body.lines[0].segments[0],
        &body.lines[1].segments[0]
    ));
    promote_genuine(body);
}

#[test]
fn should_keep_a_two_span_heading_before_one_nonrecurring_formula_case_2275() {
    let original = fixtures::formula_rows();
    assert_eq!((original.word_count, original.lines.len()), (30, 8));
    let body = paragraph(
        original.lines.into_iter().take(4).collect(),
        original.dominant_font_size,
        original.is_bold,
    );
    assert_eq!((body.word_count, body.lines.len()), (15, 4));
    assert_eq!(source_formula_rows(&body), vec![2]);
    assert_eq!(body.lines[1].segments[0].text, "< 30");
    assert!(body.lines[1].segments[0].x < candidate().lines[0].segments[1].x);
    assert!(body.lines[1].segments[1].x > candidate().lines[0].segments[1].x);
    promote_genuine(body);
}
