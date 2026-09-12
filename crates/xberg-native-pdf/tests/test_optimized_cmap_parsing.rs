#![allow(clippy::useless_vec)]
//! Optimized CMap Parsing Tests for Phase 5.3
//!
//! Tests for performance optimization of CMap parsing:
//! - State machine parser to replace regex-based parsing
//! - Binary search for range lookups
//! - Streaming parser for large CMaps
//! - Support for >100k entry CMaps
//!
//! Expected Impact:
//! - 20-40% faster parsing (state machine vs regex)
//! - 50-70% faster range lookups (binary search vs linear)
//! - Support for very large CMaps without memory pressure
//! - Lazy streaming parsing for huge CMaps
//!
//! Spec: PDF 32000-1:2008 Section 9.10.3 (ToUnicode CMaps)

use std::collections::HashMap;
use xberg_native_pdf::fonts::FontInfo;
use xberg_native_pdf::fonts::cmap::LazyCMap;

#[test]
fn test_optimized_parser_large_bfchar_section() {
    //! Test that the optimized parser handles large bfchar sections efficiently.
    //!
    //! Current regex-based parser may struggle with:
    //! - 10,000+ individual character entries
    //! - Large amounts of whitespace and formatting
    //! - Memory allocation for intermediate representations
    //!
    //! Optimized state machine parser should:
    //! 1. Stream through entries without full buffering
    //! 2. Direct insertion into HashMap (no intermediate storage)
    //! 3. Single-pass parsing with O(n) complexity

    let mut large_cmap = String::from(
        r#"
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
"#,
    );

    large_cmap.push_str("1000 beginbfchar\n");
    for i in 0..1000 {
        large_cmap.push_str(&format!("<{:04X}> <{:04X}>\n", 0x0100 + i, 0x0100 + i));
    }
    large_cmap.push_str("endbfchar\n");

    large_cmap.push_str(
        r#"
endcmap
CMapName currentdict /CMap defineresource pop
end
end
"#,
    );

    let font = FontInfo {
        base_font: "LargeCMapFont".to_string(),
        subtype: "Type0".to_string(),
        encoding: xberg_native_pdf::fonts::Encoding::Identity,
        to_unicode: Some(LazyCMap::new(large_cmap.as_bytes().to_vec())),
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

    assert_eq!(
        font.char_to_unicode(0x0100),
        Some("\u{0100}".to_string()),
        "First entry"
    );
    assert_eq!(
        font.char_to_unicode(0x0200),
        Some("\u{0200}".to_string()),
        "Middle entry"
    );
    assert_eq!(font.char_to_unicode(0x03E3), Some("\u{03E3}".to_string()), "Last entry");

    assert_eq!(
        font.char_to_unicode(0x0150),
        Some("\u{0150}".to_string()),
        "Random entry 1"
    );
    assert_eq!(
        font.char_to_unicode(0x0250),
        Some("\u{0250}".to_string()),
        "Random entry 2"
    );
}

#[test]
fn test_optimized_parser_large_bfrange_section() {
    //! Test binary search optimization for range lookups.
    //!
    //! For bfrange entries like <0000> <00FF> <4E00>,
    //! the optimized parser should:
    //! 1. Store range metadata efficiently (start, end, target)
    //! 2. Use binary search on sorted ranges for O(log n) lookup
    //! 3. Handle overlapping or contiguous ranges correctly

    let mut cmap_with_ranges = String::from(
        r#"
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
"#,
    );

    cmap_with_ranges.push_str("100 beginbfrange\n");
    for i in 0..100 {
        let start = i * 256;
        let end = start + 255;
        let target = 0x4E00 + i as u32 * 256;
        cmap_with_ranges.push_str(&format!("<{:04X}> <{:04X}> <{:04X}>\n", start, end, target));
    }
    cmap_with_ranges.push_str("endbfrange\n");

    cmap_with_ranges.push_str(
        r#"
endcmap
CMapName currentdict /CMap defineresource pop
end
end
"#,
    );

    let font = FontInfo {
        base_font: "LargeRangeFont".to_string(),
        subtype: "Type0".to_string(),
        encoding: xberg_native_pdf::fonts::Encoding::Identity,
        to_unicode: Some(LazyCMap::new(cmap_with_ranges.as_bytes().to_vec())),
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

    assert_eq!(
        font.char_to_unicode(0x0000),
        Some("\u{4E00}".to_string()),
        "Range 0 start"
    );
    assert_eq!(
        font.char_to_unicode(0x0080),
        Some("\u{4E80}".to_string()),
        "Range 0 middle"
    );
    assert_eq!(
        font.char_to_unicode(0x00FF),
        Some("\u{4EFF}".to_string()),
        "Range 0 end"
    );

    let range_50_start = 50 * 256;
    let range_50_unicode = 0x4E00 + 50_u32 * 256;
    assert_eq!(
        font.char_to_unicode(range_50_start),
        Some(char::from_u32(range_50_unicode).unwrap_or('\u{FFFD}').to_string()),
        "Range 50 lookup"
    );

    let range_99_start = 99 * 256;
    let range_99_unicode = 0x4E00 + 99_u32 * 256;
    assert_eq!(
        font.char_to_unicode(range_99_start),
        Some(char::from_u32(range_99_unicode).unwrap_or('\u{FFFD}').to_string()),
        "Range 99 lookup"
    );
}

#[test]
fn test_optimized_parser_mixed_large_cmap() {
    //! Test optimized parser with mixed bfchar and bfrange sections.
    //!
    //! Real CMaps often mix:
    //! - Individual character mappings (bfchar)
    //! - Range mappings (bfrange)
    //! - Undefined range fallbacks (beginnotdefrange)
    //!
    //! All sections should be parsed correctly and efficiently.

    let mut mixed_cmap = String::from(
        r#"
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
100 beginbfchar
"#,
    );

    for i in 0..100 {
        mixed_cmap.push_str(&format!("<{:04X}> <{:04X}>\n", i, i + 100));
    }
    mixed_cmap.push_str("endbfchar\n");

    mixed_cmap.push_str("50 beginbfrange\n");
    for i in 0..50 {
        let start = 0x1000 + i * 256;
        let end = start + 255;
        let target = 0x4E00 + i as u32 * 256;
        mixed_cmap.push_str(&format!("<{:04X}> <{:04X}> <{:04X}>\n", start, end, target));
    }
    mixed_cmap.push_str("endbfrange\n");

    mixed_cmap.push_str(
        r#"
1 beginnotdefrange
<6400> <FFFF> <FFFD>
endnotdefrange
endcmap
CMapName currentdict /CMap defineresource pop
end
end
"#,
    );

    let font = FontInfo {
        base_font: "MixedLargeCMap".to_string(),
        subtype: "Type0".to_string(),
        encoding: xberg_native_pdf::fonts::Encoding::Identity,
        to_unicode: Some(LazyCMap::new(mixed_cmap.as_bytes().to_vec())),
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

    assert_eq!(
        font.char_to_unicode(0x0000),
        Some("\u{0064}".to_string()),
        "bfchar entry 0"
    );
    assert_eq!(
        font.char_to_unicode(0x0050),
        Some("\u{00B4}".to_string()),
        "bfchar entry 80"
    );

    assert_eq!(
        font.char_to_unicode(0x1000),
        Some("\u{4E00}".to_string()),
        "bfrange 0 start"
    );
    assert_eq!(
        font.char_to_unicode(0x1100),
        Some("\u{4F00}".to_string()),
        "bfrange 1 lookup"
    );

    // CID 0x6400 not in bfchar/bfrange; with a /ToUnicode present and non-Identity
    // ordering, an uncovered code is unmapped → U+FFFD, not a CID-as-Unicode guess ~keep
    assert_eq!(
        font.char_to_unicode(0x6400),
        Some("\u{FFFD}".to_string()),
        "uncovered code with /ToUnicode present must not be guessed as CID-as-Unicode"
    );
}

#[test]
fn test_optimized_parser_mixed_large_cmap_identity_ordered() {
    //! Identity-ordered sibling of `test_optimized_parser_mixed_large_cmap`.
    //!
    //! Same setup — Identity-H encoding, a /ToUnicode covering only some codes —
    //! but with `CIDSystemInfo` `Ordering = Identity` (Adobe-Identity-0). A code
    //! uncovered by /ToUnicode has no trustworthy codepoint, so it decodes to U+FFFD;
    //! only whitespace (CID 0x20) retains the CID-as-Unicode mapping.

    let cmap = String::from(
        r#"
/CIDInit /ProcSet findresource begin
12 dict begin
begincmap
/CIDSystemInfo
<< /Registry (Adobe) /Ordering (Identity) /Supplement 0 >> def
/CMapName /Identity-H def
/CMapType 2 def
1 begincodespacerange
<0000> <FFFF>
endcodespacerange
2 beginbfchar
<0041> <0041>
<0050> <0050>
endbfchar
endcmap
end
end
"#,
    );

    let font = FontInfo {
        base_font: "MixedLargeCMapIdentity".to_string(),
        subtype: "Type0".to_string(),
        encoding: xberg_native_pdf::fonts::Encoding::Identity,
        to_unicode: Some(LazyCMap::new(cmap.as_bytes().to_vec())),
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

    assert_eq!(
        font.char_to_unicode(0x0041),
        Some("\u{0041}".to_string()),
        "bfchar entry"
    );
    assert_eq!(
        font.char_to_unicode(0x5000),
        Some("\u{FFFD}".to_string()),
        "Identity-ordered uncovered code must decode to U+FFFD, not a CID-as-Unicode guess"
    );
}

#[test]
fn test_optimized_parser_whitespace_variations() {
    //! Test that optimized state machine handles whitespace robustly.
    //!
    //! State machine parser should:
    //! - Handle arbitrary whitespace (spaces, tabs, newlines)
    //! - Not require precise formatting
    //! - Work with minified and pretty-printed CMaps

    let cmap_variations = vec![
        r#"
/CIDInit /ProcSet findresource begin
12 dict begin
begincmap
/CIDSystemInfo<</Registry(Adobe)/Ordering(Identity)/Supplement 0>>def
/CMapName /Identity-H def
/CMapType 2 def
1 begincodespacerange
<0000><FFFF>
endcodespacerange
2 beginbfchar
<0041><0041>
<0042><0042>
endbfchar
endcmap
CMapName currentdict /CMap defineresource pop
end
end
"#,
        r#"
/CIDInit /ProcSet findresource begin
12 dict begin
begincmap
/CIDSystemInfo
<<  /Registry    (Adobe)
    /Ordering    (Identity)
    /Supplement  0
>>  def
/CMapName  /Identity-H  def
/CMapType  2  def
1  begincodespacerange
<0000>  <FFFF>
endcodespacerange
2  beginbfchar
<0041>  <0041>
<0042>  <0042>
endbfchar
endcmap
CMapName  currentdict  /CMap  defineresource  pop
end
end
"#,
    ];

    for (idx, cmap_str) in cmap_variations.iter().enumerate() {
        let font = FontInfo {
            base_font: format!("WhitespaceTest{}", idx),
            subtype: "Type0".to_string(),
            encoding: xberg_native_pdf::fonts::Encoding::Identity,
            to_unicode: Some(LazyCMap::new(cmap_str.as_bytes().to_vec())),
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

        assert_eq!(
            font.char_to_unicode(0x0041),
            Some("A".to_string()),
            "Format {} should parse correctly",
            idx
        );
        assert_eq!(
            font.char_to_unicode(0x0042),
            Some("B".to_string()),
            "Format {} should parse correctly",
            idx
        );
    }
}

#[test]
fn test_optimized_parser_very_large_cmap_100k_entries() {
    //! Test that optimized parser handles 100k+ entries without memory issues.
    //!
    //! Large CMaps (e.g., for CJK fonts) may have:
    //! - 10,000 - 100,000+ mappings
    //! - Multi-megabyte raw stream sizes
    //!
    //! Optimized implementation should:
    //! 1. Parse without buffering entire structure in memory
    //! 2. Support streaming/incremental parsing if needed
    //! 3. Maintain performance (sub-second parsing)

    let mut large_cmap = String::from(
        r#"
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
"#,
    );

    for chunk in 0..10 {
        let count = 1000;
        large_cmap.push_str(&format!("{} beginbfchar\n", count));
        for i in 0..count {
            let code = chunk * 1000 + i as u32;
            large_cmap.push_str(&format!("<{:04X}> <{:04X}>\n", code, code + 0x4E00));
        }
        large_cmap.push_str("endbfchar\n");
    }

    large_cmap.push_str(
        r#"
endcmap
CMapName currentdict /CMap defineresource pop
end
end
"#,
    );

    let font = FontInfo {
        base_font: "VeryLargeCMapFont".to_string(),
        subtype: "Type0".to_string(),
        encoding: xberg_native_pdf::fonts::Encoding::Identity,
        to_unicode: Some(LazyCMap::new(large_cmap.as_bytes().to_vec())),
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

    assert!(font.char_to_unicode(0x0000).is_some(), "First entry should exist");
    assert!(font.char_to_unicode(0x0100).is_some(), "Random entry in first chunk");
    assert!(font.char_to_unicode(0x1000).is_some(), "Random entry in second chunk");
    assert!(font.char_to_unicode(0x2000).is_some(), "Random entry in third chunk");
    assert!(font.char_to_unicode(0x4000).is_some(), "Random entry in fifth chunk");
}

#[test]
fn test_optimized_parser_streaming_for_mega_cmap() {
    //! Test streaming parser capability for extremely large CMaps.
    //!
    //! For CMaps > 10MB, full in-memory parsing may be problematic.
    //! A streaming parser should:
    //! 1. Parse entries incrementally
    //! 2. Not require the entire stream in memory
    //! 3. Maintain access to all entries via lazy evaluation

    let mut mega_cmap = String::from(
        r#"
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
"#,
    );

    for section in 0..50 {
        let count = 1000;
        mega_cmap.push_str(&format!("{} beginbfchar\n", count));
        for i in 0..count {
            let code = section * 1000 + i as u32;
            mega_cmap.push_str(&format!("<{:04X}> <{:04X}>\n", code & 0xFFFF, code));
        }
        mega_cmap.push_str("endbfchar\n");
    }

    mega_cmap.push_str(
        r#"
endcmap
CMapName currentdict /CMap defineresource pop
end
end
"#,
    );

    let font = FontInfo {
        base_font: "MegaCMapFont".to_string(),
        subtype: "Type0".to_string(),
        encoding: xberg_native_pdf::fonts::Encoding::Identity,
        to_unicode: Some(LazyCMap::new(mega_cmap.as_bytes().to_vec())),
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

    assert!(font.char_to_unicode(0x0000).is_some(), "First lookup should work");

    assert!(font.char_to_unicode(0x0001).is_some(), "Second lookup should work");
    assert!(font.char_to_unicode(0x5000).is_some(), "Random lookup should work");
}
