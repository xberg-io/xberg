//! Zero-Byte Embedded Font Handling Tests
//!
//! Tests for Phase 1.3: When embedded fonts have 0 bytes of data but are marked as embedded,
//! we should skip attempting to read the embedded TrueType cmap and move to the next fallback
//! (Adobe Glyph List).
//!
//! This addresses performance and correctness issues with fonts like Aptos, Calibri, and
//! common Office fonts that are marked as embedded but have no actual data.
//!
//! Spec: PDF 32000-1:2008 Section 9.4.3 (Font Descriptors)

use std::collections::HashMap;
use std::sync::Arc;
use xberg_native_pdf::fonts::{CIDToGIDMap, Encoding, FontInfo};

#[test]
fn test_skip_truetype_cmap_when_embedded_font_zero_bytes() {
    //! Test 1: Should skip TrueType cmap for 0-byte font and use AGL fallback
    //!
    //! When a Type0 font has embedded_font_data set to empty vec (0 bytes),
    //! we should NOT attempt to parse TrueType cmap from it.
    //! Instead, skip directly to Adobe Glyph List fallback.

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
        "Should find mapping for GID 0x41 via Adobe Glyph List, not fail"
    );
    assert_eq!(
        result.unwrap(),
        "A",
        "0-byte embedded font should use AGL fallback, not U+FFFD"
    );
}

#[test]
fn test_skip_truetype_cmap_for_common_office_fonts() {
    //! Test 2: Should handle 0-byte Office fonts (Calibri, Helvetica, etc.)
    //!
    //! Many Office document PDFs have fonts marked as embedded with 0 bytes.
    //! These should fall back to Adobe Glyph List properly.

    let fonts = vec![
        ("Calibri", 0x41),
        ("Helvetica", 0x42),
        ("Arial", 0x43),
        ("TimesNewRoman", 0x44),
    ];

    for (font_name, gid) in fonts {
        let font = FontInfo {
            base_font: font_name.to_string(),
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

        let result = font.char_to_unicode(gid);
        assert!(
            result.is_some(),
            "Font {} with 0-byte data should map GID 0x{:02X}",
            font_name,
            gid
        );

        assert_ne!(
            result.unwrap(),
            "\u{FFFD}",
            "Font {} should use AGL, not U+FFFD for GID 0x{:02X}",
            font_name,
            gid
        );
    }
}

#[test]
fn test_still_use_truetype_cmap_when_embedded_font_has_data() {
    //! Test 3: Should still use TrueType cmap when font data exists
    //!
    //! This test ensures we don't break the normal path: when a font HAS
    //! actual embedded data, we should still try TrueType cmap before AGL.
    //!
    //! (This test might need adjustment based on how we detect valid TrueType data)

    let font_data = vec![0u8; 100];

    let font = FontInfo {
        base_font: "ValidFont".to_string(),
        subtype: "Type0".to_string(),
        encoding: Encoding::Identity,
        to_unicode: None,
        truetype_cmap: std::sync::OnceLock::new(),
        embedded_glyph_names: std::sync::OnceLock::new(),
        is_truetype_font: false,
        embedded_font_data: Some(Arc::new(font_data)),
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

    assert!(result.is_some(), "Font with embedded data should map successfully");
}
