//! Width accounting for Type0 fonts whose /Encoding is a UTF-8 CMap (1 to 4 byte codes). ~keep

use super::super::*;
use super::extraction_and_rotation::create_test_font;

fn utf8_cmap_type0_font() -> FontInfo {
    utf8_cmap_type0_font_with(b"2 beginbfchar\n<E697A5> <65E5>\n<E69CAC> <672C>\nendbfchar\n")
}

fn utf8_cmap_type0_font_with(bfchars: &[u8]) -> FontInfo {
    let mut to_unicode = b"2 begincodespacerange\n<00> <7F>\n<E08080> <EFBFBF>\nendcodespacerange\n".to_vec();
    to_unicode.extend_from_slice(bfchars);
    FontInfo {
        base_font: "Utf8CMapFont".to_string(),
        subtype: "Type0".to_string(),
        encoding: crate::fonts::Encoding::Standard("UniJIS-UTF8-H".to_string()),
        to_unicode: Some(crate::fonts::LazyCMap::new(to_unicode)),
        cid_font_type: Some("CIDFontType2".to_string()),
        has_explicit_dw: true,
        ..create_test_font()
    }
}

/// A UTF-8 CMap code is 1 to 4 bytes. Widths must be summed per decoded code, not per byte, or
/// "日本" (6 bytes) advances by six glyphs and carries six widths for two letters.
#[test]
fn utf8_cmap_widths_follow_the_decode_segmentation() {
    const NIHON: &[u8] = b"\xE6\x97\xA5\xE6\x9C\xAC";
    let mut extractor = TextExtractor::new();
    extractor.add_font("F1".to_string(), utf8_cmap_type0_font());
    let current_font = extractor.fonts.get("F1").cloned();
    extractor.set_cached_current_font(current_font);
    extractor.state_stack.current_mut().font_size = 12.0;
    extractor.state_stack.current_mut().horizontal_scaling = 100.0;
    let width = extractor.advance_position_for_string(NIHON, true).unwrap();
    assert!((width - 24.0).abs() < 1e-3, "two 1000-unit glyphs at 12pt, got {width}");

    for show in [
        b"BT /F1 12 Tf 100 700 Td (\xE6\x97\xA5\xE6\x9C\xAC) Tj ET".as_slice(),
        b"BT /F1 12 Tf 100 700 Td [(\xE6\x97\xA5\xE6\x9C\xAC)] TJ ET".as_slice(),
    ] {
        let mut extractor = TextExtractor::new();
        extractor.merging_config = SpanMergingConfig::legacy();
        extractor.add_font("F1".to_string(), utf8_cmap_type0_font());
        let spans = extractor.extract_text_spans(show).unwrap();
        assert_eq!(spans.len(), 1, "{spans:?}");
        assert_eq!(spans[0].text, "日本");
        assert_eq!(spans[0].char_widths.len(), 2, "{:?}", spans[0].char_widths);
        let total: f32 = spans[0].char_widths.iter().sum();
        assert!((total - 24.0).abs() < 1e-3, "got {total}");
    }
}

/// One 3-byte UTF-8 CMap code that decodes to "ff" is one ligature glyph. Counted in fixed one-byte
/// groups it was three codes, so the span lost its ligature mark.
#[test]
fn a_lone_utf8_cmap_code_that_decodes_to_a_ligature_marks_the_span() {
    let ff_font = || utf8_cmap_type0_font_with(b"1 beginbfchar\n<E697A5> <00660066>\nendbfchar\n");
    let state = crate::content::graphics_state::GraphicsStateStack::new();
    let mut buffer = TjBuffer::new(state.current(), None, Some(std::sync::Arc::new(ff_font())));
    buffer.append(b"\xE6\x97\xA5").unwrap();
    assert_eq!((buffer.unicode.as_str(), buffer.is_lone_ligature()), ("ff", true));

    let mut extractor = TextExtractor::new();
    extractor.add_font("F1".to_string(), ff_font());
    let spans = extractor
        .extract_text_spans(b"BT /F1 12 Tf 0 0 Td (\xE6\x97\xA5) Tj ET")
        .unwrap();
    let marks: Vec<_> = spans.into_iter().map(|s| (s.text, s.ligature_glyph)).collect();
    assert_eq!(marks, vec![("ff".to_string(), true)]);
}

/// `advance` equals `expected` up to f32 rounding of the 12pt font-size factor.
fn assert_advance(advance: f32, expected: f32) {
    assert!((advance - expected).abs() < 1e-3, "expected {expected}, got {advance}");
}

/// The advance of `text` shown at 12pt with word spacing `word_space`; every code is 1000 units wide.
fn advance_of(font: FontInfo, text: &[u8], word_space: f32) -> f32 {
    let mut extractor = TextExtractor::new();
    extractor.add_font("F1".to_string(), font);
    let current_font = extractor.fonts.get("F1").cloned();
    extractor.set_cached_current_font(current_font);
    let state = extractor.state_stack.current_mut();
    state.font_size = 12.0;
    state.horizontal_scaling = 100.0;
    state.word_space = word_space;
    extractor.advance_position_for_string(text, true).unwrap()
}

/// A lead byte whose sequence the string cuts short takes the bytes that remain as one code, and a
/// byte that cannot lead a sequence is a one-byte code, so neither costs a width per byte.
#[test]
fn utf8_cmap_truncated_and_invalid_sequences_advance_one_code_each() {
    // E697A5 is whole; E69C is a 3-byte lead cut to two bytes: two codes, 2 x 12. ~keep
    assert_advance(advance_of(utf8_cmap_type0_font(), b"\xE6\x97\xA5\xE6\x9C", 0.0), 24.0);
    // 0x80 is a continuation byte, not a lead: one code, then E697A5: 2 x 12. ~keep
    assert_advance(advance_of(utf8_cmap_type0_font(), b"\x80\xE6\x97\xA5", 0.0), 24.0);
}

/// ASCII in a UTF-8 CMap is one-byte codes, so the 0x20 space takes Tw (ISO 32000-1 §9.3.3).
#[test]
fn utf8_cmap_ascii_space_takes_word_spacing() {
    // "A", " ", E697A5: 3 x 12 plus Tw 5 on the space. ~keep
    assert_advance(advance_of(utf8_cmap_type0_font(), b"A \xE6\x97\xA5", 5.0), 41.0);
    assert_advance(advance_of(utf8_cmap_type0_font(), b"\xE6\x97\xA5", 5.0), 12.0);
}

/// Control: an Identity-H font keeps fixed two-byte codes. The same six bytes are three codes, and
/// the two-byte code 0x0020 takes no Tw.
#[test]
fn identity_cmap_keeps_two_byte_codes_for_the_same_bytes() {
    let identity = || FontInfo {
        encoding: crate::fonts::Encoding::Identity,
        to_unicode: None,
        ..utf8_cmap_type0_font()
    };
    assert!(!crate::fonts::unicode_decode::font_has_utf8_cmap(&identity()));
    assert_advance(advance_of(identity(), b"\xE6\x97\xA5\xE6\x9C\xAC", 0.0), 36.0);
    assert_advance(advance_of(identity(), b"\x00\x20", 5.0), 12.0);
}
