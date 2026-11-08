//! A word-boundary `TJ` offset at a pen that a positioning operator set back into the run drawn last adds a
//! space span only for the part past the end of that run, at that end (GH#2322). Elsewhere, or where a glyph
//! or an offset moved the pen first, the space span stays at the pen. ~keep

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

/// The text, the x and the width of each span, in the order the content stream makes them.
fn span_boxes(stream: &str) -> Vec<(String, f32, f32)> {
    let mut extractor = extractor();
    extractor.extract_spans = true;
    extractor.run_content_stream(stream.as_bytes()).unwrap();
    extractor.flush_tj_span_buffer().unwrap();
    extractor.settle_span_holds();
    let round = |v: f32| (v * 100.0).round() / 100.0;
    let spans = extractor.spans.iter();
    spans
        .map(|s| (s.text.clone(), round(s.bbox.x), round(s.bbox.width)))
        .collect()
}

#[test]
fn offset_to_the_end_of_the_drawn_run_after_a_pen_set_back_into_it_adds_no_space_span() {
    // "abc" ends at x = 87, 7 points past the pen. Two positioning operators in a row keep the end of the
    // run drawn last. ~keep
    for position in [
        "1 0 0 1 80 700 Tm",
        "8 0 Td",
        "8 0 TD",
        "1 0 0 1 79 700 Tm 1 0 0 1 80 700 Tm",
    ] {
        let stream = format!("BT /F1 10 Tf 12 TL 72 700 Td (abc) Tj {position} [-700 (def)] TJ ET");
        assert_eq!(span_texts(&stream), ["abc", "def"], "after {position}");
    }
}

/// The size in `Tf` against a scaled `Tm`: the rule reads the matrix, not the size. "abc" ends 11 points,
/// one unit of the matrix, past the pen.
#[test]
fn offset_to_the_end_of_the_drawn_run_after_a_scaled_tm_back_into_it_adds_no_space_span() {
    let stream = "BT 11 0 0 11 72 700 Tm /F1 1 Tf (abc) Tj 11 0 0 11 77.5 700 Tm [-1000 (def)] TJ ET";
    assert_eq!(span_texts(stream), ["abc", "def"]);
}

/// A lead that ends past the drawn run adds its space span at the end of that run, not at the pen inside
/// it, at either scale of the text matrix. ~keep
#[test]
fn offset_past_the_drawn_run_after_a_pen_set_back_into_it_adds_the_space_span_at_its_end() {
    for (setup, back, end, def) in [
        ("BT /F1 10 Tf 72 700 Td", "1 0 0 1 80 700 Tm", 87.0, 110.0),
        ("BT 11 0 0 11 72 700 Tm /F1 1 Tf", "11 0 0 11 80 700 Tm", 88.5, 113.0),
    ] {
        let spans = span_boxes(&format!("{setup} (abc) Tj {back} [-3000 (def)] TJ ET"));
        let texts: Vec<(&str, f32)> = spans.iter().map(|(t, x, _)| (t.as_str(), *x)).collect();
        assert_eq!(texts, [("abc", 72.0), (" ", end), ("def", def)], "{setup}");
    }
}

/// The part of the lead past the drawn run meets the same threshold as an offset at the end of that run:
/// 100 units past the end adds no space span, 200 units add one. ~keep
#[test]
fn offset_just_past_the_drawn_run_after_a_pen_set_back_into_it_meets_the_offset_threshold() {
    for (lead, spans) in [("-800", &["abc", "def"][..]), ("-900", &["abc", " ", "def"][..])] {
        let stream = format!("BT /F1 10 Tf 72 700 Td (abc) Tj 8 0 Td [{lead} (def)] TJ ET");
        assert_eq!(span_texts(&stream), spans, "lead {lead}");
    }
}

/// A pen at the end of the run, less than a quarter em before it, past it, or on another line.
#[test]
fn offset_after_a_pen_set_outside_the_drawn_run_keeps_the_space_span() {
    for position in [
        "1 0 0 1 87 700 Tm",
        "1 0 0 1 85 700 Tm",
        "1 0 0 1 95 700 Tm",
        "T*",
        "ET BT",
    ] {
        let stream = format!("BT /F1 10 Tf 12 TL 72 700 Td (abc) Tj {position} [-3000 (def)] TJ ET");
        assert_eq!(span_texts(&stream), ["abc", " ", "def"], "after {position}");
    }
}

/// With the size in `Tm`, a pen 1.5 units before the end of the run is less than a quarter em before it.
#[test]
fn offset_after_a_scaled_tm_just_before_the_end_keeps_the_space_span() {
    let stream = "BT 11 0 0 11 72 700 Tm /F1 1 Tf (abc) Tj 11 0 0 11 87 700 Tm [-3000 (def)] TJ ET";
    assert_eq!(span_texts(stream), ["abc", " ", "def"]);
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

/// A first offset moves the pen to the end of the run, so a second offset in the same array no longer stands
/// where `Td` put it. ~keep
#[test]
fn offset_after_an_offset_keeps_the_space_span() {
    let stream = "BT /F1 10 Tf 72 700 Td (abc) Tj 8 0 Td [-700 -3000 (def)] TJ ET";
    assert_eq!(span_texts(stream), ["abc", " ", "def"]);
}

/// `Tm` back to the start of a drawn word, then a lead that ends past that word: the gap between the
/// real glyphs is a word gap at either scale of the text matrix. ~keep
#[test]
fn offset_after_a_tm_back_into_the_run_to_a_word_gap_keeps_the_gap() {
    let words = |stream: &str| -> Vec<String> {
        let mut extractor = extractor();
        let spans = extractor.extract_text_spans(stream.as_bytes()).unwrap();
        let text = spans.iter().map(|s| s.text.as_str()).collect::<Vec<_>>().join("");
        text.split_whitespace().map(str::to_string).collect()
    };
    // "the word" is 41.25 points wide at 11 points; "word" starts at 91.25 and ends at 113.25. ~keep
    for (setup, back) in [
        ("BT /F1 11 Tf 72 700 Td", "1 0 0 1 91.25 700 Tm"),
        ("BT 11 0 0 11 72 700 Tm /F1 1 Tf", "11 0 0 11 91.25 700 Tm"),
    ] {
        let stream = format!("{setup} (the word) Tj {back} [-2250 (next)] TJ ( more) Tj ET");
        assert_eq!(words(&stream), ["the", "word", "next", "more"], "{setup}");
    }
}

/// A `Tm` set back into the run drawn last continues that run. A `TJ` lead at that pen ends the run at the
/// end of its glyphs, at either scale of the text matrix (GH#2690). Glyphs drawn at the pen itself continue
/// the run from the pen. ~keep
#[test]
fn tj_lead_after_a_tm_back_into_the_run_ends_the_run_at_its_drawn_width() {
    // "abc" is 15 points wide at 10 points and ends at x = 87. ~keep
    for (setup, back) in [
        ("BT /F1 10 Tf 72 700 Td", "1 0 0 1 80 700 Tm"),
        ("BT 10 0 0 10 72 700 Tm /F1 1 Tf", "10 0 0 10 80 700 Tm"),
    ] {
        let spans = span_boxes(&format!("{setup} (abc) Tj {back} [-3000 (def)] TJ ET"));
        assert_eq!(spans.first(), Some(&("abc".to_string(), 72.0, 15.0)), "{setup}");
    }
    let drawn_at_the_pen = span_boxes("BT /F1 10 Tf 72 700 Td (abc) Tj 1 0 0 1 80 700 Tm (d) Tj ET");
    assert_eq!(drawn_at_the_pen, [("abcd".to_string(), 72.0, 13.0)]);
    let shown_at_the_pen = span_boxes("BT /F1 10 Tf 72 700 Td (abc) Tj 1 0 0 1 80 700 Tm [(d)] TJ ET");
    assert_eq!(shown_at_the_pen, [("abc".to_string(), 72.0, 8.0), ("d".to_string(), 80.0, 5.0)]);
}
