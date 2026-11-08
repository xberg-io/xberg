//! A word-boundary `TJ` offset at a pen that a positioning operator set adds no space span (GH#2322).
//! Where a glyph or an offset moved the pen first, the space span stays. ~keep

use super::super::*;
use super::extraction_and_rotation::create_test_font;

/// A simple font: every glyph is half an em wide, the space a quarter em.
fn extractor() -> TextExtractor<'static> {
    let mut font = create_test_font();
    font.base_font = "Helvetica".to_string();
    font.widths = Some(
        (32u32..=126)
            .map(|code| if code == 32 { 250.0 } else { 500.0 })
            .collect(),
    );
    font.first_char = Some(32);
    font.last_char = Some(126);
    let mut extractor = TextExtractor::new();
    extractor.add_font("F1".to_string(), font);
    extractor
}

/// The spans in the order the content stream makes them, before the reading-order sort and the merge.
fn span_texts(stream: &str) -> Vec<String> {
    let mut extractor = extractor();
    extractor.extract_spans = true;
    extractor.run_content_stream(stream.as_bytes()).unwrap();
    extractor.flush_tj_span_buffer().unwrap();
    extractor.settle_span_holds();
    extractor.spans.iter().map(|s| s.text.clone()).collect()
}

#[test]
fn offset_after_each_positioning_operator_adds_no_space_span() {
    for position in ["1 0 0 1 80 700 Tm", "8 0 Td", "8 0 TD", "T*", "ET BT"] {
        let stream = format!("BT /F1 10 Tf 12 TL 72 700 Td (abc) Tj {position} [-3000 (def)] TJ ET");
        assert_eq!(span_texts(&stream), ["abc", "def"], "after {position}");
    }
}

/// The size in `Tf` against a scaled `Tm`: the rule reads the matrix, not the size.
#[test]
fn offset_after_a_scaled_tm_adds_no_space_span() {
    let stream = "BT 11 0 0 11 72 700 Tm /F1 1 Tf (abc) Tj 11 0 0 11 80 700 Tm [-3000 (def)] TJ ET";
    assert_eq!(span_texts(stream), ["abc", "def"]);
}

#[test]
fn offset_after_a_glyph_keeps_the_space_span() {
    for stream in [
        "BT /F1 10 Tf 72 700 Td [(abc) -3000 (def)] TJ ET",
        "BT /F1 10 Tf 72 700 Td (abc) Tj [-3000 (def)] TJ ET",
        "BT /F1 10 Tf 72 700 Td [(abc)] TJ [-3000 (def)] TJ ET",
    ] {
        assert_eq!(span_texts(stream), ["abc", " ", "def"], "{stream}");
    }
}

/// A first offset moves the pen, so a second offset in the same array no longer stands where `Td` put it.
#[test]
fn offset_after_an_offset_keeps_the_space_span() {
    let stream = "BT /F1 10 Tf 72 700 Td (abc) Tj 8 0 Td [-3000 -3000 (def)] TJ ET";
    assert_eq!(span_texts(stream), ["abc", " ", "def"]);
}
