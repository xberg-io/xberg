//! Test suite for Week 2 Day 7: Custom Encoding Support (2B)
//!
//! This test suite verifies that custom font encodings are properly normalized
//! before word boundary detection, ensuring that word boundaries are detected
//! based on actual Unicode characters rather than raw byte codes.
//!
//! Per PDF Spec ISO 32000-1:2008, Section 9.6.6:
//! - Fonts can have custom encodings with /Differences arrays
//! - /Differences override the base encoding for specific character codes
//! - Character-to-Unicode mapping must respect these custom encodings

use std::collections::HashMap;
use xberg_native_pdf::fonts::{Encoding, FontInfo};

#[test]
fn test_custom_encoding_basic() {
    let mut mappings = HashMap::new();
    mappings.insert(0x64, 'ρ');

    let font_info = create_font_with_encoding(Encoding::Custom(mappings));

    let normalized = font_info.get_encoded_char(0x64);
    assert_eq!(normalized, Some('ρ'), "Custom encoding should map 0x64 to Greek rho");
}

#[test]
fn test_custom_encoding_has_custom_method() {
    let mut mappings = HashMap::new();
    mappings.insert(0x64, 'ρ');

    let font_with_custom = create_font_with_encoding(Encoding::Custom(mappings));
    assert!(
        font_with_custom.has_custom_encoding(),
        "Font with Custom encoding should return true"
    );

    let font_with_standard = create_font_with_encoding(Encoding::Standard("WinAnsiEncoding".to_string()));
    assert!(
        !font_with_standard.has_custom_encoding(),
        "Font with Standard encoding should return false"
    );

    let font_with_identity = create_font_with_encoding(Encoding::Identity);
    assert!(
        !font_with_identity.has_custom_encoding(),
        "Font with Identity encoding should return false"
    );
}

#[test]
fn test_standard_encoding_no_custom() {
    let font_info = create_font_with_encoding(Encoding::Standard("WinAnsiEncoding".to_string()));

    assert!(
        !font_info.has_custom_encoding(),
        "Standard encoding should not be identified as custom"
    );

    let normalized = font_info.get_encoded_char(0x41);
    assert_eq!(normalized, Some('A'), "Standard encoding should pass through ASCII");
}

#[test]
fn test_identity_encoding_no_custom() {
    let font_info = create_font_with_encoding(Encoding::Identity);

    assert!(
        !font_info.has_custom_encoding(),
        "Identity encoding should not be identified as custom"
    );
}

#[test]
fn test_encoding_normalization_priority() {
    let mut mappings = HashMap::new();
    mappings.insert(0x64, 'ρ');

    let font_info = create_font_with_encoding(Encoding::Custom(mappings));

    let normalized = font_info.get_encoded_char(0x64);
    assert_eq!(normalized, Some('ρ'), "/Differences should override base encoding");
}

#[test]
fn test_custom_encoding_unmapped_code() {
    let mut mappings = HashMap::new();
    mappings.insert(0x64, 'ρ');

    let font_info = create_font_with_encoding(Encoding::Custom(mappings));

    let normalized = font_info.get_encoded_char(0x65);
    assert_eq!(normalized, None, "Unmapped code in custom encoding should return None");
}

#[test]
fn test_custom_encoding_multiple_mappings() {
    let mut mappings = HashMap::new();
    mappings.insert(0x64, 'ρ');
    mappings.insert(0x65, 'σ');
    mappings.insert(0x66, 'τ');

    let font_info = create_font_with_encoding(Encoding::Custom(mappings));

    assert_eq!(font_info.get_encoded_char(0x64), Some('ρ'));
    assert_eq!(font_info.get_encoded_char(0x65), Some('σ'));
    assert_eq!(font_info.get_encoded_char(0x66), Some('τ'));
}

#[test]
fn test_custom_encoding_with_special_characters() {
    let mut mappings = HashMap::new();
    mappings.insert(0x80, '€');
    mappings.insert(0x81, '©'); // Copyright
    mappings.insert(0x82, '®');

    let font_info = create_font_with_encoding(Encoding::Custom(mappings));

    assert_eq!(font_info.get_encoded_char(0x80), Some('€'));
    assert_eq!(font_info.get_encoded_char(0x81), Some('©'));
    assert_eq!(font_info.get_encoded_char(0x82), Some('®'));
}

#[test]
fn test_standard_encoding_ascii_range() {
    let font_info = create_font_with_encoding(Encoding::Standard("WinAnsiEncoding".to_string()));

    assert_eq!(font_info.get_encoded_char(0x61), Some('a'));
    assert_eq!(font_info.get_encoded_char(0x7A), Some('z'));

    assert_eq!(font_info.get_encoded_char(0x41), Some('A'));
    assert_eq!(font_info.get_encoded_char(0x5A), Some('Z'));

    assert_eq!(font_info.get_encoded_char(0x30), Some('0'));
    assert_eq!(font_info.get_encoded_char(0x39), Some('9'));
}

#[test]
fn test_identity_encoding_passthrough() {
    let font_info = create_font_with_encoding(Encoding::Identity);

    assert_eq!(font_info.get_encoded_char(0x41), Some('A'));
    assert_eq!(font_info.get_encoded_char(0x61), Some('a'));
    assert_eq!(font_info.get_encoded_char(0x30), Some('0'));
}

fn create_font_with_encoding(encoding: Encoding) -> FontInfo {
    FontInfo {
        base_font: "TestFont".to_string(),
        subtype: "Type1".to_string(),
        encoding,
        to_unicode: None,
        font_weight: Some(400),
        flags: Some(32),
        stem_v: Some(100.0),
        ascent: 0.95,
        descent: -0.35,
        embedded_font_data: None,
        truetype_cmap: std::sync::OnceLock::new(),
        embedded_glyph_names: std::sync::OnceLock::new(),
        is_truetype_font: false,
        cid_to_gid_map: None,
        cid_system_info: None,
        cid_font_type: None,
        cid_widths: None,
        cid_default_width: 1000.0,
        has_explicit_dw: false,
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
    }
}
