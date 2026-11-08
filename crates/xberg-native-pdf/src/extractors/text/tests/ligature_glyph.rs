//! A glyph that the font decodes to the letters of a ligature (`ff`, `fi`) marks its span, and the gap
//! checks read that mark instead of the letters (GH#2718). ~keep

use super::super::*;
use super::extraction_and_rotation::create_test_font;
use crate::fonts::{Encoding, LazyCMap};

/// A ToUnicode map in which the one code `code_hex` stands for the two letters "ff".
fn ff_cmap(codespace: &str, code_hex: &str) -> Vec<u8> {
    format!(
        "/CIDInit /ProcSet findresource begin 12 dict begin begincmap /CMapName /Test-UCS def /CMapType 2 def \
         1 begincodespacerange {codespace} endcodespacerange 1 beginbfchar <{code_hex}> <00660066> endbfchar \
         endcmap CMapName currentdict /CMap defineresource pop end end"
    )
    .into_bytes()
}

/// A simple font in which the byte 0x41 ("A") decodes to "ff".
fn simple_ff_font() -> FontInfo {
    let mut font = create_test_font();
    font.encoding = Encoding::Custom(HashMap::new());
    font.to_unicode = Some(LazyCMap::new(ff_cmap("<00> <FF>", "41")));
    font
}

/// A Type0 font in which the two-byte code 0x0041 decodes to "ff".
fn type0_ff_font() -> FontInfo {
    let mut font = create_test_font();
    font.subtype = "Type0".to_string();
    font.encoding = Encoding::Identity;
    font.to_unicode = Some(LazyCMap::new(ff_cmap("<0000> <FFFF>", "0041")));
    font
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

/// Control: the letters "ff" typed as two ordinary glyphs carry no mark, so real words and
/// abbreviations stay out of the ligature rule.
#[test]
fn two_ordinary_letters_ff_do_not_mark_the_span() {
    let spans = spans_of(create_test_font(), b"BT /F1 12 Tf 0 0 Td (ff) Tj ET");
    assert_eq!(spans, unmarked("ff"));
    let spans = spans_of(create_test_font(), b"BT /F1 12 Tf 0 0 Td [(f)(f)] TJ ET");
    assert_eq!(spans, unmarked("ff"));
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
    let mut extractor = TextExtractor::new();
    extractor.merging_config = SpanMergingConfig::legacy();
    extractor.spans = vec![
        run("di", 100.0, 10.0, false),
        run("ff", 113.5, 7.0, ligature_glyph),
        run("erent", 124.0, 30.0, false),
    ];
    extractor.merge_adjacent_spans();
    extractor.spans.into_iter().map(|s| s.text).collect()
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
