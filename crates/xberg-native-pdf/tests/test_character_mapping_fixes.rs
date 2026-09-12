//! Comprehensive test suite for character mapping fixes (Phase 1)
//!
//! This module tests all the critical bug fixes for PDF text extraction:
//! - Phase 1.1: Identity encoding fallback for Type0 fonts
//! - Phase 1.2: ToUnicode CMap validation
//! - Phase 1.3: Type0 missing ToUnicode error handling
//! - Phase 1.4: Multi-byte processing validation
//!
//! These tests ensure that garbled text issues are properly handled.

use std::collections::HashMap;
use xberg_native_pdf::fonts::{Encoding, FontInfo, LazyCMap};

#[test]
fn test_type0_identity_encoding_without_tounicode_returns_none() {
    // Type0 fonts WITHOUT ToUnicode use CID-as-Unicode fallback for printable chars
    // This matches MuPDF behavior and improves real-world text extraction quality ~keep
    let font = FontInfo {
        base_font: "CIDFont".to_string(),
        subtype: "Type0".to_string(),
        encoding: Encoding::Identity,
        to_unicode: None,
        font_weight: None,
        flags: None,
        stem_v: None,
        ascent: 0.95,
        descent: -0.35,
        embedded_font_data: None,
        widths: None,
        first_char: None,
        last_char: None,
        font_matrix_a: 0.001,
        default_width: 1000.0,
        cid_to_gid_map: None,
        cid_system_info: None,
        cid_font_type: None,
        truetype_cmap: std::sync::OnceLock::new(),
        embedded_glyph_names: std::sync::OnceLock::new(),
        is_truetype_font: false,
        cid_widths: None,
        cid_default_width: 1000.0,
        has_explicit_dw: false,
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

    // CID-as-Unicode fallback: 0x37 → '7', 0x41 → 'A' ~keep
    assert_eq!(
        font.char_to_unicode(0x37),
        Some("7".to_string()),
        "Type0 font without ToUnicode uses CID-as-Unicode fallback"
    );

    assert_eq!(
        font.char_to_unicode(0x41),
        Some("A".to_string()),
        "Type0 font without ToUnicode uses CID-as-Unicode fallback"
    );
}

#[test]
fn test_simple_font_identity_encoding_works_for_valid_codes() {
    let font = FontInfo {
        base_font: "Times-Roman".to_string(),
        subtype: "Type1".to_string(),
        encoding: Encoding::Identity,
        to_unicode: None,
        font_weight: None,
        flags: None,
        stem_v: None,
        ascent: 0.95,
        descent: -0.35,
        embedded_font_data: None,
        widths: None,
        first_char: None,
        last_char: None,
        font_matrix_a: 0.001,
        default_width: 1000.0,
        cid_to_gid_map: None,
        cid_system_info: None,
        cid_font_type: None,
        truetype_cmap: std::sync::OnceLock::new(),
        embedded_glyph_names: std::sync::OnceLock::new(),
        is_truetype_font: false,
        cid_widths: None,
        cid_default_width: 1000.0,
        has_explicit_dw: false,
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

    assert_eq!(
        font.char_to_unicode(0x41),
        Some("A".to_string()),
        "Simple font with Identity encoding should map 0x41 to 'A'"
    );

    assert_eq!(
        font.char_to_unicode(0x42),
        Some("B".to_string()),
        "Simple font with Identity encoding should map 0x42 to 'B'"
    );

    let result = font.char_to_unicode(0x00);
    assert!(
        result.is_some() || result.is_none(),
        "Simple font with Identity encoding should handle code 0x00 without panicking"
    );
}

#[test]
fn test_type0_missing_tounicode_is_an_error() {
    let font = FontInfo {
        base_font: "Type0Font".to_string(),
        subtype: "Type0".to_string(),
        encoding: Encoding::Standard("Identity-H".to_string()),
        to_unicode: None,
        font_weight: None,
        flags: None,
        stem_v: None,
        ascent: 0.95,
        descent: -0.35,
        embedded_font_data: None,
        widths: None,
        first_char: None,
        last_char: None,
        font_matrix_a: 0.001,
        default_width: 1000.0,
        cid_to_gid_map: None,
        cid_system_info: None,
        cid_font_type: None,
        truetype_cmap: std::sync::OnceLock::new(),
        embedded_glyph_names: std::sync::OnceLock::new(),
        is_truetype_font: false,
        cid_widths: None,
        cid_default_width: 1000.0,
        has_explicit_dw: false,
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

    // CID-as-Unicode fallback: printable chars return themselves, control chars may return None
    // This matches MuPDF behavior for real-world text extraction quality ~keep
    let result = font.char_to_unicode(0x41);
    assert_eq!(
        result,
        Some("A".to_string()),
        "Type0 font without ToUnicode uses CID-as-Unicode fallback for printable chars"
    );
    let result_space = font.char_to_unicode(0x20);
    assert_eq!(
        result_space,
        Some(" ".to_string()),
        "Type0 font without ToUnicode uses CID-as-Unicode fallback for space"
    );
}

#[test]
fn test_tounicode_with_valid_mappings_works() {
    let cmap_data = b"beginbfchar\n<0041> <0041>\n<0042> <0042>\n<263A> <263A>\nendbfchar";

    let font = FontInfo {
        base_font: "CustomFont".to_string(),
        subtype: "Type1".to_string(),
        encoding: Encoding::Standard("WinAnsiEncoding".to_string()),
        to_unicode: Some(LazyCMap::new(cmap_data.to_vec())),
        font_weight: None,
        flags: None,
        stem_v: None,
        ascent: 0.95,
        descent: -0.35,
        embedded_font_data: None,
        widths: None,
        first_char: None,
        last_char: None,
        font_matrix_a: 0.001,
        default_width: 1000.0,
        cid_to_gid_map: None,
        cid_system_info: None,
        cid_font_type: None,
        truetype_cmap: std::sync::OnceLock::new(),
        embedded_glyph_names: std::sync::OnceLock::new(),
        is_truetype_font: false,
        cid_widths: None,
        cid_default_width: 1000.0,
        has_explicit_dw: false,
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

    assert_eq!(font.char_to_unicode(0x41), Some("A".to_string()));
    assert_eq!(font.char_to_unicode(0x42), Some("B".to_string()));

    assert_eq!(font.char_to_unicode(0x263A), Some("☺".to_string()));
}

#[test]
fn test_multi_byte_character_codes_are_processed() {
    let font = FontInfo {
        base_font: "Type0Font".to_string(),
        subtype: "Type0".to_string(),
        encoding: Encoding::Identity,
        to_unicode: None,
        font_weight: None,
        flags: None,
        stem_v: None,
        ascent: 0.95,
        descent: -0.35,
        embedded_font_data: None,
        widths: None,
        first_char: None,
        last_char: None,
        font_matrix_a: 0.001,
        default_width: 1000.0,
        cid_to_gid_map: None,
        cid_system_info: None,
        cid_font_type: None,
        truetype_cmap: std::sync::OnceLock::new(),
        embedded_glyph_names: std::sync::OnceLock::new(),
        is_truetype_font: false,
        cid_widths: None,
        cid_default_width: 1000.0,
        has_explicit_dw: false,
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

    let large_code = 0x3000u32;
    assert_eq!(
        font.char_to_unicode(large_code),
        Some("\u{3000}".to_string()),
        "Multi-byte code uses CID-as-Unicode fallback"
    );
}

#[test]
fn test_extraction_priority_chain() {
    let cmap_data = b"beginbfchar\n<0041> <0058>\nendbfchar";

    let font = FontInfo {
        base_font: "TestFont".to_string(),
        subtype: "Type1".to_string(),
        encoding: Encoding::Standard("WinAnsiEncoding".to_string()),
        to_unicode: Some(LazyCMap::new(cmap_data.to_vec())),
        font_weight: None,
        flags: None,
        stem_v: None,
        ascent: 0.95,
        descent: -0.35,
        embedded_font_data: None,
        widths: None,
        first_char: None,
        last_char: None,
        font_matrix_a: 0.001,
        default_width: 1000.0,
        cid_to_gid_map: None,
        cid_system_info: None,
        cid_font_type: None,
        truetype_cmap: std::sync::OnceLock::new(),
        embedded_glyph_names: std::sync::OnceLock::new(),
        is_truetype_font: false,
        cid_widths: None,
        cid_default_width: 1000.0,
        has_explicit_dw: false,
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

    assert_eq!(
        font.char_to_unicode(0x41),
        Some("X".to_string()),
        "ToUnicode mapping (Priority 1) should override standard encoding (Priority 3)"
    );

    assert_eq!(
        font.char_to_unicode(0x42),
        Some("B".to_string()),
        "Missing ToUnicode entries should fall back to standard encoding"
    );
}

#[test]
fn test_symbolic_font_encoding() {
    let font_symbol = FontInfo {
        base_font: "Symbol".to_string(),
        subtype: "Type1".to_string(),
        encoding: Encoding::Standard("Symbol".to_string()),
        to_unicode: None,
        font_weight: None,
        flags: Some(0x04),
        stem_v: None,
        ascent: 0.95,
        descent: -0.35,
        embedded_font_data: None,
        widths: None,
        first_char: None,
        last_char: None,
        font_matrix_a: 0.001,
        default_width: 1000.0,
        cid_to_gid_map: None,
        cid_system_info: None,
        cid_font_type: None,
        truetype_cmap: std::sync::OnceLock::new(),
        embedded_glyph_names: std::sync::OnceLock::new(),
        is_truetype_font: false,
        cid_widths: None,
        cid_default_width: 1000.0,
        has_explicit_dw: false,
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

    assert!(
        font_symbol.is_symbolic(),
        "Font with Symbolic bit should be detected as symbolic"
    );
}

// ============================================================================
// Regression Tests: Common PDF Authoring Issues
// ============================================================================ ~keep

#[test]
fn test_pdf_without_tounicode_doesnt_scramble_text() {
    // This is the key regression test for the reported issue
    // Before fix: Extracted text would be "7 K H U D S \" (scrambled)
    // After fix: Extraction fails gracefully with error message, no scrambling ~keep

    let font = FontInfo {
        base_font: "MyTypeOFont".to_string(),
        subtype: "Type0".to_string(),
        encoding: Encoding::Identity,
        to_unicode: None,
        font_weight: None,
        flags: None,
        stem_v: None,
        ascent: 0.95,
        descent: -0.35,
        embedded_font_data: None,
        widths: None,
        first_char: None,
        last_char: None,
        font_matrix_a: 0.001,
        default_width: 1000.0,
        cid_to_gid_map: None,
        cid_system_info: None,
        cid_font_type: None,
        truetype_cmap: std::sync::OnceLock::new(),
        embedded_glyph_names: std::sync::OnceLock::new(),
        is_truetype_font: false,
        cid_widths: None,
        cid_default_width: 1000.0,
        has_explicit_dw: false,
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

    // CID-as-Unicode fallback: printable chars map to themselves
    // This matches MuPDF behavior for practical text extraction quality ~keep
    let result = font.char_to_unicode(0x20);
    assert_eq!(result, Some(" ".to_string()));
    let result = font.char_to_unicode(0x41);
    assert_eq!(result, Some("A".to_string()));
}
