//! Multi-byte CID Support Tests for Phase 6.1
//!
//! Tests for advanced multi-byte CID character code handling:
//! - Variable-length character codes (1-4 bytes)
//! - Multi-byte CID sequences in CMaps
//! - CID ranges with variable code widths
//! - Mixed encoding systems (CJK combinations)
//!
//! Expected Impact:
//! - Support for complex CJK fonts with variable-length CIDs
//! - Better handling of Adobe-GB1, Adobe-CNS1 fonts
//! - 3-5% additional document coverage
//!
//! Spec: PDF 32000-1:2008 Section 9.7.6 (CID Fonts)

use std::collections::HashMap;
use xberg_native_pdf::fonts::FontInfo;
use xberg_native_pdf::fonts::cmap::LazyCMap;

#[test]
fn test_multibyte_cid_2byte_codes() {
    //! Test that 2-byte CID codes are properly handled.
    //!
    //! Most CJK fonts use 2-byte CID codes in format:
    //! - First byte: 0x81-0xFF (lead byte)
    //! - Second byte: 0x40-0x7E or 0x80-0xFF (trail byte)
    //!
    //! Example: <8140> is a valid 2-byte CID code

    let cmap_2byte = r#"
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
5 beginbfchar
<0001> <4E00>
<0002> <4E01>
<8140> <30A1>
<8141> <30A3>
<FFFE> <FFFD>
endbfchar
endcmap
CMapName currentdict /CMap defineresource pop
end
end
"#;

    let font = FontInfo {
        base_font: "UniGB-H".to_string(),
        subtype: "Type0".to_string(),
        encoding: xberg_native_pdf::fonts::Encoding::Standard("UniGB-UCS2-H".to_string()),
        to_unicode: Some(LazyCMap::new(cmap_2byte.as_bytes().to_vec())),
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
            ordering: "GB1".to_string(),
            supplement: 2,
        }),
        cid_font_type: None,
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

    assert_eq!(
        font.char_to_unicode(0x0001),
        Some("\u{4E00}".to_string()),
        "1-byte code"
    );
    assert_eq!(
        font.char_to_unicode(0x0002),
        Some("\u{4E01}".to_string()),
        "1-byte code"
    );

    assert_eq!(
        font.char_to_unicode(0x8140),
        Some("\u{30A1}".to_string()),
        "2-byte code (lead+trail)"
    );
    assert_eq!(
        font.char_to_unicode(0x8141),
        Some("\u{30A3}".to_string()),
        "2-byte code (lead+trail)"
    );

    // Note: U+FFFD (replacement character) mappings are intentionally rejected per PDF Spec compliance.
    // CMaps with <FFFE> <FFFD> entries are treated as "unmapped" and fall through to Priority 2.
    // This is intentional behavior per ENDASH_ISSUE_ROOT_CAUSE.md to handle broken PDF tooling.
    // So we don't test for U+FFFD mappings here. ~keep
}

#[test]
fn test_multibyte_cid_variable_width_ranges() {
    //! Test that bfrange works with variable-width CID codes.
    //!
    //! Variable-width ranges map:
    //! <start1> <end1> <target1>
    //!
    //! Where start1 and end1 may span multiple bytes
    //! and represent continuous sequences

    let cmap_var_width = r#"
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
3 beginbfrange
<0001> <000F> <4E00>
<8140> <814F> <30A0>
<0100> <010F> <AC00>
endbfrange
endcmap
CMapName currentdict /CMap defineresource pop
end
end
"#;

    let font = FontInfo {
        base_font: "VarWidthGB".to_string(),
        subtype: "Type0".to_string(),
        encoding: xberg_native_pdf::fonts::Encoding::Standard("UniGB-UCS2-H".to_string()),
        to_unicode: Some(LazyCMap::new(cmap_var_width.as_bytes().to_vec())),
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
            ordering: "GB1".to_string(),
            supplement: 2,
        }),
        cid_font_type: None,
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

    // Verify: Range 1 (0x0001-0x000F → 0x4E00-0x4E0E) ~keep
    assert_eq!(
        font.char_to_unicode(0x0001),
        Some("\u{4E00}".to_string()),
        "Range 1 start"
    );
    assert_eq!(
        font.char_to_unicode(0x0008),
        Some("\u{4E07}".to_string()),
        "Range 1 middle"
    );
    assert_eq!(
        font.char_to_unicode(0x000F),
        Some("\u{4E0E}".to_string()),
        "Range 1 end"
    );

    // Verify: Range 2 (0x8140-0x814F → 0x30A0-0x30AF) ~keep
    assert_eq!(
        font.char_to_unicode(0x8140),
        Some("\u{30A0}".to_string()),
        "Range 2 start"
    );
    assert_eq!(
        font.char_to_unicode(0x8145),
        Some("\u{30A5}".to_string()),
        "Range 2 middle"
    );
    assert_eq!(
        font.char_to_unicode(0x814F),
        Some("\u{30AF}".to_string()),
        "Range 2 end"
    );

    // Verify: Range 3 (0x0100-0x010F → 0xAC00-0xAC0F) ~keep
    assert_eq!(
        font.char_to_unicode(0x0100),
        Some("\u{AC00}".to_string()),
        "Range 3 start"
    );
}

#[test]
fn test_multibyte_cid_adobe_gb1_fonts() {
    //! Test real-world Adobe-GB1 (Simplified Chinese) font CMaps.
    //!
    //! Adobe-GB1 uses:
    //! - Registry: Adobe
    //! - Ordering: GB1
    //! - Supplements: 0-5 (increasingly comprehensive)

    let adobe_gb1_cmap = r#"
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
100 beginbfchar
"#;

    let mut full_cmap = adobe_gb1_cmap.to_string();

    for i in 0..100 {
        let cid = 0x0001 + i;
        let unicode = 0x4E00 + i;
        full_cmap.push_str(&format!("<{:04X}> <{:04X}>\n", cid, unicode));
    }

    full_cmap.push_str(
        r#"
endbfchar
endcmap
CMapName currentdict /CMap defineresource pop
end
end
"#,
    );

    let font = FontInfo {
        base_font: "SimSun".to_string(),
        subtype: "Type0".to_string(),
        encoding: xberg_native_pdf::fonts::Encoding::Standard("UniGB-UCS2-H".to_string()),
        to_unicode: Some(LazyCMap::new(full_cmap.as_bytes().to_vec())),
        font_weight: Some(400),
        flags: Some(0x0010),
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
            ordering: "GB1".to_string(),
            supplement: 2,
        }),
        cid_font_type: None,
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

    assert_eq!(
        font.char_to_unicode(0x0001),
        Some("\u{4E00}".to_string()),
        "First CJK char"
    );
    assert_eq!(
        font.char_to_unicode(0x0050),
        Some("\u{4E4F}".to_string()),
        "Middle CJK char"
    );
    assert_eq!(
        font.char_to_unicode(0x0064),
        Some("\u{4E63}".to_string()),
        "Last CJK char"
    );
}

#[test]
fn test_multibyte_cid_adobe_cns1_fonts() {
    //! Test Adobe-CNS1 (Traditional Chinese) font CMaps.
    //!
    //! Adobe-CNS1 uses:
    //! - Registry: Adobe
    //! - Ordering: CNS1
    //! - Supplements: 0-7 (most comprehensive Traditional Chinese)

    let adobe_cns1_cmap = r#"
/CIDInit /ProcSet findresource begin
12 dict begin
begincmap
/CIDSystemInfo
<< /Registry (Adobe)
/Ordering (CNS1)
/Supplement 7
>> def
/CMapName /UniCNS-UCS2-H def
/CMapType 2 def
1 begincodespacerange
<0000> <FFFF>
endcodespacerange
2 beginbfrange
<0001> <00FF> <4E00>
<0100> <01FF> <9FA0>
endbfrange
endcmap
CMapName currentdict /CMap defineresource pop
end
end
"#;

    let font = FontInfo {
        base_font: "MingLiU".to_string(),
        subtype: "Type0".to_string(),
        encoding: xberg_native_pdf::fonts::Encoding::Standard("UniCNS-UCS2-H".to_string()),
        to_unicode: Some(LazyCMap::new(adobe_cns1_cmap.as_bytes().to_vec())),
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
            ordering: "CNS1".to_string(),
            supplement: 7,
        }),
        cid_font_type: None,
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

    assert_eq!(
        font.char_to_unicode(0x0001),
        Some("\u{4E00}".to_string()),
        "Range 1 start"
    );
    assert_eq!(
        font.char_to_unicode(0x0080),
        Some("\u{4E7F}".to_string()),
        "Range 1 middle"
    );
    assert_eq!(
        font.char_to_unicode(0x00FF),
        Some("\u{4EFE}".to_string()),
        "Range 1 end"
    );

    assert_eq!(
        font.char_to_unicode(0x0100),
        Some("\u{9FA0}".to_string()),
        "Range 2 start"
    );
    assert_eq!(
        font.char_to_unicode(0x01FF),
        Some("\u{A09F}".to_string()),
        "Range 2 end"
    );
}

#[test]
fn test_multibyte_cid_adobe_japan1_fonts() {
    //! Test Adobe-Japan1 font CMaps (Japanese).
    //!
    //! Adobe-Japan1 uses:
    //! - Registry: Adobe
    //! - Ordering: Japan1
    //! - Supplements: 0-6 (comprehensive Japanese support)

    let adobe_japan1_cmap = r#"
/CIDInit /ProcSet findresource begin
12 dict begin
begincmap
/CIDSystemInfo
<< /Registry (Adobe)
/Ordering (Japan1)
/Supplement 6
>> def
/CMapName /UniJIS-UCS2-H def
/CMapType 2 def
1 begincodespacerange
<0000> <FFFF>
endcodespacerange
3 beginbfrange
<0001> <00FF> <3041>
<0100> <01FF> <3341>
<0200> <02FF> <4E00>
endbfrange
endcmap
CMapName currentdict /CMap defineresource pop
end
end
"#;

    let font = FontInfo {
        base_font: "Hiragino".to_string(),
        subtype: "Type0".to_string(),
        encoding: xberg_native_pdf::fonts::Encoding::Standard("UniJIS-UCS2-H".to_string()),
        to_unicode: Some(LazyCMap::new(adobe_japan1_cmap.as_bytes().to_vec())),
        font_weight: Some(400),
        flags: Some(0x0010),
        stem_v: Some(80.0),
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
            supplement: 6,
        }),
        cid_font_type: None,
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

    assert_eq!(
        font.char_to_unicode(0x0001),
        Some("\u{3041}".to_string()),
        "Hiragana start"
    );

    assert_eq!(
        font.char_to_unicode(0x0100),
        Some("\u{3341}".to_string()),
        "Katakana start"
    );

    assert_eq!(
        font.char_to_unicode(0x0200),
        Some("\u{4E00}".to_string()),
        "Kanji start"
    );
}

#[test]
fn test_multibyte_cid_mixed_single_multibyte() {
    //! Test CMaps that mix single-byte and multi-byte CID codes.
    //!
    //! Some fonts support both ASCII (0x00-0x7F) and multi-byte codes:
    //! - Codes 0x0000-0x007F: ASCII/Latin (single byte)
    //! - Codes 0x0100-0xFFFF: CJK characters (two bytes)

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
<0000> <FFFF>
endcodespacerange
1 beginbfrange
<0020> <007E> <0020>
endbfrange
50 beginbfchar
<0001> <4E00>
<0002> <4E01>
<0003> <4E02>
"#;

    let mut full_cmap = mixed_cmap.to_string();

    for i in 0x80_u32..0xAE {
        full_cmap.push_str(&format!("<{:04X}> <{:04X}>\n", i, 0x4E00 + i));
    }

    full_cmap.push_str(
        r#"
endbfchar
endcmap
CMapName currentdict /CMap defineresource pop
end
end
"#,
    );

    let font = FontInfo {
        base_font: "MixedCJK".to_string(),
        subtype: "Type0".to_string(),
        encoding: xberg_native_pdf::fonts::Encoding::Identity,
        to_unicode: Some(LazyCMap::new(full_cmap.as_bytes().to_vec())),
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

    assert_eq!(font.char_to_unicode(0x0020), Some(" ".to_string()), "Space");
    assert_eq!(font.char_to_unicode(0x0041), Some("A".to_string()), "ASCII letter");
    assert_eq!(font.char_to_unicode(0x007E), Some("~".to_string()), "Tilde");

    assert_eq!(font.char_to_unicode(0x0001), Some("\u{4E00}".to_string()), "CJK 1");
    assert_eq!(font.char_to_unicode(0x0002), Some("\u{4E01}".to_string()), "CJK 2");
}

#[test]
fn test_multibyte_cid_large_cid_values() {
    //! Test that large CID values in multi-byte systems work correctly.
    //!
    //! Some CJK fonts use high CID values:
    //! - Values > 0x8000 for extended character sets
    //! - Values near 0xFFFF for rare/variant characters

    let large_cid_cmap = r#"
/CIDInit /ProcSet findresource begin
12 dict begin
begincmap
/CIDSystemInfo
<< /Registry (Adobe)
/Ordering (GB1)
/Supplement 5
>> def
/CMapName /UniGB-UCS2-H def
/CMapType 2 def
1 begincodespacerange
<0000> <FFFF>
endcodespacerange
5 beginbfchar
<8000> <F900>
<9000> <FA00>
<A000> <FB00>
<FFFD> <FFFD>
<FFFE> <FFFE>
endbfchar
endcmap
CMapName currentdict /CMap defineresource pop
end
end
"#;

    let font = FontInfo {
        base_font: "GBLarge".to_string(),
        subtype: "Type0".to_string(),
        encoding: xberg_native_pdf::fonts::Encoding::Standard("UniGB-UCS2-H".to_string()),
        to_unicode: Some(LazyCMap::new(large_cid_cmap.as_bytes().to_vec())),
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
            ordering: "GB1".to_string(),
            supplement: 5,
        }),
        cid_font_type: None,
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

    assert_eq!(
        font.char_to_unicode(0x8000),
        Some("\u{F900}".to_string()),
        "Large CID 0x8000"
    );
    assert_eq!(
        font.char_to_unicode(0x9000),
        Some("\u{FA00}".to_string()),
        "Large CID 0x9000"
    );
    assert_eq!(
        font.char_to_unicode(0xA000),
        Some("\u{FB00}".to_string()),
        "Large CID 0xA000"
    );

    // Note: U+FFFD (replacement character) mappings are intentionally rejected per PDF Spec compliance.
    // CMaps with <FFFD> <FFFD> entries are treated as "unmapped" and fall through to Priority 2.
    // This is intentional behavior per ENDASH_ISSUE_ROOT_CAUSE.md to handle broken PDF tooling.
    // The 0xFFFE -> 0xFFFE mapping works fine though (not the replacement character) ~keep
    assert_eq!(
        font.char_to_unicode(0xFFFE),
        Some("\u{FFFE}".to_string()),
        "Noncharacter marker"
    );
}
