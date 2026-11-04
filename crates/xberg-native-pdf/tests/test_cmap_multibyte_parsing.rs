//! CMap multi-byte parsing correctness tests (§9.7.5 / §9.10.3).
//!
//! Focuses on three specific parsing correctness requirements:
//!
//! 1. **Array-form `beginbfrange`**: `<src_start> <src_end> [<dst1> <dst2> ...]`
//!    Each array entry maps `src_start + i → dst_i`.  Used for ligatures and
//!    irregular CJK sub-ranges.
//!
//! 2. **Multi-byte hex strings**: `<4E2D>` is a single 2-byte code 0x4E2D, not
//!    bytes 0x4E and 0x2D read separately.
//!
//! 3. **`begincodespacerange` drives byte-width**: When the codespace declares
//!    2-byte codes (`<0000> <FFFF>`), `LazyCMap::code_width()` must return 2 so
//!    that the text extractor switches from 1-byte to 2-byte character reading.

use xberg_native_pdf::fonts::cmap::{LazyCMap, parse_tounicode_cmap};

#[test]
fn test_bfrange_array_form_basic() {
    let data = b"beginbfrange\n<0041> <0043> [<FF21> <FF22> <FF23>]\nendbfrange";
    let cmap = parse_tounicode_cmap(data).unwrap();

    assert_eq!(
        cmap.get(&0x0041).as_deref(),
        Some("\u{FF21}"),
        "0x41 → Fullwidth A (U+FF21)"
    );
    assert_eq!(
        cmap.get(&0x0042).as_deref(),
        Some("\u{FF22}"),
        "0x42 → Fullwidth B (U+FF22)"
    );
    assert_eq!(
        cmap.get(&0x0043).as_deref(),
        Some("\u{FF23}"),
        "0x43 → Fullwidth C (U+FF23)"
    );
}

#[test]
fn test_bfrange_array_form_ligatures() {
    // PDF spec §9.10.3 example: <005F> <0061> [<00660066> <00660069> <00660066006C>]
    // Codes 0x5F→"ff", 0x60→"fi", 0x61→"ffl" ~keep
    let data = b"beginbfrange\n<005F> <0061> [<00660066> <00660069> <00660066006C>]\nendbfrange";
    let cmap = parse_tounicode_cmap(data).unwrap();

    assert_eq!(cmap.get(&0x5F).as_deref(), Some("ff"), "code 0x5F → \"ff\"");
    assert_eq!(cmap.get(&0x60).as_deref(), Some("fi"), "code 0x60 → \"fi\"");
    assert_eq!(cmap.get(&0x61).as_deref(), Some("ffl"), "code 0x61 → \"ffl\"");
}

#[test]
fn test_bfrange_array_form_cjk() {
    // 2-byte source codes with 2-byte destinations — typical CJK ToUnicode CMap snippet
    // beginbfrange <4E00> <4E02> [<4E00> <4E01> <4E02>] ~keep
    let data = b"beginbfrange\n<4E00> <4E02> [<4E00> <4E01> <4E02>]\nendbfrange";
    let cmap = parse_tounicode_cmap(data).unwrap();

    assert_eq!(cmap.get(&0x4E00).as_deref(), Some("\u{4E00}"), "一 identity");
    assert_eq!(cmap.get(&0x4E01).as_deref(), Some("\u{4E01}"), "丁 identity");
    assert_eq!(cmap.get(&0x4E02).as_deref(), Some("\u{4E02}"), "丂 identity");
}

#[test]
fn test_bfrange_linear_form_still_works() {
    let data = b"beginbfrange\n<0041> <0045> <0061>\nendbfrange";
    let cmap = parse_tounicode_cmap(data).unwrap();

    assert_eq!(cmap.get(&0x41).as_deref(), Some("a"));
    assert_eq!(cmap.get(&0x42).as_deref(), Some("b"));
    assert_eq!(cmap.get(&0x43).as_deref(), Some("c"));
    assert_eq!(cmap.get(&0x44).as_deref(), Some("d"));
    assert_eq!(cmap.get(&0x45).as_deref(), Some("e"));
}

#[test]
fn test_bfchar_two_byte_src_code() {
    let data = b"beginbfchar\n<4E2D> <4E2D>\nendbfchar";
    let cmap = parse_tounicode_cmap(data).unwrap();

    assert_eq!(
        cmap.get(&0x4E2D).as_deref(),
        Some("\u{4E2D}"),
        "code 0x4E2D → U+4E2D (中)"
    );
    assert!(
        cmap.get(&0x4E).is_none() || cmap.get(&0x4E).as_deref() != Some("\u{4E2D}"),
        "byte 0x4E must not produce 中"
    );
}

#[test]
fn test_bfchar_two_byte_src_hiragana() {
    let data = b"beginbfchar\n<3042> <3042>\nendbfchar";
    let cmap = parse_tounicode_cmap(data).unwrap();

    assert_eq!(
        cmap.get(&0x3042).as_deref(),
        Some("\u{3042}"),
        "code 0x3042 → U+3042 (あ)"
    );
}

#[test]
fn test_bfchar_two_byte_multiple_cjk() {
    let data = b"beginbfchar\n<4E2D> <4E2D>\n<6587> <6587>\n<5B66> <5B66>\nendbfchar";
    let cmap = parse_tounicode_cmap(data).unwrap();

    assert_eq!(cmap.get(&0x4E2D).as_deref(), Some("\u{4E2D}"), "中 (0x4E2D)");
    assert_eq!(cmap.get(&0x6587).as_deref(), Some("\u{6587}"), "文 (0x6587)");
    assert_eq!(cmap.get(&0x5B66).as_deref(), Some("\u{5B66}"), "学 (0x5B66)");
}

#[test]
fn test_codespacerange_two_byte_sets_code_width() {
    let data = b"/CIDInit /ProcSet findresource begin\nbegincmap\n\
        1 begincodespacerange\n<0000> <FFFF>\nendcodespacerange\n\
        1 beginbfchar\n<3042> <3042>\nendbfchar\nendcmap";

    let cmap = parse_tounicode_cmap(data).unwrap();
    assert_eq!(cmap.code_width, 2, "2-byte codespace must set code_width = 2");

    assert_eq!(cmap.get(&0x3042).as_deref(), Some("\u{3042}"), "あ lookup");
}

#[test]
fn test_codespacerange_one_byte_keeps_default() {
    let data = b"1 begincodespacerange\n<00> <FF>\nendcodespacerange\n\
        1 beginbfchar\n<41> <41>\nendbfchar";

    let cmap = parse_tounicode_cmap(data).unwrap();
    assert_eq!(cmap.code_width, 1, "1-byte codespace keeps code_width = 1");
    assert_eq!(cmap.get(&0x41).as_deref(), Some("A"), "A lookup");
}

#[test]
fn test_lazycmap_code_width_two_byte() {
    let cmap_data = b"1 begincodespacerange\n<0000> <FFFF>\nendcodespacerange\n\
        2 beginbfchar\n<4E2D> <4E2D>\n<6587> <6587>\nendbfchar"
        .to_vec();

    let lazy = LazyCMap::new(cmap_data);
    assert_eq!(lazy.code_width(), 2, "LazyCMap::code_width() should return 2");
}

#[test]
fn test_lazycmap_code_width_one_byte() {
    let cmap_data = b"1 begincodespacerange\n<00> <FF>\nendcodespacerange\n\
        1 beginbfchar\n<41> <41>\nendbfchar"
        .to_vec();

    let lazy = LazyCMap::new(cmap_data);
    assert_eq!(lazy.code_width(), 1, "LazyCMap::code_width() should return 1");
}

#[test]
fn test_lazycmap_code_width_default_when_no_codespace() {
    let cmap_data = b"1 beginbfchar\n<41> <41>\nendbfchar".to_vec();

    let lazy = LazyCMap::new(cmap_data);
    assert_eq!(lazy.code_width(), 1, "Missing codespace defaults code_width = 1");
}

#[test]
fn test_full_cjk_cmap_roundtrip() {
    // Simulates a realistic ToUnicode CMap for a CJK composite font ~keep
    let cmap_data = br#"
/CIDInit /ProcSet findresource begin
12 dict begin
begincmap
/CIDSystemInfo
<< /Registry (Adobe)
/Ordering (UCS)
/Supplement 0
>> def
/CMapName /Adobe-Identity-UCS def
/CMapType 2 def
1 begincodespacerange
<0000> <FFFF>
endcodespacerange
3 beginbfchar
<4E2D> <4E2D>
<6587> <6587>
<3042> <3042>
endbfchar
1 beginbfrange
<4E00> <4E05> <4E00>
endbfrange
endcmap
CMapName currentdict /CMap defineresource pop
end
end
"#;

    let lazy = LazyCMap::new(cmap_data.to_vec());

    assert_eq!(lazy.code_width(), 2, "full CJK CMap code_width = 2");

    let cmap = lazy.get().expect("CMap must parse");

    assert_eq!(cmap.get(&0x4E2D).as_deref(), Some("\u{4E2D}"), "中");
    assert_eq!(cmap.get(&0x6587).as_deref(), Some("\u{6587}"), "文");
    assert_eq!(cmap.get(&0x3042).as_deref(), Some("\u{3042}"), "あ");

    assert_eq!(cmap.get(&0x4E00).as_deref(), Some("\u{4E00}"), "一");
    assert_eq!(cmap.get(&0x4E03).as_deref(), Some("\u{4E03}"), "七");
    assert_eq!(cmap.get(&0x4E05).as_deref(), Some("\u{4E05}"), "丅");
}

#[test]
fn multiline_array_maps_positions_without_treating_destinations_as_ranges() {
    let data = b"beginbfrange\n<68><6B>[ % a closing ] in this comment is ignored\r\n\
                 <0068><0069><006A>\r\n<006B>]<20><20><0020>\nendbfrange\n\
                 beginbfchar\n<6B><0078>\nendbfchar";
    let cmap = parse_tounicode_cmap(data).unwrap();

    for (code, expected) in [(0x68, "h"), (0x69, "i"), (0x6A, "j"), (0x6B, "x"), (0x20, " ")] {
        assert_eq!(cmap.get(&code).as_deref(), Some(expected), "code {code:02X}");
    }
    assert_eq!(cmap.get(&0x6C), None);
}

#[test]
fn array_opener_can_follow_source_codes_on_a_later_line() {
    let data = b"beginbfrange\r<40><42> % range header\r% comment before array\r\
                 [<0041><0042><0043>]\rendbfrange";
    let cmap = parse_tounicode_cmap(data).unwrap();
    assert_eq!(cmap.get(&0x40).as_deref(), Some("A"));
    assert_eq!(cmap.get(&0x41).as_deref(), Some("B"));
    assert_eq!(cmap.get(&0x42).as_deref(), Some("C"));
    assert_eq!(cmap.get(&0x43), None);
}

#[test]
fn unterminated_array_never_maps_its_continuations_as_sequential_ranges() {
    let data = b"beginbfrange\n<10><13>[\n<0068><0069><006A>\nendbfrange\n\
                 beginbfchar\n<50><0050>\nendbfchar";
    let cmap = parse_tounicode_cmap(data).unwrap();

    for code in [0x10, 0x11, 0x12, 0x13, 0x68, 0x69, 0x6A] {
        assert_eq!(cmap.get(&code), None, "code {code:02X}");
    }
    assert_eq!(cmap.get(&0x50).as_deref(), Some("P"));
}

#[test]
fn malformed_array_never_maps_its_continuations_as_sequential_ranges() {
    let data = b"beginbfrange\n<10><13>[\n<0068><0069><006A> invalid ]\n<50><50><0050>\nendbfrange";
    let cmap = parse_tounicode_cmap(data).unwrap();

    for code in [0x10, 0x11, 0x12, 0x13, 0x68, 0x69, 0x6A] {
        assert_eq!(cmap.get(&code), None, "code {code:02X}");
    }
    assert_eq!(cmap.get(&0x50).as_deref(), Some("P"));
}

#[test]
fn short_multiline_array_preserves_only_its_available_mappings() {
    let data = b"beginbfrange\n<10><13>[\n<0041><0042>\n]\nendbfrange";
    let cmap = parse_tounicode_cmap(data).unwrap();

    assert_eq!(cmap.get(&0x10).as_deref(), Some("A"));
    assert_eq!(cmap.get(&0x11).as_deref(), Some("B"));
    assert_eq!(cmap.get(&0x12), None);
    assert_eq!(cmap.get(&0x13), None);
}

#[test]
fn compact_range_before_array_does_not_hide_the_later_mapping() {
    let data = b"beginbfrange\n<10><10><0041><10><11>[<0042><0043>]\nendbfrange";
    let cmap = parse_tounicode_cmap(data).unwrap();

    assert_eq!(cmap.get(&0x10).as_deref(), Some("B"));
    assert_eq!(cmap.get(&0x11).as_deref(), Some("C"));
}

#[test]
fn malformed_hex_does_not_shift_later_multiline_array_entries() {
    let data = b"beginbfrange\n<10><12>[\n<0041><GG><0043>\n]\nendbfrange";
    let cmap = parse_tounicode_cmap(data).unwrap();

    assert_eq!(cmap.get(&0x10).as_deref(), Some("A"));
    assert_eq!(cmap.get(&0x11), None);
    assert_eq!(cmap.get(&0x12).as_deref(), Some("C"));
}

#[test]
fn nested_invalid_arrays_do_not_create_unrelated_mappings() {
    let data = b"beginbfrange\n<10><11>[\n<20><21>[<0041><0042>]\n]\n<50><50><0050>\nendbfrange";
    let cmap = parse_tounicode_cmap(data).unwrap();

    for code in [0x10, 0x11, 0x20, 0x21] {
        assert_eq!(cmap.get(&code), None, "code {code:02X}");
    }
    assert_eq!(cmap.get(&0x50).as_deref(), Some("P"));
}

#[test]
fn sequential_mapping_before_multiline_array_keeps_its_distinct_source() {
    let data = b"beginbfrange\n<20><20><0020> <10><11>[\n<0041><0042>]\nendbfrange";
    let cmap = parse_tounicode_cmap(data).unwrap();

    assert_eq!(cmap.get(&0x20).as_deref(), Some(" "));
    assert_eq!(cmap.get(&0x10).as_deref(), Some("A"));
    assert_eq!(cmap.get(&0x11).as_deref(), Some("B"));
}

#[test]
fn comment_with_bf_markers_does_not_change_section_order_or_close_an_array() {
    let data = b"% beginbfchar <20><005A> endbfchar\r\n\
                 beginbfchar\n<10><0058>\nendbfchar\n\
                 beginbfrange\n<10><11>[\n<0041> % endbfrange\r\n<0042>]endbfrange\n\
                 beginbfchar\n<11><0043> % endbfchar <21><005A>\nendbfchar";
    let cmap = parse_tounicode_cmap(data).unwrap();

    assert_eq!(cmap.get(&0x10).as_deref(), Some("A"));
    assert_eq!(cmap.get(&0x11).as_deref(), Some("C"));
    assert_eq!(cmap.get(&0x20), None);
    assert_eq!(cmap.get(&0x21), None);
}

#[test]
fn bfchar_after_comment_on_cr_only_line_keeps_its_mapping() {
    let data = b"beginbfchar\r<10><0041> % comment\r<11><0042>\rendbfchar";
    let cmap = parse_tounicode_cmap(data).unwrap();

    assert_eq!(cmap.get(&0x10).as_deref(), Some("A"));
    assert_eq!(cmap.get(&0x11).as_deref(), Some("B"));
}

#[test]
fn bfchar_preserves_legacy_whitespace_between_hex_operands_across_bare_cr() {
    for comment in ["", " % comment between source and destination"] {
        let data = format!("beginbfchar\r<10>{comment}\r<0041>\r<11><0042>\rendbfchar");
        let cmap = parse_tounicode_cmap(data.as_bytes()).unwrap();

        assert_eq!(cmap.get(&0x10).as_deref(), Some("A"));
        assert_eq!(cmap.get(&0x11).as_deref(), Some("B"));
    }
}
