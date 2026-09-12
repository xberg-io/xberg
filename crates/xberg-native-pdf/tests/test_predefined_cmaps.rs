//! Predefined CMap Tests for Phase 3
//!
//! Tests for supporting predefined CMaps (Identity-H, UniGB-UCS2-H, UniJIS-UCS2-H, etc.)
//! per PDF Spec 32000-1:2008 Section 9.7.5.2 and Section 9.10.2.
//!
//! Predefined CMaps are critical for:
//! - Chinese/Japanese/Korean (CJK) PDFs using standard Adobe CID collections
//! - Type 0 fonts that reference predefined CMaps instead of embedded ToUnicode CMaps
//! - Large document sets (millions of CJK documents rely on predefined CMaps)
//!
//! Spec: PDF 32000-1:2008 Section 9.7.5.2 (Predefined CMaps)

use std::collections::HashMap;
use xberg_native_pdf::fonts::{CIDSystemInfo, Encoding, FontInfo};

#[test]
fn test_identity_h_cmap_simple_cid_to_unicode() {
    //! Test 1: Identity-H CMap should map CID directly to Unicode
    //!
    //! Identity-H is the simplest predefined CMap: CID == 2-byte Unicode code point
    //! Used with any CID font when font encoding is "Identity-H"
    //!
    //! Example: CID 0x4E00 (CJK UNIFIED IDEOGRAPH, "一" in Chinese)
    //! should map directly to U+4E00

    let font = FontInfo {
        base_font: "ChineseFont+Identity-H".to_string(),
        subtype: "Type0".to_string(),
        encoding: Encoding::Standard("Identity-H".to_string()),
        to_unicode: None,
        truetype_cmap: std::sync::OnceLock::new(),
        embedded_glyph_names: std::sync::OnceLock::new(),
        is_truetype_font: false,
        embedded_font_data: None,
        cid_to_gid_map: None,
        cid_system_info: Some(CIDSystemInfo {
            registry: "Adobe".to_string(),
            ordering: "Identity".to_string(),
            supplement: 0,
        }),
        cid_font_type: Some("2".to_string()),
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

    let result = font.char_to_unicode(0x4E00);

    assert!(result.is_some(), "Identity-H should map CID 0x4E00");
    let mapped = result.unwrap();
    assert_eq!(mapped, "一", "CID 0x4E00 should map to Chinese character '一' (U+4E00)");
}

#[test]
fn test_unigb_ucs2_h_cmap_simplified_chinese() {
    //! Test 2: UniGB-UCS2-H CMap for Simplified Chinese (Adobe-GB1)
    //!
    //! UniGB-UCS2-H maps CID from Adobe-GB1 character collection to Unicode
    //! Very common in Simplified Chinese PDFs
    //!
    //! Example: CID 0x2EE5 maps to U+4E00 (CJK UNIFIED IDEOGRAPH "一")

    let font = FontInfo {
        base_font: "STSong+UniGB-UCS2-H".to_string(),
        subtype: "Type0".to_string(),
        encoding: Encoding::Standard("UniGB-UCS2-H".to_string()),
        to_unicode: None,
        truetype_cmap: std::sync::OnceLock::new(),
        embedded_glyph_names: std::sync::OnceLock::new(),
        is_truetype_font: false,
        embedded_font_data: None,
        cid_to_gid_map: None,
        cid_system_info: Some(CIDSystemInfo {
            registry: "Adobe".to_string(),
            ordering: "GB1".to_string(),
            supplement: 2,
        }),
        cid_font_type: Some("2".to_string()),
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

    let result = font.char_to_unicode(0x2EE5);

    assert!(result.is_some(), "UniGB-UCS2-H should map char code 0x2EE5");
    let mapped = result.unwrap();
    assert_eq!(
        mapped, "鱼",
        "char code 0x2EE5 (⻥, a radical presentation form) must normalize to 鱼 (U+9C7C)"
    );
}

#[test]
fn test_unijis_ucs2_h_cmap_japanese() {
    //! Test 3: UniJIS-UCS2-H CMap for Japanese (Adobe-Japan1)
    //!
    //! UniJIS-UCS2-H maps CID from Adobe-Japan1 to Unicode
    //! Very common in Japanese PDFs
    //!
    //! Example: CID 0x3042 maps to U+3042 (HIRAGANA LETTER A "あ")

    let font = FontInfo {
        base_font: "HeiseiMin-W3+UniJIS-UCS2-H".to_string(),
        subtype: "Type0".to_string(),
        encoding: Encoding::Standard("UniJIS-UCS2-H".to_string()),
        to_unicode: None,
        truetype_cmap: std::sync::OnceLock::new(),
        embedded_glyph_names: std::sync::OnceLock::new(),
        is_truetype_font: false,
        embedded_font_data: None,
        cid_to_gid_map: None,
        cid_system_info: Some(CIDSystemInfo {
            registry: "Adobe".to_string(),
            ordering: "Japan1".to_string(),
            supplement: 4,
        }),
        cid_font_type: Some("2".to_string()),
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

    let result = font.char_to_unicode(0x3042);

    assert!(result.is_some(), "UniJIS-UCS2-H should map CID 0x3042");
    let mapped = result.unwrap();
    assert_eq!(
        mapped, "あ",
        "CID 0x3042 should map to Japanese character 'あ' (U+3042)"
    );
}

#[test]
fn test_unicns_ucs2_h_cmap_traditional_chinese() {
    //! Test 4: UniCNS-UCS2-H CMap for Traditional Chinese (Adobe-CNS1)
    //!
    //! UniCNS-UCS2-H maps CID from Adobe-CNS1 to Unicode
    //! Used in Traditional Chinese (Taiwan, Hong Kong) PDFs
    //!
    //! Example: CID 0x4E00 maps to U+4E00 (CJK UNIFIED IDEOGRAPH "一")

    let font = FontInfo {
        base_font: "MingLiU+UniCNS-UCS2-H".to_string(),
        subtype: "Type0".to_string(),
        encoding: Encoding::Standard("UniCNS-UCS2-H".to_string()),
        to_unicode: None,
        truetype_cmap: std::sync::OnceLock::new(),
        embedded_glyph_names: std::sync::OnceLock::new(),
        is_truetype_font: false,
        embedded_font_data: None,
        cid_to_gid_map: None,
        cid_system_info: Some(CIDSystemInfo {
            registry: "Adobe".to_string(),
            ordering: "CNS1".to_string(),
            supplement: 3,
        }),
        cid_font_type: Some("2".to_string()),
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

    let result = font.char_to_unicode(0x4E00);

    assert!(result.is_some(), "UniCNS-UCS2-H should map CID 0x4E00");
    let mapped = result.unwrap();
    assert_eq!(mapped, "一", "CID 0x4E00 should map to Chinese character '一' (U+4E00)");
}

#[test]
fn test_uniks_ucs2_h_cmap_korean() {
    //! Test 5: UniKS-UCS2-H CMap for Korean (Adobe-Korea1)
    //!
    //! UniKS-UCS2-H maps CID from Adobe-Korea1 to Unicode
    //! Used in Korean PDFs
    //!
    //! Example: CID 0xAC00 maps to U+AC00 (HANGUL SYLLABLE GA "가")

    let font = FontInfo {
        base_font: "HYGoThic+UniKS-UCS2-H".to_string(),
        subtype: "Type0".to_string(),
        encoding: Encoding::Standard("UniKS-UCS2-H".to_string()),
        to_unicode: None,
        truetype_cmap: std::sync::OnceLock::new(),
        embedded_glyph_names: std::sync::OnceLock::new(),
        is_truetype_font: false,
        embedded_font_data: None,
        cid_to_gid_map: None,
        cid_system_info: Some(CIDSystemInfo {
            registry: "Adobe".to_string(),
            ordering: "Korea1".to_string(),
            supplement: 1,
        }),
        cid_font_type: Some("2".to_string()),
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

    let result = font.char_to_unicode(0xAC00);

    assert!(result.is_some(), "UniKS-UCS2-H should map CID 0xAC00");
    let mapped = result.unwrap();
    assert_eq!(mapped, "가", "CID 0xAC00 should map to Korean character '가' (U+AC00)");
}
