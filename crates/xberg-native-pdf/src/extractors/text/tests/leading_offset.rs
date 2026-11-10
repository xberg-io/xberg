//! A word-boundary `TJ` offset at a pen that a positioning operator set back into the run drawn last adds a
//! space span only for the part past the end of that run, at that end (GH#2322). Elsewhere, or where a glyph
//! or an offset moved the pen first, the space span stays at the pen. ~keep

use super::super::*;
use super::extraction_and_rotation::create_test_font;

/// A simple font: every glyph is half an em wide, the space a quarter em.
fn half_em_font() -> FontInfo {
    let mut font = create_test_font();
    font.base_font = "Helvetica".to_string();
    font.widths = Some(
        (32u32..=126)
            .map(|code| if code == 32 { 250.0 } else { 500.0 })
            .collect(),
    );
    font.first_char = Some(32);
    font.last_char = Some(126);
    font
}

fn extractor() -> TextExtractor<'static> {
    let mut extractor = TextExtractor::new();
    extractor.add_font("F1".to_string(), half_em_font());
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
    assert_eq!(
        shown_at_the_pen,
        [("abc".to_string(), 72.0, 8.0), ("d".to_string(), 80.0, 5.0)]
    );
}

/// The words of the spans after the merge, for a stream drawn in `font`.
fn merged_words(font: FontInfo, stream: &str) -> Vec<String> {
    let mut extractor = TextExtractor::new();
    extractor.add_font("F1".to_string(), font);
    let spans = extractor.extract_text_spans(stream.as_bytes()).unwrap();
    let text = spans.iter().map(|s| s.text.as_str()).collect::<Vec<_>>().join("");
    text.split_whitespace().map(str::to_string).collect()
}

/// A producer that sets `Tm` back to the start of a drawn line and reaches each word with a lead, as for a
/// justified line or a table cell drawn word by word: a lead that ends `gap` units past the drawn run reads
/// as an offset of `gap` units after its last glyph, so a real word gap is not lost. ~keep
#[test]
fn lead_after_a_tm_back_to_the_line_start_reads_like_an_offset_at_the_drawn_end() {
    // "abc" is 1500 units wide at 10 points and starts at the pen. ~keep
    for gap in [100, 150, 200, 250, 500, 3000, 12000] {
        let set_back = format!(
            "BT /F1 10 Tf 72 700 Td (abc) Tj 1 0 0 1 72 700 Tm [-{} (def)] TJ ET",
            1500 + gap
        );
        let at_end = format!("BT /F1 10 Tf 72 700 Td (abc) Tj [-{gap} (def)] TJ ET");
        let words = merged_words(half_em_font(), &set_back);
        assert_eq!(words, merged_words(half_em_font(), &at_end), "gap {gap}");
        // Below 250 units the plain form `[-N (def)]` gives no space either, so those gaps pin parity only:
        // the lead reads like the offset at the drawn end, whatever words that gives. ~keep
        if gap >= 250 {
            assert_eq!(words, ["abc", "def"], "gap {gap}");
        }
    }
}

/// A justified line redrawn word by word from its start keeps every word gap.
#[test]
fn justified_line_redrawn_word_by_word_from_its_start_keeps_its_words() {
    // "abc" ends at 87, "def" at 107; each lead reaches 3 points past the last drawn word. ~keep
    let stream = "BT /F1 10 Tf 72 700 Td (abc) Tj 1 0 0 1 72 700 Tm [-1800 (def)] TJ \
                  1 0 0 1 72 700 Tm [-3800 (ghi)] TJ ET";
    assert_eq!(merged_words(half_em_font(), stream), ["abc", "def", "ghi"]);
}

/// A `Tm` set back into a run that ends in a ligature, then a lead (GH#2322). The `Tm` ends the ligature, so
/// a lead that lands at the end of it joins the word, and a gap past it is a real gap.
#[test]
fn lead_after_a_tm_back_into_a_ligature_run() {
    use super::ligature_decode::ligature_font;
    // "de" and the "fi" ligature end at 92; the `Tm` sets the pen 7 points back, inside the ligature. ~keep
    for (array, words) in [("[-700 (ne)]", &["define"][..]), ("[-3000 (ne)]", &["defi", "ne"][..])] {
        let stream = format!("BT /F1 10 Tf 72 700 Td (de\\200) Tj 1 0 0 1 85 700 Tm {array} TJ ET");
        assert_eq!(merged_words(ligature_font("00660069", None), &stream), words, "{array}");
    }
}

/// A pen set back by `Td` into the end of a ligature, then a lead (GH#2322): the set-back rule and the
/// ligature pull-back act on the same `TJ`. A lead that ends at the ligature's end joins the word, a gap
/// past it keeps exactly one space, and a pulled-back space glyph inside the ligature box still drops. ~keep
#[test]
fn lead_after_a_td_back_into_a_ligature_joins_or_gaps_once() {
    use super::ligature_decode::ligature_font;
    // "de" and the "fi" ligature end at 92; the `Td` sets the pen 2.5 points back, into the ligature. ~keep
    for (array, words) in [
        ("[-250 (ne)]", &["define"][..]),
        ("[-3000 (ne)]", &["defi", "ne"][..]),
        ("[-3000 ( ne)]", &["defi", "ne"][..]),
    ] {
        let stream = format!("BT /F1 10 Tf 72 700 Td (de\\200) Tj -2.5 0 Td {array} TJ ET");
        assert_eq!(merged_words(ligature_font("00660069", None), &stream), words, "{array}");
    }
}

/// A large offset inside the `TJ` that draws a ligature, then the pulled-back space: the ligature's
/// pull-back still absorbs the space, and a large offset before the next word keeps its one space. ~keep
#[test]
fn large_offset_after_a_ligature_keeps_the_pull_back_and_the_word_gap() {
    use super::ligature_decode::ligature_font;
    for (stream, words) in [
        ("[(de\\200) 250 ( ne) -3000 (ab)] TJ", &["define", "ab"][..]),
        ("[(de\\200) -3000 (ne)] TJ", &["defi", "ne"][..]),
    ] {
        let stream = format!("BT /F1 10 Tf 72 700 Td {stream} ET");
        assert_eq!(
            merged_words(ligature_font("00660069", None), &stream),
            words,
            "{stream}"
        );
    }
}

/// GH#2322: the producer draws "define" with a space glyph that falls inside the ligature box after the
/// set back, and the extractor still outputs the split "defi ne". Dropping that space needs the ligature to
/// survive the `Tm`. The test states the right answer and is ignored until that is fixed. ~keep
#[test]
#[ignore = "GH#2322 space glyph inside a ligature after a Tm back"]
fn space_glyph_inside_a_ligature_after_a_tm_back_joins_the_word() {
    use super::ligature_decode::ligature_font;
    for array in ["[-700 ( ne)]", "[-450 ( ne)]"] {
        let stream = format!("BT /F1 10 Tf 72 700 Td (de\\200) Tj 1 0 0 1 85 700 Tm {array} TJ ET");
        let words = merged_words(ligature_font("00660069", None), &stream);
        assert_eq!(words, ["define"], "{array}");
    }
}

/// The text and the origin of each span, in the order the content stream makes them.
fn span_origins(stream: &str, vertical: bool) -> Vec<(String, f32, f32)> {
    let mut font = half_em_font();
    font.wmode = u8::from(vertical);
    let mut extractor = TextExtractor::new();
    extractor.add_font("F1".to_string(), font);
    extractor.extract_spans = true;
    extractor.run_content_stream(stream.as_bytes()).unwrap();
    extractor.flush_tj_span_buffer().unwrap();
    extractor.settle_span_holds();
    let round = |v: f32| (v * 100.0).round() / 100.0;
    let spans = extractor.spans.iter();
    spans
        .map(|s| (s.text.clone(), round(s.bbox.x), round(s.bbox.y)))
        .collect()
}

fn starts(stream: &str) -> Vec<(String, f32)> {
    span_origins(stream, false)
        .into_iter()
        .map(|(t, x, _)| (t, x))
        .collect()
}

fn start(text: &str, x: f32) -> (String, f32) {
    (text.to_string(), x)
}

/// `T*` with a leading of 0 sets the pen back to the line start, 15 points before the end of the run drawn
/// last. The space goes at the end of that run, not at the pen inside it (GH#2322). ~keep
#[test]
fn t_star_sets_the_pen_back_into_the_drawn_run() {
    let spans = starts("BT /F1 10 Tf 0 TL 72 700 Td (abc) Tj T* [-3000 (def)] TJ ET");
    assert_eq!(spans, [start("abc", 72.0), start(" ", 87.0), start("def", 102.0)]);
}

/// A new text object does not forget the run that the last one drew: `Tm` back into it sets the pen back.
#[test]
fn a_tm_in_a_new_text_object_sets_the_pen_back_into_the_run_drawn_in_the_last_one() {
    let stream = "BT /F1 10 Tf 72 700 Td (abc) Tj ET BT /F1 10 Tf 1 0 0 1 80 700 Tm [-3000 (def)] TJ ET";
    assert_eq!(
        starts(stream),
        [start("abc", 72.0), start(" ", 87.0), start("def", 110.0)]
    );
}

/// A pen more than a quarter em (2.5 points at 10 points) before the end of the run is set back; a pen
/// within it is not, and the space stays at the pen.
#[test]
fn a_pen_set_back_by_more_than_a_quarter_em_moves_the_space_to_the_end_of_the_run() {
    for (pen, space, def) in [(84.0, 87.0, 114.0), (85.0, 85.0, 115.0), (87.0, 87.0, 117.0)] {
        let stream = format!("BT /F1 10 Tf 72 700 Td (abc) Tj 1 0 0 1 {pen} 700 Tm [-3000 (def)] TJ ET");
        let want = [start("abc", 72.0), start(" ", space), start("def", def)];
        assert_eq!(starts(&stream), want, "pen at {pen}");
    }
}

/// A `Tm` past the end of the run continues it with the width of the pen move, and a lead at that pen does
/// not cut it back to its glyphs: the run keeps the 23 points between its start and the pen. ~keep
#[test]
fn tj_lead_after_a_tm_past_the_run_keeps_the_pen_move_width() {
    let spans = span_boxes("BT /F1 10 Tf 72 700 Td (abc) Tj 1 0 0 1 95 700 Tm [-3000 (def)] TJ ET");
    assert_eq!(spans.first(), Some(&("abc".to_string(), 72.0, 23.0)));
}

/// A lead that moves the pen on, back to the left, does not end the run at its glyphs: the run keeps the
/// 8 points of the pen move. The end of the run at its glyphs is for a lead that reaches forward. ~keep
#[test]
fn positive_tj_lead_after_a_tm_back_into_the_run_keeps_the_pen_move_width() {
    let spans = span_boxes("BT /F1 10 Tf 72 700 Td (abc) Tj 1 0 0 1 80 700 Tm [500 (def)] TJ ET");
    assert_eq!(spans.first(), Some(&("abc".to_string(), 72.0, 8.0)));
}

/// Horizontal scaling stretches a `TJ` offset in horizontal writing: at 200 percent, 3000 units are 60
/// points. "abc" is 30 points wide and ends at 102, so "def" starts at 162.
#[test]
fn tj_offset_is_scaled_by_horizontal_scaling_in_horizontal_writing() {
    let spans = starts("BT /F1 10 Tf 200 Tz 72 700 Td [(abc) -3000 (def)] TJ ET");
    assert_eq!(spans, [start("abc", 72.0), start(" ", 102.0), start("def", 162.0)]);
}

/// Horizontal scaling does not stretch a `TJ` offset in vertical writing: 2000 more units of offset move the
/// pen 20 points more at 100 and at 200 percent.
#[test]
fn tj_offset_is_not_scaled_by_horizontal_scaling_in_vertical_writing() {
    let last_y = |tz: u32, offset: i32| {
        let stream = format!("BT /F1 10 Tf {tz} Tz 72 700 Td [(a) {offset} (b)] TJ ET");
        span_origins(&stream, true).last().map(|(_, _, y)| *y).unwrap()
    };
    for tz in [100, 200] {
        assert_eq!((last_y(tz, -3000) - last_y(tz, -1000)).abs(), 20.0, "Tz {tz}");
    }
}

/// The drawn end belongs to the graphics state it was drawn under: after `Q` or `cm`, a text object under a
/// new CTM is not set back into a run drawn under the old one.
#[test]
fn drawn_end_does_not_survive_a_q_or_cm_into_a_text_object_under_another_ctm() {
    for stream in [
        "q BT /F1 10 Tf 72 700 Td (abc) Tj ET Q q 1 0 0 1 50 0 cm BT /F1 10 Tf 72 700 Td [-3000 (def)] TJ ET Q",
        "BT /F1 10 Tf 72 700 Td (abc) Tj ET 1 0 0 1 50 0 cm BT /F1 10 Tf 72 700 Td [-3000 (def)] TJ ET",
    ] {
        let spans = span_boxes(stream);
        let texts: Vec<(&str, f32)> = spans.iter().map(|(t, x, _)| (t.as_str(), *x)).collect();
        assert_eq!(texts, [("abc", 72.0), (" ", 122.0), ("def", 152.0)], "{stream}");
    }
    // The same two text objects with no cm and no Q are set back into the run: the space goes at its end.
    let plain = span_boxes("BT /F1 10 Tf 72 700 Td (abc) Tj ET BT /F1 10 Tf 72 700 Td [-3000 (def)] TJ ET");
    let texts: Vec<(&str, f32)> = plain.iter().map(|(t, x, _)| (t.as_str(), *x)).collect();
    assert_eq!(texts, [("abc", 72.0), (" ", 87.0), ("def", 102.0)]);
}

/// A form `Do` draws under its own matrix: the run drawn before it is not the run the pen sets back into.
#[test]
fn drawn_end_does_not_survive_a_form_do() {
    let stream = "BT /F1 10 Tf 72 700 Td (abc) Tj ET /Fm0 Do BT /F1 10 Tf 72 700 Td [-3000 (def)] TJ ET";
    let spans = span_boxes(stream);
    let texts: Vec<(&str, f32)> = spans.iter().map(|(t, x, _)| (t.as_str(), *x)).collect();
    assert_eq!(texts, [("abc", 72.0), (" ", 72.0), ("def", 102.0)]);
}

/// `'` and `"` set the pen like `T*`: with a leading of 0 they put it back at the start of the run just
/// drawn, and a lead after them reads like one after `T*`.
#[test]
fn quote_operators_with_a_zero_leading_set_the_pen_back_like_t_star() {
    let t_star = span_boxes("BT /F1 10 Tf 72 700 Td (abc) Tj T* [-3000 (def)] TJ ET");
    for quote in ["() '", "0 0 () \""] {
        let stream = format!("BT /F1 10 Tf 0 TL 72 700 Td (abc) Tj {quote} [-3000 (def)] TJ ET");
        assert_eq!(span_boxes(&stream), t_star, "{quote}");
    }
}

/// A sub-threshold offset moves the pen but not the end of the drawn run: the space of the next word gap
/// still goes at that end, not 6 points inside the run.
#[test]
fn sub_threshold_offset_after_a_pen_set_back_keeps_the_drawn_end() {
    let spans = span_boxes("BT /F1 10 Tf 72 700 Td (abc) Tj 1 0 0 1 80 700 Tm [-100 -3000 (def)] TJ ET");
    let texts: Vec<(&str, f32)> = spans.iter().map(|(t, x, _)| (t.as_str(), *x)).collect();
    assert_eq!(texts[1], (" ", 87.0), "{texts:?}");
}

/// A `Td` back into the run right after an array that marked a word boundary: the seam carries the pen
/// of the array, and the set-back rule still reads the run drawn last.
#[test]
fn td_back_after_a_seam_marked_array_keeps_the_set_back_rule() {
    let stream = "BT /F1 10 Tf 72 700 Td [(abc) -3000] TJ -4 0 Td [-3000 (def)] TJ ET";
    let texts: Vec<String> = span_boxes(stream).into_iter().map(|(t, _, _)| t).collect();
    assert_eq!(texts, ["abc", " ", "def"]);
}

/// A negative lead under the word-boundary threshold is a kern, not a gap: it does not end the run at its
/// glyphs, so the run keeps the 8 points of the pen move (GH#2690).
#[test]
fn sub_threshold_negative_lead_after_a_tm_back_keeps_the_pen_move_width() {
    let spans = span_boxes("BT /F1 10 Tf 72 700 Td (abc) Tj 1 0 0 1 80 700 Tm [-50 (def)] TJ ET");
    assert_eq!(spans.first(), Some(&("abc".to_string(), 72.0, 8.0)));
}

/// A sub-threshold kern that carries the pen past the end kept for the run drawn last leaves that end
/// behind: the pen is then the end of what was drawn, and a later set back puts its space there. A kern
/// that stops short of the end, or a glyph after the kern, keep the space where it was. ~keep
#[test]
fn sub_threshold_kern_across_the_drawn_end_does_not_keep_it() {
    for (kern, space) in [("[-120]", 87.2), ("[-100]", 87.0), ("[-100 (x)]", 92.0)] {
        let stream = format!(
            "BT /F1 10 Tf 72 700 Td (abc) Tj 1 0 0 1 86 700 Tm {kern} TJ 1 0 0 1 80 700 Tm [-3000 (def)] TJ ET"
        );
        let spans = span_boxes(&stream);
        let texts: Vec<(&str, f32)> = spans.iter().map(|(t, x, _)| (t.as_str(), *x)).collect();
        assert_eq!(
            texts.iter().find(|(t, _)| *t == " ").map(|(_, x)| *x),
            Some(space),
            "{kern}: {texts:?}"
        );
    }
}

/// `cm` moves the pen in user space without drawing (GH#2322). A pen that the new matrix puts past the end
/// of the run drawn last keeps its space span at the pen. The stream has no `Q`, so only the `cm` can forget
/// the run. ~keep
#[test]
fn cm_that_puts_the_pen_past_the_drawn_run_keeps_the_space_at_the_pen() {
    let stream = "BT /F1 10 Tf 72 700 Td (abc) Tj ET 1 0 0 1 50 0 cm BT /F1 10 Tf 1 0 0 1 80 700 Tm [-700 (def)] TJ ET";
    assert_eq!(
        starts(stream),
        [start("abc", 72.0), start(" ", 130.0), start("def", 137.0)]
    );
}

/// `Q` restores the text matrix saved by `q`, which is a raw matrix of the run drawn before the `q`, read
/// under the CTM that `q` restored (GH#2322). A pen set back to 80 sits 7 before that raw end, but the run
/// drawn inside the `q` block (under a `cm` that comes before its glyphs) was the run drawn last: the space
/// goes at the pen. Only the `Q` can forget the end here. ~keep
#[test]
fn q_that_restores_the_text_matrix_of_an_earlier_run_keeps_the_space_at_the_pen() {
    let stream = "BT /F1 10 Tf 72 700 Td (abc) Tj ET \
                  q 1 0 0 1 50 0 cm BT 1 0 0 1 72 300 Tm (xyz) Tj ET Q \
                  BT 1 0 0 1 80 700 Tm [-700 (def)] TJ ET";
    assert_eq!(
        starts(stream),
        [
            start("abc", 72.0),
            start("xyz", 122.0),
            start(" ", 80.0),
            start("def", 87.0)
        ]
    );
}

/// A `q` ... `Q` block that only sets the pen draws nothing: the page reads as if the block were not there.
#[test]
fn q_block_that_draws_nothing_leaves_the_drawn_run_alone() {
    let plain = "BT /F1 10 Tf 72 700 Td (abc) Tj 1 0 0 1 200 700 Tm ET \
                 BT /F1 10 Tf 1 0 0 1 150 700 Tm [-3000 (def)] TJ ET";
    let with_block = "BT /F1 10 Tf 72 700 Td (abc) Tj 1 0 0 1 200 700 Tm ET q BT /F1 10 Tf 300 100 Td ET Q \
                      BT /F1 10 Tf 1 0 0 1 150 700 Tm [-3000 (def)] TJ ET";
    assert_eq!(
        starts(plain),
        [start("abc", 72.0), start(" ", 150.0), start("def", 180.0)]
    );
    assert_eq!(starts(with_block), starts(plain));
}

/// A run that a word-boundary lead ends at its glyphs is as wide as its glyph advances again (GH#2690).
#[test]
fn tj_lead_after_a_tm_back_into_the_run_makes_its_width_the_sum_of_its_glyph_widths() {
    let mut extractor = extractor();
    extractor.extract_spans = true;
    let stream = "BT /F1 10 Tf 72 700 Td (abc) Tj 1 0 0 1 80 700 Tm [-3000 (def)] TJ ET";
    extractor.run_content_stream(stream.as_bytes()).unwrap();
    extractor.flush_tj_span_buffer().unwrap();
    extractor.settle_span_holds();
    let texts: Vec<&str> = extractor.spans.iter().map(|s| s.text.as_str()).collect();
    assert_eq!(texts, ["abc", " ", "def"]);
    let run = &extractor.spans[0];
    assert_eq!(run.char_widths, [5.000_000_5_f32; 3]);
    assert_eq!(run.bbox.width, 15.0);
}

/// An extractor whose run drawn last ended at 87 and whose pen the last positioning set back to 80. ~keep
fn extractor_with_a_pen_set_back() -> TextExtractor<'static> {
    let mut extractor = extractor();
    extractor.state_stack.current_mut().font_size = 10.0;
    extractor.position_pen(Matrix::translation(72.0, 700.0));
    extractor.state_stack.current_mut().text_matrix = Matrix::translation(87.0, 700.0);
    extractor.position_pen(Matrix::translation(80.0, 700.0));
    assert!(extractor.drawn_end.is_some(), "no run was drawn before the set back");
    extractor
}

/// A run with a baseline is as wide as the distance to the end of the run drawn last. The other tests
/// that use this setup need it to be live. ~keep
#[test]
fn a_run_set_back_into_the_drawn_run_is_as_wide_as_the_distance_to_its_end() {
    let extractor = extractor_with_a_pen_set_back();
    let mut run = TjBuffer::new(extractor.state_stack.current(), None, None);
    run.start_matrix = Matrix::translation(72.0, 700.0);
    run.user_h_scale = 1.0;
    run.accumulated_width = 8.0;
    extractor.end_set_back_run_at_drawn_end(&mut run);
    assert_eq!(run.accumulated_width, 15.0);
}

/// A run whose matrix maps every glyph to one point has no baseline: its width stays as it was, not NaN.
/// The scale is 0 and the distance along the baseline is 0 / 0, so the finite check refuses it too
/// (`baseline_reach` in tj_arrays.rs). ~keep
#[test]
fn a_degenerate_run_matrix_keeps_the_run_width() {
    let extractor = extractor_with_a_pen_set_back();
    let mut run = TjBuffer::new(extractor.state_stack.current(), None, None);
    run.start_matrix = Matrix {
        a: 0.0,
        b: 0.0,
        c: 0.0,
        d: 0.0,
        e: 72.0,
        f: 700.0,
    };
    run.user_h_scale = 1.0;
    run.accumulated_width = 8.0;
    extractor.end_set_back_run_at_drawn_end(&mut run);
    assert_eq!(run.accumulated_width, 8.0);
}

/// A run with a baseline whose width does not fit in an `f32` keeps its width: the scale is positive, so
/// only the finite check refuses it. ~keep
#[test]
fn a_run_width_that_overflows_keeps_the_run_width() {
    let extractor = extractor_with_a_pen_set_back();
    let mut run = TjBuffer::new(extractor.state_stack.current(), None, None);
    run.start_matrix = Matrix::translation(72.0, 700.0);
    run.user_h_scale = f32::MIN_POSITIVE;
    run.accumulated_width = 8.0;
    extractor.end_set_back_run_at_drawn_end(&mut run);
    assert_eq!(run.accumulated_width, 8.0);
}

/// Whether a glyph or an offset moved the pen since the last positioning operator: the extractor then holds
/// a run drawn last that the next positioning operator would take as its end. ~keep
fn holds_a_drawn_run(extractor: &TextExtractor<'_>) -> bool {
    extractor
        .positioned_pen
        .is_some_and(|pen| pen != extractor.state_stack.current().text_matrix)
}

const FIRST_STREAM: &[u8] = b"BT /F1 10 Tf 72 700 Td (abc) Tj ET";
const NEXT_STREAM: &[u8] = b"BT /F1 10 Tf 1 0 0 1 80 700 Tm [-3000 (def)] TJ ET";

fn origins(spans: &[TextSpan]) -> Vec<(String, f32)> {
    spans.iter().map(|s| (s.text.clone(), s.bbox.x)).collect()
}

/// `extract_text_spans` does not see the run that an earlier call drew with the same extractor. ~keep
#[test]
fn extract_text_spans_forgets_the_run_drawn_by_the_last_call() {
    let mut reused = extractor();
    reused.extract_text_spans(FIRST_STREAM).unwrap();
    assert!(holds_a_drawn_run(&reused), "the first stream left no drawn run");
    let after_a_page = origins(&reused.extract_text_spans(NEXT_STREAM).unwrap());
    let fresh = origins(&extractor().extract_text_spans(NEXT_STREAM).unwrap());
    assert_eq!(after_a_page, fresh);
    assert_eq!(fresh, [start(" ", 80.0), start("def", 110.0)]);
}

/// `prepare_for_span_extraction` does not see the run that an earlier stream drew with the same extractor.
/// The first stream runs without `extract_text_spans`, which would reset the run itself. ~keep
#[test]
fn prepare_for_span_extraction_forgets_the_run_drawn_by_the_last_stream() {
    let spans = |e: &mut TextExtractor<'_>, stream: &[u8]| {
        e.prepare_for_span_extraction();
        e.run_content_stream(stream).unwrap();
        e.flush_tj_span_buffer().unwrap();
        origins(&e.spans)
    };
    let mut reused = extractor();
    spans(&mut reused, FIRST_STREAM);
    assert!(holds_a_drawn_run(&reused), "the first stream left no drawn run");
    let after_a_page = spans(&mut reused, NEXT_STREAM);
    let fresh = spans(&mut extractor(), NEXT_STREAM);
    assert_eq!(after_a_page, fresh);
    assert_eq!(fresh, [start(" ", 80.0), start("def", 110.0)]);
}

/// A `Td` with no line move right after an array that marked a word boundary sets the pen through the
/// positioning rule, so the set-back rule reads it. A `Td` back into the array's last word puts the space span
/// at the end of that word. A `Td` with a negative `tx` puts the pen before the array's start: the lead does
/// not reach past the end of the drawn run, so it adds no space span. The pen seam itself is pinned by the
/// `c01` and `c03` tests, not here. ~keep
#[test]
fn td_after_a_marked_array_sets_the_pen_back_into_the_array() {
    // "abc" ends at 87, "def" spans 117 to 132. ~keep
    let line = "BT /F1 10 Tf 72 700 Td [(abc) -3000 (def)] TJ";
    let back_into_def = starts(&format!("{line} 52 0 Td [-3000 (ghi)] TJ ET"));
    let want = [
        start("abc", 72.0),
        start(" ", 87.0),
        start("def", 117.0),
        start(" ", 132.0),
        start("ghi", 154.0),
    ];
    assert_eq!(back_into_def, want);
    let before_abc = starts(&format!("{line} -6 0 Td [-3000 (ghi)] TJ ET"));
    let want = [
        start("abc", 72.0),
        start(" ", 87.0),
        start("def", 117.0),
        start("ghi", 96.0),
    ];
    assert_eq!(before_abc, want);
}
