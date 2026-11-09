mod common;

use xberg_native_pdf::PdfDocument;

#[test]
fn adjacent_source_glyphs_do_not_gain_a_word_boundary_from_rounding() {
    let mut stream = String::new();
    for (character, x, y) in [
        ('s', 71.12, 374.57825),
        ('t', 76.12, 374.83823),
        ('u', 78.9, 374.9828),
        ('r', 84.46001, 375.2719),
        ('e', 87.79, 375.44507),
    ] {
        stream.push_str(&format!("BT /F1 10 Tf 1 0 0 1 {x} {y} Tm ({character}) Tj ET\n"));
    }
    let document = PdfDocument::from_bytes(common::build_minimal_pdf_raw(
        stream.as_bytes(),
        b"/Type /Page /Parent 2 0 R /MediaBox [0 0 612 792]",
    ))
    .expect("parse adjacent source glyphs");
    let chars = document.extract_chars(0).expect("decode source advances");
    let previous = chars
        .iter()
        .find(|character| character.char == 'r')
        .expect("previous glyph");
    let next = chars
        .iter()
        .find(|character| character.char == 'e')
        .expect("following glyph");
    let gap = next.origin_x - (previous.origin_x + previous.advance_width);
    assert!(gap < 0.0 && gap.abs() < 0.00002, "one rounding unit of overlap: {gap}");
    let mut spans = document.extract_spans(0).expect("extract native spans");
    spans.sort_by(|a, b| a.bbox.x.total_cmp(&b.bbox.x));
    assert_eq!(spans.iter().map(|span| span.text.as_str()).collect::<String>(), "sture");
}

/// Merge the spans of a page that shows `first` at x = 100 and then `second` one ULP before the end of `first`.
/// The font has no /Widths array, so the span merge uses the fallback advance for both shows.
fn merged_text_with_one_ulp_overlap(first: &str, second: &str) -> String {
    let page = |second_x: f32| {
        let stream = format!(
            "BT /F1 12 Tf 1 0 0 1 100 500 Tm ({first}) Tj ET\nBT /F1 12 Tf 1 0 0 1 {second_x:?} 500 Tm ({second}) Tj ET\n"
        );
        PdfDocument::from_bytes(common::build_minimal_pdf_raw(
            stream.as_bytes(),
            b"/Type /Page /Parent 2 0 R /MediaBox [0 0 612 792]",
        ))
        .expect("parse the generated page")
    };
    let apart = page(400.0).extract_spans(0).expect("extract the spans of the far page");
    let first_span = apart.iter().find(|span| span.text == first).expect("first show stays its own span");
    let first_end = first_span.bbox.x + first_span.bbox.width;
    let second_x = first_end.next_down();
    assert_eq!(format!("{second_x:?}").parse::<f32>().unwrap(), second_x, "the start survives printing");
    let mut spans = page(second_x).extract_spans(0).expect("extract the spans of the touching page");
    spans.sort_by(|a, b| a.bbox.x.total_cmp(&b.bbox.x));
    spans.iter().map(|span| span.text.as_str()).collect()
}

#[test]
fn a_word_show_touching_the_previous_word_keeps_its_space() {
    assert_eq!(merged_text_with_one_ulp_overlap("hello", "world"), "hello world");
}

#[test]
fn a_one_glyph_show_touching_the_previous_glyphs_gains_no_space() {
    assert_eq!(merged_text_with_one_ulp_overlap("hell", "o"), "hello");
}
