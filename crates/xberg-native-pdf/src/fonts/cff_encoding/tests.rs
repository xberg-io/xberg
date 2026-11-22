//! Unit tests for [`super`].
//!
//! Split out of `cff_encoding.rs` purely for file size, mirroring the split used for
//! `document.rs`/`document/tests.rs` and `extractors/text/mod.rs`/`extractors/text/tests.rs`. ~keep

use super::*;

#[test]
fn test_sid_to_name_notdef() {
    assert_eq!(sid_to_name(0), Some(".notdef"));
}

#[test]
fn test_sid_to_name_space() {
    assert_eq!(sid_to_name(1), Some("space"));
}

#[test]
fn test_sid_to_name_letters() {
    assert_eq!(sid_to_name(34), Some("A"));
    assert_eq!(sid_to_name(59), Some("Z"));
    assert_eq!(sid_to_name(66), Some("a"));
    assert_eq!(sid_to_name(91), Some("z"));
}

#[test]
fn test_sid_to_name_digits() {
    assert_eq!(sid_to_name(17), Some("zero"));
    assert_eq!(sid_to_name(26), Some("nine"));
}

#[test]
fn test_sid_to_name_punctuation() {
    assert_eq!(sid_to_name(2), Some("exclam"));
    assert_eq!(sid_to_name(15), Some("period"));
    assert_eq!(sid_to_name(13), Some("comma"));
}

#[test]
fn test_sid_to_name_ligatures() {
    assert_eq!(sid_to_name(109), Some("fi"));
    assert_eq!(sid_to_name(110), Some("fl"));
    assert_eq!(sid_to_name(266), Some("ff"));
    assert_eq!(sid_to_name(267), Some("ffi"));
    assert_eq!(sid_to_name(268), Some("ffl"));
}

#[test]
fn test_sid_to_name_accented() {
    assert_eq!(sid_to_name(171), Some("Aacute"));
    assert_eq!(sid_to_name(200), Some("aacute"));
    assert_eq!(sid_to_name(227), Some("ydieresis"));
}

#[test]
fn test_sid_to_name_last_entries() {
    assert_eq!(sid_to_name(388), Some("Regular"));
    assert_eq!(sid_to_name(389), Some("Roman"));
    assert_eq!(sid_to_name(390), Some("Semibold"));
}

#[test]
fn test_sid_to_name_out_of_range() {
    assert_eq!(sid_to_name(391), None);
    assert_eq!(sid_to_name(500), None);
    assert_eq!(sid_to_name(u16::MAX), None);
}

#[test]
fn test_parse_index_too_short() {
    assert_eq!(parse_index(&[0x00], 0), None);
}

#[test]
fn test_parse_index_empty() {
    let data = [0x00, 0x00];
    let result = parse_index(&data, 0);
    assert!(result.is_some());
    let (entries, next) = result.unwrap();
    assert!(entries.is_empty());
    assert_eq!(next, 2);
}

#[test]
fn test_parse_index_single_entry() {
    let data = vec![0x00, 0x01, 0x01, 0x01, 0x04, b'A', b'B', b'C'];
    let result = parse_index(&data, 0);
    assert!(result.is_some());
    let (entries, _next) = result.unwrap();
    assert_eq!(entries.len(), 1);
    assert_eq!(entries[0], b"ABC");
}

#[test]
fn test_parse_index_multiple_entries() {
    let data = vec![0x00, 0x02, 0x01, 0x01, 0x03, 0x05, b'H', b'i', b'O', b'K'];
    let result = parse_index(&data, 0);
    assert!(result.is_some());
    let (entries, _next) = result.unwrap();
    assert_eq!(entries.len(), 2);
    assert_eq!(entries[0], b"Hi");
    assert_eq!(entries[1], b"OK");
}

#[test]
fn test_parse_index_invalid_off_size_zero() {
    let data = vec![0x00, 0x01, 0x00];
    assert_eq!(parse_index(&data, 0), None);
}

#[test]
fn test_parse_index_invalid_off_size_too_large() {
    let data = vec![0x00, 0x01, 0x05];
    assert_eq!(parse_index(&data, 0), None);
}

#[test]
fn test_parse_index_truncated_offset_array() {
    let data = vec![0x00, 0x01, 0x01, 0x01];
    assert_eq!(parse_index(&data, 0), None);
}

#[test]
fn test_parse_index_with_offset() {
    let data = vec![0xFF, 0xFF, 0xFF, 0x00, 0x01, 0x01, 0x01, 0x02, b'X'];
    let result = parse_index(&data, 3);
    assert!(result.is_some());
    let (entries, _) = result.unwrap();
    assert_eq!(entries.len(), 1);
    assert_eq!(entries[0], b"X");
}

#[test]
fn test_parse_index_off_size_2() {
    let data = vec![0x00, 0x01, 0x02, 0x00, 0x01, 0x00, 0x03, b'A', b'B'];
    let result = parse_index(&data, 0);
    assert!(result.is_some());
    let (entries, _) = result.unwrap();
    assert_eq!(entries.len(), 1);
    assert_eq!(entries[0], b"AB");
}

#[test]
fn test_parse_index_data_out_of_bounds() {
    let data = vec![0x00, 0x01, 0x01, 0x01, 0xFF];
    assert_eq!(parse_index(&data, 0), None);
}

#[test]
fn test_parse_dict_operand_empty() {
    assert_eq!(parse_dict_operand(&[], 0), None);
}

#[test]
fn test_parse_dict_operand_single_byte_zero() {
    assert_eq!(parse_dict_operand(&[139], 0), Some((0, 1)));
}

#[test]
fn test_parse_dict_operand_single_byte_positive() {
    assert_eq!(parse_dict_operand(&[246], 0), Some((107, 1)));
}

#[test]
fn test_parse_dict_operand_single_byte_negative() {
    assert_eq!(parse_dict_operand(&[32], 0), Some((-107, 1)));
}

#[test]
fn test_parse_dict_operand_two_byte_positive() {
    assert_eq!(parse_dict_operand(&[247, 0], 0), Some((108, 2)));
    assert_eq!(parse_dict_operand(&[250, 255], 0), Some((1131, 2)));
}

#[test]
fn test_parse_dict_operand_two_byte_negative() {
    assert_eq!(parse_dict_operand(&[251, 0], 0), Some((-108, 2)));
    assert_eq!(parse_dict_operand(&[254, 255], 0), Some((-1131, 2)));
}

#[test]
fn test_parse_dict_operand_two_byte_truncated() {
    assert_eq!(parse_dict_operand(&[247], 0), None);
    assert_eq!(parse_dict_operand(&[251], 0), None);
}

#[test]
fn test_parse_dict_operand_three_byte_int16() {
    assert_eq!(parse_dict_operand(&[28, 0x00, 0x01], 0), Some((1, 3)));
    assert_eq!(parse_dict_operand(&[28, 0xFF, 0xFF], 0), Some((-1, 3)));
    assert_eq!(parse_dict_operand(&[28, 0x7F, 0xFF], 0), Some((32767, 3)));
}

#[test]
fn test_parse_dict_operand_three_byte_truncated() {
    assert_eq!(parse_dict_operand(&[28, 0x00], 0), None);
}

#[test]
fn test_parse_dict_operand_five_byte_int32() {
    assert_eq!(parse_dict_operand(&[29, 0x00, 0x00, 0x00, 0x01], 0), Some((1, 5)));
    assert_eq!(parse_dict_operand(&[29, 0xFF, 0xFF, 0xFF, 0xFF], 0), Some((-1, 5)));
}

#[test]
fn test_parse_dict_operand_five_byte_truncated() {
    assert_eq!(parse_dict_operand(&[29, 0x00, 0x00, 0x00], 0), None);
}

#[test]
fn test_parse_dict_operand_real_number() {
    let data = [30, 0x1A, 0x5F];
    let result = parse_dict_operand(&data, 0);
    assert!(result.is_some());
    let (val, consumed) = result.unwrap();
    assert_eq!(val, 0);
    assert_eq!(consumed, 3);
}

#[test]
fn test_parse_dict_operand_real_nibble1_end() {
    let data = [30, 0xF0];
    let result = parse_dict_operand(&data, 0);
    assert!(result.is_some());
    assert_eq!(result.unwrap(), (0, 2));
}

#[test]
fn test_parse_dict_operand_real_unterminated() {
    let data = [30, 0x12, 0x34];
    assert_eq!(parse_dict_operand(&data, 0), None);
}

#[test]
fn test_parse_dict_operand_unknown_byte() {
    assert_eq!(parse_dict_operand(&[0], 0), None);
    assert_eq!(parse_dict_operand(&[21], 0), None);
    assert_eq!(parse_dict_operand(&[255], 0), None);
}

#[test]
fn test_parse_dict_operand_with_offset() {
    let data = [0x00, 0x00, 139];
    assert_eq!(parse_dict_operand(&data, 2), Some((0, 1)));
}

#[test]
fn test_parse_top_dict_empty() {
    let (enc, charset) = parse_top_dict(&[]);
    assert_eq!(enc, 0);
    assert_eq!(charset, 0);
}

#[test]
fn test_parse_top_dict_encoding_offset() {
    let data = [181, 16];
    let (enc, charset) = parse_top_dict(&data);
    assert_eq!(enc, 42);
    assert_eq!(charset, 0);
}

#[test]
fn test_parse_top_dict_charset_offset() {
    let data = [238, 15];
    let (enc, charset) = parse_top_dict(&data);
    assert_eq!(enc, 0);
    assert_eq!(charset, 99);
}

#[test]
fn test_parse_top_dict_both_offsets() {
    let data = [189, 16, 239, 15];
    let (enc, charset) = parse_top_dict(&data);
    assert_eq!(enc, 50);
    assert_eq!(charset, 100);
}

#[test]
fn test_parse_top_dict_two_byte_operator() {
    let data = [139, 12, 0];
    let (enc, charset) = parse_top_dict(&data);
    assert_eq!(enc, 0);
    assert_eq!(charset, 0);
}

#[test]
fn test_parse_top_dict_unknown_operator() {
    let data = [181, 17];
    let (enc, charset) = parse_top_dict(&data);
    assert_eq!(enc, 0);
    assert_eq!(charset, 0);
}

#[test]
fn test_parse_top_dict_skip_unparseable() {
    let data = [255, 181, 16];
    let (enc, charset) = parse_top_dict(&data);
    assert_eq!(enc, 42);
    assert_eq!(charset, 0);
}

#[test]
fn test_parse_charset_out_of_bounds() {
    assert_eq!(parse_charset(&[0x00], 5, 10), None);
}

#[test]
fn test_parse_charset_format0() {
    let data = vec![0x00, 0x00, 0x01, 0x00, 0x22, 0x00, 0x42];
    let result = parse_charset(&data, 0, 4);
    assert!(result.is_some());
    let sids = result.unwrap();
    assert_eq!(sids.len(), 4);
    assert_eq!(sids[0], 0);
    assert_eq!(sids[1], 1);
    assert_eq!(sids[2], 34);
    assert_eq!(sids[3], 66);
}

#[test]
fn test_parse_charset_format1() {
    let data = vec![0x01, 0x00, 0x22, 0x02];
    let result = parse_charset(&data, 0, 4);
    assert!(result.is_some());
    let sids = result.unwrap();
    assert_eq!(sids.len(), 4);
    assert_eq!(sids[0], 0);
    assert_eq!(sids[1], 34);
    assert_eq!(sids[2], 35);
    assert_eq!(sids[3], 36);
}

#[test]
fn test_parse_charset_format2() {
    let data = vec![0x02, 0x00, 0x42, 0x00, 0x03];
    let result = parse_charset(&data, 0, 5);
    assert!(result.is_some());
    let sids = result.unwrap();
    assert_eq!(sids.len(), 5);
    assert_eq!(sids[0], 0);
    assert_eq!(sids[1], 66);
    assert_eq!(sids[2], 67);
    assert_eq!(sids[3], 68);
    assert_eq!(sids[4], 69);
}

#[test]
fn test_parse_charset_unknown_format() {
    let data = vec![0x03];
    assert_eq!(parse_charset(&data, 0, 2), None);
}

#[test]
fn test_parse_charset_format0_truncated() {
    let data = vec![0x00, 0x00];
    let result = parse_charset(&data, 0, 3);
    assert!(result.is_some());
    let sids = result.unwrap();
    assert_eq!(sids[0], 0);
}

#[test]
fn test_parse_charset_format1_limits_to_num_glyphs() {
    let data = vec![0x01, 0x00, 0x01, 0xFF];
    let result = parse_charset(&data, 0, 3);
    assert!(result.is_some());
    let sids = result.unwrap();
    assert_eq!(sids.len(), 3);
}

#[test]
fn test_parse_encoding_table_out_of_bounds() {
    assert_eq!(parse_encoding_table(&[0x00], 5), None);
}

#[test]
fn test_parse_encoding_table_format0() {
    let data = vec![0x00, 0x03, 0x41, 0x42, 0x43];
    let result = parse_encoding_table(&data, 0);
    assert!(result.is_some());
    let map = result.unwrap();
    assert_eq!(map.get(&0x41), Some(&1));
    assert_eq!(map.get(&0x42), Some(&2));
    assert_eq!(map.get(&0x43), Some(&3));
}

#[test]
fn test_parse_encoding_table_format1() {
    let data = vec![0x01, 0x01, 0x41, 0x02];
    let result = parse_encoding_table(&data, 0);
    assert!(result.is_some());
    let map = result.unwrap();
    assert_eq!(map.get(&0x41), Some(&1));
    assert_eq!(map.get(&0x42), Some(&2));
    assert_eq!(map.get(&0x43), Some(&3));
}

#[test]
fn test_parse_encoding_table_unknown_format() {
    let data = vec![0x02];
    assert_eq!(parse_encoding_table(&data, 0), None);
}

#[test]
fn test_parse_encoding_table_format0_truncated() {
    let data = vec![0x00, 0x05, 0x41, 0x42];
    let result = parse_encoding_table(&data, 0);
    assert!(result.is_some());
    let map = result.unwrap();
    assert_eq!(map.len(), 2);
}

#[test]
fn test_parse_encoding_table_with_supplement() {
    let data = vec![0x80, 0x01, 0x41, 0x01, 0x42, 0x00, 0x22];
    let result = parse_encoding_table(&data, 0);
    assert!(result.is_some());
    let map = result.unwrap();
    assert_eq!(map.get(&0x41), Some(&1));
    assert_eq!(map.get(&0x42), Some(&34));
}

#[test]
fn test_parse_encoding_table_format1_truncated() {
    let data = vec![0x01, 0x02, 0x41, 0x01];
    let result = parse_encoding_table(&data, 0);
    assert!(result.is_some());
    let map = result.unwrap();
    assert_eq!(map.len(), 2);
}

#[test]
fn test_parse_encoding_table_format0_empty_pos() {
    let data = vec![0x00];
    assert_eq!(parse_encoding_table(&data, 0), None);
}

#[test]
fn test_parse_encoding_table_format1_empty_pos() {
    let data = vec![0x01];
    assert_eq!(parse_encoding_table(&data, 0), None);
}

#[test]
fn test_resolve_glyph_name_predefined() {
    let string_index: Vec<&[u8]> = vec![];
    assert_eq!(resolve_glyph_name(0, &string_index), Some(".notdef".to_string()));
    assert_eq!(resolve_glyph_name(1, &string_index), Some("space".to_string()));
    assert_eq!(resolve_glyph_name(34, &string_index), Some("A".to_string()));
    assert_eq!(resolve_glyph_name(390, &string_index), Some("Semibold".to_string()));
}

#[test]
fn test_resolve_glyph_name_custom_string() {
    let custom: Vec<&[u8]> = vec![b"MyGlyph", b"AnotherGlyph"];
    assert_eq!(resolve_glyph_name(391, &custom), Some("MyGlyph".to_string()));
    assert_eq!(resolve_glyph_name(392, &custom), Some("AnotherGlyph".to_string()));
}

#[test]
fn test_resolve_glyph_name_custom_out_of_range() {
    let custom: Vec<&[u8]> = vec![b"OnlyOne"];
    assert_eq!(resolve_glyph_name(393, &custom), None);
}

#[test]
fn test_resolve_glyph_name_custom_invalid_utf8() {
    let invalid_utf8: Vec<&[u8]> = vec![&[0xFF, 0xFE]];
    assert_eq!(resolve_glyph_name(391, &invalid_utf8), None);
}

#[test]
fn test_extract_cff_from_opentype_too_short() {
    assert_eq!(extract_cff_from_opentype(&[0; 8]), None);
}

#[test]
fn test_extract_cff_from_opentype_not_opentype() {
    let data = vec![0x00; 16];
    assert_eq!(extract_cff_from_opentype(&data), None);
}

#[test]
fn test_extract_cff_from_opentype_otto_no_cff_table() {
    let data = vec![0x4F, 0x54, 0x54, 0x4F, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00];
    assert_eq!(extract_cff_from_opentype(&data), None);
}

#[test]
fn test_extract_cff_from_opentype_with_cff_table() {
    let cff_data = b"\x01\x00\x04\x01";
    let cff_offset: u32 = 28;
    let cff_length: u32 = cff_data.len() as u32;

    let mut data = vec![0x4F, 0x54, 0x54, 0x4F, 0x00, 0x01, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00];
    data.extend_from_slice(b"CFF ");
    data.extend_from_slice(&[0, 0, 0, 0]);
    data.extend_from_slice(&cff_offset.to_be_bytes());
    data.extend_from_slice(&cff_length.to_be_bytes());
    data.extend_from_slice(cff_data);

    let result = extract_cff_from_opentype(&data);
    assert!(result.is_some());
    assert_eq!(result.unwrap(), cff_data);
}

#[test]
fn test_extract_cff_from_opentype_truncated_table_dir() {
    let data = vec![
        0x4F, 0x54, 0x54, 0x4F, 0x00, 0x01, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, b'C', b'F',
    ];
    assert_eq!(extract_cff_from_opentype(&data), None);
}

#[test]
fn test_parse_cff_encoding_too_short() {
    assert_eq!(parse_cff_encoding(&[0, 1, 2]), None);
}

#[test]
fn test_parse_cff_encoding_wrong_version() {
    let data = vec![0x02, 0x00, 0x04, 0x01, 0x00];
    assert_eq!(parse_cff_encoding(&data), None);
}

#[test]
fn test_parse_cff_encoding_version1_too_short_after_check() {
    let data = vec![0x01, 0x00, 0x04];
    assert_eq!(parse_cff_encoding(&data), None);
}

#[test]
fn test_parse_cff_encoding_expert_encoding() {
    let data = build_minimal_cff(1, 0);
    let result = parse_cff_encoding(&data);
    assert_eq!(result, None);
}

#[test]
fn test_parse_cff_encoding_standard_encoding_default_charset() {
    let data = build_minimal_cff(0, 0);
    let result = parse_cff_encoding(&data);
    assert_eq!(result, None);
}

/// Helper: builds a minimal CFF font binary with specified encoding and charset offsets.
fn build_minimal_cff(encoding_offset: i32, charset_offset: i32) -> Vec<u8> {
    let mut data = vec![1, 0, 4, 1];

    append_index(&mut data, &[b"Test"]);

    let top_dict = build_top_dict(encoding_offset, charset_offset);
    append_index(&mut data, &[&top_dict]);

    append_index(&mut data, &[]);

    append_index(&mut data, &[]);

    data
}

/// Encode a CFF DICT with encoding (op 16) and charset (op 15) operands.
fn build_top_dict(encoding_offset: i32, charset_offset: i32) -> Vec<u8> {
    let mut dict = Vec::new();
    encode_dict_int(&mut dict, encoding_offset);
    dict.push(16);
    encode_dict_int(&mut dict, charset_offset);
    dict.push(15);
    dict
}

/// Encode a CFF integer operand into DICT format.
fn encode_dict_int(out: &mut Vec<u8>, val: i32) {
    if (-107..=107).contains(&val) {
        out.push((val + 139) as u8);
    } else if (108..=1131).contains(&val) {
        let v = val - 108;
        out.push((v / 256 + 247) as u8);
        out.push((v % 256) as u8);
    } else if (-1131..=-108).contains(&val) {
        let v = -val - 108;
        out.push((v / 256 + 251) as u8);
        out.push((v % 256) as u8);
    } else if (-32768..=32767).contains(&val) {
        out.push(28);
        let bytes = (val as i16).to_be_bytes();
        out.push(bytes[0]);
        out.push(bytes[1]);
    } else {
        out.push(29);
        let bytes = val.to_be_bytes();
        out.extend_from_slice(&bytes);
    }
}

/// Append a CFF INDEX to a data vector.
fn append_index(data: &mut Vec<u8>, entries: &[&[u8]]) {
    let count = entries.len() as u16;
    data.extend_from_slice(&count.to_be_bytes());
    if count == 0 {
        return;
    }
    data.push(1);

    let mut offset: u8 = 1;
    data.push(offset);
    for entry in entries {
        offset += entry.len() as u8;
        data.push(offset);
    }
    for entry in entries {
        data.extend_from_slice(entry);
    }
}

#[test]
fn test_glyph_name_to_sid_known_names() {
    assert_eq!(glyph_name_to_sid(".notdef"), Some(0));
    assert_eq!(glyph_name_to_sid("space"), Some(1));
    assert_eq!(glyph_name_to_sid("A"), Some(34));
    assert_eq!(glyph_name_to_sid("B"), Some(35));
    assert_eq!(glyph_name_to_sid("Z"), Some(59));
    assert_eq!(glyph_name_to_sid("a"), Some(66));
    assert_eq!(glyph_name_to_sid("z"), Some(91));
    assert_eq!(glyph_name_to_sid("zero"), Some(17));
    assert_eq!(glyph_name_to_sid("nine"), Some(26));
}

#[test]
fn test_glyph_name_to_sid_unknown() {
    assert_eq!(glyph_name_to_sid("nonexistent_glyph_xyz"), None);
    assert_eq!(glyph_name_to_sid(""), None);
}

#[test]
fn test_glyph_name_to_sid_roundtrip() {
    for sid in 0u16..391 {
        if let Some(name) = sid_to_name(sid) {
            assert_eq!(
                glyph_name_to_sid(name),
                Some(sid),
                "Roundtrip failed for SID {} (name '{}')",
                sid,
                name
            );
        }
    }
}

#[test]
fn test_parse_cff_gid_mapping_invalid_data() {
    assert!(parse_cff_gid_mapping(&[]).is_none());
    assert!(parse_cff_gid_mapping(&[0, 1, 2]).is_none());
    assert!(parse_cff_gid_mapping(&[2, 0, 4, 2]).is_none());
}

// ==========================================
// resolve_bytes_via_pdf_encoding tests
//
// These exercise the name-resolution layer in isolation — the layer
// that was missing in `parse_cff_gid_mapping` and is the substantive
// fix here. The CFF binary parser is reused unchanged, so its tests
// remain authoritative for the parsing path.
// ========================================== ~keep

use crate::fonts::font_dict::Encoding;

/// Pin a real-world sparse-CFF subset pattern: charset enumerates
/// space, A, B, C, O, V, N (SIDs 1, 34, 35, 36, 48, 55, 47) on GIDs
/// 1..=7. PDF /Encoding is WinAnsiEncoding. The resolver must produce
/// GIDs for every charset entry — not just for byte 0x41 ("A") as the
/// sparse CFF Encoding table would have implied.
#[test]
fn resolve_via_pdf_encoding_recovers_all_charset_glyphs() {
    let charset = [0u16, 1, 34, 35, 36, 48, 55, 47];
    let string_index: Vec<&[u8]> = Vec::new();
    let pdf_enc = Encoding::Standard("WinAnsiEncoding".to_string());
    let differences: HashMap<u8, String> = HashMap::new();

    let map = resolve_bytes_via_pdf_encoding(&charset, &string_index, &pdf_enc, &differences);

    assert_eq!(map.get(&0x20), Some(&1), "0x20 (space) → GID 1");
    assert_eq!(map.get(&0x41), Some(&2), "0x41 (A) → GID 2");
    assert_eq!(map.get(&0x42), Some(&3), "0x42 (B) → GID 3");
    assert_eq!(map.get(&0x43), Some(&4), "0x43 (C) → GID 4");
    assert_eq!(map.get(&0x4f), Some(&5), "0x4f (O) → GID 5");
    assert_eq!(map.get(&0x56), Some(&6), "0x56 (V) → GID 6");
    assert_eq!(map.get(&0x4e), Some(&7), "0x4e (N) → GID 7");

    assert!(!map.contains_key(&0x7e), "0x7e (asciitilde) not in charset");
}

/// /Differences entries override the base predefined encoding.
#[test]
fn resolve_via_pdf_encoding_honors_differences_array() {
    let charset = [0u16, 116];
    let string_index: Vec<&[u8]> = Vec::new();
    let pdf_enc = Encoding::Standard("WinAnsiEncoding".to_string());
    let mut differences = HashMap::new();
    differences.insert(0x95u8, "bullet".to_string());

    let map = resolve_bytes_via_pdf_encoding(&charset, &string_index, &pdf_enc, &differences);
    assert_eq!(map.get(&0x95), Some(&1));
}

/// Identity encoding short-circuits via the outer function; the
/// helper itself is a no-op for Identity.
#[test]
fn resolve_via_pdf_encoding_skips_identity() {
    let charset = [0u16, 34];
    let string_index: Vec<&[u8]> = Vec::new();
    let pdf_enc = Encoding::Identity;
    let differences: HashMap<u8, String> = HashMap::new();
    let map = resolve_bytes_via_pdf_encoding(&charset, &string_index, &pdf_enc, &differences);
    assert!(map.is_empty(), "Identity → no base byte→name resolution");
}

/// Custom-string SIDs (>=391) resolved through the String INDEX
/// land in the name→GID map.
#[test]
fn resolve_via_pdf_encoding_resolves_custom_string_sids() {
    let charset = [0u16, 391];
    let custom: &[u8] = b"customGlyph";
    let string_index: Vec<&[u8]> = vec![custom];
    let pdf_enc = Encoding::Standard("WinAnsiEncoding".to_string());
    let mut differences = HashMap::new();
    differences.insert(0x21u8, "customGlyph".to_string());

    let map = resolve_bytes_via_pdf_encoding(&charset, &string_index, &pdf_enc, &differences);
    assert_eq!(map.get(&0x21), Some(&1));
}

// ==========================================
// Base-encoding selection (§9.6.6.1 / Annex D)
//
// /BaseEncoding picks which byte→glyph-name table resolves high bytes
// 0x80-0xFF. The three predefined bases diverge:
//   - WinAnsi 0x80   = "euro"      (Windows-1252)
//   - MacRoman 0x80  = "Adieresis" (Annex D Table D.2)
//   - Standard 0xA4  = "fraction"  (Annex D Table D.1; WinAnsi = "currency")
// Resolving the wrong table for a non-WinAnsi base would mis-route bytes.
// ========================================== ~keep

/// MacRomanEncoding: byte 0x80 must resolve via "Adieresis", not "euro".
#[test]
fn resolve_via_pdf_encoding_uses_mac_roman_table_for_mac_base() {
    let charset: Vec<u16> = vec![0, 173, 391];
    let string_index: Vec<&[u8]> = vec![b"euro"];
    let differences: HashMap<u8, String> = HashMap::new();
    let pdf_enc = Encoding::Standard("MacRomanEncoding".to_string());

    let map = resolve_bytes_via_pdf_encoding(&charset, &string_index, &pdf_enc, &differences);
    assert_eq!(map.get(&0x80), Some(&1), "MacRoman 0x80 → Adieresis (GID 1)");
}

/// StandardEncoding: byte 0xA4 must resolve via "fraction", not "currency".
#[test]
fn resolve_via_pdf_encoding_uses_standard_encoding_table_for_standard_base() {
    let charset: Vec<u16> = vec![0, 99, 103];
    let string_index: Vec<&[u8]> = vec![];
    let differences: HashMap<u8, String> = HashMap::new();
    let pdf_enc = Encoding::Standard("StandardEncoding".to_string());

    let map = resolve_bytes_via_pdf_encoding(&charset, &string_index, &pdf_enc, &differences);
    assert_eq!(map.get(&0xA4), Some(&1), "StandardEncoding 0xA4 → fraction (GID 1)");
}

/// WinAnsiEncoding regression guard: byte 0x80 still resolves to "euro".
#[test]
fn resolve_via_pdf_encoding_uses_winansi_table_for_winansi_base() {
    let charset: Vec<u16> = vec![0, 173, 391];
    let string_index: Vec<&[u8]> = vec![b"euro"];
    let differences: HashMap<u8, String> = HashMap::new();
    let pdf_enc = Encoding::Standard("WinAnsiEncoding".to_string());

    let map = resolve_bytes_via_pdf_encoding(&charset, &string_index, &pdf_enc, &differences);
    assert_eq!(map.get(&0x80), Some(&2), "WinAnsi 0x80 → euro (GID 2)");
}

/// `parse_charset` already handles arbitrary nGlyphs; the cap lives in the
/// caller (`parse_cff_gid_mapping_with_pdf_encoding`). This pins the
/// parser's tolerance for >256 entries so a regression there would
/// surface independently of the caller refactor.
#[test]
fn parse_charset_format0_handles_more_than_256_entries() {
    let mut data = vec![0x00u8];
    for gid in 1u16..=299u16 {
        data.extend_from_slice(&gid.to_be_bytes());
    }
    let sids = parse_charset(&data, 0, 300).expect("parse_charset returned None");
    assert_eq!(sids.len(), 300, "300 entries (GID 0 + 299 enumerated)");
    assert_eq!(sids[0], 0, "GID 0 is .notdef (SID 0)");
    assert_eq!(sids[1], 1, "GID 1 → SID 1");
    assert_eq!(sids[256], 256, "GID 256 → SID 256 (past the old 256 cap)");
    assert_eq!(sids[299], 299, "GID 299 → SID 299 (last entry)");
}

/// CFF Top DICT must surface the CharStrings INDEX offset (op 17) so
/// callers can read the real nGlyphs from the INDEX header instead of
/// guessing 256.
#[test]
fn parse_top_dict_surfaces_charstrings_offset() {
    let mut dict = vec![28u8];
    dict.extend_from_slice(&1234i16.to_be_bytes());
    dict.push(17u8);

    let (charstrings_offset, _enc, _charset) = parse_top_dict_with_charstrings(&dict);
    assert_eq!(charstrings_offset, 1234, "Top DICT op 17 → CharStrings offset");
}

/// Read the count field of a CFF INDEX header. nGlyphs is the count of
/// the CharStrings INDEX.
#[test]
fn read_index_count_returns_header_count() {
    let data = [0x01u8, 0x2C];
    assert_eq!(read_index_count(&data, 0), Some(300));
}

/// A bare CFF whose Top DICT declares `/FontMatrix [1/2048 0 0 1/2048 0 0]`.
///
/// Built here rather than embedded as a fixture. Layout: header
/// `01 00 04 01`, Name INDEX ("Test"), Top DICT INDEX carrying six real
/// operands then operator `12 7`, then an integer and operator `17`
/// (CharStrings), empty String and GlobalSubr INDEXes, and a CharStrings
/// INDEX with two trivial glyphs.
///
/// `1E 0A 00 04 88 28 12 FF` is the real-number encoding of 0.00048828125
/// — nibbles `. 0 0 0 4 8 8 2 8 1 2` terminated by `F`.
const CFF_2048: &[u8] = &[
    0x01, 0x00, 0x04, 0x01, 0x00, 0x01, 0x01, 0x01, 0x05, 0x54, 0x65, 0x73, 0x74, 0x00, 0x01, 0x01, 0x01, 0x21, 0x1E,
    0x0A, 0x00, 0x04, 0x88, 0x28, 0x12, 0xFF, 0x1E, 0x0A, 0xFF, 0x1E, 0x0A, 0xFF, 0x1E, 0x0A, 0x00, 0x04, 0x88, 0x28,
    0x12, 0xFF, 0x1E, 0x0A, 0xFF, 0x1E, 0x0A, 0xFF, 0x0C, 0x07, 0xC1, 0x11, 0x00, 0x00, 0x00, 0x00, 0x00, 0x02, 0x01,
    0x01, 0x06, 0x0B, 0x1C, 0x04, 0x00, 0x8B, 0x0E, 0x1C, 0x04, 0x00, 0x8B, 0x0E,
];

/// The same font with the `/FontMatrix` operator removed, so the CFF
/// default of 1/1000 applies and nothing should be folded in.
fn cff_without_font_matrix() -> Vec<u8> {
    let mut v = CFF_2048.to_vec();
    // Strip the six real operands and the `0C 07` operator from the Top
    // DICT, leaving only the CharStrings offset — then fix the INDEX
    // offset byte that follows the count/offSize header.
    let start = 18; // first byte of the Top DICT data
    let end = 48; // one past the `0C 07`
    v.drain(start..end);
    v[17] = 0x21 - (end - start) as u8;
    v
}

/// A subsetter targeting a 2048-unit em declares it, and the glyph-space
/// scale must come from the font rather than from the 1000-unit
/// assumption in ISO 32000-1 §9.2.2 ("For **most** types of fonts").
#[test]
fn test_font_matrix_of_one_over_2048_is_read() {
    let sx =
        parse_cff_font_matrix_scale(CFF_2048).expect("the Top DICT declares a FontMatrix, so a scale must come back");
    assert!(
        (1.0 / sx - 2048.0).abs() < 1.0,
        "expected a 2048-unit em, got 1/{sx} = {}",
        1.0 / sx
    );
}

/// No `/FontMatrix` means the CFF default, and the caller must keep its own
/// 1000-unit assumption rather than being handed a value to fold in.
#[test]
fn test_absent_font_matrix_yields_nothing_to_fold_in() {
    assert_eq!(
        parse_cff_font_matrix_scale(&cff_without_font_matrix()),
        None,
        "a CFF with no FontMatrix must not override the default em size"
    );
}

/// The real-number decoder handles the forms Adobe TN #5176 §4 defines:
/// a decimal point, a leading minus, and an exponent.
#[test]
fn dict_reals_decode() {
    assert_eq!(parse_dict_real(&[0x1E, 0x0A, 0x00, 0x1F], 0), Some((0.001, 4)));
    let (v, _) = parse_dict_real(&[0x1E, 0xE2, 0xA5, 0xFF], 0).expect("negative real");
    assert!((v + 2.5).abs() < 1e-9, "expected -2.5, got {v}");
}

/// A minimal **CID-keyed** CFF: the Top DICT carries ROS (operator `12 30`)
/// and a format-0 charset mapping GID 1 to **CID 100**.
///
/// Built here rather than embedded as a fixture. Layout: header, Name INDEX
/// ("CIDT"), Top DICT INDEX with ROS / CIDCount / charset / CharStrings,
/// empty String and GlobalSubr INDEXes, the charset, then a CharStrings
/// INDEX of two glyphs.
const CFF_CID_KEYED: &[u8] = &[
    0x01, 0x00, 0x04, 0x01, 0x00, 0x01, 0x01, 0x01, 0x05, 0x43, 0x49, 0x44, 0x54, 0x00, 0x01, 0x01, 0x01, 0x13, 0x1C,
    0x01, 0x87, 0x1C, 0x01, 0x88, 0x8B, 0x0C, 0x1E, 0x1C, 0x03, 0xE8, 0x0C, 0x22, 0xB3, 0x0F, 0xB6, 0x11, 0x00, 0x00,
    0x00, 0x00, 0x00, 0x00, 0x64, 0x00, 0x02, 0x01, 0x01, 0x05, 0x09, 0x1C, 0x03, 0xE8, 0x0E, 0x1C, 0x03, 0xE8, 0x0E,
];

/// §9.7.4.2: when the Top DICT uses
/// CIDFont operators the CID must be resolved through the charset, and the
/// NOTE warns the CID and GID "may differ". Here CID 100 is
/// glyph 1, so treating the CID as a GID would ask for glyph 100 of a
/// two-glyph font and paint nothing.
#[test]
fn test_cid_keyed_charset_maps_cid_to_gid() {
    let map = parse_cff_cid_to_gid(CFF_CID_KEYED).expect("the Top DICT carries ROS, so this font is CID-keyed");
    assert_eq!(map.get(&100), Some(&1), "CID 100 is GID 1 in this charset; got {map:?}");
}

/// A CFF with no ROS is not CID-keyed, and the same clause
/// then says "the CIDs shall be used directly as GID values" — so the
/// caller must get nothing to override that with.
#[test]
fn test_font_without_ros_yields_no_mapping() {
    // CFF_2048 is the FontMatrix fixture above: a plain, non-CID-keyed font.
    assert_eq!(
        parse_cff_cid_to_gid(CFF_2048),
        None,
        "a font whose Top DICT has no ROS operator must not produce a CID map"
    );
}

#[test]
fn charset_range_running_past_sid_65535_does_not_overflow() {
    let format1 = [1u8, 0xFF, 0xFF, 6];
    let sids = parse_charset(&format1, 0, 8).unwrap();
    assert_eq!(sids.len(), 8);
    assert!(sids[1..].iter().all(|sid| *sid == u16::MAX));

    let format2 = [2u8, 0xFF, 0xFE, 0x00, 6];
    let sids = parse_charset(&format2, 0, 8).unwrap();
    assert_eq!(sids[1], 0xFFFE);
    assert_eq!(sids[7], u16::MAX);
}
