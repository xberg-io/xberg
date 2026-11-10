//! A glyph that the font decodes to the letters of a ligature (`ff`, `fi`) marks its span, and the gap
//! checks read that mark instead of the letters (GH#2718). ~keep

use super::super::*;
use super::extraction_and_rotation::create_test_font;
use crate::fonts::{Encoding, LazyCMap};
use std::sync::Arc;

/// A ToUnicode map in which the one code `code_hex` stands for the UTF-16 text `text_hex`.
fn one_code_cmap(codespace: &str, code_hex: &str, text_hex: &str) -> Vec<u8> {
    format!(
        "/CIDInit /ProcSet findresource begin 12 dict begin begincmap /CMapName /Test-UCS def /CMapType 2 def \
         1 begincodespacerange {codespace} endcodespacerange 1 beginbfchar <{code_hex}> <{text_hex}> endbfchar \
         endcmap CMapName currentdict /CMap defineresource pop end end"
    )
    .into_bytes()
}

/// A simple font in which the byte 0x41 ("A") decodes to the UTF-16 text `text_hex`.
fn simple_font_decoding_a_to(text_hex: &str) -> FontInfo {
    let mut font = create_test_font();
    font.encoding = Encoding::Custom(HashMap::new());
    font.to_unicode = Some(LazyCMap::new(one_code_cmap("<00> <FF>", "41", text_hex)));
    font
}

/// A Type0 font in which the two-byte code 0x0041 decodes to the UTF-16 text `text_hex`.
fn type0_font_decoding_0041_to(text_hex: &str) -> FontInfo {
    let mut font = create_test_font();
    font.subtype = "Type0".to_string();
    font.encoding = Encoding::Identity;
    font.to_unicode = Some(LazyCMap::new(one_code_cmap("<0000> <FFFF>", "0041", text_hex)));
    font
}

/// A simple font in which the byte 0x41 ("A") decodes to "ff".
fn simple_ff_font() -> FontInfo {
    simple_font_decoding_a_to("00660066")
}

/// A Type0 font in which the two-byte code 0x0041 decodes to "ff".
fn type0_ff_font() -> FontInfo {
    type0_font_decoding_0041_to("00660066")
}

fn spans_of(font: FontInfo, stream: &[u8]) -> Vec<(String, bool)> {
    let mut extractor = TextExtractor::new();
    extractor.add_font("F1".to_string(), font);
    let spans = extractor.extract_text_spans(stream).unwrap();
    spans.into_iter().map(|s| (s.text, s.ligature_glyph)).collect()
}

fn marked(text: &str) -> Vec<(String, bool)> {
    vec![(text.to_string(), true)]
}

fn unmarked(text: &str) -> Vec<(String, bool)> {
    vec![(text.to_string(), false)]
}

#[test]
fn a_simple_font_code_that_decodes_to_letters_marks_a_tj_span() {
    let spans = spans_of(simple_ff_font(), b"BT /F1 12 Tf 0 0 Td (A) Tj ET");
    assert_eq!(spans, marked("ff"));
}

#[test]
fn a_simple_font_code_that_decodes_to_letters_marks_a_tj_array_span() {
    let spans = spans_of(simple_ff_font(), b"BT /F1 12 Tf 0 0 Td [(A)] TJ ET");
    assert_eq!(spans, marked("ff"));
}

#[test]
fn a_type0_code_that_decodes_to_letters_marks_the_span() {
    let spans = spans_of(type0_ff_font(), b"BT /F1 12 Tf 0 0 Td <0041> Tj ET");
    assert_eq!(spans, marked("ff"));
    let spans = spans_of(type0_ff_font(), b"BT /F1 12 Tf 0 0 Td [<0041>] TJ ET");
    assert_eq!(spans, marked("ff"));
}

/// Every text that a Latin ligature code point stands for marks the span when one code gives it.
#[test]
fn a_code_that_decodes_to_any_ligature_expansion_marks_the_span() {
    for (text, hex) in [
        ("ff", "00660066"),
        ("fi", "00660069"),
        ("fl", "0066006C"),
        ("ffi", "006600660069"),
        ("ffl", "00660066006C"),
        ("st", "00730074"),
    ] {
        let spans = spans_of(simple_font_decoding_a_to(hex), b"BT /F1 12 Tf 0 0 Td (A) Tj ET");
        assert_eq!(spans, marked(text), "{text}");
    }
}

/// Control: a font that draws a short word as one glyph is not drawing a ligature. "to" between "ab"
/// and "cd" keeps both of its word gaps.
#[test]
fn a_code_that_decodes_to_a_short_word_does_not_mark_the_span() {
    let spans = spans_of(simple_font_decoding_a_to("0074006F"), b"BT /F1 12 Tf 0 0 Td (A) Tj ET");
    assert_eq!(spans, unmarked("to"));
    let spans = spans_of(
        simple_font_decoding_a_to("0074006F"),
        b"BT /F1 12 Tf 0 0 Td [(A)] TJ ET",
    );
    assert_eq!(spans, unmarked("to"));
    let spans = spans_of(
        type0_font_decoding_0041_to("0074006F"),
        b"BT /F1 12 Tf 0 0 Td <0041> Tj ET",
    );
    assert_eq!(spans, unmarked("to"));
}

/// Control: one letter, or one code that gives a letter and a digit or a space, is no ligature.
#[test]
fn a_single_letter_or_a_code_with_a_non_letter_does_not_mark_the_span() {
    let spans = spans_of(create_test_font(), b"BT /F1 12 Tf 0 0 Td (a) Tj ET");
    assert_eq!(spans, unmarked("a"));
    let spans = spans_of(simple_font_decoding_a_to("00660031"), b"BT /F1 12 Tf 0 0 Td (A) Tj ET");
    assert_eq!(spans, unmarked("f1"));
    let spans = spans_of(simple_font_decoding_a_to("00660020"), b"BT /F1 12 Tf 0 0 Td (A) Tj ET");
    assert_eq!(spans, unmarked("f "));
}

/// Control: the letters "ff" typed as two ordinary glyphs carry no mark, so real words and
/// abbreviations stay out of the ligature rule. Each case takes a different decoder: a simple font in
/// a Tj, a simple font in a TJ array, and a Type0 font.
#[test]
fn two_ordinary_letters_ff_do_not_mark_the_span() {
    let spans = spans_of(create_test_font(), b"BT /F1 12 Tf 0 0 Td (ff) Tj ET");
    assert_eq!(spans, unmarked("ff"));
    let spans = spans_of(create_test_font(), b"BT /F1 12 Tf 0 0 Td [(ff)] TJ ET");
    assert_eq!(spans, unmarked("ff"));
    let spans = spans_of(create_test_font(), b"BT /F1 12 Tf 0 0 Td [(f)(f)] TJ ET");
    assert_eq!(spans, unmarked("ff"));
    let f_font = type0_font_decoding_0041_to("0066");
    let spans = spans_of(f_font.clone(), b"BT /F1 12 Tf 0 0 Td <00410041> Tj ET");
    assert_eq!(spans, unmarked("ff"));
    let spans = spans_of(f_font, b"BT /F1 12 Tf 0 0 Td [<00410041>] TJ ET");
    assert_eq!(spans, unmarked("ff"));
}

/// Control: two strings that each give one "f" fill one run with "ff", and a run of two codes is no
/// ligature even though its whole text is a ligature's letters.
#[test]
fn two_strings_that_spell_ff_do_not_mark_the_span() {
    let spans = spans_of(
        simple_font_decoding_a_to("0066"),
        b"BT /F1 12 Tf 0 0 Td (A) Tj (A) Tj ET",
    );
    assert_eq!(spans, unmarked("ff"));
    let spans = spans_of(simple_font_decoding_a_to("0066"), b"BT /F1 12 Tf 0 0 Td [(A)(A)] TJ ET");
    assert_eq!(spans, unmarked("ff"));
    let spans = spans_of(
        type0_font_decoding_0041_to("0066"),
        b"BT /F1 12 Tf 0 0 Td [<0041><0041>] TJ ET",
    );
    assert_eq!(spans, unmarked("ff"));
}

/// Control at the buffer's own simple-font decoder. The extractor decodes a simple font inline and
/// hands the buffer only Type0 or fontless text, so this decoder runs when the buffer's font is older
/// than the extractor's (a form XObject that ends without ET). One code that gives "ff" marks the
/// buffer; two codes that give "ff" do not.
#[test]
fn the_buffer_decoder_of_a_simple_font_marks_one_code_and_not_two() {
    let state = crate::content::graphics_state::GraphicsStateStack::new();
    let mut buffer = TjBuffer::new(state.current(), None, Some(Arc::new(simple_ff_font())));
    buffer.append(b"A").unwrap();
    assert_eq!((buffer.unicode.as_str(), buffer.is_lone_ligature()), ("ff", true));
    let mut buffer = TjBuffer::new(state.current(), None, Some(Arc::new(create_test_font())));
    buffer.append(b"ff").unwrap();
    assert_eq!((buffer.unicode.as_str(), buffer.is_lone_ligature()), ("ff", false));
}

/// The buffer's own simple-font decoder reads a UTF-8 literal like the inline decoder, and a U+FFFE
/// in it is dropped: the symbol-loss marker is the extractor's alone.
#[test]
fn the_buffer_decoder_of_a_simple_font_drops_a_fffe_from_a_utf8_literal() {
    let state = crate::content::graphics_state::GraphicsStateStack::new();
    let mut font = create_test_font();
    font.to_unicode = None;
    let mut buffer = TjBuffer::new(state.current(), None, Some(Arc::new(font)));
    buffer.append(b"\xef\xbf\xbe\xd0\x96BC").unwrap();
    assert_eq!(buffer.unicode.as_str(), "\u{416}BC");
}

/// An `/ActualText` of "ff" or "fi" that replaces a sequence stands for one ligature glyph, so its
/// run is marked like a decoded ligature, in a Tj and in a TJ array.
#[test]
fn a_lone_actual_text_ligature_marks_the_span() {
    let stream = b"BT /F1 12 Tf 0 0 Td /Span <</ActualText (ff)>> BDC (xx) Tj EMC ET";
    assert_eq!(spans_of(create_test_font(), stream), marked("ff"));
    let stream = b"BT /F1 12 Tf 0 0 Td /Span <</ActualText (fi)>> BDC [(xx)] TJ EMC ET";
    assert_eq!(spans_of(create_test_font(), stream), marked("fi"));
}

/// The item's case with the ligature as an `/ActualText` over a TJ array: "di", the replacement "ff"
/// drawn over two glyphs, "erent". The replacement is marked and as wide as its glyphs, so the gap
/// after it is measured from its end and the merged line is one word.
#[test]
fn an_actual_text_ligature_in_a_tj_array_joins_the_word_around_it() {
    let stream = b"BT /F1 12 Tf 0 0 Td (di) Tj /Span <</ActualText (ff)>> BDC [(xx)] TJ EMC (erent) Tj ET";
    let texts: Vec<String> = spans_of(create_test_font(), stream)
        .into_iter()
        .map(|(text, _)| text)
        .collect();
    assert_eq!(texts, vec!["different".to_string()]);
}

fn merged_texts(font: FontInfo, stream: &[u8]) -> Vec<String> {
    let mut extractor = TextExtractor::new();
    extractor.add_font("F1".to_string(), font);
    let spans = extractor.extract_text_spans(stream).unwrap();
    spans.into_iter().map(|s| s.text).collect()
}

fn span_widths(font: FontInfo, stream: &[u8]) -> Vec<(String, f32)> {
    let mut extractor = TextExtractor::new();
    extractor.add_font("F1".to_string(), font);
    let spans = extractor.extract_text_spans(stream).unwrap();
    spans.into_iter().map(|s| (s.text, s.bbox.width)).collect()
}

/// An `/ActualText` run drawn with a TJ array is as wide as the glyphs it replaces, like the same run
/// drawn with Tj, so the gap to the next run is measured from its end and not from its start.
#[test]
fn an_actual_text_run_in_a_tj_array_has_the_width_of_its_glyphs() {
    let tj = span_widths(
        create_test_font(),
        b"BT /F1 12 Tf 0 0 Td /Span <</ActualText (ff)>> BDC (x) Tj EMC ET",
    );
    let array = span_widths(
        create_test_font(),
        b"BT /F1 12 Tf 0 0 Td /Span <</ActualText (ff)>> BDC [(x)] TJ EMC ET",
    );
    assert_eq!(tj.len(), 1, "{tj:?}");
    assert!(tj[0].1 > 0.0, "{tj:?}");
    assert_eq!(array, tj);
}

/// A second TJ array inside the same `/ActualText` scope draws more of the replaced glyphs. Their
/// width lands on the open replacement run, as it does when one Tj draws every glyph.
#[test]
fn an_actual_text_run_spans_every_array_in_its_scope() {
    let one = span_widths(
        create_test_font(),
        b"BT /F1 12 Tf 0 0 Td /Span <</ActualText (ff)>> BDC (xy) Tj EMC ET",
    );
    let two = span_widths(
        create_test_font(),
        b"BT /F1 12 Tf 0 0 Td /Span <</ActualText (ff)>> BDC (x) Tj [(y)] TJ EMC ET",
    );
    assert_eq!(one.len(), 1, "{one:?}");
    assert_eq!(one[0].1, 12.0, "{one:?}");
    assert_eq!(two, one);
}

/// Two TJ arrays in one `/ActualText` scope are one replacement run as wide as both, like one Tj.
#[test]
fn an_actual_text_run_spans_two_tj_arrays_in_its_scope() {
    let one = span_widths(
        create_test_font(),
        b"BT /F1 12 Tf 0 0 Td /Span <</ActualText (ff)>> BDC (xy) Tj EMC ET",
    );
    let two = span_widths(
        create_test_font(),
        b"BT /F1 12 Tf 0 0 Td /Span <</ActualText (ff)>> BDC [(x)] TJ [(y)] TJ EMC ET",
    );
    assert_eq!(one.len(), 1, "{one:?}");
    assert_eq!(one[0].1, 12.0, "{one:?}");
    assert_eq!(two, one);
}

/// A TJ offset inside an `/ActualText` array moves the pen like it does in an ordinary array, so the
/// replacement run is as wide as the matrix advance: 6 + 6 for the glyphs and 300/1000 em = 3.6 pt.
#[test]
fn an_actual_text_run_includes_the_offsets_of_its_tj_array() {
    let widths = span_widths(
        create_test_font(),
        b"BT /F1 12 Tf 0 0 Td /Span <</ActualText (ff)>> BDC [(x) -300 (y)] TJ EMC ET",
    );
    assert_eq!(widths.len(), 1, "{widths:?}");
    assert!((widths[0].1 - 15.6).abs() < 1e-3, "{widths:?}");
}

/// A Td ends the open replacement run, so a Tj after it in the same scope draws no more of the
/// replaced glyphs and adds nothing to the run.
#[test]
fn a_td_inside_the_scope_closes_the_open_replacement_run() {
    let spans = span_widths(
        create_test_font(),
        b"BT /F1 12 Tf 0 0 Td /Span <</ActualText (ff)>> BDC [(x)] TJ 0 -30 Td (z) Tj EMC ET",
    );
    assert_eq!(spans, vec![("ff".to_string(), 6.0)]);
}

/// An EMC ends the scope: a Tj after it is a new run that the replacement does not absorb.
#[test]
fn an_emc_closes_the_open_replacement_run() {
    let spans = span_widths(
        create_test_font(),
        b"BT /F1 12 Tf 0 0 Td /Span <</ActualText (ff)>> BDC [(x)] TJ EMC 0 -30 Td (z) Tj ET",
    );
    assert_eq!(spans, vec![("ff".to_string(), 6.0), ("z".to_string(), 5.328)]);
}

/// The width of an `/ActualText` array decides the space after it: a word that follows directly joins
/// it, and a word set 20 pt along stays a separate run.
#[test]
fn the_gap_after_an_actual_text_array_run_is_measured_from_its_end() {
    let font = create_test_font;
    let joined = merged_texts(
        font(),
        b"BT /F1 12 Tf 0 0 Td /Span <</ActualText (ab)>> BDC [(xx)] TJ EMC (cd) Tj ET",
    );
    assert_eq!(joined, vec!["abcd".to_string()]);
    let spaced = merged_texts(
        font(),
        b"BT /F1 12 Tf 0 0 Td /Span <</ActualText (ab)>> BDC [(xx)] TJ EMC 20 0 Td (cd) Tj ET",
    );
    assert_eq!(spaced, vec!["ab".to_string(), "cd".to_string()]);
}

/// Control: an `/ActualText` that is a short word is no ligature.
#[test]
fn a_lone_actual_text_word_does_not_mark_the_span() {
    let stream = b"BT /F1 12 Tf 0 0 Td /Span <</ActualText (to)>> BDC (xx) Tj EMC ET";
    assert_eq!(spans_of(create_test_font(), stream), unmarked("to"));
}

/// Control: a run of more than one glyph is not a lone ligature, even when one glyph is.
#[test]
fn a_run_of_several_glyphs_does_not_mark_the_span() {
    let spans = spans_of(simple_ff_font(), b"BT /F1 12 Tf 0 0 Td (AA) Tj ET");
    assert_eq!(spans, unmarked("ffff"));
    let spans = spans_of(simple_ff_font(), b"BT /F1 12 Tf 0 0 Td [(di)(A)] TJ ET");
    assert_eq!(spans, unmarked("diff"));
    let spans = spans_of(type0_ff_font(), b"BT /F1 12 Tf 0 0 Td <00410041> Tj ET");
    assert_eq!(spans, unmarked("ffff"));
}

fn run(text: &str, x: f32, width: f32, ligature_glyph: bool) -> TextSpan {
    TextSpan {
        text: text.to_string(),
        bbox: Rect::new(x, 700.0, width, 12.0),
        ligature_glyph,
        ..TextSpan::default()
    }
}

/// "di", a lone expanded "ff" and "erent", 3.5 pt apart at 12 pt: wider than the ordinary 3 pt word gap
/// of a run with no font, narrower than the 4.5 pt gap that a ligature edge allows.
fn merged_text(ligature_glyph: bool) -> Vec<String> {
    merged_text_around("ff", ligature_glyph)
}

/// "di", the run `middle` and "erent", 3.5 pt apart at 12 pt.
fn merged_text_around(middle: &str, ligature_glyph: bool) -> Vec<String> {
    let mut extractor = TextExtractor::new();
    extractor.merging_config = SpanMergingConfig::legacy();
    extractor.spans = vec![
        run("di", 100.0, 10.0, false),
        run(middle, 113.5, 7.0, ligature_glyph),
        run("erent", 124.0, 30.0, false),
    ];
    extractor.merge_adjacent_spans();
    extractor.spans.into_iter().map(|s| s.text).collect()
}

/// A marked run whose text is no longer a ligature's letters is no ligature to the merge pass, on
/// either side of it: "ff2" between "di" and "erent" keeps both spaces, as the same unmarked run does.
#[test]
fn merging_keeps_the_spaces_around_a_marked_ligature_that_gained_text() {
    assert_eq!(merged_text_around("ff2", false), vec!["di ff2 erent".to_string()]);
    assert_eq!(merged_text_around("ff2", true), merged_text_around("ff2", false));
}

#[test]
fn merging_joins_the_word_on_both_sides_of_an_expanded_ligature() {
    assert_eq!(merged_text(true), vec!["different".to_string()]);
}

/// Control: the same three spans with an "ff" that is not an expanded ligature keep their spaces.
#[test]
fn merging_keeps_the_spaces_around_a_plain_ff_run() {
    assert_eq!(merged_text(false), vec!["di ff erent".to_string()]);
}

/// The merged run is no longer one glyph, so it loses the mark.
#[test]
fn merging_clears_the_mark_on_the_merged_run() {
    let mut extractor = TextExtractor::new();
    extractor.merging_config = SpanMergingConfig::legacy();
    extractor.spans = vec![run("di", 100.0, 10.0, false), run("ff", 113.5, 7.0, true)];
    extractor.merge_adjacent_spans();
    assert_eq!(extractor.spans.len(), 1);
    assert!(!extractor.spans[0].ligature_glyph);
}

/// A flagged run that absorbs the next span is a longer run, so it loses the mark. The first span
/// carries the mark here: the merge itself has to clear it.
#[test]
fn merging_clears_the_mark_a_flagged_first_run_carried() {
    let mut extractor = TextExtractor::new();
    extractor.merging_config = SpanMergingConfig::legacy();
    extractor.spans = vec![run("ff", 100.0, 7.0, true), run("erent", 107.5, 30.0, false)];
    extractor.merge_adjacent_spans();
    assert_eq!(extractor.spans.len(), 1);
    assert_eq!(extractor.spans[0].text, "fferent");
    assert!(!extractor.spans[0].ligature_glyph);
}

/// A marked-content start flushes the run, so the ActualText text is a span of its own that merges with
/// the lone ligature before it. The merged run is longer than one glyph and carries no mark.
#[test]
fn a_lone_ligature_then_actual_text_merges_into_an_unmarked_span() {
    let stream = b"BT /F1 12 Tf 0 0 Td (A) Tj /Span <</ActualText (ab)>> BDC (Z) Tj EMC ET";
    assert_eq!(spans_of(simple_ff_font(), stream), unmarked("ffab"));
}

/// A fixed-pitch word gap pushes a space into the run, so the run is longer than one glyph. No glyph
/// follows the gap here, so only the longer run can end the mark.
#[test]
fn a_fixed_pitch_space_after_a_lone_ligature_leaves_the_span_unmarked() {
    let mut font = simple_ff_font();
    font.flags = Some(1);
    let stream = b"BT /F1 12 Tf 1 0 0 1 100 700 Tm (A) Tj 1 0 0 1 160 700 Tm ET";
    let spans = spans_of(font, stream);
    assert_eq!(spans.len(), 1, "{spans:?}");
    assert!(spans[0].0.starts_with("ff"), "{spans:?}");
    assert!(!spans[0].1, "{spans:?}");
}
