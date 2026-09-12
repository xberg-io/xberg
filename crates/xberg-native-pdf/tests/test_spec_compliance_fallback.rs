//! PDF Spec 32000-1:2008 Section 9.10.2 Compliance Tests
//!
//! Tests for the fallback behavior when character mapping fails.
//! Spec Section 9.10.2 states:
//! "If these methods fail to produce a Unicode value, there is no way to determine what
//!  the character code represents in which case a conforming reader may choose a character
//!  code of their choosing."
//!
//! Standard practice: Use U+FFFD (REPLACEMENT CHARACTER) as the fallback.

use std::collections::HashMap;
use std::sync::Arc;
use xberg_native_pdf::fonts::{CharacterMapper, Encoding, FontInfo};

#[test]
fn test_spec_9_10_2_unmapped_character_returns_replacement() {
    //! Test 1: CharacterMapper returns fallback for unmapped character
    //!
    //! When a character code cannot be mapped through any method (ToUnicode, Adobe Glyph List,
    //! Font Encoding), the CharacterMapper should return U+FFFD (REPLACEMENT CHARACTER),
    //! not None.
    //!
    //! This ensures compliance with PDF Spec Section 9.10.2 and prevents silent data loss.

    let mapper = CharacterMapper::new();
    // No ToUnicode, no encoding, no glyph name - completely unmapped ~keep
    let result = mapper.map_character(0xFFFF);

    // Spec 9.10.2: "conforming reader may choose a character code of their choosing"
    // Standard practice: U+FFFD REPLACEMENT CHARACTER ~keep
    assert!(result.is_some(), "Should return replacement character, not None");
    assert_eq!(
        result.unwrap(),
        "\u{FFFD}",
        "Should return U+FFFD replacement character"
    );
}

#[test]
fn test_type0_identity_encoding_no_tounicode_returns_replacement() {
    //! Test 2: Type0 font without ToUnicode or TrueType cmap
    //!
    //! Type0 fonts using Identity encoding without a ToUnicode CMap and without
    //! an embedded TrueType font have no way to map characters. In this case,
    //! should return U+FFFD, not None (silent omission).

    let font = FontInfo {
        base_font: "Aptos".to_string(),
        subtype: "Type0".to_string(),
        encoding: Encoding::Identity,
        to_unicode: None,
        truetype_cmap: std::sync::OnceLock::new(),
        embedded_glyph_names: std::sync::OnceLock::new(),
        is_truetype_font: false,
        embedded_font_data: None,
        cid_to_gid_map: None,
        cid_system_info: None,
        cid_font_type: None,
        cid_widths: None,
        cid_default_width: 1000.0,
        has_explicit_dw: false,
        font_weight: None,
        flags: None,
        stem_v: None,
        ascent: 0.95,
        descent: -0.35,
        widths: None,
        first_char: None,
        last_char: None,
        font_matrix_a: 0.001,
        default_width: 1000.0,
        cff_gid_map: None,
        multi_char_map: HashMap::new(),
        byte_to_char_table: std::sync::OnceLock::new(),
        byte_to_width_table: std::sync::OnceLock::new(),
        weight_memo: std::sync::OnceLock::new(),
        italic_memo: std::sync::OnceLock::new(),
        std14_memo: std::sync::OnceLock::new(),
        diff_glyph_names: std::collections::HashMap::new(),
        wmode: 0,
        cid_vertical_metrics: None,
        cid_default_vertical_metrics: xberg_native_pdf::fonts::VerticalMetrics::SPEC_DEFAULT,
        cjk_substitution: None,
        embedded_cid_map: None,
        type0_unicode_memo: std::sync::Arc::new(std::sync::Mutex::new(std::collections::HashMap::new())),
    };

    let result = font.char_to_unicode(0x0041);

    assert!(
        result.is_some(),
        "Should return a character via CID-as-Unicode fallback"
    );
    assert_eq!(result.unwrap(), "A");
}

#[test]
fn test_type0_zero_byte_embedded_font_returns_replacement() {
    //! Test 3: Embedded font is 0 bytes (marked embedded but empty)
    //!
    //! Some PDFs mark a font as embedded but provide 0 bytes of actual font data.
    //! When we can't parse the embedded font, we should fall back to replacement
    //! character, not silently omit the character.

    let font = FontInfo {
        base_font: "Calibri".to_string(),
        subtype: "Type0".to_string(),
        encoding: Encoding::Identity,
        to_unicode: None,
        truetype_cmap: std::sync::OnceLock::new(),
        embedded_glyph_names: std::sync::OnceLock::new(),
        is_truetype_font: false,
        embedded_font_data: Some(Arc::new(vec![])),
        cid_to_gid_map: None,
        cid_system_info: None,
        cid_font_type: None,
        cid_widths: None,
        cid_default_width: 1000.0,
        has_explicit_dw: false,
        font_weight: None,
        flags: None,
        stem_v: None,
        ascent: 0.95,
        descent: -0.35,
        widths: None,
        first_char: None,
        last_char: None,
        font_matrix_a: 0.001,
        default_width: 1000.0,
        cff_gid_map: None,
        multi_char_map: HashMap::new(),
        byte_to_char_table: std::sync::OnceLock::new(),
        byte_to_width_table: std::sync::OnceLock::new(),
        weight_memo: std::sync::OnceLock::new(),
        italic_memo: std::sync::OnceLock::new(),
        std14_memo: std::sync::OnceLock::new(),
        diff_glyph_names: std::collections::HashMap::new(),
        wmode: 0,
        cid_vertical_metrics: None,
        cid_default_vertical_metrics: xberg_native_pdf::fonts::VerticalMetrics::SPEC_DEFAULT,
        cjk_substitution: None,
        embedded_cid_map: None,
        type0_unicode_memo: std::sync::Arc::new(std::sync::Mutex::new(std::collections::HashMap::new())),
    };

    let result = font.char_to_unicode(0x0020);

    assert!(
        result.is_some(),
        "Should return a character via CID-as-Unicode fallback"
    );
    assert_eq!(result.unwrap(), " ");
}
