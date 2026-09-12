//! ToUnicode CMap CID-miss must not emit ciphertext.
//!
//! Subset Type0 fonts (`ABCDEE+Cambria`, `XFVTFT+Cambria-Bold`, etc.) with
//! Identity-H encoding and Adobe-Identity `CIDSystemInfo` typically carry a
//! ToUnicode CMap that covers only the CIDs the subset uses. CIDs in the
//! subset are insertion-ordered (0x0001, 0x0002, …) and do NOT correspond to
//! any Unicode code point.
//!
//! Before this fix, `FontInfo::char_to_unicode` on a miss fell through to the
//! Identity-H CID-as-Unicode fallback (`char::from_u32(cid)`), producing
//! ASCII-shifted ciphertext like `%B+$%8A//$2*%01*1%6APP` for nougat_035.pdf
//! page 13.
//!
//! Per ISO 32000-1:2008 §9.10.2, a ToUnicode CMap attached to a font is the
//! authoritative character→Unicode mapping: a miss must produce U+FFFD, not
//! fall through to encoding-derived heuristics whose output is meaningless
//! for subset CIDs.
//!
//! Scope of the fix: narrow. Simple fonts (Type1 / TrueType) with standard
//! encodings still fall through on miss — see
//! `test_character_mapping_fixes::test_extraction_priority_chain`, which
//! remains green.
use std::collections::HashMap;
use xberg_native_pdf::fonts::{CIDSystemInfo, Encoding, FontInfo, LazyCMap};

/// Build a subset Type0 font whose ToUnicode CMap covers CID 0x0001 → 'T'
/// only. Any other CID is a miss.
fn subset_type0_font() -> FontInfo {
    // Minimal bfchar with one entry: 0x0001 → U+0054 ('T'). ~keep
    let cmap_data = b"beginbfchar\n<0001> <0054>\nendbfchar\n";

    FontInfo {
        base_font: "ABCDEE+Cambria".to_string(),
        subtype: "Type0".to_string(),
        encoding: Encoding::Standard("Identity-H".to_string()),
        to_unicode: Some(LazyCMap::new(cmap_data.to_vec())),
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
        cid_font_type: None,
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
    }
}

#[test]
fn tounicode_hit_returns_mapped_char() {
    let font = subset_type0_font();
    assert_eq!(font.char_to_unicode(0x0001), Some("T".to_string()));
}

#[test]
fn tounicode_miss_on_subset_type0_identity_h_returns_replacement() {
    // The defect: the font's ToUnicode does not cover CID
    // 0x0025, so char_to_unicode used to return '%' via the Identity-H
    // CID-as-Unicode fallback at font_dict.rs:~2467. That '%' is what shows
    // up in `extract_text` on nougat_035.pdf page 13 as ASCII-shifted
    // ciphertext. ~keep
    let font = subset_type0_font();

    let result = font.char_to_unicode(0x0025);
    assert_eq!(
        result,
        Some("\u{FFFD}".to_string()),
        "Type0 + Identity-H + ToUnicode-present-but-missed must produce \
         U+FFFD per ISO 32000-1 §9.10.2, not CID-as-Unicode ('%'). Got {result:?}"
    );
}

#[test]
fn tounicode_miss_on_subset_type0_is_not_ascii_shifted_ciphertext() {
    let font = subset_type0_font();

    let result = font.char_to_unicode(0x0041);
    assert_ne!(
        result,
        Some("A".to_string()),
        "A subset Type0 font with an explicit ToUnicode CMap must not \
         produce 'A' for an unmapped CID 0x0041 — its subset assigns \
         CIDs in insertion order, not by codepoint. Got {result:?}"
    );
    assert_eq!(result, Some("\u{FFFD}".to_string()));
}
