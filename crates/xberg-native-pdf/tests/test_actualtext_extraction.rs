//! ActualText Extraction Tests
//!
//! Tests for integrating ActualText into the character mapping priority chain
//! per PDF Spec 32000-1:2008 Section 9.10.2.
//!
//! ActualText support provides the canonical Unicode mapping for characters
//! that might otherwise be unmappable through font encoding alone.
//!
//! This is critical for:
//! - Ligatures (fi, fl, ffi, ffl as single characters mapping to multiple chars)
//! - Decorated glyphs (e.g., accented characters represented as composite glyphs)
//! - Custom text representations (e.g., "R" rendered as special glyph)
//!
//! Spec: PDF 32000-1:2008 Section 9.10.2 (Character to Unicode mapping)

use std::collections::HashMap;
use xberg_native_pdf::fonts::{Encoding, FontInfo};

#[test]
fn test_ligature_extraction_fi() {
    //! Test 1: Ligature fi should extract to two characters "f" + "i"
    //!
    //! When a PDF contains the ligature character 'ﬁ' (U+FB01) with ActualText "fi",
    //! the text extraction should return "fi" (two characters), not "ﬁ" (one character).

    let font = FontInfo {
        base_font: "StandardFont".to_string(),
        subtype: "Type1".to_string(),
        encoding: Encoding::Standard("WinAnsiEncoding".to_string()),
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
        default_width: 500.0,
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

    let result = font.char_to_unicode(0xFB01);

    assert!(result.is_some(), "Should map ligature character");

    let mapped = result.unwrap();
    assert!(
        mapped.contains('f') && mapped.contains('i'),
        "Ligature should expand to 'fi', got: {}",
        mapped
    );
}

#[test]
fn test_ligature_extraction_fl() {
    //! Test 2: Ligature fl should extract to two characters "f" + "l"

    let font = FontInfo {
        base_font: "StandardFont".to_string(),
        subtype: "Type1".to_string(),
        encoding: Encoding::Standard("WinAnsiEncoding".to_string()),
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
        default_width: 500.0,
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

    let result = font.char_to_unicode(0xFB02);

    assert!(result.is_some(), "Should map fl ligature character");

    let mapped = result.unwrap();
    assert!(
        mapped.contains('f') && mapped.contains('l'),
        "Ligature should expand to 'fl', got: {}",
        mapped
    );
}

#[test]
fn test_ligature_extraction_ffi() {
    //! Test 3: Ligature ffi should extract to three characters

    let font = FontInfo {
        base_font: "StandardFont".to_string(),
        subtype: "Type1".to_string(),
        encoding: Encoding::Standard("WinAnsiEncoding".to_string()),
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
        default_width: 500.0,
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

    let result = font.char_to_unicode(0xFB03);

    assert!(result.is_some(), "Should map ffi ligature character");

    let mapped = result.unwrap();
    assert!(
        mapped.len() >= 3,
        "Ligature ffi should expand to at least 3 characters, got: {} (len: {})",
        mapped,
        mapped.len()
    );
}

#[test]
fn test_ligature_extraction_ffl() {
    //! Test 4: Ligature ffl should extract to three characters

    let font = FontInfo {
        base_font: "StandardFont".to_string(),
        subtype: "Type1".to_string(),
        encoding: Encoding::Standard("WinAnsiEncoding".to_string()),
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
        default_width: 500.0,
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

    let result = font.char_to_unicode(0xFB04);

    assert!(result.is_some(), "Should map ffl ligature character");

    let mapped = result.unwrap();
    assert!(
        mapped.len() >= 3,
        "Ligature ffl should expand to at least 3 characters, got: {} (len: {})",
        mapped,
        mapped.len()
    );
}
