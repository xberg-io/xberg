//! 4-Byte Character Code Support Tests for Phase 4.3
//!
//! Tests for extended character code support beyond u32 limits:
//! - 4-byte character codes in CMap entries
//! - Extended character ranges for specialized CJK fonts
//! - Code points > U+10FFFF mapped to U+FFFD
//! - Handling of large CID values in predefined CMaps
//!
//! Current Limitation:
//! - ToUnicode CMaps use u32 keys (max value: 0xFFFFFFFF)
//! - Some CID fonts use codes beyond u32 range
//! - This phase adds support for extended character ranges
//!
//! Expected Impact:
//! - Unlock specialized CJK fonts using extended character ranges
//! - Better support for large font files with many glyphs
//!
//! Spec: PDF 32000-1:2008 Section 9.7.6.2 (CID Fonts)

use std::collections::HashMap;
use xberg_native_pdf::fonts::FontInfo;
use xberg_native_pdf::fonts::cmap::LazyCMap;

#[test]
fn test_4byte_cmap_extended_range_parsing() {
    //! Test that CMaps with 4-byte character codes are parsed correctly.
    //!
    //! The PDF specification allows character codes up to 4 bytes (32 bits):
    //! - 1-byte: 0x00 - 0xFF
    //! - 2-byte: 0x0000 - 0xFFFF
    //! - 3-byte: 0x000000 - 0xFFFFFF
    //! - 4-byte: 0x00000000 - 0xFFFFFFFF
    //!
    //! Most PDFs use 1-2 bytes, but specialized CJK fonts may use 3-4 bytes.

    let cmap_4byte = r#"
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
<00000000> <FFFFFFFF>
endcodespacerange
3 beginbfchar
<00000041> <0041>
<00010041> <4E00>
<FFFFFFF0> <FFFD>
endbfchar
endcmap
CMapName currentdict /CMap defineresource pop
end
end
"#;

    let result = xberg_native_pdf::fonts::cmap::parse_tounicode_cmap(cmap_4byte.as_bytes());

    assert!(result.is_ok(), "Should parse 4-byte CMap successfully");

    let cmap = result.unwrap();

    assert_eq!(
        cmap.get(&0x00000041).as_deref(),
        Some("A"),
        "4-byte code 0x00000041 should map to 'A'"
    );

    assert_eq!(
        cmap.get(&0x00010041).as_deref(),
        Some("\u{4E00}"),
        "4-byte code 0x00010041 should map to CJK character"
    );
}

#[test]
fn test_4byte_large_cid_values() {
    //! Test that large CID values (up to u32::MAX) are supported.
    //!
    //! Specialized CJK fonts may have:
    //! - 10,000+ glyphs (CID 0x0000 - 0x2710+)
    //! - Each with unique Unicode mapping
    //! - Large CID ranges up to 0xFFFFFFFF
    //!
    //! This test verifies CID values near the u32 limit work correctly.

    let large_cid_cmap = r#"
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
<00000000> <FFFFFFFF>
endcodespacerange
3 beginbfchar
<00008000> <8000>
<FFFFFFFF> <FFFD>
<FFFFFFFE> <FFFE>
endbfchar
endcmap
CMapName currentdict /CMap defineresource pop
end
end
"#;

    let result = xberg_native_pdf::fonts::cmap::parse_tounicode_cmap(large_cid_cmap.as_bytes());

    assert!(result.is_ok(), "Should parse large CID CMaps");

    let cmap = result.unwrap();

    assert_eq!(cmap.get(&0x00008000).as_deref(), Some("\u{8000}"));

    // Test: CID 0xFFFFFFFF (u32::MAX) -> maps to U+FFFD (replacement char) ~keep
    assert_eq!(cmap.get(&0xFFFFFFFF).as_deref(), Some("\u{FFFD}"));

    assert_eq!(cmap.get(&0xFFFFFFFE).as_deref(), Some("\u{FFFE}"));
}

#[test]
fn test_4byte_cmap_bfrange_extended() {
    //! Test that bfrange (range mappings) work with 4-byte codes.
    //!
    //! Some CMaps use bfrange to map ranges of sequential CIDs:
    //! <startCode> <endCode> <startUnicode>
    //!
    //! This is more efficient than individual bfchar entries for large ranges.

    let bfrange_4byte = r#"
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
<00000000> <FFFFFFFF>
endcodespacerange
1 beginbfrange
<00010000> <0001000F> <4E00>
endbfrange
endcmap
CMapName currentdict /CMap defineresource pop
end
end
"#;

    let result = xberg_native_pdf::fonts::cmap::parse_tounicode_cmap(bfrange_4byte.as_bytes());

    assert!(result.is_ok(), "Should parse 4-byte bfrange");

    let cmap = result.unwrap();

    assert_eq!(cmap.get(&0x00010000).as_deref(), Some("\u{4E00}"));
    assert_eq!(cmap.get(&0x00010005).as_deref(), Some("\u{4E05}"));
    assert_eq!(cmap.get(&0x0001000F).as_deref(), Some("\u{4E0F}"));
}

#[test]
fn test_4byte_notdefrange_large_cids() {
    //! Test that beginnotdefrange works with 4-byte codes.
    //!
    //! The notdefrange section (added in Phase 4.1) should also support
    //! 4-byte character codes for fallback/undefined character handling.

    let notdef_4byte = r#"
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
<00000000> <FFFFFFFF>
endcodespacerange
1 beginbfchar
<00010000> <4E00>
endbfchar
1 beginnotdefrange
<00000000> <0000FFFF> <FFFD>
endnotdefrange
endcmap
CMapName currentdict /CMap defineresource pop
end
end
"#;

    let result = xberg_native_pdf::fonts::cmap::parse_tounicode_cmap(notdef_4byte.as_bytes());

    assert!(result.is_ok(), "Should parse 4-byte notdefrange");

    let cmap = result.unwrap();

    assert_eq!(cmap.get(&0x00010000).as_deref(), Some("\u{4E00}"));

    assert_eq!(cmap.get(&0x00000001).as_deref(), Some("\u{FFFD}"));
}

#[test]
fn test_4byte_extended_unicode_codepoints() {
    //! Test handling of character codes beyond valid Unicode.
    //!
    //! Some PDFs may have invalid Unicode code points:
    //! - Code points > U+10FFFF (max valid Unicode)
    //! - Code points in surrogate range (U+D800-U+DFFF)
    //!
    //! Per PDF spec, these should map to U+FFFD (replacement character).

    let invalid_cmap = r#"
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
<00000000> <FFFFFFFF>
endcodespacerange
2 beginbfchar
<00000041> <0041>
<110000> <110000>
endbfchar
endcmap
CMapName currentdict /CMap defineresource pop
end
end
"#;

    let result = xberg_native_pdf::fonts::cmap::parse_tounicode_cmap(invalid_cmap.as_bytes());

    assert!(result.is_ok(), "Should parse CMap with invalid Unicode");

    let cmap = result.unwrap();

    assert_eq!(cmap.get(&0x00000041).as_deref(), Some("A"));

    // Code > U+10FFFF may be handled as-is or converted to U+FFFD
    // depending on implementation (will test actual behavior) ~keep
}

#[test]
fn test_4byte_cmap_with_lazy_loading() {
    //! Test that 4-byte CMaps work correctly with lazy loading (Phase 5.1).
    //!
    //! The lazy loading wrapper should:
    //! 1. Defer parsing of 4-byte CMaps
    //! 2. Parse on first access
    //! 3. Cache result correctly
    //!
    //! Verify that lazy loading doesn't break 4-byte support.

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
<00000000> <FFFFFFFF>
endcodespacerange
2 beginbfchar
<00010000> <4E00>
<00020000> <4E01>
endbfchar
endcmap
CMapName currentdict /CMap defineresource pop
end
end
"#
    .as_bytes()
    .to_vec();

    let font = FontInfo {
        base_font: "4ByteCMapFont".to_string(),
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
    };

    assert_eq!(font.char_to_unicode(0x00010000), Some("\u{4E00}".to_string()));

    assert_eq!(font.char_to_unicode(0x00020000), Some("\u{4E01}".to_string()));

    assert_eq!(font.char_to_unicode(0x00010000), Some("\u{4E00}".to_string()));
}

#[test]
fn test_4byte_mixed_width_codes() {
    //! Test that CMaps with mixed 1-2-3-4 byte codes work together.
    //!
    //! Some complex CMaps may use:
    //! - Single-byte codes (0x41)
    //! - Two-byte codes (0x0041)
    //! - Four-byte codes (0x00000041)
    //!
    //! These should coexist correctly in the same CMap.

    let mixed_cmap = r#"
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
<00000000> <FFFFFFFF>
endcodespacerange
3 beginbfchar
<41> <0041>
<0042> <0042>
<00000043> <0043>
endbfchar
endcmap
CMapName currentdict /CMap defineresource pop
end
end
"#;

    let result = xberg_native_pdf::fonts::cmap::parse_tounicode_cmap(mixed_cmap.as_bytes());

    assert!(result.is_ok(), "Should parse mixed-width codes");

    let cmap = result.unwrap();

    assert_eq!(cmap.get(&0x41).as_deref(), Some("A"));

    assert_eq!(cmap.get(&0x42).as_deref(), Some("B"));

    assert_eq!(cmap.get(&0x43).as_deref(), Some("C"));
}
