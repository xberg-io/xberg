//! Ligature glyphs at the glyph stage (GH#2322): a space glyph that takes no room, a ToUnicode value of
//! U+0000 for a named glyph, and a ligature presentation form that must come out as letters. ~keep

use super::super::*;
use super::extraction_and_rotation::create_test_font;

const LIGATURE_CODE: u32 = 0x80;

/// A ToUnicode map in which only the ligature code is mapped, to `target` (hex code points, no separators).
fn ligature_cmap(target: &str) -> crate::fonts::LazyCMap {
    let body = format!(
        "/CIDInit /ProcSet findresource begin 12 dict begin begincmap /CMapName /X def /CMapType 2 def\n\
         1 begincodespacerange <00> <FF> endcodespacerange\n1 beginbfchar\n<80> <{target}>\nendbfchar\n\
         endcmap CMapName currentdict /CMap defineresource pop end end\n"
    );
    crate::fonts::LazyCMap::new(body.into_bytes())
}

/// A simple font: every glyph is half an em wide, the space a quarter em, the ligature code one em.
fn ligature_font(target: &str, glyph_name: Option<&str>) -> FontInfo {
    let mut font = create_test_font();
    font.base_font = "Helvetica".to_string();
    font.widths = Some(
        (32u32..=0x84)
            .map(|code| match code {
                32 => 250.0,
                LIGATURE_CODE => 1000.0,
                _ => 500.0,
            })
            .collect(),
    );
    font.first_char = Some(32);
    font.last_char = Some(0x84);
    font.to_unicode = Some(ligature_cmap(target));
    if let Some(name) = glyph_name {
        font.diff_glyph_names.insert(LIGATURE_CODE as u8, name.to_string());
    }
    font
}

fn extractor_with(font: FontInfo) -> TextExtractor<'static> {
    let mut extractor = TextExtractor::new();
    extractor.add_font("F1".to_string(), font);
    extractor
}

fn span_text(extractor: &mut TextExtractor, stream: &[u8]) -> String {
    let spans = extractor.extract_text_spans(stream).unwrap();
    assert!(!spans.is_empty(), "the stream produced no spans");
    spans.iter().map(|s| s.text.as_str()).collect::<Vec<_>>().join("")
}

fn char_text(extractor: &mut TextExtractor, stream: &[u8]) -> String {
    let chars = extractor.extract(stream).unwrap();
    assert!(!chars.is_empty(), "the stream produced no characters");
    chars.iter().map(|c| c.char).collect()
}

/// The producer pulls the pen back by the width of the space (250 units), draws the space inside the
/// ligature box, and the next letter starts at the ligature's right edge.
const OVERLAP_SPACE: &[u8] = b"BT /F1 10 Tf 72 700 Td [(de\\200) 250 ( ne)] TJ ET";

#[test]
fn space_glyph_inside_the_ligature_box_is_not_a_word_break_in_spans() {
    let mut extractor = extractor_with(ligature_font("00660069", None));
    assert_eq!(span_text(&mut extractor, OVERLAP_SPACE), "define");
}

#[test]
fn space_glyph_inside_the_ligature_box_is_not_a_word_break_in_chars() {
    let mut extractor = extractor_with(ligature_font("00660069", None));
    assert_eq!(char_text(&mut extractor, OVERLAP_SPACE), "define");
}

#[test]
fn space_glyph_with_its_own_room_after_a_ligature_stays_in_spans() {
    let mut extractor = extractor_with(ligature_font("00660069", None));
    assert_eq!(
        span_text(&mut extractor, b"BT /F1 10 Tf 72 700 Td [(of \\200ne if)] TJ ET"),
        "of fine if"
    );
}

#[test]
fn space_glyph_with_its_own_room_after_a_ligature_stays_in_chars() {
    let mut extractor = extractor_with(ligature_font("00660069", None));
    assert_eq!(
        char_text(&mut extractor, b"BT /F1 10 Tf 72 700 Td [(of \\200ne if)] TJ ET"),
        "of fine if"
    );
}

#[test]
fn space_after_a_pulled_back_glyph_that_is_not_a_ligature_is_also_dropped() {
    let mut extractor = extractor_with(ligature_font("00660069", None));
    assert_eq!(
        span_text(&mut extractor, b"BT /F1 10 Tf 72 700 Td [(ab) 250 ( cd)] TJ ET"),
        "abcd"
    );
}

#[test]
fn small_kern_before_a_space_keeps_the_space_in_spans() {
    let mut extractor = extractor_with(ligature_font("00660069", None));
    assert_eq!(
        span_text(&mut extractor, b"BT /F1 10 Tf 72 700 Td [(ab) 5 ( cd)] TJ ET"),
        "ab cd"
    );
}

#[test]
fn to_unicode_zero_is_a_miss_and_the_glyph_name_decides() {
    let font = ligature_font("0000", Some("fi"));
    assert_eq!(font.char_to_unicode(LIGATURE_CODE).as_deref(), Some("fi"));
}

#[test]
fn to_unicode_zero_without_a_glyph_name_stays_unmapped() {
    let font = ligature_font("0000", None);
    assert_eq!(font.char_to_unicode(LIGATURE_CODE).as_deref(), Some("\u{FFFD}"));
}

#[test]
fn to_unicode_zero_ligature_comes_out_as_letters_in_spans() {
    let mut extractor = extractor_with(ligature_font("0000", Some("ffi")));
    assert_eq!(
        span_text(&mut extractor, b"BT /F1 10 Tf 72 700 Td (o\\200ce) Tj ET"),
        "office"
    );
}

#[test]
fn ligature_presentation_forms_come_out_as_letters() {
    for (target, letters) in [
        ("FB00", "ff"),
        ("FB01", "fi"),
        ("FB02", "fl"),
        ("FB03", "ffi"),
        ("FB04", "ffl"),
        ("FB05", "st"),
        ("FB06", "st"),
    ] {
        let font = ligature_font(target, None);
        assert_eq!(font.char_to_unicode(LIGATURE_CODE).as_deref(), Some(letters), "{target}");
    }
}

#[test]
fn ligature_presentation_form_spreads_its_advance_over_its_letters() {
    let mut extractor = extractor_with(ligature_font("FB01", None));
    assert_eq!(span_text(&mut extractor, b"BT /F1 10 Tf 72 700 Td (de\\200ne) Tj ET"), "define");
}
