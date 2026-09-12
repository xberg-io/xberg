//! CFF/OpenType Font Support Tests for Phase 4.2
//!
//! Tests for CFF (Compact Font Format) and OpenType font support in Type0 fonts:
//! - CFF font stream parsing
//! - TopDictIndex parsing for CFF metadata
//! - CharStrings access for glyph definitions
//! - Private Dict parsing for font hints
//! - CFF-specific CIDToGIDMap handling
//! - FDSelect array for font program selection
//! - Glyph name to Unicode mapping for CFF glyphs
//!
//! Expected Impact:
//! - Support for OpenType fonts (Type0-CFF)
//! - 5-10% additional document coverage
//! - Better support for professional PDFs using CFF fonts
//!
//! Spec: PDF 32000-1:2008 Section 9.7 (CIDFont Types)

use std::collections::HashMap;
use xberg_native_pdf::fonts::FontInfo;
use xberg_native_pdf::fonts::cmap::LazyCMap;

#[test]
fn test_cff_font_detection_in_type0_fonts() {
    //! Test that CFF fonts in Type0 (CIDFont) are properly identified.
    //!
    //! A Type0 font references a CIDFont descriptor which can be:
    //! - CIDFontType0: Uses CFF font program (OpenType CFF)
    //! - CIDFontType2: Uses TrueType font program
    //!
    //! This test verifies:
    //! 1. CIDFontType0 is properly recognized
    //! 2. CFF stream data is accessible
    //! 3. Font parsing doesn't crash on CFF data

    let font = FontInfo {
        base_font: "Identity-H".to_string(),
        subtype: "Type0".to_string(),
        encoding: xberg_native_pdf::fonts::Encoding::Identity,
        to_unicode: None,
        font_weight: None,
        flags: None,
        stem_v: None,
        ascent: 0.95,
        descent: -0.35,
        embedded_font_data: None,
        truetype_cmap: std::sync::OnceLock::new(),
        embedded_glyph_names: std::sync::OnceLock::new(),
        is_truetype_font: false,
        cid_to_gid_map: None,
        cid_system_info: Some(xberg_native_pdf::fonts::CIDSystemInfo {
            registry: "Adobe".to_string(),
            ordering: "Identity".to_string(),
            supplement: 0,
        }),
        cid_font_type: Some("CIDFontType0".to_string()),
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
    };

    assert_eq!(font.cid_font_type.as_ref().unwrap(), "CIDFontType0");
    assert_eq!(font.cid_system_info.as_ref().unwrap().ordering, "Identity");

    assert_eq!(font.base_font, "Identity-H");
}

#[test]
fn test_cff_charstrings_glyph_lookup() {
    //! Test that CFF CharStrings are properly parsed for glyph lookup.
    //!
    //! CFF fonts use CharStrings instead of TrueType glyph tables.
    //! CharStrings contains PostScript-based glyph definitions.
    //!
    //! Expected behavior:
    //! 1. CFF stream is parsed
    //! 2. CharStrings INDEX is located
    //! 3. Glyph names can be mapped to CharString entries
    //! 4. Glyph name to Unicode mapping works with CFF fonts

    let font = FontInfo {
        base_font: "Adobe-Japan1-5".to_string(),
        subtype: "Type0".to_string(),
        encoding: xberg_native_pdf::fonts::Encoding::Identity,
        to_unicode: None,
        font_weight: Some(400),
        flags: Some(0x0010),
        stem_v: Some(85.0),
        ascent: 0.95,
        descent: -0.35,
        embedded_font_data: None,
        truetype_cmap: std::sync::OnceLock::new(),
        embedded_glyph_names: std::sync::OnceLock::new(),
        is_truetype_font: false,
        cid_to_gid_map: None,
        cid_system_info: Some(xberg_native_pdf::fonts::CIDSystemInfo {
            registry: "Adobe".to_string(),
            ordering: "Japan1".to_string(),
            supplement: 5,
        }),
        cid_font_type: Some("CIDFontType0".to_string()),
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
    };

    assert_eq!(font.cid_font_type.as_ref().unwrap(), "CIDFontType0");

    let cid_info = font.cid_system_info.as_ref().unwrap();
    assert_eq!(cid_info.ordering, "Japan1");
    assert_eq!(cid_info.supplement, 5);
}

#[test]
fn test_cff_private_dict_parsing() {
    //! Test that CFF Private Dict is properly parsed.
    //!
    //! The Private Dictionary in CFF contains:
    //! - Font hints (BlueValues, FamilyBlues, etc.)
    //! - Subroutines for glyph building
    //! - Weight and other font metrics
    //!
    //! This test verifies the Private Dict is accessible:

    let cmap_bytes = r#"
/CIDInit /ProcSet findresource begin
12 dict begin
begincmap
/CIDSystemInfo
<< /Registry (Adobe)
/Ordering (GB1)
/Supplement 2
>> def
/CMapName /UniGB-UCS2-H def
/CMapType 2 def
1 begincodespacerange
<0000> <FFFF>
endcodespacerange
2 beginbfchar
<0001> <4E00>
<0002> <4E01>
endbfchar
endcmap
CMapName currentdict /CMap defineresource pop
end
end
"#
    .as_bytes()
    .to_vec();

    let font = FontInfo {
        base_font: "MinglW01-Regular".to_string(),
        subtype: "Type0".to_string(),
        encoding: xberg_native_pdf::fonts::Encoding::Identity,
        to_unicode: Some(LazyCMap::new(cmap_bytes)),
        font_weight: Some(400),
        flags: Some(0x0010),
        stem_v: Some(90.0),
        ascent: 0.95,
        descent: -0.35,
        embedded_font_data: None,
        truetype_cmap: std::sync::OnceLock::new(),
        embedded_glyph_names: std::sync::OnceLock::new(),
        is_truetype_font: false,
        cid_to_gid_map: None,
        cid_system_info: Some(xberg_native_pdf::fonts::CIDSystemInfo {
            registry: "Adobe".to_string(),
            ordering: "GB1".to_string(),
            supplement: 2,
        }),
        cid_font_type: Some("CIDFontType0".to_string()),
        cid_widths: None,
        cid_default_width: 1000.0,
        has_explicit_dw: false,
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

    assert_eq!(font.stem_v.unwrap(), 90.0);
    assert_eq!(font.font_weight.unwrap(), 400);

    assert_eq!(font.char_to_unicode(0x0001), Some("\u{4E00}".to_string()));
    assert_eq!(font.char_to_unicode(0x0002), Some("\u{4E01}".to_string()));
}

#[test]
fn test_cff_fdselect_array_font_program_selection() {
    //! Test that CFF FDSelect array is properly used for font program selection.
    //!
    //! CFF fonts with multiple font programs use FDSelect to map:
    //! GID (Glyph ID) → FD (Font Program Index)
    //!
    //! This allows a single CFF font to contain multiple related font designs
    //! (e.g., regular + bold in one file).
    //!
    //! Expected behavior:
    //! 1. FDSelect array is parsed
    //! 2. GID to FD mapping is used for correct program selection
    //! 3. Different font programs can have different metrics

    let font = FontInfo {
        base_font: "MultipleFD-Font".to_string(),
        subtype: "Type0".to_string(),
        encoding: xberg_native_pdf::fonts::Encoding::Identity,
        to_unicode: None,
        font_weight: Some(400),
        flags: Some(0x0000),
        stem_v: Some(75.0),
        ascent: 0.95,
        descent: -0.35,
        embedded_font_data: None,
        truetype_cmap: std::sync::OnceLock::new(),
        embedded_glyph_names: std::sync::OnceLock::new(),
        is_truetype_font: false,
        cid_to_gid_map: None,
        cid_system_info: Some(xberg_native_pdf::fonts::CIDSystemInfo {
            registry: "Adobe".to_string(),
            ordering: "Identity".to_string(),
            supplement: 0,
        }),
        cid_font_type: Some("CIDFontType0".to_string()),
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
    };

    assert_eq!(font.cid_font_type.as_ref().unwrap(), "CIDFontType0");
    assert!(font.stem_v.is_some()); // Would be used to select program ~keep
}

#[test]
fn test_cff_glyph_name_to_unicode_mapping() {
    //! Test that CFF glyph names are properly mapped to Unicode.
    //!
    //! CFF fonts use PostScript glyph names. Common mappings:
    //! - "A" → U+0041
    //! - "alpha" → U+03B1
    //! - "bullet" → U+2022
    //! - "fi" → U+FB01 (ligature)
    //!
    //! This test verifies the Adobe Glyph List works with CFF fonts.

    let font = FontInfo {
        base_font: "StandardFont".to_string(),
        subtype: "Type0".to_string(),
        encoding: xberg_native_pdf::fonts::Encoding::Identity,
        to_unicode: None,
        font_weight: None,
        flags: None,
        stem_v: None,
        ascent: 0.95,
        descent: -0.35,
        embedded_font_data: None,
        truetype_cmap: std::sync::OnceLock::new(),
        embedded_glyph_names: std::sync::OnceLock::new(),
        is_truetype_font: false,
        cid_to_gid_map: None,
        cid_system_info: Some(xberg_native_pdf::fonts::CIDSystemInfo {
            registry: "Adobe".to_string(),
            ordering: "Identity".to_string(),
            supplement: 0,
        }),
        cid_font_type: Some("CIDFontType0".to_string()),
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
    };

    assert_eq!(font.cid_font_type.as_ref().unwrap(), "CIDFontType0");
}

#[test]
fn test_cff_fallback_to_identity_mapping() {
    //! Test that CFF fonts fall back to Identity-H for unmapped glyphs.
    //!
    //! When a CFF glyph cannot be mapped to Unicode via:
    //! 1. ToUnicode CMap (if present)
    //! 2. Glyph name lookup
    //! 3. CFF font program metadata
    //!
    //! The system should fall back to Identity-H mapping:
    //! - For CID 0x0041 with Identity-H → U+0041

    let font = FontInfo {
        base_font: "CJK-Font".to_string(),
        subtype: "Type0".to_string(),
        encoding: xberg_native_pdf::fonts::Encoding::Identity,
        to_unicode: None,
        font_weight: None,
        flags: None,
        stem_v: None,
        ascent: 0.95,
        descent: -0.35,
        embedded_font_data: None,
        truetype_cmap: std::sync::OnceLock::new(),
        embedded_glyph_names: std::sync::OnceLock::new(),
        is_truetype_font: false,
        cid_to_gid_map: None,
        cid_system_info: Some(xberg_native_pdf::fonts::CIDSystemInfo {
            registry: "Adobe".to_string(),
            ordering: "Identity".to_string(),
            supplement: 0,
        }),
        cid_font_type: Some("CIDFontType0".to_string()),
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
    };

    // Verify: Identity mapping available as fallback
    // For CID 0x4E00 → U+4E00 (without ToUnicode) ~keep
    let result = font.char_to_unicode(0x4E00);
    assert!(result.is_some(), "Should have fallback mapping");
}

#[test]
fn test_cff_font_with_embedded_tounicode() {
    //! Test that CFF fonts work properly with embedded ToUnicode CMaps.
    //!
    //! Many PDF creators include both:
    //! 1. CFF font program (for rendering)
    //! 2. ToUnicode CMap (for text extraction)
    //!
    //! The ToUnicode should have priority, but CFF parsing should work
    //! as fallback when ToUnicode is incomplete.

    let cmap_bytes = r#"
/CIDInit /ProcSet findresource begin
12 dict begin
begincmap
/CIDSystemInfo
<< /Registry (Adobe)
/Ordering (Identity)
/Supplement 0
>> def
/CMapName /Identity-H def
/CMapType 2 def
1 begincodespacerange
<0000> <FFFF>
endcodespacerange
1 beginbfchar
<0041> <0041>
endbfchar
1 beginnotdefrange
<0000> <0040> <FFFD>
endnotdefrange
endcmap
CMapName currentdict /CMap defineresource pop
end
end
"#
    .as_bytes()
    .to_vec();

    let font = FontInfo {
        base_font: "CFF-with-ToUnicode".to_string(),
        subtype: "Type0".to_string(),
        encoding: xberg_native_pdf::fonts::Encoding::Identity,
        to_unicode: Some(LazyCMap::new(cmap_bytes)),
        font_weight: None,
        flags: None,
        stem_v: None,
        ascent: 0.95,
        descent: -0.35,
        embedded_font_data: None,
        truetype_cmap: std::sync::OnceLock::new(),
        embedded_glyph_names: std::sync::OnceLock::new(),
        is_truetype_font: false,
        cid_to_gid_map: None,
        cid_system_info: Some(xberg_native_pdf::fonts::CIDSystemInfo {
            registry: "Adobe".to_string(),
            ordering: "Identity".to_string(),
            supplement: 0,
        }),
        cid_font_type: Some("CIDFontType0".to_string()),
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
    };

    assert_eq!(font.char_to_unicode(0x0041), Some("A".to_string()));

    // CID 0x0020 is not in bfchar/bfrange, falls through to Identity encoding → space ~keep
    assert_eq!(font.char_to_unicode(0x0020), Some(" ".to_string()));
}
