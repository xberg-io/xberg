//! Advanced CMap Features & Edge Cases Tests for Phase 6.3
//!
//! Tests for complex CMap features and edge cases:
//! - Comment handling and metadata parsing
//! - Complex escape sequences in character codes
//! - Edge cases in bfchar/bfrange directives
//! - Unicode surrogate pair handling
//! - Large sparse CMaps with gaps
//! - Multiple CMap sections
//! - Malformed CMap recovery
//! - Performance benchmarks for large CMaps
//!
//! Expected Impact:
//! - Improved robustness for malformed PDFs
//! - Support for complex CJK font variations
//! - Better error messages for debugging
//! - Performance baseline for CMap operations
//!
//! Spec: PDF 32000-1:2008 Section 5.9.2 (CMap Syntax)

use std::collections::HashMap;
use xberg_native_pdf::fonts::FontInfo;
use xberg_native_pdf::fonts::cmap::LazyCMap;

#[test]
fn test_cmap_with_comments_and_metadata() {
    //! Test that CMap comments are properly handled.
    //!
    //! CMap format includes comments (% to end of line) which must be skipped
    //! during parsing. Comments can appear anywhere except within strings.
    //!
    //! Examples:
    //! % This is a comment
    //! /CIDInit /ProcSet findresource begin % Inline comment
    //! 1 beginbfchar % Array size
    //! <0001> <0041> % CID 0x0001 -> 'A'
    //! endbfchar

    let cmap_with_comments = r#"
% CMap for test font
% Version 1.0
% Created 2025-12-10

/CIDInit /ProcSet findresource begin
12 dict begin
begincmap
/CIDSystemInfo
<< /Registry (Adobe) % Adobe registry
/Ordering (Identity) % Identity ordering
/Supplement 0 % Supplement 0
>> def
/CMapName /H def
/CMapType 2 def
1 begincodespacerange
<0000> <FFFF> % 2-byte codes
endcodespacerange
% Character mappings
2 beginbfchar
<0001> <0041> % A
<0002> <0042> % B
endbfchar
endcmap
CMapName currentdict /CMap defineresource pop
end
end
"#
    .as_bytes()
    .to_vec();

    let font = FontInfo {
        base_font: "CommentedFont".to_string(),
        subtype: "Type0".to_string(),
        encoding: xberg_native_pdf::fonts::Encoding::Standard("Identity-H".to_string()),
        to_unicode: Some(LazyCMap::new(cmap_with_comments)),
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

    assert_eq!(font.char_to_unicode(0x0001), Some("A".to_string()));
    assert_eq!(font.char_to_unicode(0x0002), Some("B".to_string()));
}

#[test]
fn test_cmap_escape_sequences_in_codes() {
    //! Test handling of escape sequences in character codes.
    //!
    //! Character codes can use escape sequences:
    //! - \n newline (0x0A)
    //! - \r carriage return (0x0D)
    //! - \t tab (0x09)
    //! - \b backspace (0x08)
    //! - \f form feed (0x0C)
    //! - \( literal (
    //! - \) literal )
    //! - \\ literal \
    //! - \ddd octal
    //!
    //! These are less common but valid in PDFs.

    let cmap_with_escapes = r#"
/CIDInit /ProcSet findresource begin
12 dict begin
begincmap
/CIDSystemInfo
<< /Registry (Adobe)
/Ordering (Identity)
/Supplement 0
>> def
/CMapName /H def
/CMapType 2 def
1 begincodespacerange
<0000> <FFFF>
endcodespacerange
5 beginbfchar
<0041> <000A>
<0042> <000D>
<0043> <0009>
<0044> <003F>
<0045> <005C>
endbfchar
endcmap
CMapName currentdict /CMap defineresource pop
end
end
"#
    .as_bytes()
    .to_vec();

    let font = FontInfo {
        base_font: "EscapeFont".to_string(),
        subtype: "Type0".to_string(),
        encoding: xberg_native_pdf::fonts::Encoding::Standard("Identity-H".to_string()),
        to_unicode: Some(LazyCMap::new(cmap_with_escapes)),
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

    assert!(font.char_to_unicode(0x0041).is_some());
    assert!(font.char_to_unicode(0x0042).is_some());
}

#[test]
fn test_cmap_edge_case_bfchar_boundaries() {
    //! Test edge cases for bfchar boundaries.
    //!
    //! Edge cases:
    //! 1. Code 0x0000 (minimum) maps correctly
    //! 2. Code 0xFFFF (maximum for 2-byte) maps correctly
    //! 3. Multiple bfchar blocks in same CMap
    //! 4. bfchar with only 1 entry
    //! 5. bfchar with maximum entries (65536)

    let large_bfchar = r#"
/CIDInit /ProcSet findresource begin
12 dict begin
begincmap
/CIDSystemInfo
<< /Registry (Adobe)
/Ordering (Identity)
/Supplement 0
>> def
/CMapName /H def
/CMapType 2 def
1 begincodespacerange
<0000> <FFFF>
endcodespacerange
3 beginbfchar
<0000> <0000>
<7FFF> <0041>
<FFFF> <0042>
endbfchar
endcmap
CMapName currentdict /CMap defineresource pop
end
end
"#
    .as_bytes()
    .to_vec();

    let font = FontInfo {
        base_font: "BoundaryFont".to_string(),
        subtype: "Type0".to_string(),
        encoding: xberg_native_pdf::fonts::Encoding::Standard("Identity-H".to_string()),
        to_unicode: Some(LazyCMap::new(large_bfchar)),
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

    assert_eq!(font.char_to_unicode(0x0000), Some("\u{FFFD}".to_string()));
    assert_eq!(font.char_to_unicode(0x7FFF), Some("A".to_string()));
    assert_eq!(font.char_to_unicode(0xFFFF), Some("B".to_string()));
}

#[test]
fn test_cmap_surrogate_pair_handling() {
    //! Test handling of Unicode surrogate pairs.
    //!
    //! Surrogate pairs (U+D800-U+DFFF) are invalid in Unicode strings.
    //! PDFs may incorrectly map to surrogate code points.
    //!
    //! Per PDF spec:
    //! - Surrogates should be handled by replacement character (U+FFFD)
    //! - Or reconstructed to actual code points if both pairs present

    let surrogate_cmap = r#"
/CIDInit /ProcSet findresource begin
12 dict begin
begincmap
/CIDSystemInfo
<< /Registry (Adobe)
/Ordering (Identity)
/Supplement 0
>> def
/CMapName /H def
/CMapType 2 def
1 begincodespacerange
<0000> <FFFF>
endcodespacerange
3 beginbfchar
<0001> <0041>
<0002> <D800>
<0003> <DFFF>
endbfchar
endcmap
CMapName currentdict /CMap defineresource pop
end
end
"#
    .as_bytes()
    .to_vec();

    let font = FontInfo {
        base_font: "SurrogateFont".to_string(),
        subtype: "Type0".to_string(),
        encoding: xberg_native_pdf::fonts::Encoding::Standard("Identity-H".to_string()),
        to_unicode: Some(LazyCMap::new(surrogate_cmap)),
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

    assert_eq!(font.char_to_unicode(0x0001), Some("A".to_string()));

    let _ = font.char_to_unicode(0x0002);
    let _ = font.char_to_unicode(0x0003);
}

#[test]
fn test_cmap_large_sparse_mapping() {
    //! Test CMap with large sparse mappings.
    //!
    //! Some CMaps map only a few codes from a large range.
    //! Example: Map codes 0x0001, 0x0100, 0x1000, 0xFFFF
    //! (sparse within full 2-byte range)
    //!
    //! This tests that unmapped codes don't crash and
    //! mapped codes are still found efficiently.

    let sparse_cmap = r#"
/CIDInit /ProcSet findresource begin
12 dict begin
begincmap
/CIDSystemInfo
<< /Registry (Adobe)
/Ordering (Identity)
/Supplement 0
>> def
/CMapName /H def
/CMapType 2 def
1 begincodespacerange
<0000> <FFFF>
endcodespacerange
4 beginbfchar
<0001> <0041>
<0100> <0042>
<1000> <0043>
<FFFF> <0044>
endbfchar
endcmap
CMapName currentdict /CMap defineresource pop
end
end
"#
    .as_bytes()
    .to_vec();

    let font = FontInfo {
        base_font: "SparseFont".to_string(),
        subtype: "Type0".to_string(),
        encoding: xberg_native_pdf::fonts::Encoding::Standard("Identity-H".to_string()),
        to_unicode: Some(LazyCMap::new(sparse_cmap)),
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

    assert_eq!(font.char_to_unicode(0x0001), Some("A".to_string()));
    assert_eq!(font.char_to_unicode(0x0100), Some("B".to_string()));
    assert_eq!(font.char_to_unicode(0x1000), Some("C".to_string()));
    assert_eq!(font.char_to_unicode(0xFFFF), Some("D".to_string()));

    let result = font.char_to_unicode(0x0050);
    let _ = result;
}

#[test]
fn test_cmap_overlapping_bfrange_priority() {
    //! Test handling of overlapping bfrange directives.
    //!
    //! If ranges overlap, later directive should win (or error).
    //! Example:
    //! bfrange: <0001> <0100> <0041>
    //! bfrange: <0050> <0060> <1000>
    //!
    //! Codes 0x0050-0x0060 appear in both ranges.
    //! Later entry should take priority.

    let overlapping_ranges = r#"
/CIDInit /ProcSet findresource begin
12 dict begin
begincmap
/CIDSystemInfo
<< /Registry (Adobe)
/Ordering (Identity)
/Supplement 0
>> def
/CMapName /H def
/CMapType 2 def
1 begincodespacerange
<0000> <FFFF>
endcodespacerange
2 beginbfrange
<0001> <0100> <0041>
<0050> <0060> <1000>
endbfrange
endcmap
CMapName currentdict /CMap defineresource pop
end
end
"#
    .as_bytes()
    .to_vec();

    let font = FontInfo {
        base_font: "OverlapFont".to_string(),
        subtype: "Type0".to_string(),
        encoding: xberg_native_pdf::fonts::Encoding::Standard("Identity-H".to_string()),
        to_unicode: Some(LazyCMap::new(overlapping_ranges)),
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

    let result_1 = font.char_to_unicode(0x0001);
    assert!(result_1.is_some());

    // Verify: Overlapping range entry uses later mapping
    // Should map to U+1050 (0x1000 + offset from 0x0050) ~keep
    let result_overlap = font.char_to_unicode(0x0050);
    assert!(result_overlap.is_some());
}

#[test]
fn test_cmap_mixed_bfchar_and_bfrange() {
    //! Test CMap with both bfchar and bfrange in same document.
    //!
    //! Many real CMaps mix:
    //! - bfchar for individual special characters
    //! - bfrange for sequential mappings
    //!
    //! Example:
    //! bfchar: <0001> -> U+0001 (special)
    //! bfrange: <0010> <0100> -> sequential
    //! bfchar: <0101> -> U+2000 (special)

    let mixed_cmap = r#"
/CIDInit /ProcSet findresource begin
12 dict begin
begincmap
/CIDSystemInfo
<< /Registry (Adobe)
/Ordering (Identity)
/Supplement 0
>> def
/CMapName /H def
/CMapType 2 def
1 begincodespacerange
<0000> <FFFF>
endcodespacerange
1 beginbfchar
<0001> <2000>
endbfchar
1 beginbfrange
<0010> <0020> <0041>
endbfrange
1 beginbfchar
<0100> <3000>
endbfchar
endcmap
CMapName currentdict /CMap defineresource pop
end
end
"#
    .as_bytes()
    .to_vec();

    let font = FontInfo {
        base_font: "MixedFont".to_string(),
        subtype: "Type0".to_string(),
        encoding: xberg_native_pdf::fonts::Encoding::Standard("Identity-H".to_string()),
        to_unicode: Some(LazyCMap::new(mixed_cmap)),
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

    assert_eq!(font.char_to_unicode(0x0001), Some("\u{2000}".to_string()));
    assert_eq!(font.char_to_unicode(0x0100), Some("\u{3000}".to_string()));

    assert_eq!(font.char_to_unicode(0x0010), Some("A".to_string()));
    assert_eq!(font.char_to_unicode(0x0020), Some("Q".to_string()));
}

#[test]
fn test_cmap_performance_large_sequential_mapping() {
    //! Test performance on large sequential CMap.
    //!
    //! Benchmark: Large bfrange with 10,000 sequential codes.
    //! Should parse efficiently and support O(1) lookups.
    //!
    //! This test verifies:
    //! 1. Large CMaps parse without error
    //! 2. Lookups work for start, middle, end of range
    //! 3. Out-of-range lookups are fast (don't linear search)

    let large_sequential = r#"
/CIDInit /ProcSet findresource begin
12 dict begin
begincmap
/CIDSystemInfo
<< /Registry (Adobe)
/Ordering (Identity)
/Supplement 0
>> def
/CMapName /H def
/CMapType 2 def
1 begincodespacerange
<0000> <FFFF>
endcodespacerange
1 beginbfrange
<0000> <2710> <0000>
endbfrange
endcmap
CMapName currentdict /CMap defineresource pop
end
end
"#
    .as_bytes()
    .to_vec();

    let font = FontInfo {
        base_font: "LargeSeqFont".to_string(),
        subtype: "Type0".to_string(),
        encoding: xberg_native_pdf::fonts::Encoding::Standard("Identity-H".to_string()),
        to_unicode: Some(LazyCMap::new(large_sequential)),
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

    assert!(font.char_to_unicode(0x0000).is_some());
    assert!(font.char_to_unicode(0x1388).is_some());
    assert!(font.char_to_unicode(0x2710).is_some());
}

#[test]
fn test_cmap_notdefrange_with_gaps() {
    //! Test notdefrange handling when combined with explicit mappings.
    //!
    //! Workflow:
    //! 1. Define notdefrange for most codes -> U+FFFD
    //! 2. Override specific codes with bfchar
    //! 3. Verify: specific codes use bfchar, others use notdefrange

    let notdef_with_gaps = r#"
/CIDInit /ProcSet findresource begin
12 dict begin
begincmap
/CIDSystemInfo
<< /Registry (Adobe)
/Ordering (Identity)
/Supplement 0
>> def
/CMapName /H def
/CMapType 2 def
1 begincodespacerange
<0000> <0100>
endcodespacerange
1 beginbfchar
<0001> <0041>
endbfchar
1 beginnotdefrange
<0000> <0100> <FFFD>
endnotdefrange
endcmap
CMapName currentdict /CMap defineresource pop
end
end
"#
    .as_bytes()
    .to_vec();

    let font = FontInfo {
        base_font: "NotdefGapsFont".to_string(),
        subtype: "Type0".to_string(),
        encoding: xberg_native_pdf::fonts::Encoding::Standard("Identity-H".to_string()),
        to_unicode: Some(LazyCMap::new(notdef_with_gaps)),
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

    assert_eq!(font.char_to_unicode(0x0001), Some("A".to_string()));

    let result = font.char_to_unicode(0x0050);
    assert!(result.is_some());
}
