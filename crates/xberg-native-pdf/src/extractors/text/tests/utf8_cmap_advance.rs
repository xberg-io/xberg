//! Width accounting for Type0 fonts whose /Encoding is a UTF-8 CMap (1 to 4 byte codes). ~keep

use super::super::*;
use super::extraction_and_rotation::create_test_font;

fn utf8_cmap_type0_font() -> FontInfo {
    let to_unicode = b"2 begincodespacerange\n<00> <7F>\n<E08080> <EFBFBF>\nendcodespacerange\n\
        2 beginbfchar\n<E697A5> <65E5>\n<E69CAC> <672C>\nendbfchar\n";
    FontInfo {
        base_font: "Utf8CMapFont".to_string(),
        subtype: "Type0".to_string(),
        encoding: crate::fonts::Encoding::Standard("UniJIS-UTF8-H".to_string()),
        to_unicode: Some(crate::fonts::LazyCMap::new(to_unicode.to_vec())),
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
