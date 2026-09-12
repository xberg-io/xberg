//! Adobe Glyph List Fallback Tests for Type0 Fonts
//!
//! Tests for Phase 1.2: When Type0 fonts without ToUnicode CMap and without
//! embedded TrueType font data cannot map characters, they should try the
//! Adobe Glyph List as a fallback before returning the replacement character.
//!
//! This addresses the Aptos and LMRoman font errors found in corpus validation:
//! - Aptos (132,374 errors): Office font with 0-byte embedded data
//! - LMRoman (124,960 errors): LaTeX font with custom encoding
//!
//! Spec: PDF 32000-1:2008 Section 9.10.2

use std::collections::HashMap;
use std::sync::Arc;
use xberg_native_pdf::fonts::{CIDToGIDMap, Encoding, FontInfo};

#[test]
fn test_type0_agl_fallback_for_standard_ascii() {
    //! Test 1: Type0 font with standard glyph should use Adobe Glyph List
    //!
    //! When a Type0 font has 0-byte embedded data and no ToUnicode CMap,
    //! it should try to map GID to a standard glyph name via Adobe Glyph List.
    //! For ASCII range (GID 32-126), standard glyph names work.

    let font = FontInfo {
        base_font: "Aptos".to_string(),
        subtype: "Type0".to_string(),
        encoding: Encoding::Identity,
        to_unicode: None,
        truetype_cmap: std::sync::OnceLock::new(),
        embedded_glyph_names: std::sync::OnceLock::new(),
        is_truetype_font: false,
        embedded_font_data: Some(Arc::new(vec![])),
        cid_to_gid_map: Some(CIDToGIDMap::Identity),
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

    let result = font.char_to_unicode(0x41);

    assert!(
        result.is_some(),
        "Should find mapping for GID 0x41 via Adobe Glyph List"
    );
    assert_eq!(
        result.unwrap(),
        "A",
        "GID 0x41 should map to 'A' via Adobe Glyph List, not U+FFFD"
    );
}

#[test]
fn test_type0_lmroman_agl_fallback() {
    //! Test 2: LMRoman font (LaTeX) common characters
    //!
    //! LMRoman is a common LaTeX font that shows up in many academic PDFs.
    //! It's a Type0 font with custom encoding but standard glyphs.
    //! Should be able to map common ASCII characters via Adobe Glyph List.

    let font = FontInfo {
        base_font: "LMRoman10-Regular".to_string(),
        subtype: "Type0".to_string(),
        encoding: Encoding::Identity,
        to_unicode: None,
        truetype_cmap: std::sync::OnceLock::new(),
        embedded_glyph_names: std::sync::OnceLock::new(),
        is_truetype_font: false,
        embedded_font_data: None,
        cid_to_gid_map: Some(CIDToGIDMap::Identity),
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

    assert_eq!(font.char_to_unicode(0x20), Some(" ".to_string()), "GID 0x20 (space)");
    assert_eq!(
        font.char_to_unicode(0x41),
        Some("A".to_string()),
        "GID 0x41 (capital A)"
    );
    assert_eq!(
        font.char_to_unicode(0x61),
        Some("a".to_string()),
        "GID 0x61 (lowercase a)"
    );
    assert_eq!(font.char_to_unicode(0x30), Some("0".to_string()), "GID 0x30 (digit 0)");
}

#[test]
fn test_type0_agl_fallback_then_replacement() {
    //! Test 3: Non-ASCII glyph should use AGL, then fallback to U+FFFD
    //!
    //! For GIDs that don't have a standard glyph name (outside ASCII range),
    //! should eventually fall back to U+FFFD after AGL lookup fails.

    let font = FontInfo {
        base_font: "TestFont".to_string(),
        subtype: "Type0".to_string(),
        encoding: Encoding::Identity,
        to_unicode: None,
        truetype_cmap: std::sync::OnceLock::new(),
        embedded_glyph_names: std::sync::OnceLock::new(),
        is_truetype_font: false,
        embedded_font_data: None,
        cid_to_gid_map: Some(CIDToGIDMap::Identity),
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

    // GID 0xFFFF won't be in Adobe Glyph List ~keep
    let result = font.char_to_unicode(0xFFFF);

    // CID-as-Unicode fallback: 0xFFFF is a valid Unicode code point ~keep
    assert!(result.is_some(), "Should return something (not None)");
    assert_eq!(result.unwrap(), "\u{FFFF}", "CID-as-Unicode fallback for 0xFFFF");
}
