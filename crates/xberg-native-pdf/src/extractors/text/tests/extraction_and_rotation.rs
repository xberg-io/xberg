//! Basic extraction, CTM/rotation, and fill-color tests split out of `extractors/text/tests.rs` for file size. ~keep

use super::super::*;
use crate::fonts::{Encoding, LazyCMap};

/// A condensed bold heading typeset with no space glyph — the
/// intra-word glyph gaps cluster near zero (tight/overlapping side-bearings)
/// while inter-word gaps sit at ~0.18 em. The split must land between the
/// clusters so a gap of ~0.18 em reads as a word boundary.
#[test]
fn test_bimodal_gap_split_heading() {
    // fs = 20.5; intra-word ~0/negative, inter-word ~3.7pt (0.18 em). ~keep
    let gaps = [-0.5, -0.7, -0.3, 3.72, 3.70, 3.68, -0.4, 3.71];
    let split = TextExtractor::bimodal_gap_split(&gaps, 20.5);
    let split = split.expect("clearly bimodal line must yield a split");
    assert!(
        split > 0.0 && split < 3.5,
        "split {split} must separate the ~0 and ~3.7pt clusters"
    );
}

/// A normally-spaced line (all gaps already a full word-space) is NOT
/// bimodal — there is no narrow-gap rescue to perform, so `None`.
#[test]
fn test_bimodal_gap_split_uniform_word_spacing_none() {
    let gaps = [6.0, 6.1, 5.9, 6.05, 5.95];
    assert!(TextExtractor::bimodal_gap_split(&gaps, 12.0).is_none());
}

/// A single word (all gaps intra-word, near zero) has no inter-word
/// cluster — must return `None`, never fabricate a boundary.
#[test]
fn test_bimodal_gap_split_single_word_none() {
    let gaps = [-0.5, -0.7, -0.3, 0.1, -0.4];
    assert!(TextExtractor::bimodal_gap_split(&gaps, 20.5).is_none());
}

/// Multi-level condensed footer: near-zero/overlapping intra-word gaps, a
/// NARROW ~0.10 em word gap (1.14 pt @ 11 pt), AND a wide ~0.25 em real
/// space (2.75 pt) on one line. The split must land just above the
/// intra-word cluster — below the narrow gap — so BOTH the narrow word gap
/// and the wide space read as boundaries (recovering `All` / `rights` in
/// `© ISO 2021 - All rights…`, matching pdfminer/poppler).
#[test]
fn test_bimodal_gap_split_multilevel_footer() {
    let gaps = [-0.1, -0.2, -0.15, 1.14, -0.1, -0.05, 2.75, -0.2];
    let split = TextExtractor::bimodal_gap_split(&gaps, 11.0).expect("a multi-level line must yield a split");
    assert!(
        split > 0.0 && split < 1.14,
        "split {split} must sit below the narrow 1.14pt word gap so both it and the wide space split"
    );
}

/// The narrow-gap rescue's math guard: a full subscript glyph occupying the
/// gap between a variable and the next symbol must be detected (suppress the
/// split, `λᵢr` stays whole), while a mere descender/ascender edge clipping
/// the gap band must NOT (so ordinary prose word gaps are still recovered).
#[test]
fn test_gap_has_intervening_glyph() {
    let r = |x, y, w, h| crate::geometry::Rect {
        x,
        y,
        width: w,
        height: h,
    };
    // `left` ends at x=10, `right` starts at x=24: a 14-unit gap on the
    // baseline band [0, 10]. ~keep
    let left = r(0.0, 0.0, 10.0, 10.0);
    let right = r(24.0, 0.0, 10.0, 10.0);
    // A subscript glyph centred in the gap (x 13..21 = 8 units ≈ 57% of the
    // 14-unit gap), shifted down but overlapping the band. ~keep
    let subscript = r(13.0, -3.0, 8.0, 8.0);
    assert!(
        gap_has_intervening_glyph(&[left, right, subscript], &left, &right),
        "a full subscript occupying the gap must be detected"
    );
    // A descender edge just clipping the gap (x 9..12 = only ~2 units into
    // the 14-unit gap, < 35%) must NOT count. ~keep
    let descender_edge = r(9.0, -4.0, 3.0, 6.0);
    assert!(
        !gap_has_intervening_glyph(&[left, right, descender_edge], &left, &right),
        "a descender edge clipping the gap must not be treated as an intervening glyph"
    );
}

/// The writing-axis continuation test, quadrant by quadrant.
///
/// Upright cases must be no stricter than the raw `e`/`f` tests they are
/// combined with — that implication is why unrotated output cannot move.
/// Rotated along-axis cases pin the helper alone: in the composed
/// predicate the raw `f` band still gates them, so there the helper is
/// veto-only.
#[test]
fn test_advances_along_writing_axis_by_quadrant() {
    let m = |a, b, c, d| Matrix {
        a,
        b,
        c,
        d,
        e: 100.0,
        f: 500.0,
    };
    let fs = 10.0;
    let at =
        |mat: Matrix, de: f32, df: f32| TextExtractor::advances_along_writing_axis(mat, 0, mat.e + de, mat.f + df, fs);

    let upright = m(1.0, 0.0, 0.0, 1.0);
    assert!(at(upright, 14.0, 0.0), "upright advance must continue");
    assert!(!at(upright, 0.0, -14.0), "upright line break must not");
    assert!(!at(upright, -14.0, 0.0), "upright backwards must not");
    // Perpendicular tolerance: 0.5 × font size (5pt here) admits a
    // sub-glyph baseline offset; a full line step is vetoed. ~keep
    assert!(at(upright, 14.0, 4.0), "upright sub-glyph offset must not be vetoed");
    assert!(!at(upright, 14.0, 8.0), "upright line step must be vetoed");

    // Advances along +y; lines separate along +x. ~keep
    let cw = m(0.0, 1.0, -1.0, 0.0);
    assert!(at(cw, 0.0, 14.0), "90° along-axis advance must not be vetoed");
    assert!(!at(cw, 14.0, 0.0), "90° line break must not continue");
    assert!(at(cw, -4.0, 14.0), "90° sub-glyph offset must not be vetoed");
    assert!(!at(cw, -8.0, 14.0), "90° line step must be vetoed");

    // Advances along -y; the sign a single-rotation fixture cannot catch. ~keep
    let ccw = m(0.0, -1.0, 1.0, 0.0);
    assert!(at(ccw, 0.0, -14.0), "270° along-axis advance must not be vetoed");
    assert!(!at(ccw, 0.0, 14.0), "270° backwards advance must not continue");
    assert!(!at(ccw, 14.0, 0.0), "270° line break must not continue");

    // 180°: advances along -x. ~keep
    let flip = m(-1.0, 0.0, 0.0, -1.0);
    assert!(at(flip, -14.0, 0.0), "180° along-axis advance must not be vetoed");
    assert!(!at(flip, 14.0, 0.0), "180° backwards advance must not continue");

    // No writing direction: falls back to +x, as before. ~keep
    let degenerate = m(0.0, 0.0, 0.0, 0.0);
    assert!(at(degenerate, 14.0, 0.0));
    assert!(!at(degenerate, -14.0, 0.0));

    // WMode 1 advances along (c, d), so this test never vetoes it. ~keep
    for (de, df) in [(14.0, 0.0), (0.0, 14.0), (-14.0, 0.0), (0.0, -14.0)] {
        assert!(
            TextExtractor::advances_along_writing_axis(cw, 1, cw.e + de, cw.f + df, fs),
            "vertical run vetoed at ({de}, {df})"
        );
    }
}

#[test]
fn test_snap_run_rotation() {
    let m = |a, b, c, d| Matrix {
        a,
        b,
        c,
        d,
        e: 0.0,
        f: 0.0,
    };
    // Horizontal identity-scale → 0.0 (byte-identical path). ~keep
    assert_eq!(snap_run_rotation(&m(12.0, 0.0, 0.0, 12.0)), 0.0);
    // Tiny float noise still counts as horizontal. ~keep
    assert_eq!(snap_run_rotation(&m(12.0, 1e-5, -1e-5, 12.0)), 0.0);
    // 90° CCW (a=0, b=+s, c=-s, d=0). ~keep
    assert_eq!(snap_run_rotation(&m(0.0, 12.0, -12.0, 0.0)), 90.0);
    // 270° / -90° (a=0, b=-s, c=+s, d=0). ~keep
    assert_eq!(snap_run_rotation(&m(0.0, -12.0, 12.0, 0.0)), -90.0);
    // 180° (a=-s, d=-s, b=c=0) must not alias to 0° — both have
    // b≈0, c≈0, so only the sign of `a` (cos 0° vs cos 180°)
    // distinguishes them. ~keep
    assert_eq!(snap_run_rotation(&m(-12.0, 0.0, 0.0, -12.0)), 180.0);
    // Tiny float noise on a 180° matrix still counts as 180°, not 0°. ~keep
    assert_eq!(snap_run_rotation(&m(-12.0, 1e-5, -1e-5, -12.0)), 180.0);
    // ~88° snaps to 90. ~keep
    let r = 12.0_f32;
    let th = 88.0_f32.to_radians();
    assert_eq!(
        snap_run_rotation(&m(r * th.cos(), r * th.sin(), -r * th.sin(), r * th.cos())),
        90.0
    );
    // 45° watermark is NOT snapped (kept as its own block downstream). ~keep
    let th = 45.0_f32.to_radians();
    let got = snap_run_rotation(&m(r * th.cos(), r * th.sin(), -r * th.sin(), r * th.cos()));
    assert!((got - 45.0).abs() < 0.5, "45° should pass through, got {got}");
}

pub(super) fn create_test_font() -> FontInfo {
    FontInfo {
        base_font: "Times-Roman".to_string(),
        subtype: "Type1".to_string(),
        encoding: Encoding::Standard("WinAnsiEncoding".to_string()),
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
        widths: None,
        first_char: None,
        last_char: None,
        font_matrix_a: 0.001,
        default_width: 1000.0,
        cid_to_gid_map: None,
        cid_system_info: None,
        cid_font_type: None,
        cid_widths: None,
        cid_default_width: 1000.0,
        has_explicit_dw: false,
        cff_gid_map: None,
        multi_char_map: HashMap::new(),
        byte_to_char_table: std::sync::OnceLock::new(),
        type0_unicode_memo: std::sync::Arc::new(std::sync::Mutex::new(std::collections::HashMap::new())),
        byte_to_width_table: std::sync::OnceLock::new(),
        weight_memo: std::sync::OnceLock::new(),
        italic_memo: std::sync::OnceLock::new(),
        std14_memo: std::sync::OnceLock::new(),
        diff_glyph_names: std::collections::HashMap::new(),
        wmode: 0,
        cid_vertical_metrics: None,
        cid_default_vertical_metrics: crate::fonts::VerticalMetrics::SPEC_DEFAULT,
        cjk_substitution: None,
        embedded_cid_map: None,
    }
}

#[test]
fn test_text_extractor_new() {
    let extractor = TextExtractor::new();
    assert_eq!(extractor.char_count(), 0);
}

#[test]
fn test_text_extractor_add_font() {
    let mut extractor = TextExtractor::new();
    let font = create_test_font();
    extractor.add_font("F1".to_string(), font);
    assert_eq!(extractor.fonts.len(), 1);
}

#[test]
fn test_extract_simple_text() {
    let mut extractor = TextExtractor::new();
    let font = create_test_font();
    extractor.add_font("F1".to_string(), font);

    let stream = b"BT /F1 12 Tf 100 700 Td (Hello) Tj ET";
    let chars = extractor.extract(stream).unwrap();

    assert_eq!(chars.len(), 5);
    assert_eq!(chars[0].char, 'H');
    assert_eq!(chars[1].char, 'e');
    assert_eq!(chars[2].char, 'l');
    assert_eq!(chars[3].char, 'l');
    assert_eq!(chars[4].char, 'o');
}

#[test]
fn test_extract_with_matrix() {
    let mut extractor = TextExtractor::new();
    let font = create_test_font();
    extractor.add_font("F1".to_string(), font);

    let stream = b"BT /F1 12 Tf 1 0 0 1 100 700 Tm (Hi) Tj ET";
    let chars = extractor.extract(stream).unwrap();

    assert_eq!(chars.len(), 2);
    assert_eq!(chars[0].char, 'H');
    assert_eq!(chars[1].char, 'i');
    assert!(chars[0].bbox.x >= 99.0 && chars[0].bbox.x <= 101.0);
}

/// Regression test: CTM must be applied to text positions
///
/// Per PDF Spec ISO 32000-1:2008 Section 9.4.4, the text rendering matrix is:
/// T_rm = [font_matrix] × T_m × CTM
///
/// This test verifies that when CTM contains a translation, text positions
/// are correctly transformed from text space to user space.
#[test]
fn test_ctm_applied_to_text_position() {
    let mut extractor = TextExtractor::new();
    let font = create_test_font();
    extractor.add_font("F1".to_string(), font);

    let stream = b"q 1 0 0 1 100 200 cm BT /F1 12 Tf (A) Tj ET Q";
    let chars = extractor.extract(stream).unwrap();

    assert_eq!(chars.len(), 1);
    assert_eq!(chars[0].char, 'A');
    assert!(
        chars[0].bbox.x >= 99.0 && chars[0].bbox.x <= 101.0,
        "X position should be ~100 (got {})",
        chars[0].bbox.x
    );
    assert!(
        chars[0].bbox.y >= 199.0 && chars[0].bbox.y <= 201.0,
        "Y position should be ~200 (got {})",
        chars[0].bbox.y
    );
}

/// Regression test: char mode must run the same parser as
/// span mode.
///
/// The stream forces the streaming parser's >256KB prescan route: the
/// rotating `q`/`cm` sits >4KB before `BT`, so the CTM reaches the text
/// region only via the forward scan's injected `Cm`. The unbalanced
/// literal-string opens are invisible to the prescan (they lie outside
/// every text region) but feed the old char-mode parser,
/// `parse_content_stream_text_only`, over `MAX_CONSECUTIVE_ERRORS`
/// consecutive scan failures — it bails before `BT` and extracts nothing.
#[test]
fn test_char_mode_rotated_ctm_survives_large_hostile_stream() {
    let mut extractor = TextExtractor::new();
    let font = create_test_font();
    extractor.add_font("F1".to_string(), font);

    let mut cs = Vec::new();
    cs.extend_from_slice(b"q\n0 1 -1 0 612 0 cm\n");
    cs.extend_from_slice(&[b'('; 1500]);
    cs.push(b'\n');
    for i in 0..13000u32 {
        let line = format!(
            "{}.0 {}.0 m {}.0 {}.0 l S\n",
            i % 500,
            (i * 7) % 500,
            (i * 3) % 500,
            (i * 11) % 500
        );
        cs.extend_from_slice(line.as_bytes());
    }
    assert!(cs.len() > 256 * 1024, "stream must exceed the 256KB prescan threshold");
    cs.extend_from_slice(b"BT /F1 12 Tf 100 200 Td (Hello) Tj ET\nQ\n");

    let chars = extractor.extract(&cs).unwrap();
    let mut glyphs: Vec<char> = chars.iter().map(|c| c.char).collect();
    glyphs.sort_unstable();
    assert_eq!(glyphs, vec!['H', 'e', 'l', 'l', 'o']);
    for c in &chars {
        assert!(
            (c.rotation_degrees - 90.0).abs() < 1.0,
            "expected 90 degrees from the cm before BT, got {}",
            c.rotation_degrees
        );
    }
}

/// Regression test: CTM scaling must affect text positions
///
/// This test verifies that CTM scaling is correctly applied to text positions.
#[test]
fn test_ctm_scaling_applied_to_text_position() {
    let mut extractor = TextExtractor::new();
    let font = create_test_font();
    extractor.add_font("F1".to_string(), font);

    let stream = b"q 2 0 0 2 0 0 cm BT /F1 12 Tf 1 0 0 1 50 100 Tm (B) Tj ET Q";
    let chars = extractor.extract(stream).unwrap();

    assert_eq!(chars.len(), 1);
    assert_eq!(chars[0].char, 'B');
    assert!(
        chars[0].bbox.x >= 99.0 && chars[0].bbox.x <= 101.0,
        "X position should be ~100 (got {})",
        chars[0].bbox.x
    );
    assert!(
        chars[0].bbox.y >= 199.0 && chars[0].bbox.y <= 201.0,
        "Y position should be ~200 (got {})",
        chars[0].bbox.y
    );
}

/// Regression test: Combined CTM translation and text matrix
///
/// This test verifies the complete transformation chain works correctly.
#[test]
fn test_ctm_combined_with_text_matrix() {
    let mut extractor = TextExtractor::new();
    let font = create_test_font();
    extractor.add_font("F1".to_string(), font);

    let stream = b"q 1 0 0 1 50 50 cm BT /F1 12 Tf 1 0 0 1 25 25 Tm (C) Tj ET Q";
    let chars = extractor.extract(stream).unwrap();

    assert_eq!(chars.len(), 1);
    assert_eq!(chars[0].char, 'C');
    assert!(
        chars[0].bbox.x >= 74.0 && chars[0].bbox.x <= 76.0,
        "X position should be ~75 (got {})",
        chars[0].bbox.x
    );
    assert!(
        chars[0].bbox.y >= 74.0 && chars[0].bbox.y <= 76.0,
        "Y position should be ~75 (got {})",
        chars[0].bbox.y
    );
}

#[test]
fn test_extract_with_tj_array() {
    let mut extractor = TextExtractor::new();
    let font = create_test_font();
    extractor.add_font("F1".to_string(), font);

    let stream = b"BT /F1 12 Tf 0 0 Td [(H)(i)] TJ ET";
    let chars = extractor.extract(stream).unwrap();

    assert_eq!(chars.len(), 2);
    assert_eq!(chars[0].char, 'H');
    assert_eq!(chars[1].char, 'i');
}

/// Test extraction of multi-byte characters from Type0 fonts (Identity-H)
/// This verifies the fix for extract_chars() garbling CJK text.
#[test]
fn test_extract_type0_multibyte_character_extraction() {
    let mut extractor = TextExtractor::new();

    let mut font = create_test_font();
    font.subtype = "Type0".to_string();
    font.encoding = Encoding::Standard("Identity-H".to_string());

    // Create a valid ToUnicode CMap stream that maps CID 0x4E2D to '中' and 0x6587 to '文' ~keep
    let cmap_data = b"
            /CIDInit /ProcSet findresource begin
            12 dict begin
            begincmap
            /CIDSystemInfo << /Registry (Adobe) /Ordering (UCS) /Supplement 0 >> def
            /CMapName /Adobe-Identity-UCS def
            /CMapType 2 def
            1 begincodespacerange <0000> <FFFF> endcodespacerange
            2 beginbfchar
            <4E2D> <4E2D>
            <6587> <6587>
            endbfchar
            endcmap
            CMapName currentdict /CMap defineresource pop
            end
            end
        ";

    let lazy_cmap = LazyCMap::new(cmap_data.to_vec());
    font.to_unicode = Some(lazy_cmap);

    extractor.add_font("F1".to_string(), font);

    // Content stream with 2-byte CIDs for "中文" (0x4E2D 0x6587) ~keep
    let stream = b"BT /F1 12 Tf 0 0 Td <4E2D6587> Tj ET";
    let chars = extractor.extract(stream).unwrap();

    assert_eq!(chars.len(), 2);
    assert_eq!(chars[0].char, '中');
    assert_eq!(chars[1].char, '文');
}

/// A Type0 font whose ToUnicode CMap covers only some codes: the glyphs without a Unicode
/// value stay in the span text as U+FFFD, so the OCR routing gates can count them.
#[test]
fn type0_glyphs_without_unicode_keep_a_replacement_character() {
    let mut extractor = TextExtractor::new();
    let mut font = create_test_font();
    font.subtype = "Type0".to_string();
    font.encoding = Encoding::Identity;
    font.to_unicode = Some(LazyCMap::new(type0_partial_to_unicode()));
    extractor.add_font("F1".to_string(), font);

    let stream = b"BT /F1 12 Tf 0 0 Td <004100420043> Tj ET";
    let spans = extractor.extract_text_spans(stream).unwrap();
    let text: String = spans.iter().map(|s| s.text.as_str()).collect();

    assert_eq!(text, "A\u{FFFD}\u{FFFD}");
}

/// A ToUnicode target that is a bare control character is the same loss as a missing entry.
#[test]
fn type0_nul_to_unicode_target_keeps_a_replacement_character() {
    let mut extractor = TextExtractor::new();
    let mut font = create_test_font();
    font.subtype = "Type0".to_string();
    font.encoding = Encoding::Identity;
    font.to_unicode = Some(LazyCMap::new(type0_partial_to_unicode()));
    extractor.add_font("F1".to_string(), font);

    let stream = b"BT /F1 12 Tf 0 0 Td <0044> Tj ET";
    let spans = extractor.extract_text_spans(stream).unwrap();
    let text: String = spans.iter().map(|s| s.text.as_str()).collect();

    assert_eq!(text, "\u{FFFD}");
}

/// Control: a simple font keeps dropping a glyph that maps to NUL, so only Type0 text changes.
#[test]
fn simple_font_unmapped_glyph_is_still_dropped() {
    let mut extractor = TextExtractor::new();
    let mut font = create_test_font();
    font.encoding = Encoding::Custom(HashMap::new());
    font.to_unicode = Some(LazyCMap::new(type0_partial_to_unicode()));
    extractor.add_font("F1".to_string(), font);

    let stream = b"BT /F1 12 Tf 0 0 Td <44> Tj ET";
    let spans = extractor.extract_text_spans(stream).unwrap();
    let text: String = spans.iter().map(|s| s.text.as_str()).collect();

    assert_eq!(text, "", "simple font text changed");
}

/// ToUnicode CMap: code 0x0041 maps to "A", code 0x0044 maps to NUL, other codes have no entry.
fn type0_partial_to_unicode() -> Vec<u8> {
    b"
        /CIDInit /ProcSet findresource begin
        12 dict begin
        begincmap
        /CMapName /Test-UCS def
        /CMapType 2 def
        1 begincodespacerange <0000> <FFFF> endcodespacerange
        2 beginbfchar
        <0041> <0041>
        <0044> <0000>
        endbfchar
        endcmap
        end
        end
    "
    .to_vec()
}

#[test]
fn test_extract_color() {
    let mut extractor = TextExtractor::new();
    let font = create_test_font();
    extractor.add_font("F1".to_string(), font);

    let stream = b"BT 1 0 0 rg /F1 12 Tf 0 0 Td (R) Tj ET";
    let chars = extractor.extract(stream).unwrap();

    assert_eq!(chars.len(), 1);
    assert_eq!(chars[0].char, 'R');
    assert_eq!(chars[0].color.r, 1.0);
    assert_eq!(chars[0].color.g, 0.0);
    assert_eq!(chars[0].color.b, 0.0);
}

/// Regression test for a text-only-parser bug where a fill colour set by
/// `scn` *before* the enclosing `BT` was silently dropped, leaving the
/// text drawn in the GraphicsState default (black) instead of the
/// colour the content stream actually requested.
///
/// Root cause: `scan_graphics_region()` (src/content/parser.rs) is used
/// by `parse_and_execute_text_only()` to fast-scan non-text regions
/// looking for the next `BT`. It classified `scn`/`cs`/`sc`/`rg`/`g`/`k`
/// (and friends) as unconditionally "skippable" - correct only when a
/// matching `Q` is guaranteed to revert the change before any `BT`, but
/// wrong at the top level (outside any q/Q scope), where the colour
/// change legitimately persists into the next text object per
/// ISO 32000-1:2008 SS8.4. Reproduces the exact operator sequence found
/// on a real-world govdocs1 slide-deck PDF: a marked-content BDC opens,
/// `scn` sets a blue fill colour *outside* any text object, then `BT`
/// opens the text object that draws the (should-be-blue) heading.
#[test]
fn test_fill_color_scn_before_bt_after_bdc_not_dropped() {
    let mut extractor = TextExtractor::new();
    let font = create_test_font();
    extractor.add_font("F1".to_string(), font);

    let stream = b"/Shape <</MCID 3 >>BDC \
                        0.2 0.2 0.604 scn \
                        BT /F1 12 Tf 100 700 Td (Blue Heading) Tj ET \
                        EMC";
    let spans = extractor.extract_text_spans(stream).unwrap();

    assert_eq!(spans.len(), 1);
    assert!(
        (spans[0].color.r - 0.2).abs() < 0.01,
        "expected blue fill (0.2, 0.2, 0.604), got {:?}",
        spans[0].color
    );
    assert!((spans[0].color.g - 0.2).abs() < 0.01);
    assert!((spans[0].color.b - 0.604).abs() < 0.01);
}

/// Same bug, second real-world pattern: a `Q` (RestoreGraphicsState)
/// immediately precedes the out-of-text-object `scn`. Reproduces the
/// gold author-block sequence from the same source PDF.
#[test]
fn test_fill_color_scn_after_q_before_bt_not_dropped() {
    let mut extractor = TextExtractor::new();
    let font = create_test_font();
    extractor.add_font("F1".to_string(), font);

    let stream = b"q 1 0 0 1 0 0 cm Q \
                        1 1 0 scn \
                        BT /F1 12 Tf 100 700 Td (Gold Author) Tj ET";
    let spans = extractor.extract_text_spans(stream).unwrap();

    assert_eq!(spans.len(), 1);
    assert!(
        (spans[0].color.r - 1.0).abs() < 0.01,
        "expected gold fill (1, 1, 0), got {:?}",
        spans[0].color
    );
    assert!((spans[0].color.g - 1.0).abs() < 0.01);
    assert!((spans[0].color.b - 0.0).abs() < 0.01);
}

/// Must-not-regress guard: `scn` issued *inside* an already-open text
/// object (continuing after a prior `Tj`, still within the same BT/ET)
/// always worked correctly - it goes through the ordinary text-operator
/// parse path, not the non-text `scan_graphics_region` fast scanner.
/// Confirms the fix above did not disturb this working case.
#[test]
fn test_fill_color_scn_inside_open_text_object_still_works() {
    let mut extractor = TextExtractor::new();
    let font = create_test_font();
    extractor.add_font("F1".to_string(), font);

    let stream = b"BT /F1 12 Tf 100 700 Td (Black Text) Tj \
                        0.2 0.2 0.604 scn \
                        0 -20 Td (Blue Text) Tj ET";
    let spans = extractor.extract_text_spans(stream).unwrap();

    assert_eq!(spans.len(), 2);
    assert!(
        (spans[0].color.r - 0.0).abs() < 0.01 && (spans[0].color.b - 0.0).abs() < 0.01,
        "first run should still be default black, got {:?}",
        spans[0].color
    );
    assert!(
        (spans[1].color.r - 0.2).abs() < 0.01,
        "second run should be blue (0.2, 0.2, 0.604), got {:?}",
        spans[1].color
    );
    assert!((spans[1].color.g - 0.2).abs() < 0.01);
    assert!((spans[1].color.b - 0.604).abs() < 0.01);
}

/// Regression test: is_monospace flag must propagate from FontInfo flags
/// through TjBuffer into the final TextSpan.
///
/// When font descriptor flags have bit 0 (FixedPitch) set, spans produced
/// by extract_text_spans() must report is_monospace == true.
/// Conversely, a proportional font (e.g. Helvetica) must yield false.
#[test]
fn test_is_monospace_from_font_flags() {
    let mut mono_font = create_test_font();
    mono_font.base_font = "Courier".to_string();
    mono_font.flags = Some(1); // bit 0 = FixedPitch ~keep

    let mut extractor = TextExtractor::new();
    extractor.add_font("F1".to_string(), mono_font);

    let stream = b"BT /F1 12 Tf 100 700 Td (Code) Tj ET";
    let spans = extractor.extract_text_spans(stream).unwrap();

    assert!(!spans.is_empty(), "should produce at least one span");
    assert!(
        spans[0].is_monospace,
        "Courier with FixedPitch flag should be monospace, got is_monospace=false"
    );

    let mut prop_font = create_test_font();
    prop_font.base_font = "Helvetica".to_string();
    prop_font.flags = Some(0); // no FixedPitch ~keep

    let mut extractor2 = TextExtractor::new();
    extractor2.add_font("F2".to_string(), prop_font);

    let stream2 = b"BT /F2 12 Tf 100 700 Td (Text) Tj ET";
    let spans2 = extractor2.extract_text_spans(stream2).unwrap();

    assert!(!spans2.is_empty(), "should produce at least one span");
    assert!(
        !spans2[0].is_monospace,
        "Helvetica without FixedPitch flag should not be monospace"
    );

    let mut mono_name_font = create_test_font();
    mono_name_font.base_font = "DejaVuSansMono".to_string();
    mono_name_font.flags = None;

    let mut extractor3 = TextExtractor::new();
    extractor3.add_font("F3".to_string(), mono_name_font);

    let stream3 = b"BT /F3 12 Tf 100 700 Td (Mono) Tj ET";
    let spans3 = extractor3.extract_text_spans(stream3).unwrap();

    assert!(!spans3.is_empty(), "should produce at least one span");
    assert!(
        spans3[0].is_monospace,
        "Font named DejaVuSansMono should be detected as monospace via name heuristic"
    );
}

/// Regression test: a `/PlacedPDF` marked-content scope
/// whose `BDC` lands inside the first prescanned (>256KB fast-path)
/// text region but whose matching `EMC` falls outside it — past tens
/// of thousands of bytes of artwork the prescan never turns into a
/// text region — must not suppress text in every subsequent region.
/// Before the fix, `inside_placed_pdf` stayed `true` forever once the
/// scope's `EMC` fell outside a prescanned region's byte range, since
/// the marked-content stack (unlike CTM/font) got no
/// per-region balancing.
#[test]
fn test_prescan_marked_content_scope_does_not_leak_across_regions() {
    let mut cs = Vec::new();
    cs.extend_from_slice(b"/PlacedPDF /MC0 BDC\n");
    cs.extend_from_slice(b"BT /F1 12 Tf 100 700 Td (Figure Label) Tj ET\n");
    // >256KB of filler path data (the artwork) with no BT/Do at all,
    // so the prescan never turns it into its own text region — the
    // EMC below lands in the gap between the two BT regions. ~keep
    for i in 0..13000u32 {
        let line = format!(
            "{}.0 {}.0 m {}.0 {}.0 l n\n",
            i % 500,
            (i * 7) % 500,
            (i * 3) % 500,
            (i * 11) % 500
        );
        cs.extend_from_slice(line.as_bytes());
    }
    cs.extend_from_slice(b"EMC\n");
    cs.extend_from_slice(b"BT /F1 12 Tf 100 600 Td (Body Text After Figure) Tj ET\n");
    assert!(cs.len() > 256 * 1024, "stream must exceed 256KB prescan threshold");

    let font = create_test_font();
    let mut extractor = TextExtractor::new();
    extractor.add_font("F1".to_string(), font);

    let spans = extractor.extract_text_spans(&cs).unwrap();
    let all_text: String = spans.iter().map(|s| s.text.as_str()).collect::<Vec<_>>().join(" ");

    assert!(
        !all_text.contains("Figure Label"),
        "text inside the /PlacedPDF scope must still be suppressed, got: {all_text:?}"
    );
    assert!(
        all_text.contains("Body Text After Figure"),
        "text after the /PlacedPDF scope closes (EMC) must not be suppressed \
             just because the EMC fell outside the prescanned region, got: {all_text:?}"
    );
}

/// Regression test: invisible text (Tr 3/7) must never
/// be classified monospace, even under a FixedPitch-flagged or
/// "GlyphLessFont"-named font. Such text is an OCR text-sandwich layer
/// sitting under a scanned page image — a synthetic OCR font commonly
/// sets FixedPitch purely for positioning simplicity, since the glyphs
/// are never rendered. Markdown conversion uses `is_monospace` to fence
/// a paragraph as a code block; without this gate, a scanned novel's
/// OCR'd dialogue trips FixedPitch and gets served as a code block.
#[test]
fn test_invisible_text_is_never_monospace() {
    // Invisible render mode (Tr 3) + FixedPitch-flagged font: must NOT
    // be monospace despite the flag. ~keep
    let mut ocr_font = create_test_font();
    ocr_font.base_font = "GlyphLessFont".to_string();
    ocr_font.flags = Some(1); // bit 0 = FixedPitch ~keep

    let mut extractor = TextExtractor::new();
    extractor.add_font("F1".to_string(), ocr_font);

    let stream = b"BT /F1 12 Tf 3 Tr 100 700 Td (\"I don't know,\" she said.) Tj ET";
    let spans = extractor.extract_text_spans(stream).unwrap();

    assert!(!spans.is_empty(), "should produce at least one span");
    assert!(
        !spans[0].is_monospace,
        "invisible (Tr 3) text under a FixedPitch OCR font must not be \
             classified monospace"
    );

    // Control: the SAME FixedPitch font, but VISIBLE (default Tr 0),
    // must still be classified monospace — the gate must not
    // over-suppress real code/monospace content. ~keep
    let mut visible_font = create_test_font();
    visible_font.base_font = "Courier".to_string();
    visible_font.flags = Some(1);

    let mut extractor2 = TextExtractor::new();
    extractor2.add_font("F2".to_string(), visible_font);

    let stream2 = b"BT /F2 12 Tf 100 700 Td (let x = 1;) Tj ET";
    let spans2 = extractor2.extract_text_spans(stream2).unwrap();

    assert!(!spans2.is_empty(), "should produce at least one span");
    assert!(
        spans2[0].is_monospace,
        "visible FixedPitch text must still be classified monospace"
    );
}

#[test]
fn test_extract_save_restore() {
    let mut extractor = TextExtractor::new();
    let font = create_test_font();
    extractor.add_font("F1".to_string(), font);

    let stream = b"BT /F1 12 Tf q /F1 14 Tf (A) Tj Q (B) Tj ET";
    let chars = extractor.extract(stream).unwrap();

    assert_eq!(chars.len(), 2);
    assert_eq!(chars[0].font_size, 14.0);
    assert_eq!(chars[1].font_size, 12.0);
}

#[test]
fn test_extract_no_font() {
    let mut extractor = TextExtractor::new();

    let stream = b"BT /F1 12 Tf (ABC) Tj ET";
    let chars = extractor.extract(stream).unwrap();

    assert_eq!(chars.len(), 3);
}

#[test]
fn test_char_count() {
    let mut extractor = TextExtractor::new();
    assert_eq!(extractor.char_count(), 0);

    let font = create_test_font();
    extractor.add_font("F1".to_string(), font);

    let stream = b"BT /F1 12 Tf (Test) Tj ET";
    extractor.extract(stream).unwrap();
    assert_eq!(extractor.char_count(), 4);
}

#[test]
fn test_clear() {
    let mut extractor = TextExtractor::new();
    let font = create_test_font();
    extractor.add_font("F1".to_string(), font);

    let stream = b"BT /F1 12 Tf (Test) Tj ET";
    extractor.extract(stream).unwrap();
    assert_eq!(extractor.char_count(), 4);

    extractor.clear();
    assert_eq!(extractor.char_count(), 0);
}

#[test]
fn test_default() {
    let extractor = TextExtractor::default();
    assert_eq!(extractor.char_count(), 0);
}

/// Test unified space decision: Boundary space already present
#[test]
fn test_space_decision_boundary_space() {
    let config = SpanMergingConfig::default();
    let fonts = std::collections::HashMap::new();

    let decision = should_insert_space(
        "word ", "next", 0.0, 12.0, "TestFont", &fonts, false, &config, None, None, 12.0, 12.0,
    );
    assert!(!decision.insert_space);
    assert_eq!(decision.source, SpaceSource::AlreadyPresent);

    let decision = should_insert_space(
        "word", " next", 0.0, 12.0, "TestFont", &fonts, false, &config, None, None, 12.0, 12.0,
    );
    assert!(!decision.insert_space);
    assert_eq!(decision.source, SpaceSource::AlreadyPresent);
}

/// Regression test:
/// a long number emitted as multiple digit-only spans with a kerning-sized
/// positive gap must NOT have a space inserted between the digits (would
/// turn "123456" into "123 456"). Adjacent table cell digit values with a
/// larger gap must still be separated.
#[test]
fn test_space_decision_digit_digit_gap_threshold() {
    let config = SpanMergingConfig::default();
    let fonts = std::collections::HashMap::new();

    // Kerning-sized gap (0.3pt) between digit spans — must NOT insert.
    // For 12pt font with no font-info fallback, geometric_threshold is
    // typically around 1.5pt, so half of that is 0.75pt. ~keep
    let kerning = should_insert_space(
        "123", "456", 0.3, 12.0, "TestFont", &fonts, false, &config, None, None, 12.0, 12.0,
    );
    assert!(
        !kerning.insert_space,
        "Kerning-sized gap (0.3pt) between digits must not split the number, got: {:?}",
        kerning
    );

    // Larger gap (2pt) between digit spans — adjacent table cell values,
    // must still insert a space. ~keep
    let table_cells = should_insert_space(
        "123", "456", 2.0, 12.0, "TestFont", &fonts, false, &config, None, None, 12.0, 12.0,
    );
    assert!(
        table_cells.insert_space,
        "2pt gap between digits should still split adjacent table values, got: {:?}",
        table_cells
    );
}

/// Test split boundary merging with space insertion
///
/// When split_boundary_before=true, it indicates the span is part of a boundary
/// that was previously split (e.g., from CamelCase fusion like "theGeneral").
/// These spans should be merged WITH a space to preserve word separation.
#[test]
fn test_split_boundary_merges_with_space() {
    // Only the fields the assertion cares about are set explicitly; every other
    // field matches `TextSpan::default()` exactly (verified against the impl in
    // layout/text_block.rs), so this is byte-identical to the previous
    // fully-spelled-out literals. ~keep
    let spans = vec![
        TextSpan {
            text: "the".to_string(),
            bbox: Rect {
                x: 0.0,
                y: 100.0,
                width: 10.0,
                height: 12.0,
            },
            font_name: "Arial".to_string(),
            sequence: 0,
            split_boundary_before: false,
            ..TextSpan::default()
        },
        TextSpan {
            text: "General".to_string(),
            bbox: Rect {
                x: 10.0,
                y: 100.0,
                width: 25.0,
                height: 12.0,
            },
            font_name: "Arial".to_string(),
            sequence: 1,
            split_boundary_before: true,
            ..TextSpan::default()
        },
    ];

    let mut extractor = TextExtractor::new();
    extractor.spans = spans;
    extractor.merging_config = SpanMergingConfig::default();

    extractor.merge_adjacent_spans();

    // Per PDF Spec ISO 32000-1:2008 Section 9.4.4 and implementation design: ~keep
    // split_boundary_before=true means "merge with a space, never without" ~keep
    // This ensures "length" + "This" becomes "length This" not "lengthThis" ~keep
    // The spans are merged INTO ONE span with space-separated text ~keep
    assert_eq!(extractor.spans.len(), 1);
    assert_eq!(extractor.spans[0].text, "the General");
}

// Removed: test_should_insert_space_heuristic - function doesn't exist in current codebase
// ~keep

/// Test boundary space detection
#[test]
fn test_has_boundary_space() {
    assert!(has_boundary_space("word ", "next"));

    assert!(has_boundary_space("word", " next"));

    assert!(has_boundary_space("word ", " next"));

    assert!(!has_boundary_space("word", "next"));

    assert!(has_boundary_space("word\t", "next"));
    assert!(has_boundary_space("word\n", "next"));
    assert!(has_boundary_space("word", "\tnext"));
}

#[test]
fn test_text_extraction_config_new_defaults() {
    let config = TextExtractionConfig::new();
    assert_eq!(config.space_insertion_threshold, -120.0);
    assert_eq!(config.word_margin_ratio, 0.1);
    assert!(!config.use_adaptive_tj_threshold);
    assert!(config.profile.is_none());
}

#[test]
fn test_text_extraction_config_with_space_threshold() {
    let config = TextExtractionConfig::with_space_threshold(-80.0);
    assert_eq!(config.space_insertion_threshold, -80.0);
    assert_eq!(config.word_margin_ratio, 0.1);
    assert!(!config.use_adaptive_tj_threshold);
}

#[test]
fn test_text_extraction_config_with_word_margin_ratio() {
    let config = TextExtractionConfig::with_word_margin_ratio(0.15);
    assert_eq!(config.word_margin_ratio, 0.15);
    assert!(config.use_adaptive_tj_threshold);
    assert_eq!(config.space_insertion_threshold, -120.0); // fallback ~keep
}
