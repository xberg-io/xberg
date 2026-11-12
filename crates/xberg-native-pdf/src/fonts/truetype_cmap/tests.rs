//! Unit tests for [`super`].
//!
//! Split out of `truetype_cmap.rs` purely for file size, mirroring the split used for
//! `document.rs`/`document/tests.rs` and `extractors/text/mod.rs`/`extractors/text/tests.rs`. ~keep

use super::*;
use byteorder::{BigEndian, WriteBytesExt};

/// Build a minimal TrueType font with a cmap format 4 table.
fn build_truetype_with_cmap_format4(mappings: &[(u16, u16)]) -> Vec<u8> {
    let mut data = Vec::new();

    data.write_u32::<BigEndian>(0x00010000).unwrap();
    data.write_u16::<BigEndian>(1).unwrap();
    data.write_u16::<BigEndian>(16).unwrap();
    data.write_u16::<BigEndian>(0).unwrap();
    data.write_u16::<BigEndian>(0).unwrap();

    let cmap_offset: u32 = 12 + 16;
    data.write_u32::<BigEndian>(0x636D6170).unwrap();
    data.write_u32::<BigEndian>(0).unwrap();
    data.write_u32::<BigEndian>(cmap_offset).unwrap();
    data.write_u32::<BigEndian>(0).unwrap();

    data.write_u16::<BigEndian>(0).unwrap();
    data.write_u16::<BigEndian>(1).unwrap();

    let subtable_offset: u32 = 4 + 8;
    data.write_u16::<BigEndian>(3).unwrap();
    data.write_u16::<BigEndian>(1).unwrap();
    data.write_u32::<BigEndian>(subtable_offset).unwrap();

    let mut segments: Vec<(u16, u16, i16)> = Vec::new();
    for &(char_code, gid) in mappings {
        let delta = gid as i16 - char_code as i16;
        segments.push((char_code, char_code, delta));
    }
    segments.push((0xFFFF, 0xFFFF, 1));

    let seg_count = segments.len();
    let seg_count_x2 = (seg_count * 2) as u16;

    data.write_u16::<BigEndian>(4).unwrap();
    let length_pos = data.len();
    data.write_u16::<BigEndian>(0).unwrap();
    data.write_u16::<BigEndian>(0).unwrap();

    data.write_u16::<BigEndian>(seg_count_x2).unwrap();
    data.write_u16::<BigEndian>(0).unwrap();
    data.write_u16::<BigEndian>(0).unwrap();
    data.write_u16::<BigEndian>(0).unwrap();

    for seg in &segments {
        data.write_u16::<BigEndian>(seg.1).unwrap();
    }
    data.write_u16::<BigEndian>(0).unwrap();
    for seg in &segments {
        data.write_u16::<BigEndian>(seg.0).unwrap();
    }
    for seg in &segments {
        data.write_i16::<BigEndian>(seg.2).unwrap();
    }
    for _ in &segments {
        data.write_u16::<BigEndian>(0).unwrap();
    }

    let fmt4_start = length_pos - 2;
    let fmt4_len = data.len() - fmt4_start;
    let len_bytes = (fmt4_len as u16).to_be_bytes();
    data[length_pos] = len_bytes[0];
    data[length_pos + 1] = len_bytes[1];

    data
}

/// Build a minimal TrueType font with a cmap format 0 table.
/// `glyph_ids` is the 256-entry byte → glyph id array.
fn build_truetype_with_cmap_format0(glyph_ids: [u8; 256]) -> Vec<u8> {
    let mut data = Vec::new();

    data.write_u32::<BigEndian>(0x00010000).unwrap();
    data.write_u16::<BigEndian>(1).unwrap();
    data.write_u16::<BigEndian>(16).unwrap();
    data.write_u16::<BigEndian>(0).unwrap();
    data.write_u16::<BigEndian>(0).unwrap();

    let cmap_offset: u32 = 12 + 16;
    data.write_u32::<BigEndian>(0x636D6170).unwrap();
    data.write_u32::<BigEndian>(0).unwrap();
    data.write_u32::<BigEndian>(cmap_offset).unwrap();
    data.write_u32::<BigEndian>(0).unwrap();

    data.write_u16::<BigEndian>(0).unwrap();
    data.write_u16::<BigEndian>(1).unwrap();

    let subtable_offset: u32 = 4 + 8;
    data.write_u16::<BigEndian>(1).unwrap();
    data.write_u16::<BigEndian>(0).unwrap();
    data.write_u32::<BigEndian>(subtable_offset).unwrap();

    data.write_u16::<BigEndian>(0).unwrap();
    data.write_u16::<BigEndian>(262).unwrap();
    data.write_u16::<BigEndian>(0).unwrap();
    data.extend_from_slice(&glyph_ids);

    data
}

/// Build a minimal TrueType font with a cmap format 6 table.
fn build_truetype_with_cmap_format6(first_code: u16, gids: &[u16]) -> Vec<u8> {
    let mut data = Vec::new();

    data.write_u32::<BigEndian>(0x00010000).unwrap();
    data.write_u16::<BigEndian>(1).unwrap();
    data.write_u16::<BigEndian>(16).unwrap();
    data.write_u16::<BigEndian>(0).unwrap();
    data.write_u16::<BigEndian>(0).unwrap();

    let cmap_offset: u32 = 12 + 16;
    data.write_u32::<BigEndian>(0x636D6170).unwrap();
    data.write_u32::<BigEndian>(0).unwrap();
    data.write_u32::<BigEndian>(cmap_offset).unwrap();
    data.write_u32::<BigEndian>(0).unwrap();

    data.write_u16::<BigEndian>(0).unwrap();
    data.write_u16::<BigEndian>(1).unwrap();
    data.write_u16::<BigEndian>(3).unwrap();
    data.write_u16::<BigEndian>(1).unwrap();
    data.write_u32::<BigEndian>(4 + 8).unwrap();

    data.write_u16::<BigEndian>(6).unwrap();
    data.write_u16::<BigEndian>((10 + gids.len() * 2) as u16).unwrap();
    data.write_u16::<BigEndian>(0).unwrap();
    data.write_u16::<BigEndian>(first_code).unwrap();
    data.write_u16::<BigEndian>(gids.len() as u16).unwrap();
    for &gid in gids {
        data.write_u16::<BigEndian>(gid).unwrap();
    }

    data
}

/// Build a minimal TrueType font with a cmap format 12 table.
fn build_truetype_with_cmap_format12(groups: &[(u32, u32, u32)]) -> Vec<u8> {
    let mut data = Vec::new();

    data.write_u32::<BigEndian>(0x00010000).unwrap();
    data.write_u16::<BigEndian>(1).unwrap();
    data.write_u16::<BigEndian>(16).unwrap();
    data.write_u16::<BigEndian>(0).unwrap();
    data.write_u16::<BigEndian>(0).unwrap();

    let cmap_offset: u32 = 12 + 16;
    data.write_u32::<BigEndian>(0x636D6170).unwrap();
    data.write_u32::<BigEndian>(0).unwrap();
    data.write_u32::<BigEndian>(cmap_offset).unwrap();
    data.write_u32::<BigEndian>(0).unwrap();

    data.write_u16::<BigEndian>(0).unwrap();
    data.write_u16::<BigEndian>(1).unwrap();
    data.write_u16::<BigEndian>(3).unwrap();
    data.write_u16::<BigEndian>(10).unwrap();
    data.write_u32::<BigEndian>(4 + 8).unwrap();

    data.write_u16::<BigEndian>(12).unwrap();
    data.write_u16::<BigEndian>(0).unwrap();
    data.write_u32::<BigEndian>((16 + groups.len() * 12) as u32).unwrap();
    data.write_u32::<BigEndian>(0).unwrap();
    data.write_u32::<BigEndian>(groups.len() as u32).unwrap();
    for &(start, end, start_gid) in groups {
        data.write_u32::<BigEndian>(start).unwrap();
        data.write_u32::<BigEndian>(end).unwrap();
        data.write_u32::<BigEndian>(start_gid).unwrap();
    }

    data
}

/// A standalone cmap format-4 subtable body (format … idRangeOffset), one
/// segment per `(code, gid)` mapping plus the `0xFFFF` sentinel.
fn fmt4_subtable(mappings: &[(u16, u16)]) -> Vec<u8> {
    let mut segs: Vec<(u16, u16, i16)> = mappings
        .iter()
        .map(|&(c, g)| (c, c, (g as i32 - c as i32) as i16))
        .collect();
    segs.push((0xFFFF, 0xFFFF, 1));
    let mut s = Vec::new();
    s.write_u16::<BigEndian>(4).unwrap();
    let len_pos = s.len();
    s.write_u16::<BigEndian>(0).unwrap();
    s.write_u16::<BigEndian>(0).unwrap();
    s.write_u16::<BigEndian>((segs.len() * 2) as u16).unwrap();
    for _ in 0..3 {
        s.write_u16::<BigEndian>(0).unwrap();
    }
    for seg in &segs {
        s.write_u16::<BigEndian>(seg.1).unwrap();
    }
    s.write_u16::<BigEndian>(0).unwrap();
    for seg in &segs {
        s.write_u16::<BigEndian>(seg.0).unwrap();
    }
    for seg in &segs {
        s.write_i16::<BigEndian>(seg.2).unwrap();
    }
    for _ in &segs {
        s.write_u16::<BigEndian>(0).unwrap();
    }
    let len = (s.len() as u16).to_be_bytes();
    s[len_pos] = len[0];
    s[len_pos + 1] = len[1];
    s
}

/// Two-subtable cmap: a `(3,1)` Unicode subtable and a `(3,0)` symbol one.
fn cmap_two_subtables(unicode: &[(u16, u16)], symbol: &[(u16, u16)]) -> Vec<u8> {
    let body_uni = fmt4_subtable(unicode);
    let body_sym = fmt4_subtable(symbol);
    let off_uni = 4 + 2 * 8;
    let off_sym = off_uni + body_uni.len();
    let mut t = Vec::new();
    t.write_u16::<BigEndian>(0).unwrap();
    t.write_u16::<BigEndian>(2).unwrap();
    for (plat, enc, off) in [(3u16, 1u16, off_uni), (3, 0, off_sym)] {
        t.write_u16::<BigEndian>(plat).unwrap();
        t.write_u16::<BigEndian>(enc).unwrap();
        t.write_u32::<BigEndian>(off as u32).unwrap();
    }
    t.extend(body_uni);
    t.extend(body_sym);
    t
}

/// Assemble an sfnt from `(tag, data)` tables — sorted by tag (the directory
/// is binary-searched), 4-byte aligned. Just enough for `ttf-parser`.
fn assemble(mut tables: Vec<(u32, Vec<u8>)>) -> Vec<u8> {
    tables.sort_by_key(|(tag, _)| *tag);
    let mut out = Vec::new();
    out.write_u32::<BigEndian>(0x0001_0000).unwrap();
    out.write_u16::<BigEndian>(tables.len() as u16).unwrap();
    for _ in 0..3 {
        out.write_u16::<BigEndian>(0).unwrap();
    }
    let mut offset = 12 + tables.len() * 16;
    for (tag, data) in &tables {
        out.write_u32::<BigEndian>(*tag).unwrap();
        out.write_u32::<BigEndian>(0).unwrap();
        out.write_u32::<BigEndian>(offset as u32).unwrap();
        out.write_u32::<BigEndian>(data.len() as u32).unwrap();
        offset += (data.len() + 3) & !3;
    }
    for (_, data) in &tables {
        out.extend_from_slice(data);
        while out.len() % 4 != 0 {
            out.push(0);
        }
    }
    out
}

/// Regression for the symbolic-font Ç→Ê bug, exercising the real
/// `from_font_data` → `ttf-parser` extraction path. The content byte is NOT
/// the GID: the `(3,0)` subtable maps byte 3 → GID 2 (the `Ç` glyph), while
/// the byte used directly as a GID hits GID 3 → `Ê` (the old behaviour).
#[test]
fn symbolic_byte_resolves_via_symbol_cmap() {
    let mut head = Vec::new();
    head.write_u16::<BigEndian>(1).unwrap();
    head.write_u16::<BigEndian>(0).unwrap();
    head.write_u32::<BigEndian>(0).unwrap();
    head.write_u32::<BigEndian>(0).unwrap();
    head.write_u32::<BigEndian>(0x5F0F_3CF5).unwrap();
    head.write_u16::<BigEndian>(0).unwrap();
    head.write_u16::<BigEndian>(1000).unwrap();
    head.write_u64::<BigEndian>(0).unwrap();
    head.write_u64::<BigEndian>(0).unwrap();
    for v in [0i16, 0, 1000, 1000, 0, 0, 0, 0, 0] {
        head.write_i16::<BigEndian>(v).unwrap();
    }
    assert_eq!(head.len(), 54);

    let mut hhea = Vec::new();
    hhea.write_u16::<BigEndian>(1).unwrap();
    hhea.write_u16::<BigEndian>(0).unwrap();
    for v in [800i16, -200, 0, 1000, 0, 0, 1000, 1, 0, 0, 0, 0, 0, 0, 0] {
        hhea.write_i16::<BigEndian>(v).unwrap();
    }
    hhea.write_u16::<BigEndian>(1).unwrap();
    assert_eq!(hhea.len(), 36);

    let mut maxp = Vec::new();
    maxp.write_u32::<BigEndian>(0x0000_5000).unwrap();
    maxp.write_u16::<BigEndian>(4).unwrap();

    let cmap = cmap_two_subtables(&[(0x00C7, 2), (0x00CA, 3)], &[(3, 2)]);
    let font = assemble(vec![
        (0x6865_6164, head),
        (0x6868_6561, hhea),
        (0x6D61_7870, maxp),
        (0x636D_6170, cmap),
    ]);

    let parsed = TrueTypeCMap::from_font_data(&font).expect("parse synthetic font");
    assert_eq!(parsed.code_to_gid(3), Some(2)); // byte → GID via (3,0) ~keep
    assert_eq!(parsed.get_unicode(2), Some('Ç')); // fixed: byte → GID → Unicode ~keep
    assert_eq!(parsed.get_unicode(3), Some('Ê')); // old byte-as-GID path was wrong ~keep
    assert_eq!(parsed.code_to_gid(99), None);
    // ~keep
}

#[test]
fn test_sfnt_header_parsing() {
    let data = build_truetype_with_cmap_format4(&[]);
    let cmap = TrueTypeCMap::from_font_data(&data).unwrap();
    assert!(cmap.is_empty());
}

#[test]
fn test_invalid_sfnt_version() {
    let mut data = vec![0u8; 100];
    data[0] = 0xFF;
    data[1] = 0xFF;
    data[2] = 0xFF;
    data[3] = 0xFF;
    let result = TrueTypeCMap::from_font_data(&data);
    assert!(result.is_err());
    assert!(result.unwrap_err().contains("Invalid sfnt version"));
}

#[test]
fn test_opentype_version_accepted() {
    let mut data = build_truetype_with_cmap_format4(&[(65, 1)]);
    data[0] = 0x4F;
    data[1] = 0x54;
    data[2] = 0x54;
    data[3] = 0x4F;
    let result = TrueTypeCMap::from_font_data(&data);
    assert!(result.is_ok());
}

#[test]
fn test_apple_truetype_version_accepted() {
    let mut data = build_truetype_with_cmap_format4(&[(65, 1)]);
    data[0] = 0x74;
    data[1] = 0x72;
    data[2] = 0x75;
    data[3] = 0x65;
    let result = TrueTypeCMap::from_font_data(&data);
    assert!(result.is_ok());
}

#[test]
fn test_no_cmap_table() {
    let mut data = Vec::new();
    data.write_u32::<BigEndian>(0x00010000).unwrap();
    data.write_u16::<BigEndian>(1).unwrap();
    data.write_u16::<BigEndian>(16).unwrap();
    data.write_u16::<BigEndian>(0).unwrap();
    data.write_u16::<BigEndian>(0).unwrap();
    data.write_u32::<BigEndian>(0x68656164).unwrap();
    data.write_u32::<BigEndian>(0).unwrap();
    data.write_u32::<BigEndian>(28).unwrap();
    data.write_u32::<BigEndian>(0).unwrap();

    let result = TrueTypeCMap::from_font_data(&data);
    assert!(result.is_err());
    assert!(result.unwrap_err().contains("cmap table not found"));
}

#[test]
fn test_format4_basic_ascii() {
    let data = build_truetype_with_cmap_format4(&[(65, 1), (66, 2), (67, 3)]);
    let cmap = TrueTypeCMap::from_font_data(&data).unwrap();
    assert_eq!(cmap.len(), 3);
    assert_eq!(cmap.get_unicode(1), Some('A'));
    assert_eq!(cmap.get_unicode(2), Some('B'));
    assert_eq!(cmap.get_unicode(3), Some('C'));
    assert_eq!(cmap.get_unicode(4), None);
}

#[test]
fn test_format4_extended_unicode() {
    let data = build_truetype_with_cmap_format4(&[(233, 10), (241, 11)]);
    let cmap = TrueTypeCMap::from_font_data(&data).unwrap();
    assert_eq!(cmap.get_unicode(10), Some('é'));
    assert_eq!(cmap.get_unicode(11), Some('ñ'));
}

#[test]
fn test_format6_basic() {
    let data = build_truetype_with_cmap_format6(65, &[1, 2, 3]);
    let cmap = TrueTypeCMap::from_font_data(&data).unwrap();
    assert_eq!(cmap.len(), 3);
    assert_eq!(cmap.get_unicode(1), Some('A'));
    assert_eq!(cmap.get_unicode(2), Some('B'));
    assert_eq!(cmap.get_unicode(3), Some('C'));
}

#[test]
fn test_format6_non_zero_first_code() {
    let data = build_truetype_with_cmap_format6(48, &[10, 11, 12]);
    let cmap = TrueTypeCMap::from_font_data(&data).unwrap();
    assert_eq!(cmap.get_unicode(10), Some('0'));
    assert_eq!(cmap.get_unicode(11), Some('1'));
    assert_eq!(cmap.get_unicode(12), Some('2'));
}

#[test]
fn test_format12_basic() {
    let data = build_truetype_with_cmap_format12(&[(65, 67, 1)]);
    let cmap = TrueTypeCMap::from_font_data(&data).unwrap();
    assert_eq!(cmap.len(), 3);
    assert_eq!(cmap.get_unicode(1), Some('A'));
    assert_eq!(cmap.get_unicode(2), Some('B'));
    assert_eq!(cmap.get_unicode(3), Some('C'));
}

#[test]
fn test_format12_multiple_groups() {
    let data = build_truetype_with_cmap_format12(&[(65, 67, 1), (48, 50, 10)]);
    let cmap = TrueTypeCMap::from_font_data(&data).unwrap();
    assert_eq!(cmap.len(), 6);
    assert_eq!(cmap.get_unicode(1), Some('A'));
    assert_eq!(cmap.get_unicode(10), Some('0'));
    assert_eq!(cmap.get_unicode(12), Some('2'));
}

#[test]
fn test_get_unicode_missing() {
    let data = build_truetype_with_cmap_format4(&[(65, 1)]);
    let cmap = TrueTypeCMap::from_font_data(&data).unwrap();
    assert_eq!(cmap.get_unicode(999), None);
}

#[test]
fn test_len_and_is_empty() {
    let data_empty = build_truetype_with_cmap_format4(&[]);
    let cmap_empty = TrueTypeCMap::from_font_data(&data_empty).unwrap();
    assert_eq!(cmap_empty.len(), 0);
    assert!(cmap_empty.is_empty());

    let data_one = build_truetype_with_cmap_format4(&[(65, 1)]);
    let cmap_one = TrueTypeCMap::from_font_data(&data_one).unwrap();
    assert_eq!(cmap_one.len(), 1);
    assert!(!cmap_one.is_empty());
}

#[test]
fn test_cmap_format0_byte_indexed() {
    let mut gids = [0u8; 256];
    gids[0x41] = 10;
    gids[0x42] = 11;
    gids[0x7A] = 50;
    let data = build_truetype_with_cmap_format0(gids);
    let cmap = TrueTypeCMap::from_font_data(&data).expect("format 0 parse");
    assert_eq!(cmap.get_unicode(10), Some('A'));
    assert_eq!(cmap.get_unicode(11), Some('B'));
    assert_eq!(cmap.get_unicode(50), Some('z'));
    assert_eq!(cmap.get_unicode(99), None);
}

#[test]
fn test_cmap_format0_mac_roman_high_half() {
    let mut gids = [0u8; 256];
    gids[0x41] = 10;
    gids[0x8A] = 20;
    gids[0xA5] = 30;
    let data = build_truetype_with_cmap_format0(gids);
    let cmap = TrueTypeCMap::from_font_data(&data).expect("format 0 parse");
    assert_eq!(cmap.get_unicode(10), Some('A'));
    assert_eq!(
        cmap.get_unicode(20),
        Some('ä'),
        "Mac Roman 0x8A must map to U+00E4 (a-umlaut), not raw 0x8A"
    );
    assert_eq!(cmap.get_unicode(30), Some('•'));
}

#[test]
fn test_cmap_format0_rejects_truncated() {
    let mut data = Vec::new();
    data.write_u32::<BigEndian>(0x00010000).unwrap();
    data.write_u16::<BigEndian>(1).unwrap();
    data.write_u16::<BigEndian>(16).unwrap();
    data.write_u16::<BigEndian>(0).unwrap();
    data.write_u16::<BigEndian>(0).unwrap();
    let cmap_offset: u32 = 12 + 16;
    data.write_u32::<BigEndian>(0x636D6170).unwrap();
    data.write_u32::<BigEndian>(0).unwrap();
    data.write_u32::<BigEndian>(cmap_offset).unwrap();
    data.write_u32::<BigEndian>(0).unwrap();
    data.write_u16::<BigEndian>(0).unwrap();
    data.write_u16::<BigEndian>(1).unwrap();
    data.write_u16::<BigEndian>(1).unwrap();
    data.write_u16::<BigEndian>(0).unwrap();
    data.write_u32::<BigEndian>(4 + 8).unwrap();
    data.write_u16::<BigEndian>(0).unwrap();
    // Declare the correct length (262) but only append 8 bytes of
    // glyphIdArray instead of 256 — parser should detect the
    // truncation via read_exact. ~keep
    data.write_u16::<BigEndian>(262).unwrap();
    data.write_u16::<BigEndian>(0).unwrap();
    data.extend_from_slice(&[0u8; 8]);
    let result = TrueTypeCMap::from_font_data(&data);
    assert!(result.is_err());
}

#[test]
fn test_unsupported_cmap_format() {
    let mut data = Vec::new();
    data.write_u32::<BigEndian>(0x00010000).unwrap();
    data.write_u16::<BigEndian>(1).unwrap();
    data.write_u16::<BigEndian>(16).unwrap();
    data.write_u16::<BigEndian>(0).unwrap();
    data.write_u16::<BigEndian>(0).unwrap();
    let cmap_offset: u32 = 12 + 16;
    data.write_u32::<BigEndian>(0x636D6170).unwrap();
    data.write_u32::<BigEndian>(0).unwrap();
    data.write_u32::<BigEndian>(cmap_offset).unwrap();
    data.write_u32::<BigEndian>(0).unwrap();
    data.write_u16::<BigEndian>(0).unwrap();
    data.write_u16::<BigEndian>(1).unwrap();
    data.write_u16::<BigEndian>(3).unwrap();
    data.write_u16::<BigEndian>(1).unwrap();
    data.write_u32::<BigEndian>(4 + 8).unwrap();
    data.write_u16::<BigEndian>(2).unwrap();

    let result = TrueTypeCMap::from_font_data(&data);
    assert!(result.is_err());
    assert!(result.unwrap_err().contains("Unsupported cmap format"));
}

#[test]
fn test_unsupported_cmap_version() {
    let mut data = Vec::new();
    data.write_u32::<BigEndian>(0x00010000).unwrap();
    data.write_u16::<BigEndian>(1).unwrap();
    data.write_u16::<BigEndian>(16).unwrap();
    data.write_u16::<BigEndian>(0).unwrap();
    data.write_u16::<BigEndian>(0).unwrap();
    let cmap_offset: u32 = 12 + 16;
    data.write_u32::<BigEndian>(0x636D6170).unwrap();
    data.write_u32::<BigEndian>(0).unwrap();
    data.write_u32::<BigEndian>(cmap_offset).unwrap();
    data.write_u32::<BigEndian>(0).unwrap();
    data.write_u16::<BigEndian>(99).unwrap();

    let result = TrueTypeCMap::from_font_data(&data);
    assert!(result.is_err());
    assert!(result.unwrap_err().contains("Unsupported cmap table version"));
}

#[test]
fn test_no_suitable_subtable() {
    let mut data = Vec::new();
    data.write_u32::<BigEndian>(0x00010000).unwrap();
    data.write_u16::<BigEndian>(1).unwrap();
    data.write_u16::<BigEndian>(16).unwrap();
    data.write_u16::<BigEndian>(0).unwrap();
    data.write_u16::<BigEndian>(0).unwrap();
    let cmap_offset: u32 = 12 + 16;
    data.write_u32::<BigEndian>(0x636D6170).unwrap();
    data.write_u32::<BigEndian>(0).unwrap();
    data.write_u32::<BigEndian>(cmap_offset).unwrap();
    data.write_u32::<BigEndian>(0).unwrap();
    data.write_u16::<BigEndian>(0).unwrap();
    data.write_u16::<BigEndian>(0).unwrap();

    let result = TrueTypeCMap::from_font_data(&data);
    assert!(result.is_err());
    assert!(result.unwrap_err().contains("No suitable cmap subtable"));
}

#[test]
fn test_truncated_data() {
    let data = vec![0u8; 4];
    let result = TrueTypeCMap::from_font_data(&data);
    assert!(result.is_err());
}

#[test]
fn test_clone_and_debug() {
    let data = build_truetype_with_cmap_format4(&[(65, 1)]);
    let cmap = TrueTypeCMap::from_font_data(&data).unwrap();
    let cloned = cmap.clone();
    assert_eq!(cloned.get_unicode(1), Some('A'));
    let debug = format!("{:?}", cmap);
    assert!(debug.contains("TrueTypeCMap"));
}

/// Issue #3019: a malformed embedded font whose cmap subtable offset is huge
/// must be rejected with a parse error, not overflow the u32
/// `cmap_offset + subtable_offset` arithmetic — a debug panic that crashes the
/// renderer, or a release wrap that silently walks garbage.
#[test]
fn test_cmap_subtable_offset_out_of_bounds_rejected() {
    let mut data = build_truetype_with_cmap_format4(&[(65, 1)]);
    // The (3,1) record is the only subtable: its offset field is the four
    // bytes at 36..40 — 12-byte sfnt header + one 16-byte directory entry +
    // 4-byte cmap version/count header + 4-byte platform/encoding pair. ~keep
    assert_eq!(&data[36..40], &12u32.to_be_bytes(), "subtable offset field location");
    data[36..40].copy_from_slice(&0xFFFF_FFF0u32.to_be_bytes());

    let err = TrueTypeCMap::from_font_data(&data).expect_err("huge subtable offset must be rejected, not overflowed");
    assert!(err.contains("out of bounds"), "unexpected error: {err}");
}

/// The offset need not overflow to be invalid: one pointing past the end of
/// the font data must be rejected in release builds too, where the unchecked
/// arithmetic wraps instead of panicking. The fix is a bounds check, not just
/// overflow-checked arithmetic.
#[test]
fn test_cmap_subtable_offset_beyond_data_rejected() {
    let mut data = build_truetype_with_cmap_format4(&[(65, 1)]);
    data[36..40].copy_from_slice(&0x0000_1000u32.to_be_bytes()); // 4096 ≫ this ~70-byte font
    let result = TrueTypeCMap::from_font_data(&data);
    assert!(result.is_err());
}

/// Issue #3019 (audit): the format-4 glyphIdArray index,
/// `idRangeOffset/2 + (charCode - startCode) + i - segCount`, subtracts
/// `segCount` from a `usize` that a malformed subtable can push below zero —
/// a debug underflow panic (release wrap happens to fail the bounds check and
/// resolve to GID 0). A subtable whose first segment has a nonzero
/// idRangeOffset smaller than segCount reaches it.
#[test]
fn test_format4_id_range_offset_index_underflow_yields_no_glyph() {
    let mut data = Vec::new();

    data.write_u32::<BigEndian>(0x00010000).unwrap();
    data.write_u16::<BigEndian>(1).unwrap();
    data.write_u16::<BigEndian>(16).unwrap();
    data.write_u16::<BigEndian>(0).unwrap();
    data.write_u16::<BigEndian>(0).unwrap();

    let cmap_offset: u32 = 12 + 16;
    data.write_u32::<BigEndian>(0x636D6170).unwrap();
    data.write_u32::<BigEndian>(0).unwrap();
    data.write_u32::<BigEndian>(cmap_offset).unwrap();
    data.write_u32::<BigEndian>(0).unwrap();

    data.write_u16::<BigEndian>(0).unwrap();
    data.write_u16::<BigEndian>(1).unwrap();
    data.write_u16::<BigEndian>(3).unwrap();
    data.write_u16::<BigEndian>(1).unwrap();
    data.write_u32::<BigEndian>(4 + 8).unwrap();

    data.write_u16::<BigEndian>(4).unwrap();
    data.write_u16::<BigEndian>(0).unwrap(); // length (unused by the parser)
    data.write_u16::<BigEndian>(0).unwrap(); // language
    data.write_u16::<BigEndian>(6).unwrap(); // segCountX2: three segments
    data.write_u16::<BigEndian>(0).unwrap();
    data.write_u16::<BigEndian>(0).unwrap();
    data.write_u16::<BigEndian>(0).unwrap();

    // End codes, reserved pad, start codes. ~keep
    for code in [0x41u16, 0x42, 0xFFFF] {
        data.write_u16::<BigEndian>(code).unwrap();
    }
    data.write_u16::<BigEndian>(0).unwrap();
    for code in [0x41u16, 0x42, 0xFFFF] {
        data.write_u16::<BigEndian>(code).unwrap();
    }
    // idDelta per segment. ~keep
    for delta in [0i16, 2, 1] {
        data.write_i16::<BigEndian>(delta).unwrap();
    }
    // idRangeOffset: segment 0 points at the glyphIdArray, segments 1-2 do
    // not. For segment 0 the index is 2/2 + (0x41 - 0x41) + 0 - 3 < 0. ~keep
    for range_offset in [2u16, 0, 0] {
        data.write_u16::<BigEndian>(range_offset).unwrap();
    }
    data.write_u16::<BigEndian>(9).unwrap(); // glyphIdArray
    data.write_u16::<BigEndian>(8).unwrap();

    let cmap = TrueTypeCMap::from_font_data(&data).expect("parse malformed-but-recoverable cmap");
    assert_eq!(cmap.get_unicode(68), Some('B'), "well-formed segment still parses");
    assert_eq!(cmap.len(), 1, "underflowing segment contributes no glyph");
}

/// Issue #3019 (audit): format 12 computes each glyph id as
/// `startGlyphId + (charCode - startCharCode)`, an unchecked u32 add that a
/// malformed group's `startGlyphId` can overflow — a debug panic; release
/// wraps and then truncates through the `as u16` cast. `wrapping_add` keeps
/// the release mapping byte-for-byte while removing the panic.
#[test]
fn test_format12_glyph_id_overflow_wraps_like_release() {
    let data = build_truetype_with_cmap_format12(&[(65, 67, 0xFFFF_FFFE)]);
    let cmap = TrueTypeCMap::from_font_data(&data).expect("parse");
    // Release behaviour today: the u32 add wraps, then `as u16` truncates —
    // 0xFFFF_FFFE → 0xFFFE, 0xFFFF_FFFF → 0xFFFF, 0x1_0000_0000 → 0x0000. ~keep
    assert_eq!(cmap.get_unicode(0xFFFE), Some('A'));
    assert_eq!(cmap.get_unicode(0xFFFF), Some('B'));
    assert_eq!(cmap.get_unicode(0), Some('C'));
    assert_eq!(cmap.len(), 3);
}

#[test]
fn test_platform_priority_windows_full_over_bmp() {
    let mut data = Vec::new();

    data.write_u32::<BigEndian>(0x00010000).unwrap();
    data.write_u16::<BigEndian>(1).unwrap();
    data.write_u16::<BigEndian>(16).unwrap();
    data.write_u16::<BigEndian>(0).unwrap();
    data.write_u16::<BigEndian>(0).unwrap();

    let cmap_offset: u32 = 12 + 16;
    data.write_u32::<BigEndian>(0x636D6170).unwrap();
    data.write_u32::<BigEndian>(0).unwrap();
    data.write_u32::<BigEndian>(cmap_offset).unwrap();
    data.write_u32::<BigEndian>(0).unwrap();

    data.write_u16::<BigEndian>(0).unwrap();
    data.write_u16::<BigEndian>(2).unwrap();

    let subtable_off: u32 = 4 + 8 * 2;
    data.write_u16::<BigEndian>(3).unwrap();
    data.write_u16::<BigEndian>(1).unwrap();
    data.write_u32::<BigEndian>(subtable_off).unwrap();
    data.write_u16::<BigEndian>(3).unwrap();
    data.write_u16::<BigEndian>(10).unwrap();
    data.write_u32::<BigEndian>(subtable_off).unwrap();

    data.write_u16::<BigEndian>(12).unwrap();
    data.write_u16::<BigEndian>(0).unwrap();
    data.write_u32::<BigEndian>(28).unwrap();
    data.write_u32::<BigEndian>(0).unwrap();
    data.write_u32::<BigEndian>(1).unwrap();
    data.write_u32::<BigEndian>(65).unwrap();
    data.write_u32::<BigEndian>(65).unwrap();
    data.write_u32::<BigEndian>(1).unwrap();

    let cmap = TrueTypeCMap::from_font_data(&data).unwrap();
    assert_eq!(cmap.get_unicode(1), Some('A'));
}
