//! A `TJ` word-boundary space span carries the weight of the font it is drawn in, so span merging
//! joins it to the bold word before it as it does in a regular run. ~keep

use super::super::*;
use super::extraction_and_rotation::create_test_font;

/// A simple font named `base_font`: every glyph is half an em wide, the space a quarter em.
fn half_em_font(base_font: &str) -> FontInfo {
    let mut font = create_test_font();
    font.base_font = base_font.to_string();
    font.widths = Some(
        (32u32..=126)
            .map(|code| if code == 32 { 250.0 } else { 500.0 })
            .collect(),
    );
    font.first_char = Some(32);
    font.last_char = Some(126);
    font
}

/// The span texts after the merge pass, for a stream drawn in `F1` set in `base_font`.
fn merged_span_texts(base_font: &str, stream: &str) -> Vec<String> {
    let mut extractor = TextExtractor::new();
    extractor.add_font("F1".to_string(), half_em_font(base_font));
    extractor.extract_spans = true;
    extractor.run_content_stream(stream.as_bytes()).unwrap();
    extractor.flush_tj_span_buffer().unwrap();
    extractor.settle_span_holds();
    extractor.merge_adjacent_spans();
    extractor.spans.iter().map(|span| span.text.clone()).collect()
}

/// The weight of the space span inserted after a run in `base_font`.
fn space_weight(base_font: &str, closes_ltr_run: bool) -> FontWeight {
    let mut extractor = TextExtractor::new();
    extractor.add_font("F1".to_string(), half_em_font(base_font));
    extractor.state_stack.current_mut().font_size = 12.0;
    extractor.state_stack.current_mut().horizontal_scaling = 100.0;
    extractor.state_stack.current_mut().font_name = Some("F1".to_string());
    extractor.insert_space_as_span(closes_ltr_run).unwrap();
    extractor.spans.last().unwrap().font_weight
}

#[test]
fn test_insert_space_as_span_after_a_left_to_right_run_carries_the_weight_of_the_font() {
    assert_eq!(space_weight("Helvetica", true), FontWeight::Normal);
    assert_eq!(space_weight("Helvetica-Bold", true), FontWeight::Bold);
}

/// A right-to-left run is put in logical order span by span: a bold space joined to it would let the
/// merge pass weld the next word drawn onto it in visual order (rtl_arabic_native.pdf).
#[test]
fn test_insert_space_as_span_after_a_right_to_left_run_stays_regular() {
    assert_eq!(space_weight("Helvetica-Bold", false), FontWeight::Normal);
}

/// A bold heading set with 0.132 em `TJ` word gaps (84034880.pdf, "Two Minute Paper Summary"): the
/// space span the offset adds joins the bold word before it, as it does in a regular run, instead of
/// staying a blank span of its own that segment projection drops.
#[test]
fn test_tj_word_space_joins_the_bold_word_before_it() {
    let stream = "BT /F1 10 Tf 72 700 Td [(Two) -132 (Minute)] TJ ET";
    for base_font in ["Helvetica", "Helvetica-Bold"] {
        assert_eq!(merged_span_texts(base_font, stream), ["Two ", "Minute"], "{base_font}");
    }
}
