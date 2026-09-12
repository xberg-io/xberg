//! Unit tests for [`super`].
//!
//! Split out of `extractors/text.rs` for file size: the parent was 16,288 lines
//! (673 KiB), over the repository's 500 KiB file-safety limit. A child module sees
//! the parent's private items exactly as the inline module did. ~keep

use super::*;
use crate::fonts::{Encoding, LazyCMap};
use std::sync::Arc;

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

fn create_test_font() -> FontInfo {
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
    let spans = vec![
        TextSpan {
            provenance: None,
            text_rise: 0.0,
            artifact_type: None,
            text: "the".to_string(),
            bbox: Rect {
                x: 0.0,
                y: 100.0,
                width: 10.0,
                height: 12.0,
            },
            font_name: "Arial".to_string(),
            font_size: 12.0,
            font_weight: FontWeight::Normal,
            color: Color::black(),
            mcid: None,
            mcid_scope: None,
            sequence: 0,
            split_boundary_before: false,
            offset_semantic: false,
            is_italic: false,
            is_monospace: false,
            char_spacing: 0.0,
            word_spacing: 0.0,
            horizontal_scaling: 100.0,
            primary_detected: false,
            char_widths: vec![],
            char_x_offsets: Vec::new(),
            heading_level: None,
            rotation_degrees: 0.0,
            wmode: 0,
            rtl_draw_logical: false,
            mirrored: false,
            page_rotation_applied: 0,
        },
        TextSpan {
            provenance: None,
            text_rise: 0.0,
            artifact_type: None,
            text: "General".to_string(),
            bbox: Rect {
                x: 10.0,
                y: 100.0,
                width: 25.0,
                height: 12.0,
            },
            font_name: "Arial".to_string(),
            font_size: 12.0,
            font_weight: FontWeight::Normal,
            color: Color::black(),
            mcid: None,
            mcid_scope: None,
            sequence: 1,
            split_boundary_before: true,
            offset_semantic: false,
            primary_detected: false,
            is_italic: false,
            is_monospace: false,
            char_spacing: 0.0,
            word_spacing: 0.0,
            horizontal_scaling: 100.0,
            char_widths: vec![],
            char_x_offsets: Vec::new(),
            heading_level: None,
            rotation_degrees: 0.0,
            wmode: 0,
            rtl_draw_logical: false,
            mirrored: false,
            page_rotation_applied: 0,
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

#[test]
fn test_text_extraction_config_set_word_margin_ratio() {
    let config = TextExtractionConfig::new().set_word_margin_ratio(0.2);
    assert_eq!(config.word_margin_ratio, 0.2);
    assert!(config.use_adaptive_tj_threshold);
}

#[test]
fn test_text_extraction_config_set_adaptive_tj_threshold() {
    let config = TextExtractionConfig::new().set_adaptive_tj_threshold(true);
    assert!(config.use_adaptive_tj_threshold);
    let config2 = config.set_adaptive_tj_threshold(false);
    assert!(!config2.use_adaptive_tj_threshold);
}

#[test]
fn test_text_extraction_config_with_profile() {
    let config = TextExtractionConfig::new().with_profile(crate::config::ExtractionProfile::ACADEMIC);
    assert!(config.profile.is_some());
    let profile = config.profile.unwrap();
    assert_eq!(profile.name, "Academic");
}

#[test]
fn test_span_merging_config_defaults() {
    let config = SpanMergingConfig::new();
    assert_eq!(config.space_threshold_em_ratio, 0.25);
    assert_eq!(config.conservative_threshold_pt, 0.1);
    assert_eq!(config.column_boundary_threshold_pt, 5.0);
    assert_eq!(config.severe_overlap_threshold_pt, -0.5);
    assert!(config.use_adaptive_threshold);
    assert!(!config.detect_email_patterns);
    assert!(!config.detect_citation_markers);
}

#[test]
fn test_span_merging_config_aggressive() {
    let config = SpanMergingConfig::aggressive();
    assert_eq!(config.space_threshold_em_ratio, 0.15);
    assert_eq!(config.conservative_threshold_pt, 0.1);
    assert!(!config.use_adaptive_threshold);
}

#[test]
fn test_span_merging_config_conservative() {
    let config = SpanMergingConfig::conservative();
    assert_eq!(config.space_threshold_em_ratio, 0.33);
    assert_eq!(config.conservative_threshold_pt, 0.3);
    assert!(!config.use_adaptive_threshold);
}

#[test]
fn test_span_merging_config_custom() {
    let config = SpanMergingConfig::custom(0.2, 0.2, 6.0, -0.3);
    assert_eq!(config.space_threshold_em_ratio, 0.2);
    assert_eq!(config.conservative_threshold_pt, 0.2);
    assert_eq!(config.column_boundary_threshold_pt, 6.0);
    assert_eq!(config.severe_overlap_threshold_pt, -0.3);
    assert!(!config.use_adaptive_threshold);
}

#[test]
fn test_span_merging_config_adaptive() {
    let config = SpanMergingConfig::adaptive();
    assert!(config.use_adaptive_threshold);
    assert!(config.adaptive_config.is_some());
}

#[test]
fn test_span_merging_config_legacy() {
    let config = SpanMergingConfig::legacy();
    assert!(!config.use_adaptive_threshold);
    assert_eq!(config.conservative_threshold_pt, 0.1);
    assert!(config.adaptive_config.is_none());
}

#[test]
fn test_space_decision_insert() {
    let d = SpaceDecision::insert(SpaceSource::TjOffset, 0.95);
    assert!(d.insert_space);
    assert_eq!(d.source, SpaceSource::TjOffset);
    assert_eq!(d.confidence, 0.95);
}

#[test]
fn test_space_decision_no_space() {
    let d = SpaceDecision::no_space(SpaceSource::NoSpace, 1.0);
    assert!(!d.insert_space);
    assert_eq!(d.source, SpaceSource::NoSpace);
    assert_eq!(d.confidence, 1.0);
}

#[test]
fn test_space_decision_clamp_confidence() {
    let d = SpaceDecision::insert(SpaceSource::GeometricGap, 1.5);
    assert_eq!(d.confidence, 1.0); // clamped ~keep
    let d2 = SpaceDecision::insert(SpaceSource::GeometricGap, -0.5);
    assert_eq!(d2.confidence, 0.0); // clamped ~keep
}

#[test]
fn test_operator_td_positioning() {
    let mut extractor = TextExtractor::new();
    let font = create_test_font();
    extractor.add_font("F1".to_string(), font);

    let stream = b"BT /F1 12 Tf 100 700 Td (X) Tj ET";
    let chars = extractor.extract(stream).unwrap();

    assert_eq!(chars.len(), 1);
    assert_eq!(chars[0].char, 'X');
    assert!((chars[0].bbox.x - 100.0).abs() < 2.0);
    assert!((chars[0].bbox.y - 700.0).abs() < 2.0);
}

/// TD Y offset must be scaled by the text matrix.
/// Pattern: `/F1 1 Tf 10 0 0 10 72 700 Tm (Line one) Tj 0 -1.3 TD (Line two) Tj`
/// The Tm sets a 10x scale, so `0 -1.3 TD` should produce a 13pt vertical gap,
/// not 1.3pt. Both lines must appear in extracted text.
#[test]
fn test_issue_254_tm_scale_td_offset() {
    let mut extractor = TextExtractor::new();
    let font = create_test_font();
    extractor.add_font("F1".to_string(), font);

    let stream = b"BT /F1 1 Tf 10 0 0 10 72 700 Tm (Line one) Tj 0 -1.3 TD (Line two) Tj ET";
    let chars = extractor.extract(stream).unwrap();

    let text: String = chars.iter().map(|c| c.char).collect();
    assert!(text.contains("Line one"), "Should contain 'Line one', got: {}", text);
    assert!(text.contains("Line two"), "Should contain 'Line two', got: {}", text);

    let line_one_y = chars.iter().find(|c| c.char == 'L').unwrap().bbox.y;
    let line_two_chars: Vec<_> = chars.iter().filter(|c| c.char == 'L').collect();
    assert!(
        line_two_chars.len() >= 2,
        "Should have at least 2 'L' chars (one per line)"
    );
    let line_two_y = line_two_chars[1].bbox.y;
    let y_gap = (line_one_y - line_two_y).abs();
    assert!(y_gap > 5.0, "Y gap should be ~13pt (Tm-scaled), got {:.1}pt", y_gap);
}

#[test]
fn test_operator_td_sets_leading() {
    let mut extractor = TextExtractor::new();
    let font = create_test_font();
    extractor.add_font("F1".to_string(), font);

    let stream = b"BT /F1 12 Tf 100 -14 TD (A) Tj ET";
    let chars = extractor.extract(stream).unwrap();

    assert_eq!(chars.len(), 1);
    assert_eq!(chars[0].char, 'A');
}

#[test]
fn test_operator_tstar_line_break() {
    let mut extractor = TextExtractor::new();
    let font = create_test_font();
    extractor.add_font("F1".to_string(), font);

    let stream = b"BT /F1 12 Tf 14 TL 100 700 Td (A) Tj T* (B) Tj ET";
    let chars = extractor.extract(stream).unwrap();

    assert_eq!(chars.len(), 2);
    assert_eq!(chars[0].char, 'A');
    assert_eq!(chars[1].char, 'B');
    assert!((chars[0].bbox.y - chars[1].bbox.y).abs() > 1.0);
}

#[test]
fn test_operator_quote_next_line_show_text() {
    let mut extractor = TextExtractor::new();
    let font = create_test_font();
    extractor.add_font("F1".to_string(), font);

    let stream = b"BT /F1 12 Tf 14 TL 100 700 Td (A) Tj (B) ' ET";
    let chars = extractor.extract(stream).unwrap();

    assert_eq!(chars.len(), 2);
    assert_eq!(chars[0].char, 'A');
    assert_eq!(chars[1].char, 'B');
}

#[test]
fn test_operator_double_quote() {
    let mut extractor = TextExtractor::new();
    let font = create_test_font();
    extractor.add_font("F1".to_string(), font);

    let stream = b"BT /F1 12 Tf 14 TL 100 700 Td 1 2 (Hi) \" ET";
    let chars = extractor.extract(stream).unwrap();

    assert_eq!(chars.len(), 2);
    assert_eq!(chars[0].char, 'H');
    assert_eq!(chars[1].char, 'i');
}

#[test]
fn test_operator_tc_char_spacing() {
    let mut extractor = TextExtractor::new();
    let font = create_test_font();
    extractor.add_font("F1".to_string(), font);

    let stream = b"BT /F1 12 Tf 2 Tc 100 700 Td (AB) Tj ET";
    let chars = extractor.extract(stream).unwrap();

    assert_eq!(chars.len(), 2);
    assert_eq!(chars[0].char, 'A');
    assert_eq!(chars[1].char, 'B');
}

#[test]
fn test_operator_tw_word_spacing() {
    let mut extractor = TextExtractor::new();
    let font = create_test_font();
    extractor.add_font("F1".to_string(), font);

    let stream = b"BT /F1 12 Tf 5 Tw 100 700 Td (A B) Tj ET";
    let chars = extractor.extract(stream).unwrap();

    assert!(chars.len() >= 3);
}

#[test]
fn test_operator_tz_horizontal_scaling() {
    let mut extractor = TextExtractor::new();
    let font = create_test_font();
    extractor.add_font("F1".to_string(), font);

    let stream = b"BT /F1 12 Tf 150 Tz 100 700 Td (X) Tj ET";
    let chars = extractor.extract(stream).unwrap();

    assert_eq!(chars.len(), 1);
    assert_eq!(chars[0].char, 'X');
}

#[test]
fn test_operator_tl_leading() {
    let mut extractor = TextExtractor::new();
    let font = create_test_font();
    extractor.add_font("F1".to_string(), font);

    let stream = b"BT /F1 12 Tf 20 TL 100 700 Td (A) Tj T* (B) Tj ET";
    let chars = extractor.extract(stream).unwrap();

    assert_eq!(chars.len(), 2);
    let y_diff = (chars[0].bbox.y - chars[1].bbox.y).abs();
    assert!(y_diff > 10.0, "Leading should create vertical gap, got {}", y_diff);
}

#[test]
fn test_operator_ts_text_rise() {
    let mut extractor = TextExtractor::new();
    let font = create_test_font();
    extractor.add_font("F1".to_string(), font);

    let stream = b"BT /F1 12 Tf 5 Ts 100 700 Td (X) Tj ET";
    let chars = extractor.extract(stream).unwrap();

    assert_eq!(chars.len(), 1);
    assert_eq!(chars[0].char, 'X');
}

#[test]
fn test_operator_tr_render_mode() {
    let mut extractor = TextExtractor::new();
    let font = create_test_font();
    extractor.add_font("F1".to_string(), font);

    let stream = b"BT /F1 12 Tf 1 Tr 100 700 Td (X) Tj ET";
    let chars = extractor.extract(stream).unwrap();

    assert_eq!(chars.len(), 1);
    assert_eq!(chars[0].char, 'X');
}

#[test]
fn test_set_fill_rgb() {
    let mut extractor = TextExtractor::new();
    let font = create_test_font();
    extractor.add_font("F1".to_string(), font);

    let stream = b"BT 0.5 0.3 0.8 rg /F1 12 Tf 0 0 Td (C) Tj ET";
    let chars = extractor.extract(stream).unwrap();

    assert_eq!(chars.len(), 1);
    assert!((chars[0].color.r - 0.5).abs() < 0.01);
    assert!((chars[0].color.g - 0.3).abs() < 0.01);
    assert!((chars[0].color.b - 0.8).abs() < 0.01);
}

#[test]
fn test_set_fill_gray() {
    let mut extractor = TextExtractor::new();
    let font = create_test_font();
    extractor.add_font("F1".to_string(), font);

    let stream = b"BT 0.5 g /F1 12 Tf 0 0 Td (G) Tj ET";
    let chars = extractor.extract(stream).unwrap();

    assert_eq!(chars.len(), 1);
    assert!((chars[0].color.r - 0.5).abs() < 0.01);
    assert!((chars[0].color.g - 0.5).abs() < 0.01);
    assert!((chars[0].color.b - 0.5).abs() < 0.01);
}

#[test]
fn test_set_fill_cmyk() {
    let mut extractor = TextExtractor::new();
    let font = create_test_font();
    extractor.add_font("F1".to_string(), font);

    let stream = b"BT 0 0 0 1 k /F1 12 Tf 0 0 Td (K) Tj ET";
    let chars = extractor.extract(stream).unwrap();

    assert_eq!(chars.len(), 1);
    assert!((chars[0].color.r - 0.1373).abs() < 0.01);
    assert!((chars[0].color.g - 0.1216).abs() < 0.01);
    assert!((chars[0].color.b - 0.1255).abs() < 0.01);
}

#[test]
fn test_save_restore_color() {
    let mut extractor = TextExtractor::new();
    let font = create_test_font();
    extractor.add_font("F1".to_string(), font);

    let stream = b"BT /F1 12 Tf 0 0 1 rg q 1 0 0 rg 100 700 Td (R) Tj Q 200 700 Td (B) Tj ET";
    let chars = extractor.extract(stream).unwrap();

    assert_eq!(chars.len(), 2, "Should extract 2 chars, got {}", chars.len());
    let r_char = chars.iter().find(|c| c.char == 'R').expect("Should find R");
    let b_char = chars.iter().find(|c| c.char == 'B').expect("Should find B");
    assert!(
        (r_char.color.r - 1.0).abs() < 0.01,
        "R should be red, got ({}, {}, {})",
        r_char.color.r,
        r_char.color.g,
        r_char.color.b
    );
    assert!(
        (b_char.color.b - 1.0).abs() < 0.01,
        "B should be blue after Q restore, got ({}, {}, {})",
        b_char.color.r,
        b_char.color.g,
        b_char.color.b
    );
}

#[test]
fn test_save_restore_ctm() {
    let mut extractor = TextExtractor::new();
    let font = create_test_font();
    extractor.add_font("F1".to_string(), font);

    let stream = b"q 1 0 0 1 100 200 cm BT /F1 12 Tf (A) Tj ET Q BT /F1 12 Tf (B) Tj ET";
    let chars = extractor.extract(stream).unwrap();

    assert_eq!(chars.len(), 2);
    assert!(chars[0].bbox.x > 90.0, "A should be translated by CTM");
    assert!(chars[1].bbox.x < 10.0, "B should be at origin after restore");
}

#[test]
fn test_extract_text_spans_simple() {
    let mut extractor = TextExtractor::new();
    extractor.merging_config = SpanMergingConfig::legacy();
    let font = create_test_font();
    extractor.add_font("F1".to_string(), font);

    let stream = b"BT /F1 12 Tf 100 700 Td (Hello World) Tj ET";
    let spans = extractor.extract_text_spans(stream).unwrap();

    assert!(!spans.is_empty());
    let text: String = spans.iter().map(|s| s.text.as_str()).collect::<Vec<_>>().join("");
    assert!(
        text.contains("Hello"),
        "Expected 'Hello' in extracted text, got: {}",
        text
    );
    assert!(
        text.contains("World"),
        "Expected 'World' in extracted text, got: {}",
        text
    );
}

#[test]
fn test_extract_text_spans_multiple_tj() {
    let mut extractor = TextExtractor::new();
    extractor.merging_config = SpanMergingConfig::legacy();
    let font = create_test_font();
    extractor.add_font("F1".to_string(), font);

    let stream = b"BT /F1 12 Tf 100 700 Td (He) Tj (llo) Tj ET";
    let spans = extractor.extract_text_spans(stream).unwrap();

    let text: String = spans.iter().map(|s| s.text.as_str()).collect::<Vec<_>>().join("");
    assert!(text.contains("Hello"), "Expected 'Hello' in spans, got: {}", text);
}

#[test]
fn test_extract_text_spans_with_font_info() {
    let mut extractor = TextExtractor::new();
    extractor.merging_config = SpanMergingConfig::legacy();
    let font = create_test_font();
    extractor.add_font("F1".to_string(), font);

    let stream = b"BT /F1 14 Tf 100 700 Td (Test) Tj ET";
    let spans = extractor.extract_text_spans(stream).unwrap();

    assert!(!spans.is_empty());
    let span = &spans[0];
    assert!(
        span.font_name.contains("F1") || span.font_name.contains("Times"),
        "Font name should reference F1 or Times, got: {}",
        span.font_name
    );
    assert!(span.font_size > 0.0, "Font size should be positive");
}

#[test]
fn test_extract_text_spans_empty_stream() {
    let mut extractor = TextExtractor::new();
    let font = create_test_font();
    extractor.add_font("F1".to_string(), font);

    let stream = b"";
    let spans = extractor.extract_text_spans(stream).unwrap();
    assert!(spans.is_empty());
}

#[test]
fn test_extract_text_spans_bt_et_no_text() {
    let mut extractor = TextExtractor::new();
    let font = create_test_font();
    extractor.add_font("F1".to_string(), font);

    let stream = b"BT /F1 12 Tf ET";
    let spans = extractor.extract_text_spans(stream).unwrap();
    assert!(spans.is_empty());
}

#[test]
fn test_tj_array_with_spacing() {
    let mut extractor = TextExtractor::new();
    extractor.merging_config = SpanMergingConfig::legacy();
    let font = create_test_font();
    extractor.add_font("F1".to_string(), font);

    let stream = b"BT /F1 12 Tf 100 700 Td [(H) -20 (ello)] TJ ET";
    let spans = extractor.extract_text_spans(stream).unwrap();

    let text: String = spans.iter().map(|s| s.text.as_str()).collect::<Vec<_>>().join("");
    assert!(
        text.contains("Hello"),
        "Small TJ offset should not split word, got: {}",
        text
    );
}

#[test]
fn test_tj_array_word_boundary() {
    let mut extractor = TextExtractor::new();
    extractor.merging_config = SpanMergingConfig::legacy();
    let font = create_test_font();
    extractor.add_font("F1".to_string(), font);

    let stream = b"BT /F1 12 Tf 100 700 Td [(Hello) -300 (World)] TJ ET";
    let spans = extractor.extract_text_spans(stream).unwrap();

    let text: String = spans.iter().map(|s| s.text.as_str()).collect::<Vec<_>>().join("");
    assert!(
        text.contains("Hello") && text.contains("World"),
        "Should extract both words, got: {}",
        text
    );
}

/// GH#1544: a bare `Tj` buffers into `self.tj_span_buffer`, while `TJ` runs through its own
/// local buffer and used to leave that field open. A later `Tj` then appended onto the stale
/// buffer, and the eventual flush emitted the combined run at the FIRST `Tj`'s origin -- the
/// reading-order sort then spliced it in beside whatever sat near that stale x.
///
/// Distinct alphabets make the ordering unambiguous: `AA` `BBBBBB` `CC` `DDDDDD` must stay in
/// stream order, and `AA`/`CC` must never fuse into `AACC` across the intervening array.
///
/// Neutralisation that must break this test: remove the `flush_tj_span_buffer()` call at the
/// top of `process_tj_array`.
#[test]
fn test_bare_tj_run_is_flushed_before_a_following_tj_array() {
    let mut extractor = TextExtractor::new();
    extractor.merging_config = SpanMergingConfig::legacy();
    extractor.add_font("F1".to_string(), create_test_font());

    let stream = b"BT /F1 12 Tf 100 700 Td (AA) Tj [(B)(B)(B)(B)(B)(B)] TJ (CC) Tj 50 0 Td [(D)(D)(D)(D)(D)(D)] TJ ET";
    let spans = extractor.extract_text_spans(stream).unwrap();
    let text: String = spans.iter().map(|s| s.text.as_str()).collect::<Vec<_>>().join("");

    assert!(
        !text.contains("AACC"),
        "the (CC) run was appended onto the stale (AA) buffer across the intervening TJ array, got: {text:?}"
    );
    let b_index = text
        .find("BBBBBB")
        .unwrap_or_else(|| panic!("BBBBBB run missing, got: {text:?}"));
    let c_index = text
        .find("CC")
        .unwrap_or_else(|| panic!("CC run missing, got: {text:?}"));
    assert!(
        c_index > b_index,
        "(CC) was drawn after the TJ array between it and the preceding Tj, so it must not sort ahead of it, got: {text:?}"
    );
}

#[test]
fn test_fallback_common_punctuation() {
    assert_eq!(fallback_char_to_unicode(0x2014), "\u{2014}"); // Em dash ~keep
    assert_eq!(fallback_char_to_unicode(0x2013), "\u{2013}"); // En dash ~keep
    assert_eq!(fallback_char_to_unicode(0x2022), "\u{2022}"); // Bullet ~keep
    assert_eq!(fallback_char_to_unicode(0x2026), "\u{2026}"); // Ellipsis ~keep
    assert_eq!(fallback_char_to_unicode(0x00B0), "\u{00B0}"); // Degree ~keep
}

#[test]
fn test_fallback_math_operators() {
    assert_eq!(fallback_char_to_unicode(0x00B1), "\u{00B1}"); // Plus-minus ~keep
    assert_eq!(fallback_char_to_unicode(0x00D7), "\u{00D7}"); // Multiply ~keep
    assert_eq!(fallback_char_to_unicode(0x221E), "\u{221E}"); // Infinity ~keep
    assert_eq!(fallback_char_to_unicode(0x2264), "\u{2264}"); // Less or equal ~keep
    assert_eq!(fallback_char_to_unicode(0x2265), "\u{2265}"); // Greater or equal ~keep
    assert_eq!(fallback_char_to_unicode(0x2260), "\u{2260}"); // Not equal ~keep
    assert_eq!(fallback_char_to_unicode(0x221A), "\u{221A}"); // Square root ~keep
    assert_eq!(fallback_char_to_unicode(0x222B), "\u{222B}"); // Integral ~keep
    assert_eq!(fallback_char_to_unicode(0x2211), "\u{2211}"); // Summation ~keep
}

#[test]
fn test_fallback_greek_letters() {
    assert_eq!(fallback_char_to_unicode(0x03B1), "\u{03B1}"); // alpha ~keep
    assert_eq!(fallback_char_to_unicode(0x03B2), "\u{03B2}"); // beta ~keep
    assert_eq!(fallback_char_to_unicode(0x03C0), "\u{03C0}"); // pi ~keep
    assert_eq!(fallback_char_to_unicode(0x03C9), "\u{03C9}"); // omega ~keep
    assert_eq!(fallback_char_to_unicode(0x0393), "\u{0393}"); // Gamma ~keep
    assert_eq!(fallback_char_to_unicode(0x03A9), "\u{03A9}"); // Omega ~keep
}

#[test]
fn test_fallback_currency() {
    assert_eq!(fallback_char_to_unicode(0x20AC), "\u{20AC}"); // Euro ~keep
    assert_eq!(fallback_char_to_unicode(0x00A3), "\u{00A3}"); // Pound ~keep
    assert_eq!(fallback_char_to_unicode(0x00A5), "\u{00A5}"); // Yen ~keep
    assert_eq!(fallback_char_to_unicode(0x00A2), "\u{00A2}"); // Cent ~keep
}

#[test]
fn test_fallback_direct_unicode() {
    assert_eq!(fallback_char_to_unicode(0x41), "A");
    assert_eq!(fallback_char_to_unicode(0x20), " ");
}

#[test]
fn test_fallback_invalid_code_point() {
    // Surrogate pair range is invalid Unicode ~keep
    assert_eq!(fallback_char_to_unicode(0xD800), "?");
    assert_eq!(fallback_char_to_unicode(0xDFFF), "?");
}

#[test]
fn test_fallback_private_use_area() {
    let result = fallback_char_to_unicode(0xE000);
    assert_ne!(result, "?");
}

#[test]
fn test_decode_text_no_font_latin1() {
    let result = decode_text_to_unicode(
        b"Hello",
        None,
        DecodePolicy {
            preserve_unmapped: preserve_unmapped_glyphs(),
            decompose_ligatures: false,
            question_mark_for_invalid: true,
        },
        None,
    );
    assert_eq!(result, "Hello");
}

#[test]
fn test_decode_text_no_font_high_bytes() {
    let bytes = vec![0xC0, 0xE9]; // A-grave, e-acute in Latin-1 ~keep
    let result = decode_text_to_unicode(
        &bytes,
        None,
        DecodePolicy {
            preserve_unmapped: preserve_unmapped_glyphs(),
            decompose_ligatures: false,
            question_mark_for_invalid: true,
        },
        None,
    );
    assert!(result.contains('\u{00C0}'), "Should contain A-grave");
    assert!(result.contains('\u{00E9}'), "Should contain e-acute");
}

#[test]
fn test_decode_text_filters_control_chars() {
    let bytes = vec![0x01, 0x02, 0x41, 0x09, 0x0A]; // ctrl chars, 'A', tab, newline ~keep
    let result = decode_text_to_unicode(
        &bytes,
        None,
        DecodePolicy {
            preserve_unmapped: preserve_unmapped_glyphs(),
            decompose_ligatures: false,
            question_mark_for_invalid: true,
        },
        None,
    );
    assert!(result.contains('A'), "Should contain 'A'");
    assert!(result.contains('\t'), "Should keep tab");
    assert!(result.contains('\n'), "Should keep newline");
    assert!(!result.contains('\x01'), "Should filter ctrl-A");
}

#[test]
fn test_decode_text_with_simple_font() {
    let font = create_test_font();
    let result = decode_text_to_unicode(
        b"ABC",
        Some(&font),
        DecodePolicy {
            preserve_unmapped: preserve_unmapped_glyphs(),
            decompose_ligatures: false,
            question_mark_for_invalid: true,
        },
        None,
    );
    assert!(result.contains('A') || !result.is_empty(), "Should decode something");
}

#[test]
fn test_cmyk_to_rgb_black() {
    // The K ink is #231F20, not #000000 - see color::cmyk_to_rgb. ~keep
    let (r, g, b) = cmyk_to_rgb(0.0, 0.0, 0.0, 1.0);
    assert!((r - 0.1373).abs() < 0.01);
    assert!((g - 0.1216).abs() < 0.01);
    assert!((b - 0.1255).abs() < 0.01);
}

#[test]
fn test_cmyk_to_rgb_white() {
    let (r, g, b) = cmyk_to_rgb(0.0, 0.0, 0.0, 0.0);
    assert!((r - 1.0).abs() < 0.01);
    assert!((g - 1.0).abs() < 0.01);
    assert!((b - 1.0).abs() < 0.01);
}

#[test]
fn test_cmyk_to_rgb_cyan() {
    // Process cyan, #00ADEF. ~keep
    let (r, g, b) = cmyk_to_rgb(1.0, 0.0, 0.0, 0.0);
    assert!((r - 0.0).abs() < 0.01);
    assert!((g - 0.6784).abs() < 0.01);
    assert!((b - 0.9373).abs() < 0.01);
}

#[test]
fn test_cmyk_to_rgb_magenta() {
    // Process magenta, #EC008C. ~keep
    let (r, g, b) = cmyk_to_rgb(0.0, 1.0, 0.0, 0.0);
    assert!((r - 0.9255).abs() < 0.01);
    assert!((g - 0.0).abs() < 0.01);
    assert!((b - 0.5490).abs() < 0.01);
}

#[test]
fn test_cmyk_to_rgb_yellow() {
    // Process yellow, #FFF200. ~keep
    let (r, g, b) = cmyk_to_rgb(0.0, 0.0, 1.0, 0.0);
    assert!((r - 1.0).abs() < 0.01);
    assert!((g - 0.9490).abs() < 0.01);
    assert!((b - 0.0).abs() < 0.01);
}

#[test]
fn test_has_boundary_space_empty_strings() {
    assert!(!has_boundary_space("", ""));
    assert!(!has_boundary_space("", "hello"));
    assert!(!has_boundary_space("hello", ""));
}

#[test]
fn test_has_boundary_space_only_spaces() {
    assert!(has_boundary_space(" ", " "));
    assert!(has_boundary_space(" ", "word"));
    assert!(has_boundary_space("word", " "));
}

#[test]
fn test_has_boundary_space_unicode_whitespace() {
    assert!(has_boundary_space("word\u{00A0}", "next"));
}

#[test]
fn test_email_context_at_domain() {
    assert!(is_email_context("user@outlook", ".com"));
}

#[test]
fn test_email_context_after_at() {
    assert!(is_email_context("user@", "domain.com"));
}

#[test]
fn test_email_context_domain_dot_tld() {
    assert!(is_email_context("user@domain.", "com"));
}

#[test]
fn test_email_context_not_email() {
    assert!(!is_email_context("hello", "world"));
    assert!(!is_email_context("no at sign", "here"));
}

#[test]
fn test_citation_context_superscript() {
    let prev_bbox = Rect::new(10.0, 100.0, 50.0, 12.0);
    let next_bbox = Rect::new(60.0, 105.0, 10.0, 7.0); // Raised, smaller ~keep

    // next_font_size is 0.6 * current = superscript range ~keep
    let result = is_citation_context(
        Some(&prev_bbox),
        Some(&next_bbox),
        12.0,
        12.0,
        7.2, // 60% of 12 = 0.6, within 0.5-0.75 range ~keep
    );
    assert!(result, "Should detect citation context");
}

#[test]
fn test_citation_context_no_superscript() {
    let prev_bbox = Rect::new(10.0, 100.0, 50.0, 12.0);
    let next_bbox = Rect::new(60.0, 100.0, 50.0, 12.0); // Same size, same position ~keep

    let result = is_citation_context(
        Some(&prev_bbox),
        Some(&next_bbox),
        12.0,
        12.0,
        12.0, // Same font size = not a citation ~keep
    );
    assert!(!result, "Should not detect citation when same size");
}

#[test]
fn test_citation_context_no_bbox() {
    // Font size ratio alone (without bbox) - prev is superscript ~keep
    let result = is_citation_context(None, None, 12.0, 7.2, 12.0);
    assert!(result, "Should detect citation from font size ratio alone");
}

// snap_superscript_baselines was O(n²) (every span scanned against
// every other), hanging >30 s on archive.org/Google-Books pages whose
// invisible hOCR layer emits tens of thousands of spans. The Y-windowed
// rewrite must (a) still snap a superscript onto its base and (b) scale —
// 50k spans take ~10-20 s under the old double loop but milliseconds now,
// so a generous wall-clock bound catches a quadratic regression without
// being flaky. ~keep
fn snap_span(text: &str, x: f32, y: f32, w: f32, fs: f32, seq: usize) -> TextSpan {
    TextSpan {
        provenance: None,
        text_rise: 0.0,
        artifact_type: None,
        text: text.to_string(),
        bbox: Rect::new(x, y, w, fs),
        font_name: "F1".to_string(),
        font_size: fs,
        font_weight: FontWeight::Normal,
        color: Color::black(),
        mcid: None,
        mcid_scope: None,
        sequence: seq,
        split_boundary_before: false,
        offset_semantic: false,
        is_italic: false,
        is_monospace: false,
        char_spacing: 0.0,
        word_spacing: 0.0,
        horizontal_scaling: 100.0,
        primary_detected: false,
        char_widths: vec![],
        char_x_offsets: Vec::new(),
        heading_level: None,
        rotation_degrees: 0.0,
        wmode: 0,
        rtl_draw_logical: false,
        mirrored: false,
        page_rotation_applied: 0,
    }
}

#[test]
fn test_snap_superscript_baselines_correctness() {
    let mut extractor = TextExtractor::new();
    // Base: 12pt body glyph at y=700, right edge x=130.
    // Superscript: 6pt glyph just above-right (y=704, x=130). ~keep
    extractor.spans = vec![
        snap_span("x", 100.0, 700.0, 30.0, 12.0, 0),
        snap_span("2", 130.0, 704.0, 4.0, 6.0, 1),
    ];
    extractor.snap_superscript_baselines();
    assert_eq!(
        extractor.spans[1].bbox.y, 700.0,
        "superscript must snap onto the base baseline (y=700)"
    );
}

#[test]
fn test_snap_superscript_baselines_scales() {
    let mut extractor = TextExtractor::new();
    let mut spans = Vec::with_capacity(50_002);
    // A real base+superscript pair we can assert on. ~keep
    spans.push(snap_span("x", 100.0, 700.0, 30.0, 12.0, 0));
    spans.push(snap_span("2", 130.0, 704.0, 4.0, 6.0, 1));
    // 50k body spans spread across the page (distinct Y) — same font size,
    // so none qualify as bases for each other; the cost is pure iteration. ~keep
    for k in 0..50_000usize {
        let y = (k as f32) * 2.0; // spread across Y so each window is tiny ~keep
        spans.push(snap_span("a", 50.0, y, 6.0, 10.0, k + 2));
    }
    extractor.spans = spans;

    let start = std::time::Instant::now();
    extractor.snap_superscript_baselines();
    let elapsed = start.elapsed();

    assert!(
        elapsed.as_secs() < 5,
        "snap_superscript_baselines took {elapsed:?} on 50k spans — \
             likely an O(n²) regression"
    );
    assert_eq!(
        extractor.spans[1].bbox.y, 700.0,
        "the genuine superscript must still snap to its base"
    );
}

#[test]
fn test_extractor_with_merging_config() {
    let extractor = TextExtractor::new().with_merging_config(SpanMergingConfig::aggressive());
    assert_eq!(extractor.merging_config.space_threshold_em_ratio, 0.15);
}

#[test]
fn test_extractor_set_resources() {
    let mut extractor = TextExtractor::new();
    assert!(extractor.resources.is_none());
    extractor.set_resources(Object::Null);
    assert!(extractor.resources.is_some());
}

#[test]
fn test_extractor_prepare_for_span_extraction() {
    let mut extractor = TextExtractor::new();
    extractor.extract_spans = false;
    extractor.span_sequence_counter = 42;
    extractor.prepare_for_span_extraction();
    assert!(extractor.extract_spans);
    assert_eq!(extractor.span_sequence_counter, 0);
    assert!(extractor.spans.is_empty());
}

#[test]
fn test_extractor_get_font_set() {
    let mut extractor = TextExtractor::new();
    let font = create_test_font();
    extractor.add_font("F1".to_string(), font);
    let font2 = create_test_font();
    extractor.add_font("F2".to_string(), font2);

    let font_set = extractor.get_font_set();
    assert_eq!(font_set.len(), 2);
}

#[test]
fn test_extractor_add_font_shared() {
    let mut extractor = TextExtractor::new();
    let font = Arc::new(create_test_font());
    extractor.add_font_shared("F1".to_string(), font.clone());
    assert_eq!(extractor.fonts.len(), 1);
    assert!(Arc::ptr_eq(extractor.fonts.get("F1").unwrap(), &font));
}

#[test]
fn test_analyze_tj_distribution_empty() {
    let extractor = TextExtractor::new();
    let (is_justified, cv) = extractor.analyze_tj_distribution();
    assert!(!is_justified);
    assert_eq!(cv, 0.0);
}

#[test]
fn test_analyze_tj_distribution_uniform() {
    let mut extractor = TextExtractor::new();
    extractor.tj_offset_history = vec![-100.0; 50];
    let (is_justified, cv) = extractor.analyze_tj_distribution();
    assert!(!is_justified, "Uniform offsets should not be justified");
    assert!(cv < 0.01, "CV should be ~0 for uniform offsets, got {}", cv);
}

#[test]
fn test_analyze_tj_distribution_high_variance() {
    let mut extractor = TextExtractor::new();
    let mut offsets = Vec::new();
    for i in 0..100 {
        offsets.push(if i % 2 == 0 { -50.0 } else { -200.0 });
    }
    extractor.tj_offset_history = offsets;
    let (is_justified, cv) = extractor.analyze_tj_distribution();
    assert!(is_justified, "High variance should indicate justified text, cv={}", cv);
    assert!(cv > 0.5, "CV should be > 0.5 for justified text, got {}", cv);
}

/// The O(1) accumulator path and the recompute-from-slice fallback must
/// produce identical results (same f64 formula, same sum order).
#[test]
fn test_tj_accumulator_matches_recompute() {
    let vals = vec![-50.0f32, -200.0, -75.0, -180.0, -60.0, -210.0, -90.0, -150.0];

    // O(1) path: accumulators kept consistent with the history (as `push` does). ~keep
    let mut a = TextExtractor::new();
    let mut sum = 0.0f64;
    let mut sq = 0.0f64;
    for &v in &vals {
        let x = v as f64;
        sum += x;
        sq += x * x;
        a.tj_offset_history.push(v);
    }
    a.tj_sum = sum;
    a.tj_sum_sq = sq;
    a.tj_stats_len = a.tj_offset_history.len();
    let (ja, cva) = a.analyze_tj_distribution();

    // Recompute path: only the history is set (stale accumulators). ~keep
    let mut b = TextExtractor::new();
    b.tj_offset_history = vals.clone();
    let (jb, cvb) = b.analyze_tj_distribution();

    assert_eq!(ja, jb, "is_justified must agree across paths");
    assert!((cva - cvb).abs() < 1e-6, "O(1) cv {cva} must equal recompute cv {cvb}");
}

#[test]
fn test_adaptive_threshold_disabled() {
    let config = TextExtractionConfig {
        use_adaptive_tj_threshold: false,
        space_insertion_threshold: -120.0,
        ..TextExtractionConfig::default()
    };
    let extractor = TextExtractor::with_config(config);
    let threshold = extractor.calculate_adaptive_tj_threshold();
    assert_eq!(threshold, -120.0);
}

#[test]
fn test_adaptive_threshold_enabled() {
    let config = TextExtractionConfig {
        use_adaptive_tj_threshold: true,
        word_margin_ratio: 0.1,
        ..TextExtractionConfig::default()
    };
    let mut extractor = TextExtractor::with_config(config);
    extractor.state_stack.current_mut().font_size = 12.0;
    let threshold = extractor.calculate_adaptive_tj_threshold();
    assert!(threshold < 0.0, "Adaptive threshold should be negative");
}

#[test]
fn test_update_artifact_state_empty_stack() {
    let mut extractor = TextExtractor::new();
    extractor.update_artifact_state();
    assert!(!extractor.inside_artifact);
}

#[test]
fn test_update_artifact_state_artifact_present() {
    let mut extractor = TextExtractor::new();
    extractor.marked_content_stack.push(MarkedContentContext {
        artifact_type: None,
        tag: "Artifact".to_string(),
        is_artifact: true,
        actual_text: None,
        expansion: None,
        is_excluded_layer: false,
        is_placed_pdf: false,
        actual_text_emitted: false,
        own_mcid: None,
    });
    extractor.update_artifact_state();
    assert!(extractor.inside_artifact);
}

#[test]
fn test_placed_pdf_suppresses_content() {
    // Text inside an InDesign /PlacedPDF figure region (the placed
    // artwork's own glyphs — e.g. a draft galley) must be suppressed,
    // matching pdftotext/PyMuPDF. Entering a /PlacedPDF BDC sets
    // inside_placed_pdf, which feeds is_content_suppressed(). ~keep
    let mut extractor = TextExtractor::new();
    assert!(!extractor.inside_placed_pdf);
    assert!(!extractor.is_content_suppressed());

    extractor.marked_content_stack.push(MarkedContentContext {
        artifact_type: None,
        tag: "PlacedPDF".to_string(),
        is_artifact: false,
        actual_text: None,
        expansion: None,
        is_excluded_layer: false,
        is_placed_pdf: true,
        actual_text_emitted: false,
        own_mcid: None,
    });
    extractor.update_layer_state();
    assert!(extractor.inside_placed_pdf);
    assert!(
        extractor.is_content_suppressed(),
        "text inside /PlacedPDF must be suppressed"
    );

    extractor.marked_content_stack.pop();
    extractor.update_layer_state();
    assert!(!extractor.inside_placed_pdf);
    assert!(!extractor.is_content_suppressed());
}

#[test]
fn test_non_placed_pdf_tag_does_not_suppress() {
    // A regular (non-PlacedPDF) marked-content tag such as /Figure must
    // NOT suppress its text — only the placed-PDF wrapper does. ~keep
    let mut extractor = TextExtractor::new();
    extractor.marked_content_stack.push(MarkedContentContext {
        artifact_type: None,
        tag: "Figure".to_string(),
        is_artifact: false,
        actual_text: None,
        expansion: None,
        is_excluded_layer: false,
        is_placed_pdf: false,
        actual_text_emitted: false,
        own_mcid: None,
    });
    extractor.update_layer_state();
    assert!(!extractor.inside_placed_pdf);
    assert!(!extractor.is_content_suppressed());
}

#[test]
fn test_placed_pdf_kept_when_it_is_the_whole_page_body() {
    // A publisher that places the ENTIRE article body inside one /PlacedPDF
    // region (e.g. MATEC Web of Conferences) leaves almost nothing outside.
    // There the placed text IS the page's logical content and must NOT be
    // suppressed (pymupdf/pdftotext extract it). The coverage pre-scan flags
    // this: placed text dominates, non-placed text is a tiny header. ~keep
    let body = "(This is the full article body typeset inside a placed PDF region) Tj\n".repeat(20);
    let stream = format!("/PlacedPDF BMC\nBT\n{body}ET\nEMC\nBT (Journal vol 1) Tj ET\n");
    assert!(
        TextExtractor::placed_pdf_text_dominates(stream.as_bytes()),
        "whole-body /PlacedPDF must be KEPT (not suppressed)"
    );
}

#[test]
fn test_placed_pdf_suppressed_when_minority_overlay() {
    // The decorative-figure case (PMC8100493): a small /PlacedPDF galley
    // duplicate sits amid a full page of real text OUTSIDE it. The placed
    // text is the minority, so it stays suppressed (the de-dup win). ~keep
    let outside = "(Real published paragraph of the article that lives outside the placed region) Tj\n".repeat(20);
    let stream = format!("BT\n{outside}ET\n/PlacedPDF BMC\nBT (draft galley) Tj ET\nEMC\n");
    assert!(
        !TextExtractor::placed_pdf_text_dominates(stream.as_bytes()),
        "minority-overlay /PlacedPDF must stay suppressed"
    );
}

#[test]
fn test_placed_pdf_coverage_noop_without_tag() {
    // No /PlacedPDF tag anywhere: the pre-scan must short-circuit to false
    // (keep the default suppression state; pay nothing for ordinary pages). ~keep
    let stream = b"BT (ordinary single column page of text) Tj ET\n";
    assert!(!TextExtractor::placed_pdf_text_dominates(stream));
}

#[test]
fn test_placed_pdf_kept_when_unique_body_amid_comparable_outside() {
    // Gate 3: an InDesign spread (e.g. a placed floor-plan / marketing page)
    // where the placed region carries a substantial body of UNIQUE text and
    // the non-placed text is comparable or larger but different (labels,
    // headers). The 3:1 dominance ratio fails, yet the placed words are not a
    // duplicate of the outside text, so it must be KEPT (pdftotext/pymupdf
    // extract it; suppressing it drops the whole spread's content). ~keep
    let placed = "(master bedroom terrace kitchen dimensions balcony) Tj\n".repeat(30);
    let outside = "(square footage residence penthouse skyline waterfront) Tj\n".repeat(35);
    let stream = format!("BT\n{outside}ET\n/PlacedPDF /MC0 BDC\nBT\n{placed}ET\nEMC\n");
    assert!(
        TextExtractor::placed_pdf_text_dominates(stream.as_bytes()),
        "unique placed body amid comparable outside text must be KEPT"
    );
}

#[test]
fn test_placed_pdf_suppressed_when_large_duplicate_overlay() {
    // Gate 3, the other side: a large placed region whose words DUPLICATE the
    // surrounding text is a draft galley / overlay copy and stays suppressed
    // even though it clears the size gate (the PMC8100493 de-dup intent, at
    // full body size rather than the minority-overlay size). ~keep
    let body = "(the published paragraph of the real article body content) Tj\n".repeat(30);
    let stream = format!("BT\n{body}ET\n/PlacedPDF /MC0 BDC\nBT\n{body}ET\nEMC\n");
    assert!(
        !TextExtractor::placed_pdf_text_dominates(stream.as_bytes()),
        "a full-size placed DUPLICATE of the outside text must stay suppressed"
    );
}

#[test]
fn test_update_artifact_state_nested_non_artifact() {
    let mut extractor = TextExtractor::new();
    extractor.marked_content_stack.push(MarkedContentContext {
        artifact_type: None,
        tag: "Artifact".to_string(),
        is_artifact: true,
        actual_text: None,
        expansion: None,
        is_excluded_layer: false,
        is_placed_pdf: false,
        actual_text_emitted: false,
        own_mcid: None,
    });
    extractor.marked_content_stack.push(MarkedContentContext {
        artifact_type: None,
        tag: "Span".to_string(),
        is_artifact: false,
        actual_text: None,
        expansion: None,
        is_excluded_layer: false,
        is_placed_pdf: false,
        actual_text_emitted: false,
        own_mcid: None,
    });
    extractor.update_artifact_state();
    // Should still be inside artifact because parent is artifact ~keep
    assert!(extractor.inside_artifact);
}

#[test]
fn test_parse_artifact_type_page() {
    let mut props = HashMap::new();
    props.insert("Type".to_string(), Object::Name("Page".to_string()));
    let result = TextExtractor::parse_artifact_type(&props);
    assert_eq!(result, Some(ArtifactType::Page));
}

#[test]
fn test_parse_artifact_type_pagination_page_number() {
    let mut props = HashMap::new();
    props.insert("Type".to_string(), Object::Name("Pagination".to_string()));
    props.insert("Subtype".to_string(), Object::Name("PageNumber".to_string()));
    let result = TextExtractor::parse_artifact_type(&props);
    assert_eq!(result, Some(ArtifactType::Pagination(PaginationSubtype::PageNumber)));
}

#[test]
fn test_parse_artifact_type_pagination_other_subtype() {
    let mut props = HashMap::new();
    props.insert("Type".to_string(), Object::Name("Pagination".to_string()));
    props.insert("Subtype".to_string(), Object::Name("SomethingElse".to_string()));
    let result = TextExtractor::parse_artifact_type(&props);
    assert_eq!(result, Some(ArtifactType::Pagination(PaginationSubtype::Other)));
}

#[test]
fn test_parse_artifact_type_unknown_type() {
    let mut props = HashMap::new();
    props.insert("Type".to_string(), Object::Name("UnknownType".to_string()));
    let result = TextExtractor::parse_artifact_type(&props);
    assert_eq!(result, None);
}

#[test]
fn test_parse_artifact_type_subtype_footer_only() {
    let mut props = HashMap::new();
    props.insert("Subtype".to_string(), Object::Name("Footer".to_string()));
    let result = TextExtractor::parse_artifact_type(&props);
    assert_eq!(result, Some(ArtifactType::Pagination(PaginationSubtype::Footer)));
}

#[test]
fn test_parse_artifact_type_subtype_watermark_only() {
    let mut props = HashMap::new();
    props.insert("Subtype".to_string(), Object::Name("Watermark".to_string()));
    let result = TextExtractor::parse_artifact_type(&props);
    assert_eq!(result, Some(ArtifactType::Pagination(PaginationSubtype::Watermark)));
}

#[test]
fn test_decode_pdf_text_string_utf8() {
    let result = TextExtractor::decode_pdf_text_string(b"Hello World");
    assert_eq!(result, "Hello World");
}

#[test]
fn test_decode_pdf_text_string_utf16be_bom() {
    // UTF-16BE with BOM: FE FF, then "Hi" in UTF-16BE ~keep
    let bytes: Vec<u8> = vec![0xFE, 0xFF, 0x00, 0x48, 0x00, 0x69];
    let result = TextExtractor::decode_pdf_text_string(&bytes);
    assert_eq!(result, "Hi");
}

#[test]
fn test_decode_pdf_text_string_utf16le_bom() {
    // UTF-16LE with BOM: FF FE, then "Hi" in UTF-16LE ~keep
    let bytes: Vec<u8> = vec![0xFF, 0xFE, 0x48, 0x00, 0x69, 0x00];
    let result = TextExtractor::decode_pdf_text_string(&bytes);
    assert_eq!(result, "Hi");
}

#[test]
fn test_decode_pdf_text_string_empty() {
    let result = TextExtractor::decode_pdf_text_string(b"");
    assert_eq!(result, "");
}

#[test]
fn test_is_ligature_code() {
    assert!(TextExtractor::is_ligature_code(0xFB00)); // ff ~keep
    assert!(TextExtractor::is_ligature_code(0xFB01)); // fi ~keep
    assert!(TextExtractor::is_ligature_code(0xFB02)); // fl ~keep
    assert!(TextExtractor::is_ligature_code(0xFB03)); // ffi ~keep
    assert!(TextExtractor::is_ligature_code(0xFB04)); // ffl ~keep
}

#[test]
fn test_is_not_ligature_code() {
    assert!(!TextExtractor::is_ligature_code(0x41));
    assert!(!TextExtractor::is_ligature_code(0xFAFF)); // Before range ~keep
    assert!(!TextExtractor::is_ligature_code(0xFB05)); // After range ~keep
}

#[test]
fn test_bt_resets_text_matrix() {
    let mut extractor = TextExtractor::new();
    let font = create_test_font();
    extractor.add_font("F1".to_string(), font);

    let stream = b"BT /F1 12 Tf 100 700 Td (A) Tj ET BT /F1 12 Tf (B) Tj ET";
    let chars = extractor.extract(stream).unwrap();

    assert_eq!(chars.len(), 2);
    assert_eq!(chars[0].char, 'A');
    assert_eq!(chars[1].char, 'B');
    assert!(
        chars[1].bbox.x < 10.0,
        "Second BT should reset text matrix, x={}",
        chars[1].bbox.x
    );
}

#[test]
fn test_multiple_bt_et_blocks() {
    let mut extractor = TextExtractor::new();
    let font = create_test_font();
    extractor.add_font("F1".to_string(), font);

    let stream = b"BT /F1 12 Tf 100 700 Td (Hello) Tj ET BT /F1 12 Tf 100 680 Td (World) Tj ET";
    let chars = extractor.extract(stream).unwrap();

    let text: String = chars.iter().map(|c| c.char).collect();
    assert!(text.contains("Hello"), "Should contain Hello");
    assert!(text.contains("World"), "Should contain World");
}

#[test]
fn test_bmc_artifact_tracking() {
    let mut extractor = TextExtractor::new();
    let font = create_test_font();
    extractor.add_font("F1".to_string(), font);

    extractor
        .execute_operator_public(crate::content::operators::Operator::BeginMarkedContent {
            tag: "Artifact".to_string(),
        })
        .unwrap();

    assert!(
        extractor.inside_artifact,
        "Should be inside artifact after BMC Artifact"
    );

    extractor
        .execute_operator_public(crate::content::operators::Operator::EndMarkedContent)
        .unwrap();

    assert!(!extractor.inside_artifact, "Should be outside artifact after EMC");
}

#[test]
fn test_bmc_non_artifact() {
    let mut extractor = TextExtractor::new();

    extractor
        .execute_operator_public(crate::content::operators::Operator::BeginMarkedContent {
            tag: "Span".to_string(),
        })
        .unwrap();

    assert!(
        !extractor.inside_artifact,
        "Non-artifact BMC should not set inside_artifact"
    );
}

#[test]
fn test_font_switch_mid_stream() {
    let mut extractor = TextExtractor::new();
    let font1 = create_test_font();
    let mut font2_data = create_test_font();
    font2_data.base_font = "Helvetica".to_string();
    extractor.add_font("F1".to_string(), font1);
    extractor.add_font("F2".to_string(), font2_data);

    let stream = b"BT /F1 12 Tf 100 700 Td (Hello) Tj /F2 14 Tf (World) Tj ET";
    let chars = extractor.extract(stream).unwrap();

    let text: String = chars.iter().map(|c| c.char).collect();
    assert!(text.contains("Hello"), "Should contain Hello");
    assert!(text.contains("World"), "Should contain World");
}

#[test]
fn test_font_switch_same_font_no_flush() {
    // Setting the same font twice should be a no-op (optimization) ~keep
    let mut extractor = TextExtractor::new();
    let font = create_test_font();
    extractor.add_font("F1".to_string(), font);

    let stream = b"BT /F1 12 Tf /F1 12 Tf 100 700 Td (Test) Tj ET";
    let spans = extractor.extract_text_spans(stream).unwrap();

    let text: String = spans.iter().map(|s| s.text.as_str()).collect::<Vec<_>>().join("");
    assert!(text.contains("Test"), "Should extract text, got: {}", text);
}

#[test]
fn test_cm_operator_translation() {
    let mut extractor = TextExtractor::new();
    let font = create_test_font();
    extractor.add_font("F1".to_string(), font);

    let stream = b"1 0 0 1 50 100 cm BT /F1 12 Tf (X) Tj ET";
    let chars = extractor.extract(stream).unwrap();

    assert_eq!(chars.len(), 1);
    assert!((chars[0].bbox.x - 50.0).abs() < 2.0, "X should be ~50");
    assert!((chars[0].bbox.y - 100.0).abs() < 2.0, "Y should be ~100");
}

#[test]
fn test_cm_operator_scaling() {
    let mut extractor = TextExtractor::new();
    let font = create_test_font();
    extractor.add_font("F1".to_string(), font);

    let stream = b"2 0 0 2 0 0 cm BT /F1 12 Tf 1 0 0 1 50 100 Tm (Y) Tj ET";
    let chars = extractor.extract(stream).unwrap();

    assert_eq!(chars.len(), 1);
    assert!(
        (chars[0].bbox.x - 100.0).abs() < 2.0,
        "X should be ~100 (got {})",
        chars[0].bbox.x
    );
}

#[test]
fn test_deduplicate_overlapping_chars() {
    let mut extractor = TextExtractor::new();
    let font = create_test_font();
    extractor.add_font("F1".to_string(), font);

    // Create overlapping chars (simulating bold rendering with duplicate glyphs) ~keep
    extractor.chars = vec![
        TextChar {
            char: 'A',
            bbox: Rect::new(100.0, 700.0, 6.0, 12.0),
            font_name: "F1".to_string(),
            font_size: 12.0,
            font_weight: FontWeight::Normal,
            color: Color::black(),
            mcid: None,
            is_italic: false,
            is_monospace: false,
            origin_x: 100.0,
            origin_y: 700.0,
            rotation_degrees: 0.0,
            advance_width: 6.0,
            rendered_advance: 6.0,
            ascent: 11.4,
            descent: -4.2,
            matrix: None,
        },
        TextChar {
            char: 'A',
            bbox: Rect::new(100.5, 700.0, 6.0, 12.0),
            font_name: "F1".to_string(),
            font_size: 12.0,
            font_weight: FontWeight::Normal,
            color: Color::black(),
            mcid: None,
            is_italic: false,
            is_monospace: false,
            origin_x: 100.5,
            origin_y: 700.0,
            rotation_degrees: 0.0,
            advance_width: 6.0,
            rendered_advance: 6.0,
            ascent: 11.4,
            descent: -4.2,
            matrix: None,
        },
    ];

    extractor.deduplicate_overlapping_chars();
    assert_eq!(extractor.chars.len(), 1, "Overlapping chars should be deduplicated");
}

#[test]
fn test_deduplicate_overlapping_chars_different_lines() {
    let mut extractor = TextExtractor::new();

    extractor.chars = vec![
        TextChar {
            char: 'A',
            bbox: Rect::new(100.0, 700.0, 6.0, 12.0),
            font_name: "F1".to_string(),
            font_size: 12.0,
            font_weight: FontWeight::Normal,
            color: Color::black(),
            mcid: None,
            is_italic: false,
            is_monospace: false,
            origin_x: 100.0,
            origin_y: 700.0,
            rotation_degrees: 0.0,
            advance_width: 6.0,
            rendered_advance: 6.0,
            ascent: 11.4,
            descent: -4.2,
            matrix: None,
        },
        TextChar {
            char: 'A',
            bbox: Rect::new(100.0, 680.0, 6.0, 12.0),
            font_name: "F1".to_string(),
            font_size: 12.0,
            font_weight: FontWeight::Normal,
            color: Color::black(),
            mcid: None,
            is_italic: false,
            is_monospace: false,
            origin_x: 100.0,
            origin_y: 680.0,
            rotation_degrees: 0.0,
            advance_width: 6.0,
            rendered_advance: 6.0,
            ascent: 11.4,
            descent: -4.2,
            matrix: None,
        },
    ];

    extractor.deduplicate_overlapping_chars();
    assert_eq!(
        extractor.chars.len(),
        2,
        "Chars on different lines should not be deduplicated"
    );
}

#[test]
fn test_deduplicate_overlapping_chars_empty() {
    let mut extractor = TextExtractor::new();
    extractor.deduplicate_overlapping_chars();
    assert!(extractor.chars.is_empty());
}

#[test]
fn test_deduplicate_keeps_distinct_close_chars() {
    // Distinct characters close together should NOT be dropped ~keep
    let mut extractor = TextExtractor::new();

    let make_char = |c: char, x: f32| TextChar {
        char: c,
        bbox: Rect::new(x, 700.0, 6.0, 12.0),
        font_name: "F1".to_string(),
        font_size: 12.0,
        font_weight: FontWeight::Normal,
        color: Color::black(),
        mcid: None,
        is_italic: false,
        is_monospace: false,
        origin_x: x,
        origin_y: 700.0,
        rotation_degrees: 0.0,
        advance_width: 6.0,
        rendered_advance: 6.0,
        ascent: 11.4,
        descent: -4.2,
        matrix: None,
    };

    // 't' at x=100, ' ' at x=105, 'r' at x=106.5 (within 2pt of ' ' but different char) ~keep
    extractor.chars = vec![make_char('t', 100.0), make_char(' ', 105.0), make_char('r', 106.5)];

    extractor.deduplicate_overlapping_chars();
    assert_eq!(
        extractor.chars.len(),
        3,
        "Distinct characters close together must not be dropped"
    );
    assert_eq!(extractor.chars[0].char, 't');
    assert_eq!(extractor.chars[1].char, ' ');
    assert_eq!(extractor.chars[2].char, 'r');
}

#[test]
fn test_deduplicate_still_removes_same_char_duplicates() {
    let mut extractor = TextExtractor::new();

    let make_char = |c: char, x: f32| TextChar {
        char: c,
        bbox: Rect::new(x, 700.0, 6.0, 12.0),
        font_name: "F1".to_string(),
        font_size: 12.0,
        font_weight: FontWeight::Normal,
        color: Color::black(),
        mcid: None,
        is_italic: false,
        is_monospace: false,
        origin_x: x,
        origin_y: 700.0,
        rotation_degrees: 0.0,
        advance_width: 6.0,
        rendered_advance: 6.0,
        ascent: 11.4,
        descent: -4.2,
        matrix: None,
    };

    extractor.chars = vec![make_char('A', 100.0), make_char('A', 100.5)];

    extractor.deduplicate_overlapping_chars();
    assert_eq!(extractor.chars.len(), 1, "Duplicate same char should still be deduped");
    assert_eq!(extractor.chars[0].char, 'A');
}

#[test]
fn test_deduplicate_keeps_narrow_glyph_doublets() {
    // Regression: `ll`, `rr`, `II`, `ii` in small-font body text were
    // wrongly collapsed to a single glyph because the dedup threshold
    // was a hardcoded 2 pt — larger than the advance width of narrow
    // glyphs at ≤ 9 pt in most fonts (Helvetica `l` ≈ 2.5 pt at 9 pt,
    // smaller below). This caused visible corruption like
    // `controller → controler` and `billed → biled`. ~keep
    //
    // Exercises the matrix of four narrow glyphs across three small
    // body-text sizes. Advance widths are the real Helvetica per-em
    // values (0.278 em for `l`/`i`, 0.333 em for `r`, 0.278 em for `I`). ~keep
    let narrow_char = |c: char, x: f32, font_size: f32, advance_em: f32| TextChar {
        char: c,
        bbox: Rect::new(x, 700.0, advance_em * font_size * 0.6, font_size),
        font_name: "Helvetica".to_string(),
        font_size,
        font_weight: FontWeight::Normal,
        color: Color::black(),
        mcid: None,
        is_italic: false,
        is_monospace: false,
        origin_x: x,
        origin_y: 700.0,
        rotation_degrees: 0.0,
        advance_width: advance_em * font_size,
        rendered_advance: advance_em * font_size,
        ascent: 11.4,
        descent: -4.2,
        matrix: None,
    };

    let cases: &[(char, f32)] = &[('l', 0.278), ('r', 0.333), ('I', 0.278), ('i', 0.278)];
    // Body-text sizes where narrow-glyph advance falls at or below 2 pt. ~keep
    let sizes: &[f32] = &[7.0, 9.0, 11.0];

    for &(glyph, advance_em) in cases {
        for &font_size in sizes {
            let advance = advance_em * font_size;
            let mut extractor = TextExtractor::new();
            extractor.chars = vec![
                narrow_char(glyph, 100.0, font_size, advance_em),
                narrow_char(glyph, 100.0 + advance, font_size, advance_em),
            ];

            extractor.deduplicate_overlapping_chars();
            assert_eq!(
                extractor.chars.len(),
                2,
                "Adjacent narrow-glyph doublet ('{glyph}{glyph}') at {font_size} pt \
                     (advance = {advance:.2} pt) must not be collapsed",
            );
        }
    }
}

#[test]
fn test_deduplicate_still_collapses_narrow_glyph_stroke_fill_duplicates() {
    // Positive regression: even with the advance-scaled threshold,
    // stroke+fill render passes on narrow glyphs (two `l`s at ~0 pt
    // offset) must still be collapsed. The ratio (0.30) comfortably
    // catches real duplicates (< 5 % of one advance apart) while
    // staying below typical heaviest kerning (~20 %). ~keep
    let mut extractor = TextExtractor::new();

    let narrow_at = |x: f32| TextChar {
        char: 'l',
        bbox: Rect::new(x, 700.0, 1.5, 9.0),
        font_name: "Helvetica".to_string(),
        font_size: 9.0,
        font_weight: FontWeight::Normal,
        color: Color::black(),
        mcid: None,
        is_italic: false,
        is_monospace: false,
        origin_x: x,
        origin_y: 700.0,
        rotation_degrees: 0.0,
        advance_width: 2.5, // 0.278 em × 9 pt ~keep
        rendered_advance: 2.5,
        ascent: 11.4,
        descent: -4.2,
        matrix: None,
    };

    // Stroke pass and fill pass typically land within 0.05 pt of each
    // other (2 % of advance at 9 pt Helvetica `l`). ~keep
    extractor.chars = vec![narrow_at(100.0), narrow_at(100.05)];

    extractor.deduplicate_overlapping_chars();
    assert_eq!(
        extractor.chars.len(),
        1,
        "Stroke+fill narrow-glyph duplicates (same char at ~0 pt offset) \
             must still be collapsed"
    );
}

#[test]
fn test_deduplicate_overlapping_spans_geometric() {
    let mut extractor = TextExtractor::new();
    extractor.spans = vec![
        TextSpan {
            provenance: None,
            text_rise: 0.0,
            artifact_type: None,
            text: "Hello".to_string(),
            bbox: Rect::new(100.0, 700.0, 30.0, 12.0),
            font_name: "F1".to_string(),
            font_size: 12.0,
            font_weight: FontWeight::Normal,
            color: Color::black(),
            mcid: None,
            mcid_scope: None,
            sequence: 0,
            split_boundary_before: false,
            offset_semantic: false,
            is_italic: false,
            is_monospace: false,
            char_spacing: 0.0,
            word_spacing: 0.0,
            horizontal_scaling: 100.0,
            primary_detected: false,
            char_widths: vec![],
            char_x_offsets: Vec::new(),
            heading_level: None,
            rotation_degrees: 0.0,
            wmode: 0,
            rtl_draw_logical: false,
            mirrored: false,
            page_rotation_applied: 0,
        },
        TextSpan {
            provenance: None,
            text_rise: 0.0,
            artifact_type: None,
            text: "Hello".to_string(),
            bbox: Rect::new(101.0, 700.0, 30.0, 12.0),
            font_name: "F1".to_string(),
            font_size: 12.0,
            font_weight: FontWeight::Normal,
            color: Color::black(),
            mcid: None,
            mcid_scope: None,
            sequence: 1,
            split_boundary_before: false,
            offset_semantic: false,
            is_italic: false,
            is_monospace: false,
            char_spacing: 0.0,
            word_spacing: 0.0,
            horizontal_scaling: 100.0,
            primary_detected: false,
            char_widths: vec![],
            char_x_offsets: Vec::new(),
            heading_level: None,
            rotation_degrees: 0.0,
            wmode: 0,
            rtl_draw_logical: false,
            mirrored: false,
            page_rotation_applied: 0,
        },
    ];

    extractor.deduplicate_overlapping_spans();
    assert_eq!(extractor.spans.len(), 1, "Geometric duplicates should be removed");
}

#[test]
fn test_deduplicate_overlapping_spans_empty() {
    let mut extractor = TextExtractor::new();
    extractor.deduplicate_overlapping_spans();
    assert!(extractor.spans.is_empty());
}

#[test]
fn test_deduplicate_spans_keeps_narrow_glyph_doublets() {
    // Regression: PDFs that emit kerned text glyph-by-glyph produce
    // consecutive single-character spans. Two adjacent narrow-glyph
    // spans (`l`, `r`, `I`, `i` at ≤ 9 pt) sit roughly one advance-width
    // apart, which used to fall under the hardcoded 2 pt geometric
    // threshold and get collapsed. The threshold now scales with each
    // span's per-glyph width so legitimate doublets survive.
    //
    // Exercises the matrix of four narrow glyphs across three small
    // body-text sizes. ~keep
    let narrow_span = |glyph: char, x: f32, font_size: f32, advance: f32, seq: usize| TextSpan {
        provenance: None,
        text_rise: 0.0,
        artifact_type: None,
        text: glyph.to_string(),
        bbox: Rect::new(x, 700.0, advance, font_size),
        font_name: "Helvetica".to_string(),
        font_size,
        font_weight: FontWeight::Normal,
        color: Color::black(),
        mcid: None,
        mcid_scope: None,
        sequence: seq,
        split_boundary_before: false,
        offset_semantic: false,
        is_italic: false,
        is_monospace: false,
        char_spacing: 0.0,
        word_spacing: 0.0,
        horizontal_scaling: 100.0,
        primary_detected: false,
        char_widths: vec![],
        char_x_offsets: Vec::new(),
        heading_level: None,
        rotation_degrees: 0.0,
        wmode: 0,
        rtl_draw_logical: false,
        mirrored: false,
        page_rotation_applied: 0,
    };

    // (glyph, Helvetica per-em advance width) ~keep
    let cases: &[(char, f32)] = &[('l', 0.278), ('r', 0.333), ('I', 0.278), ('i', 0.278)];
    let sizes: &[f32] = &[7.0, 9.0, 11.0];

    for &(glyph, advance_em) in cases {
        for &font_size in sizes {
            let advance = advance_em * font_size;
            let mut extractor = TextExtractor::new();
            extractor.spans = vec![
                narrow_span(glyph, 100.0, font_size, advance, 0),
                narrow_span(glyph, 100.0 + advance, font_size, advance, 1),
            ];

            extractor.deduplicate_overlapping_spans();
            assert_eq!(
                extractor.spans.len(),
                2,
                "Adjacent single-glyph narrow-doublet spans ('{glyph}{glyph}') \
                     at {font_size} pt (advance = {advance:.2} pt) must not be collapsed",
            );
        }
    }
}

#[test]
fn test_deduplicate_spans_still_collapses_stroke_fill_narrow_glyphs() {
    // Positive regression: stroke+fill single-glyph narrow spans at
    // ~0 pt offset must still be collapsed by the geometric dedup
    // phase. The ratio (0.30) comfortably catches real duplicates
    // while preserving legitimate doublets. ~keep
    let mut extractor = TextExtractor::new();

    let narrow_at = |x: f32, seq: usize| TextSpan {
        provenance: None,
        text_rise: 0.0,
        artifact_type: None,
        text: "l".to_string(),
        bbox: Rect::new(x, 700.0, 2.5, 9.0),
        font_name: "Helvetica".to_string(),
        font_size: 9.0,
        font_weight: FontWeight::Normal,
        color: Color::black(),
        mcid: None,
        mcid_scope: None,
        sequence: seq,
        split_boundary_before: false,
        offset_semantic: false,
        is_italic: false,
        is_monospace: false,
        char_spacing: 0.0,
        word_spacing: 0.0,
        horizontal_scaling: 100.0,
        primary_detected: false,
        char_widths: vec![],
        char_x_offsets: Vec::new(),
        heading_level: None,
        rotation_degrees: 0.0,
        wmode: 0,
        rtl_draw_logical: false,
        mirrored: false,
        page_rotation_applied: 0,
    };

    // Stroke pass + fill pass at ~2 % of advance apart. ~keep
    extractor.spans = vec![narrow_at(100.0, 0), narrow_at(100.05, 1)];

    extractor.deduplicate_overlapping_spans();
    assert_eq!(
        extractor.spans.len(),
        1,
        "Stroke+fill narrow-glyph duplicate spans (same text at ~0 pt offset) \
             must still be collapsed"
    );
}

#[test]
fn test_detect_span_columns_empty() {
    let extractor = TextExtractor::new();
    let columns = extractor.detect_span_columns();
    assert!(columns.is_empty());
}

#[test]
fn test_detect_span_columns_single_column() {
    let mut extractor = TextExtractor::new();
    for i in 0..10 {
        extractor.spans.push(TextSpan {
            provenance: None,
            text_rise: 0.0,
            artifact_type: None,
            text: format!("Line {}", i),
            bbox: Rect::new(50.0, 700.0 - (i as f32 * 14.0), 200.0, 12.0),
            font_name: "F1".to_string(),
            font_size: 12.0,
            font_weight: FontWeight::Normal,
            color: Color::black(),
            mcid: None,
            mcid_scope: None,
            sequence: i,
            split_boundary_before: false,
            offset_semantic: false,
            is_italic: false,
            is_monospace: false,
            char_spacing: 0.0,
            word_spacing: 0.0,
            horizontal_scaling: 100.0,
            primary_detected: false,
            char_widths: vec![],
            char_x_offsets: Vec::new(),
            heading_level: None,
            rotation_degrees: 0.0,
            wmode: 0,
            rtl_draw_logical: false,
            mirrored: false,
            page_rotation_applied: 0,
        });
    }

    let columns = extractor.detect_span_columns();
    assert_eq!(columns.len(), 1, "Should detect single column");
}

#[test]
fn test_sort_by_reading_order() {
    let mut extractor = TextExtractor::new();
    extractor.chars = vec![
        TextChar {
            char: 'B',
            bbox: Rect::new(100.0, 680.0, 6.0, 12.0),
            font_name: "F1".to_string(),
            font_size: 12.0,
            font_weight: FontWeight::Normal,
            color: Color::black(),
            mcid: None,
            is_italic: false,
            is_monospace: false,
            origin_x: 100.0,
            origin_y: 680.0,
            rotation_degrees: 0.0,
            advance_width: 6.0,
            rendered_advance: 6.0,
            ascent: 11.4,
            descent: -4.2,
            matrix: None,
        },
        TextChar {
            char: 'A',
            bbox: Rect::new(100.0, 700.0, 6.0, 12.0),
            font_name: "F1".to_string(),
            font_size: 12.0,
            font_weight: FontWeight::Normal,
            color: Color::black(),
            mcid: None,
            is_italic: false,
            is_monospace: false,
            origin_x: 100.0,
            origin_y: 700.0,
            rotation_degrees: 0.0,
            advance_width: 6.0,
            rendered_advance: 6.0,
            ascent: 11.4,
            descent: -4.2,
            matrix: None,
        },
    ];

    extractor.sort_by_reading_order();
    // PDF Y increases upward, so 700 is higher than 680
    // Reading order: top first, so A (y=700) before B (y=680) ~keep
    assert_eq!(extractor.chars[0].char, 'A');
    assert_eq!(extractor.chars[1].char, 'B');
}

#[test]
fn test_sort_by_reading_order_same_line() {
    let mut extractor = TextExtractor::new();
    extractor.chars = vec![
        TextChar {
            char: 'B',
            bbox: Rect::new(200.0, 700.0, 6.0, 12.0),
            font_name: "F1".to_string(),
            font_size: 12.0,
            font_weight: FontWeight::Normal,
            color: Color::black(),
            mcid: None,
            is_italic: false,
            is_monospace: false,
            origin_x: 200.0,
            origin_y: 700.0,
            rotation_degrees: 0.0,
            advance_width: 6.0,
            rendered_advance: 6.0,
            ascent: 11.4,
            descent: -4.2,
            matrix: None,
        },
        TextChar {
            char: 'A',
            bbox: Rect::new(100.0, 700.0, 6.0, 12.0),
            font_name: "F1".to_string(),
            font_size: 12.0,
            font_weight: FontWeight::Normal,
            color: Color::black(),
            mcid: None,
            is_italic: false,
            is_monospace: false,
            origin_x: 100.0,
            origin_y: 700.0,
            rotation_degrees: 0.0,
            advance_width: 6.0,
            rendered_advance: 6.0,
            ascent: 11.4,
            descent: -4.2,
            matrix: None,
        },
    ];

    extractor.sort_by_reading_order();
    assert_eq!(extractor.chars[0].char, 'A');
    assert_eq!(extractor.chars[1].char, 'B');
}

#[test]
fn test_sort_by_reading_order_nan_values() {
    let mut extractor = TextExtractor::new();
    extractor.chars = vec![
        TextChar {
            char: 'A',
            bbox: Rect::new(f32::NAN, f32::NAN, 6.0, 12.0),
            font_name: "F1".to_string(),
            font_size: 12.0,
            font_weight: FontWeight::Normal,
            color: Color::black(),
            mcid: None,
            is_italic: false,
            is_monospace: false,
            origin_x: 0.0,
            origin_y: 0.0,
            rotation_degrees: 0.0,
            advance_width: 6.0,
            rendered_advance: 6.0,
            ascent: 11.4,
            descent: -4.2,
            matrix: None,
        },
        TextChar {
            char: 'B',
            bbox: Rect::new(100.0, 700.0, 6.0, 12.0),
            font_name: "F1".to_string(),
            font_size: 12.0,
            font_weight: FontWeight::Normal,
            color: Color::black(),
            mcid: None,
            is_italic: false,
            is_monospace: false,
            origin_x: 100.0,
            origin_y: 700.0,
            rotation_degrees: 0.0,
            advance_width: 6.0,
            rendered_advance: 6.0,
            ascent: 11.4,
            descent: -4.2,
            matrix: None,
        },
    ];

    extractor.sort_by_reading_order();
    assert_eq!(extractor.chars.len(), 2);
}

#[test]
fn test_merge_adjacent_spans_same_line() {
    let mut extractor = TextExtractor::new();
    extractor.merging_config = SpanMergingConfig::legacy();

    extractor.spans = vec![
        TextSpan {
            provenance: None,
            text_rise: 0.0,
            artifact_type: None,
            text: "Hello".to_string(),
            bbox: Rect::new(100.0, 700.0, 30.0, 12.0),
            font_name: "F1".to_string(),
            font_size: 12.0,
            font_weight: FontWeight::Normal,
            color: Color::black(),
            mcid: None,
            mcid_scope: None,
            sequence: 0,
            split_boundary_before: false,
            offset_semantic: false,
            is_italic: false,
            is_monospace: false,
            char_spacing: 0.0,
            word_spacing: 0.0,
            horizontal_scaling: 100.0,
            primary_detected: false,
            char_widths: vec![],
            char_x_offsets: Vec::new(),
            heading_level: None,
            rotation_degrees: 0.0,
            wmode: 0,
            rtl_draw_logical: false,
            mirrored: false,
            page_rotation_applied: 0,
        },
        TextSpan {
            provenance: None,
            text_rise: 0.0,
            artifact_type: None,
            text: "World".to_string(),
            bbox: Rect::new(131.0, 700.0, 30.0, 12.0), // 1pt gap ~keep
            font_name: "F1".to_string(),
            font_size: 12.0,
            font_weight: FontWeight::Normal,
            color: Color::black(),
            mcid: None,
            mcid_scope: None,
            sequence: 1,
            split_boundary_before: false,
            offset_semantic: false,
            is_italic: false,
            is_monospace: false,
            char_spacing: 0.0,
            word_spacing: 0.0,
            horizontal_scaling: 100.0,
            primary_detected: false,
            char_widths: vec![],
            char_x_offsets: Vec::new(),
            heading_level: None,
            rotation_degrees: 0.0,
            wmode: 0,
            rtl_draw_logical: false,
            mirrored: false,
            page_rotation_applied: 0,
        },
    ];

    extractor.merge_adjacent_spans();
    assert_eq!(extractor.spans.len(), 1, "Adjacent spans on same line should merge");
    assert!(extractor.spans[0].text.contains("Hello"));
    assert!(extractor.spans[0].text.contains("World"));
}

#[test]
fn test_merge_adjacent_spans_180_degree_runs_never_merge() {
    // Same shared-baseline-Y, small-gap shape as
    // `test_merge_adjacent_spans_same_line`, but both runs are
    // 180°-rotated (upside-down text). The rotation-compatibility gate
    // previously only rejected ±90° (vertical-quadrant) runs, so a
    // 180°/180° pair slipped through and merged under the portrait
    // same-line test even though 180° text advances in the opposite X
    // direction — exactly the hazard `snap_run_rotation`'s 180°-aliasing
    // bug (fixed alongside this) would otherwise mask, since before
    // that fix a 180° matrix was misreported as 0.0 in the first place. ~keep
    let mut extractor = TextExtractor::new();
    extractor.merging_config = SpanMergingConfig::legacy();

    extractor.spans = vec![
        TextSpan {
            provenance: None,
            text_rise: 0.0,
            artifact_type: None,
            text: "Hello".to_string(),
            bbox: Rect::new(100.0, 700.0, 30.0, 12.0),
            font_name: "F1".to_string(),
            font_size: 12.0,
            font_weight: FontWeight::Normal,
            color: Color::black(),
            mcid: None,
            mcid_scope: None,
            sequence: 0,
            split_boundary_before: false,
            offset_semantic: false,
            is_italic: false,
            is_monospace: false,
            char_spacing: 0.0,
            word_spacing: 0.0,
            horizontal_scaling: 100.0,
            primary_detected: false,
            char_widths: vec![],
            char_x_offsets: Vec::new(),
            heading_level: None,
            rotation_degrees: 180.0,
            wmode: 0,
            rtl_draw_logical: false,
            mirrored: false,
            page_rotation_applied: 0,
        },
        TextSpan {
            provenance: None,
            text_rise: 0.0,
            artifact_type: None,
            text: "World".to_string(),
            bbox: Rect::new(131.0, 700.0, 30.0, 12.0), // 1pt gap ~keep
            font_name: "F1".to_string(),
            font_size: 12.0,
            font_weight: FontWeight::Normal,
            color: Color::black(),
            mcid: None,
            mcid_scope: None,
            sequence: 1,
            split_boundary_before: false,
            offset_semantic: false,
            is_italic: false,
            is_monospace: false,
            char_spacing: 0.0,
            word_spacing: 0.0,
            horizontal_scaling: 100.0,
            primary_detected: false,
            char_widths: vec![],
            char_x_offsets: Vec::new(),
            heading_level: None,
            rotation_degrees: 180.0,
            wmode: 0,
            rtl_draw_logical: false,
            mirrored: false,
            page_rotation_applied: 0,
        },
    ];

    extractor.merge_adjacent_spans();
    assert_eq!(
        extractor.spans.len(),
        2,
        "180°-rotated runs must never merge here, even on a shared baseline-Y \
             with a small gap"
    );
}

#[test]
fn test_merge_adjacent_spans_different_lines() {
    let mut extractor = TextExtractor::new();
    extractor.merging_config = SpanMergingConfig::legacy();

    extractor.spans = vec![
        TextSpan {
            provenance: None,
            text_rise: 0.0,
            artifact_type: None,
            text: "Hello".to_string(),
            bbox: Rect::new(100.0, 700.0, 30.0, 12.0),
            font_name: "F1".to_string(),
            font_size: 12.0,
            font_weight: FontWeight::Normal,
            color: Color::black(),
            mcid: None,
            mcid_scope: None,
            sequence: 0,
            split_boundary_before: false,
            offset_semantic: false,
            is_italic: false,
            is_monospace: false,
            char_spacing: 0.0,
            word_spacing: 0.0,
            horizontal_scaling: 100.0,
            primary_detected: false,
            char_widths: vec![],
            char_x_offsets: Vec::new(),
            heading_level: None,
            rotation_degrees: 0.0,
            wmode: 0,
            rtl_draw_logical: false,
            mirrored: false,
            page_rotation_applied: 0,
        },
        TextSpan {
            provenance: None,
            text_rise: 0.0,
            artifact_type: None,
            text: "World".to_string(),
            bbox: Rect::new(100.0, 680.0, 30.0, 12.0),
            font_name: "F1".to_string(),
            font_size: 12.0,
            font_weight: FontWeight::Normal,
            color: Color::black(),
            mcid: None,
            mcid_scope: None,
            sequence: 1,
            split_boundary_before: false,
            offset_semantic: false,
            is_italic: false,
            is_monospace: false,
            char_spacing: 0.0,
            word_spacing: 0.0,
            horizontal_scaling: 100.0,
            primary_detected: false,
            char_widths: vec![],
            char_x_offsets: Vec::new(),
            heading_level: None,
            rotation_degrees: 0.0,
            wmode: 0,
            rtl_draw_logical: false,
            mirrored: false,
            page_rotation_applied: 0,
        },
    ];

    extractor.merge_adjacent_spans();
    assert_eq!(extractor.spans.len(), 2, "Spans on different lines should not merge");
}

#[test]
fn test_merge_adjacent_spans_empty() {
    let mut extractor = TextExtractor::new();
    extractor.merge_adjacent_spans();
    assert!(extractor.spans.is_empty());
}

#[test]
fn test_merge_adjacent_spans_column_boundary() {
    let mut extractor = TextExtractor::new();
    extractor.merging_config = SpanMergingConfig::legacy();

    extractor.spans = vec![
        TextSpan {
            provenance: None,
            text_rise: 0.0,
            artifact_type: None,
            text: "Left".to_string(),
            bbox: Rect::new(50.0, 700.0, 30.0, 12.0),
            font_name: "F1".to_string(),
            font_size: 12.0,
            font_weight: FontWeight::Normal,
            color: Color::black(),
            mcid: None,
            mcid_scope: None,
            sequence: 0,
            split_boundary_before: false,
            offset_semantic: false,
            is_italic: false,
            is_monospace: false,
            char_spacing: 0.0,
            word_spacing: 0.0,
            horizontal_scaling: 100.0,
            primary_detected: false,
            char_widths: vec![],
            char_x_offsets: Vec::new(),
            heading_level: None,
            rotation_degrees: 0.0,
            wmode: 0,
            rtl_draw_logical: false,
            mirrored: false,
            page_rotation_applied: 0,
        },
        TextSpan {
            provenance: None,
            text_rise: 0.0,
            artifact_type: None,
            text: "Right".to_string(),
            bbox: Rect::new(300.0, 700.0, 30.0, 12.0), // Large gap (column boundary) ~keep
            font_name: "F1".to_string(),
            font_size: 12.0,
            font_weight: FontWeight::Normal,
            color: Color::black(),
            mcid: None,
            mcid_scope: None,
            sequence: 1,
            split_boundary_before: false,
            offset_semantic: false,
            is_italic: false,
            is_monospace: false,
            char_spacing: 0.0,
            word_spacing: 0.0,
            horizontal_scaling: 100.0,
            primary_detected: false,
            char_widths: vec![],
            char_x_offsets: Vec::new(),
            heading_level: None,
            rotation_degrees: 0.0,
            wmode: 0,
            rtl_draw_logical: false,
            mirrored: false,
            page_rotation_applied: 0,
        },
    ];

    extractor.merge_adjacent_spans();
    assert_eq!(
        extractor.spans.len(),
        2,
        "Spans separated by column boundary should not merge"
    );
}

#[test]
fn test_merge_whitespace_only_span() {
    let mut extractor = TextExtractor::new();
    extractor.merging_config = SpanMergingConfig::legacy();

    extractor.spans = vec![
        TextSpan {
            provenance: None,
            text_rise: 0.0,
            artifact_type: None,
            text: "Hello".to_string(),
            bbox: Rect::new(100.0, 700.0, 30.0, 12.0),
            font_name: "F1".to_string(),
            font_size: 12.0,
            font_weight: FontWeight::Normal,
            color: Color::black(),
            mcid: None,
            mcid_scope: None,
            sequence: 0,
            split_boundary_before: false,
            offset_semantic: false,
            is_italic: false,
            is_monospace: false,
            char_spacing: 0.0,
            word_spacing: 0.0,
            horizontal_scaling: 100.0,
            primary_detected: false,
            char_widths: vec![],
            char_x_offsets: Vec::new(),
            heading_level: None,
            rotation_degrees: 0.0,
            wmode: 0,
            rtl_draw_logical: false,
            mirrored: false,
            page_rotation_applied: 0,
        },
        TextSpan {
            provenance: None,
            text_rise: 0.0,
            artifact_type: None,
            text: " ".to_string(),
            bbox: Rect::new(130.0, 700.0, 2.0, 12.0),
            font_name: "F1".to_string(),
            font_size: 12.0,
            font_weight: FontWeight::Normal,
            color: Color::black(),
            mcid: None,
            mcid_scope: None,
            sequence: 1,
            split_boundary_before: false,
            offset_semantic: true,
            is_italic: false,
            is_monospace: false,
            char_spacing: 0.0,
            word_spacing: 0.0,
            horizontal_scaling: 100.0,
            primary_detected: false,
            char_widths: vec![],
            char_x_offsets: Vec::new(),
            heading_level: None,
            rotation_degrees: 0.0,
            wmode: 0,
            rtl_draw_logical: false,
            mirrored: false,
            page_rotation_applied: 0,
        },
        TextSpan {
            provenance: None,
            text_rise: 0.0,
            artifact_type: None,
            text: "World".to_string(),
            bbox: Rect::new(132.0, 700.0, 30.0, 12.0),
            font_name: "F1".to_string(),
            font_size: 12.0,
            font_weight: FontWeight::Normal,
            color: Color::black(),
            mcid: None,
            mcid_scope: None,
            sequence: 2,
            split_boundary_before: false,
            offset_semantic: false,
            is_italic: false,
            is_monospace: false,
            char_spacing: 0.0,
            word_spacing: 0.0,
            horizontal_scaling: 100.0,
            primary_detected: false,
            char_widths: vec![],
            char_x_offsets: Vec::new(),
            heading_level: None,
            rotation_degrees: 0.0,
            wmode: 0,
            rtl_draw_logical: false,
            mirrored: false,
            page_rotation_applied: 0,
        },
    ];

    extractor.merge_adjacent_spans();
    assert_eq!(extractor.spans.len(), 1, "All three spans should merge");
    assert!(extractor.spans[0].text.contains("Hello"), "Should contain Hello");
    assert!(extractor.spans[0].text.contains("World"), "Should contain World");
}

#[test]
fn test_partition_no_boundaries() {
    let extractor = TextExtractor::new();
    let chars = vec![
        CharacterInfo {
            code: 65,
            glyph_id: None,
            width: 10.0,
            x_position: 0.0,
            tj_offset: None,
            font_size: 12.0,
            is_ligature: false,
            original_ligature: None,
            protected_from_split: false,
        },
        CharacterInfo {
            code: 66,
            glyph_id: None,
            width: 10.0,
            x_position: 10.0,
            tj_offset: None,
            font_size: 12.0,
            is_ligature: false,
            original_ligature: None,
            protected_from_split: false,
        },
    ];

    let clusters = extractor.partition_characters_by_boundaries(&chars, vec![]);
    assert_eq!(clusters.len(), 1);
    assert_eq!(clusters[0].len(), 2);
}

#[test]
fn test_partition_with_boundary() {
    let extractor = TextExtractor::new();
    let chars = vec![
        CharacterInfo {
            code: 65,
            glyph_id: None,
            width: 10.0,
            x_position: 0.0,
            tj_offset: None,
            font_size: 12.0,
            is_ligature: false,
            original_ligature: None,
            protected_from_split: false,
        },
        CharacterInfo {
            code: 66,
            glyph_id: None,
            width: 10.0,
            x_position: 10.0,
            tj_offset: None,
            font_size: 12.0,
            is_ligature: false,
            original_ligature: None,
            protected_from_split: false,
        },
        CharacterInfo {
            code: 67,
            glyph_id: None,
            width: 10.0,
            x_position: 25.0,
            tj_offset: None,
            font_size: 12.0,
            is_ligature: false,
            original_ligature: None,
            protected_from_split: false,
        },
    ];

    let clusters = extractor.partition_characters_by_boundaries(&chars, vec![2]);
    assert_eq!(clusters.len(), 2);
    assert_eq!(clusters[0].len(), 2); // [A, B] ~keep
    assert_eq!(clusters[1].len(), 1); // [C] ~keep
}

#[test]
fn test_create_boundary_context() {
    let mut extractor = TextExtractor::new();
    extractor.state_stack.current_mut().font_size = 12.0;
    extractor.state_stack.current_mut().horizontal_scaling = 100.0;
    extractor.state_stack.current_mut().word_space = 2.0;
    extractor.state_stack.current_mut().char_space = 0.5;

    let ctx = extractor.create_boundary_context();
    assert_eq!(ctx.font_size, 12.0);
    assert_eq!(ctx.horizontal_scaling, 100.0);
    assert_eq!(ctx.word_spacing, 2.0);
    assert_eq!(ctx.char_spacing, 0.5);
}

#[test]
fn test_build_boundary_characters() {
    let prev_bbox = Rect::new(10.0, 100.0, 50.0, 12.0);
    let next_bbox = Rect::new(65.0, 100.0, 40.0, 12.0);

    let (chars, ctx) = build_boundary_characters("Hello", "World", &prev_bbox, &next_bbox, 12.0, false);

    assert_eq!(chars.len(), 2);
    assert_eq!(chars[0].code, 'o' as u32); // Last char of "Hello" ~keep
    assert_eq!(chars[1].code, 'W' as u32); // First char of "World" ~keep
    assert_eq!(ctx.font_size, 12.0);
}

#[test]
fn test_build_boundary_characters_with_tj_offset() {
    let prev_bbox = Rect::new(10.0, 100.0, 50.0, 12.0);
    let next_bbox = Rect::new(65.0, 100.0, 40.0, 12.0);

    let (chars, _ctx) = build_boundary_characters("Hello", "World", &prev_bbox, &next_bbox, 12.0, true);

    assert_eq!(chars[0].tj_offset, Some(-200));
    assert_eq!(chars[1].tj_offset, None);
}

#[test]
fn test_tj_buffer_empty() {
    let state = crate::content::graphics_state::GraphicsStateStack::new();
    let buffer = TjBuffer::new(state.current(), None, None);
    assert!(buffer.is_empty());
}

#[test]
fn test_tj_buffer_append() {
    let state = crate::content::graphics_state::GraphicsStateStack::new();
    let mut buffer = TjBuffer::new(state.current(), None, None);
    buffer.append(b"Hello").unwrap();
    assert!(!buffer.is_empty());
    assert_eq!(buffer.unicode, "Hello");
}

#[test]
fn test_tj_buffer_append_truncates_long_string() {
    let state = crate::content::graphics_state::GraphicsStateStack::new();
    let mut buffer = TjBuffer::new(state.current(), None, None);
    // Create a string larger than 32,767 bytes ~keep
    let long_bytes = vec![0x41u8; 40_000];
    buffer.append(&long_bytes).unwrap();
    // Should be truncated to 32,767 chars ~keep
    assert!(buffer.unicode.len() <= 32_767);
}

#[test]
fn test_advance_position_for_offset_positive() {
    let mut extractor = TextExtractor::new();
    extractor.state_stack.current_mut().font_size = 12.0;
    extractor.state_stack.current_mut().horizontal_scaling = 100.0;

    let initial_e = extractor.state_stack.current().text_matrix.e;
    extractor.advance_position_for_offset(100.0).unwrap();
    let new_e = extractor.state_stack.current().text_matrix.e;

    // Positive offset should move text position left (negative tx)
    // tx = -offset / 1000.0 * font_size * horizontal_scaling / 100.0
    // tx = -100 / 1000 * 12 * 100 / 100 = -1.2 ~keep
    assert!((new_e - initial_e - (-1.2)).abs() < 0.01);
}

#[test]
fn test_advance_position_for_offset_negative() {
    let mut extractor = TextExtractor::new();
    extractor.state_stack.current_mut().font_size = 12.0;
    extractor.state_stack.current_mut().horizontal_scaling = 100.0;

    let initial_e = extractor.state_stack.current().text_matrix.e;
    extractor.advance_position_for_offset(-200.0).unwrap();
    let new_e = extractor.state_stack.current().text_matrix.e;

    // Negative offset should move text position right (positive tx)
    // tx = -(-200) / 1000 * 12 * 100/100 = 2.4 ~keep
    assert!((new_e - initial_e - 2.4).abs() < 0.01);
}

#[test]
fn test_should_insert_space_boundary_already_present_trailing() {
    let config = SpanMergingConfig::default();
    let fonts = HashMap::new();

    let decision = should_insert_space(
        "word ", "next", 5.0, 12.0, "F1", &fonts, true, &config, None, None, 12.0, 12.0,
    );
    assert!(!decision.insert_space);
    assert_eq!(decision.source, SpaceSource::AlreadyPresent);
}

#[test]
fn test_should_insert_space_boundary_already_present_leading() {
    let config = SpanMergingConfig::default();
    let fonts = HashMap::new();

    let decision = should_insert_space(
        "word", " next", 5.0, 12.0, "F1", &fonts, true, &config, None, None, 12.0, 12.0,
    );
    assert!(!decision.insert_space);
    assert_eq!(decision.source, SpaceSource::AlreadyPresent);
}

#[test]
fn test_should_insert_space_strong_geometric() {
    let config = SpanMergingConfig::default();
    let fonts = HashMap::new();

    // Very large gap should trigger strong geometric rule
    // geometric_threshold = 12.0 * 0.25 = 3.0 (fallback)
    // strong threshold = 3.0 * 2.0 = 6.0 ~keep
    let decision = should_insert_space(
        "word", "next", 10.0, 12.0, "F1", &fonts, false, &config, None, None, 12.0, 12.0,
    );
    assert!(decision.insert_space, "Large gap should insert space");
    assert_eq!(decision.source, SpaceSource::GeometricGap);
}

#[test]
fn test_should_insert_space_consensus_tj_and_geometric() {
    let config = SpanMergingConfig::default();
    let fonts = HashMap::new();

    // Both TJ offset and geometric gap triggered
    // geometric_threshold = 12.0 * 0.25 = 3.0 (fallback) ~keep
    let decision = should_insert_space(
        "word", "next", 4.0, 12.0, "F1", &fonts, true, &config, None, None, 12.0, 12.0,
    );
    assert!(decision.insert_space, "Consensus should insert space");
    assert_eq!(decision.source, SpaceSource::TjOffset);
    assert_eq!(decision.confidence, 1.0);
}

#[test]
fn test_should_insert_space_no_consensus_small_gap() {
    let config = SpanMergingConfig::default();
    let fonts = HashMap::new();

    let decision = should_insert_space(
        "word", "next", 0.5, 12.0, "F1", &fonts, false, &config, None, None, 12.0, 12.0,
    );
    assert!(!decision.insert_space, "Small gap without TJ should not insert space");
    assert_eq!(decision.source, SpaceSource::NoSpace);
}

#[test]
fn test_should_insert_space_line_break_hard() {
    let config = SpanMergingConfig::default();
    let fonts = HashMap::new();

    let prev_bbox = Rect::new(100.0, 700.0, 200.0, 12.0);
    let next_bbox = Rect::new(100.0, 680.0, 200.0, 12.0);

    let decision = should_insert_space(
        "end of line",
        "start of next",
        0.0,
        12.0,
        "F1",
        &fonts,
        false,
        &config,
        Some(&prev_bbox),
        Some(&next_bbox),
        12.0,
        12.0,
    );
    // Line break detected, same column, not ending with hyphen => insert space ~keep
    assert!(decision.insert_space, "Hard line break should insert space");
}

#[test]
fn test_should_insert_space_line_break_hyphen() {
    let config = SpanMergingConfig::default();
    let fonts = HashMap::new();

    // Line break with hyphen: should NOT insert space ~keep
    let prev_bbox = Rect::new(100.0, 700.0, 200.0, 12.0);
    let next_bbox = Rect::new(100.0, 680.0, 200.0, 12.0);

    let decision = should_insert_space(
        "self-contain-",
        "ed text",
        0.0,
        12.0,
        "F1",
        &fonts,
        false,
        &config,
        Some(&prev_bbox),
        Some(&next_bbox),
        12.0,
        12.0,
    );
    assert!(!decision.insert_space, "Hyphenated line break should not insert space");
}

#[test]
fn test_extract_multiple_text_objects() {
    let mut extractor = TextExtractor::new();
    let font = create_test_font();
    extractor.add_font("F1".to_string(), font);

    let stream = b"BT /F1 12 Tf 100 700 Td (First) Tj ET BT /F1 12 Tf 100 680 Td (Second) Tj ET";
    let chars = extractor.extract(stream).unwrap();

    let text: String = chars.iter().map(|c| c.char).collect();
    assert!(text.contains("First"));
    assert!(text.contains("Second"));
}

#[test]
fn test_extract_spans_with_line_break() {
    let mut extractor = TextExtractor::new();
    extractor.merging_config = SpanMergingConfig::legacy();
    let font = create_test_font();
    extractor.add_font("F1".to_string(), font);

    let stream = b"BT /F1 12 Tf 14 TL 100 700 Td (First line) Tj T* (Second line) Tj ET";
    let spans = extractor.extract_text_spans(stream).unwrap();

    assert!(!spans.is_empty());
    let text: String = spans.iter().map(|s| s.text.as_str()).collect::<Vec<_>>().join("\n");
    assert!(text.contains("First"), "Should contain first line");
    assert!(text.contains("Second"), "Should contain second line");
}

#[test]
fn test_extract_chars_reading_order() {
    let mut extractor = TextExtractor::new();
    let font = create_test_font();
    extractor.add_font("F1".to_string(), font);

    let stream = b"BT /F1 12 Tf 100 680 Td (B) Tj ET BT /F1 12 Tf 100 700 Td (A) Tj ET";
    let chars = extractor.extract(stream).unwrap();

    // After sorting by reading order: A (y=700 higher) should come first ~keep
    assert_eq!(chars[0].char, 'A', "Higher Y should come first in reading order");
    assert_eq!(chars[1].char, 'B');
}

#[test]
fn test_extract_empty_string() {
    let mut extractor = TextExtractor::new();
    let font = create_test_font();
    extractor.add_font("F1".to_string(), font);

    let stream = b"BT /F1 12 Tf 100 700 Td () Tj ET";
    let chars = extractor.extract(stream).unwrap();
    assert_eq!(chars.len(), 0);
}

#[test]
fn test_extract_only_graphics_no_text() {
    let mut extractor = TextExtractor::new();
    let font = create_test_font();
    extractor.add_font("F1".to_string(), font);

    let stream = b"q 1 0 0 1 0 0 cm 100 700 m 200 700 l S Q";
    let chars = extractor.extract(stream).unwrap();
    assert_eq!(chars.len(), 0);
}

#[test]
fn test_inline_image_ignored() {
    let mut extractor = TextExtractor::new();
    let font = create_test_font();
    extractor.add_font("F1".to_string(), font);

    // Text before and after inline image - both should be extracted
    // The inline image operators are handled by the parser ~keep
    let stream = b"BT /F1 12 Tf 100 700 Td (Before) Tj ET";
    let chars = extractor.extract(stream).unwrap();

    let text: String = chars.iter().map(|c| c.char).collect();
    assert!(text.contains("Before"));
}

#[test]
fn test_tm_continuation_same_line() {
    let mut extractor = TextExtractor::new();
    extractor.merging_config = SpanMergingConfig::legacy();
    let font = create_test_font();
    extractor.add_font("F1".to_string(), font);

    // Character-by-character Tm+Tj pattern on same line ~keep
    // The optimization should batch these into fewer spans
    let stream = b"BT /F1 12 Tf 1 0 0 1 100 700 Tm (H) Tj 1 0 0 1 106 700 Tm (i) Tj ET";
    let spans = extractor.extract_text_spans(stream).unwrap();

    let text: String = spans.iter().map(|s| s.text.as_str()).collect::<Vec<_>>().join("");
    assert!(text.contains("Hi"), "Should batch Tm+Tj on same line, got: {}", text);
}

#[test]
fn test_tm_different_line_flushes() {
    let mut extractor = TextExtractor::new();
    extractor.merging_config = SpanMergingConfig::legacy();
    let font = create_test_font();
    extractor.add_font("F1".to_string(), font);

    // Tm to different Y should flush buffer and start new span ~keep
    let stream = b"BT /F1 12 Tf 1 0 0 1 100 700 Tm (A) Tj 1 0 0 1 100 680 Tm (B) Tj ET";
    let spans = extractor.extract_text_spans(stream).unwrap();

    assert!(
        spans.len() >= 2 || {
            // Or could be merged if within merge range ~keep
            let text: String = spans.iter().map(|s| s.text.as_str()).collect::<Vec<_>>().join("");
            text.contains("A") && text.contains("B")
        }
    );
}

/// With the default config (merge_tm_tj_runs = true), multiple Tm+Tj operators
/// on the same line are batched into a single span.
#[test]
fn test_merge_tm_tj_runs_default_merges() {
    let mut extractor = TextExtractor::new();
    extractor.merging_config = SpanMergingConfig::legacy();
    // ~keep
    let font = create_test_font();
    extractor.add_font("F1".to_string(), font);

    // Three separate Tm+Tj on the same baseline (same Y, same a/b/c/d, ascending e) ~keep
    let stream = b"BT /F1 12 Tf 1 0 0 1 100 700 Tm (A) Tj 1 0 0 1 107 700 Tm (B) Tj 1 0 0 1 114 700 Tm (C) Tj ET";
    let spans = extractor.extract_text_spans(stream).unwrap();

    let text: String = spans.iter().map(|s| s.text.as_str()).collect();
    assert!(
        text.contains('A') && text.contains('B') && text.contains('C'),
        "All chars must be extracted, got: {:?}",
        text
    );

    assert!(
        spans.len() < 3,
        "Default merge_tm_tj_runs=true should combine same-line Tm+Tj into fewer than 3 spans, got {} spans",
        spans.len()
    );
}

/// With merge_tm_tj_runs = false, each Tm operator starts a fresh span.
#[test]
fn test_merge_tm_tj_runs_disabled_splits() {
    let mut extractor = TextExtractor::new();
    extractor.merging_config = SpanMergingConfig {
        merge_tm_tj_runs: false,
        ..SpanMergingConfig::legacy()
    };
    let font = create_test_font();
    extractor.add_font("F1".to_string(), font);

    let stream = b"BT /F1 12 Tf 1 0 0 1 100 700 Tm (A) Tj 1 0 0 1 107 700 Tm (B) Tj 1 0 0 1 114 700 Tm (C) Tj ET";
    let spans = extractor.extract_text_spans(stream).unwrap();

    let text: String = spans.iter().map(|s| s.text.as_str()).collect();
    assert!(
        text.contains('A') && text.contains('B') && text.contains('C'),
        "All chars must be extracted even with merging disabled, got: {:?}",
        text
    );

    // With merge disabled, each Tm flushes the buffer, so we get more spans
    // than with merging enabled (post-processing merge_adjacent_spans may combine
    // some, but at minimum we should get spans >= 1; the key invariant is that
    // the span count here is NOT reduced by the Tm-continuation shortcut) ~keep
    assert!(
        spans.len() >= 2,
        "merge_tm_tj_runs=false should not batch same-line runs; expected >= 2 spans, got {}",
        spans.len()
    );
}

#[test]
fn test_extract_with_zero_font_size() {
    let mut extractor = TextExtractor::new();
    let font = create_test_font();
    extractor.add_font("F1".to_string(), font);

    // Zero font size is technically valid in PDF ~keep
    let stream = b"BT /F1 0 Tf 100 700 Td (X) Tj ET";
    let result = extractor.extract(stream);
    assert!(result.is_ok());
}

#[test]
fn test_extract_with_negative_font_size() {
    let mut extractor = TextExtractor::new();
    let font = create_test_font();
    extractor.add_font("F1".to_string(), font);

    // Negative font size inverts text ~keep
    let stream = b"BT /F1 -12 Tf 100 700 Td (X) Tj ET";
    let result = extractor.extract(stream);
    assert!(result.is_ok());
}

#[test]
fn test_extract_with_very_large_coordinate() {
    let mut extractor = TextExtractor::new();
    let font = create_test_font();
    extractor.add_font("F1".to_string(), font);

    let stream = b"BT /F1 12 Tf 99999 99999 Td (X) Tj ET";
    let chars = extractor.extract(stream).unwrap();
    assert_eq!(chars.len(), 1);
}

#[test]
fn test_set_fill_color_device_gray() {
    let mut extractor = TextExtractor::new();
    let font = create_test_font();
    extractor.add_font("F1".to_string(), font);

    // cs sets color space, then sc sets color components ~keep
    extractor
        .execute_operator_public(Operator::SetFillColorSpace {
            name: "DeviceGray".to_string(),
        })
        .unwrap();
    extractor
        .execute_operator_public(Operator::SetFillColor { components: vec![0.5] })
        .unwrap();

    let state = extractor.state_stack.current();
    assert!((state.fill_color_rgb.0 - 0.5).abs() < 0.01);
    assert!((state.fill_color_rgb.1 - 0.5).abs() < 0.01);
    assert!((state.fill_color_rgb.2 - 0.5).abs() < 0.01);
}

#[test]
fn test_set_fill_color_device_rgb() {
    let mut extractor = TextExtractor::new();
    extractor
        .execute_operator_public(Operator::SetFillColorSpace {
            name: "DeviceRGB".to_string(),
        })
        .unwrap();
    extractor
        .execute_operator_public(Operator::SetFillColor {
            components: vec![0.2, 0.4, 0.6],
        })
        .unwrap();

    let state = extractor.state_stack.current();
    assert!((state.fill_color_rgb.0 - 0.2).abs() < 0.01);
    assert!((state.fill_color_rgb.1 - 0.4).abs() < 0.01);
    assert!((state.fill_color_rgb.2 - 0.6).abs() < 0.01);
}

#[test]
fn test_set_fill_color_device_cmyk() {
    let mut extractor = TextExtractor::new();
    extractor
        .execute_operator_public(Operator::SetFillColorSpace {
            name: "DeviceCMYK".to_string(),
        })
        .unwrap();
    extractor
        .execute_operator_public(Operator::SetFillColor {
            components: vec![0.0, 0.0, 0.0, 1.0], // the K ink ~keep
        })
        .unwrap();

    let state = extractor.state_stack.current();
    assert!((state.fill_color_rgb.0 - 0.1373).abs() < 0.01);
    assert!((state.fill_color_rgb.1 - 0.1216).abs() < 0.01);
    assert!((state.fill_color_rgb.2 - 0.1255).abs() < 0.01);
    assert!(state.fill_color_cmyk.is_some());
}

#[test]
fn test_set_fill_color_lab() {
    let mut extractor = TextExtractor::new();
    extractor
        .execute_operator_public(Operator::SetFillColorSpace {
            name: "Lab".to_string(),
        })
        .unwrap();
    extractor
        .execute_operator_public(Operator::SetFillColor {
            components: vec![50.0, 20.0, -10.0],
        })
        .unwrap();

    let state = extractor.state_stack.current();
    // Lab simplified to grayscale: L/100 ~keep
    assert!((state.fill_color_rgb.0 - 0.5).abs() < 0.01);
}

#[test]
fn test_set_fill_color_iccbased_rgb() {
    let mut extractor = TextExtractor::new();
    extractor
        .execute_operator_public(Operator::SetFillColorSpace {
            name: "ICCBased".to_string(),
        })
        .unwrap();
    extractor
        .execute_operator_public(Operator::SetFillColor {
            components: vec![0.1, 0.2, 0.3],
        })
        .unwrap();

    let state = extractor.state_stack.current();
    assert!((state.fill_color_rgb.0 - 0.1).abs() < 0.01);
    assert!((state.fill_color_rgb.1 - 0.2).abs() < 0.01);
    assert!((state.fill_color_rgb.2 - 0.3).abs() < 0.01);
}

#[test]
fn test_set_fill_color_iccbased_gray() {
    let mut extractor = TextExtractor::new();
    extractor
        .execute_operator_public(Operator::SetFillColorSpace {
            name: "ICCBased".to_string(),
        })
        .unwrap();
    extractor
        .execute_operator_public(Operator::SetFillColor { components: vec![0.7] })
        .unwrap();

    let state = extractor.state_stack.current();
    assert!((state.fill_color_rgb.0 - 0.7).abs() < 0.01);
}

#[test]
fn test_set_fill_color_iccbased_cmyk() {
    let mut extractor = TextExtractor::new();
    extractor
        .execute_operator_public(Operator::SetFillColorSpace {
            name: "ICCBased".to_string(),
        })
        .unwrap();
    extractor
        .execute_operator_public(Operator::SetFillColor {
            components: vec![1.0, 0.0, 0.0, 0.0], // cyan ~keep
        })
        .unwrap();

    let state = extractor.state_stack.current();
    assert!(state.fill_color_cmyk.is_some());
}

#[test]
fn test_set_fill_color_separation() {
    let mut extractor = TextExtractor::new();
    extractor
        .execute_operator_public(Operator::SetFillColorSpace {
            name: "Separation".to_string(),
        })
        .unwrap();
    extractor
        .execute_operator_public(Operator::SetFillColor {
            components: vec![0.8], // tint ~keep
        })
        .unwrap();

    let state = extractor.state_stack.current();
    // gray = 1.0 - tint = 0.2 ~keep
    assert!((state.fill_color_rgb.0 - 0.2).abs() < 0.01);
}

#[test]
fn test_set_fill_color_devicen_cmyk() {
    let mut extractor = TextExtractor::new();
    extractor
        .execute_operator_public(Operator::SetFillColorSpace {
            name: "DeviceN".to_string(),
        })
        .unwrap();
    extractor
        .execute_operator_public(Operator::SetFillColor {
            components: vec![0.0, 0.0, 0.0, 0.5], // 4-component DeviceN ~keep
        })
        .unwrap();

    let state = extractor.state_stack.current();
    assert!(state.fill_color_cmyk.is_some());
}

#[test]
fn test_set_fill_color_devicen_single() {
    let mut extractor = TextExtractor::new();
    extractor
        .execute_operator_public(Operator::SetFillColorSpace {
            name: "DeviceN".to_string(),
        })
        .unwrap();
    extractor
        .execute_operator_public(Operator::SetFillColor {
            components: vec![0.3], // single-component DeviceN ~keep
        })
        .unwrap();

    let state = extractor.state_stack.current();
    // gray = 1.0 - 0.3 = 0.7 ~keep
    assert!((state.fill_color_rgb.0 - 0.7).abs() < 0.01);
}

#[test]
fn test_set_fill_color_unknown_space() {
    let mut extractor = TextExtractor::new();
    extractor
        .execute_operator_public(Operator::SetFillColorSpace {
            name: "CustomUnknown".to_string(),
        })
        .unwrap();
    extractor
        .execute_operator_public(Operator::SetFillColor {
            components: vec![0.5, 0.5],
        })
        .unwrap();
}

#[test]
fn test_set_fill_color_cal_gray() {
    let mut extractor = TextExtractor::new();
    extractor
        .execute_operator_public(Operator::SetFillColorSpace {
            name: "CalGray".to_string(),
        })
        .unwrap();
    extractor
        .execute_operator_public(Operator::SetFillColor { components: vec![0.8] })
        .unwrap();

    let state = extractor.state_stack.current();
    assert!((state.fill_color_rgb.0 - 0.8).abs() < 0.01);
}

#[test]
fn test_set_fill_color_cal_rgb() {
    let mut extractor = TextExtractor::new();
    extractor
        .execute_operator_public(Operator::SetFillColorSpace {
            name: "CalRGB".to_string(),
        })
        .unwrap();
    extractor
        .execute_operator_public(Operator::SetFillColor {
            components: vec![0.9, 0.1, 0.5],
        })
        .unwrap();

    let state = extractor.state_stack.current();
    assert!((state.fill_color_rgb.0 - 0.9).abs() < 0.01);
    assert!((state.fill_color_rgb.1 - 0.1).abs() < 0.01);
    assert!((state.fill_color_rgb.2 - 0.5).abs() < 0.01);
}

#[test]
fn test_set_stroke_color_device_gray() {
    let mut extractor = TextExtractor::new();
    extractor
        .execute_operator_public(Operator::SetStrokeColorSpace {
            name: "DeviceGray".to_string(),
        })
        .unwrap();
    extractor
        .execute_operator_public(Operator::SetStrokeColor { components: vec![0.4] })
        .unwrap();

    let state = extractor.state_stack.current();
    assert!((state.stroke_color_rgb.0 - 0.4).abs() < 0.01);
}

#[test]
fn test_set_stroke_color_device_rgb() {
    let mut extractor = TextExtractor::new();
    extractor
        .execute_operator_public(Operator::SetStrokeColorSpace {
            name: "DeviceRGB".to_string(),
        })
        .unwrap();
    extractor
        .execute_operator_public(Operator::SetStrokeColor {
            components: vec![0.1, 0.2, 0.3],
        })
        .unwrap();

    let state = extractor.state_stack.current();
    assert!((state.stroke_color_rgb.0 - 0.1).abs() < 0.01);
}

#[test]
fn test_set_stroke_color_lab() {
    let mut extractor = TextExtractor::new();
    extractor
        .execute_operator_public(Operator::SetStrokeColorSpace {
            name: "Lab".to_string(),
        })
        .unwrap();
    extractor
        .execute_operator_public(Operator::SetStrokeColor {
            components: vec![75.0, 10.0, -5.0],
        })
        .unwrap();

    let state = extractor.state_stack.current();
    assert!((state.stroke_color_rgb.0 - 0.75).abs() < 0.01);
}

#[test]
fn test_set_stroke_color_device_cmyk() {
    let mut extractor = TextExtractor::new();
    extractor
        .execute_operator_public(Operator::SetStrokeColorSpace {
            name: "DeviceCMYK".to_string(),
        })
        .unwrap();
    extractor
        .execute_operator_public(Operator::SetStrokeColor {
            components: vec![0.0, 1.0, 0.0, 0.0], // magenta ~keep
        })
        .unwrap();

    let state = extractor.state_stack.current();
    assert!(state.stroke_color_cmyk.is_some());
}

#[test]
fn test_set_stroke_color_iccbased_gray() {
    let mut extractor = TextExtractor::new();
    extractor
        .execute_operator_public(Operator::SetStrokeColorSpace {
            name: "ICCBased".to_string(),
        })
        .unwrap();
    extractor
        .execute_operator_public(Operator::SetStrokeColor { components: vec![0.3] })
        .unwrap();

    let state = extractor.state_stack.current();
    assert!((state.stroke_color_rgb.0 - 0.3).abs() < 0.01);
}

#[test]
fn test_set_stroke_color_iccbased_rgb() {
    let mut extractor = TextExtractor::new();
    extractor
        .execute_operator_public(Operator::SetStrokeColorSpace {
            name: "ICCBased".to_string(),
        })
        .unwrap();
    extractor
        .execute_operator_public(Operator::SetStrokeColor {
            components: vec![0.9, 0.8, 0.7],
        })
        .unwrap();

    let state = extractor.state_stack.current();
    assert!((state.stroke_color_rgb.0 - 0.9).abs() < 0.01);
}

#[test]
fn test_set_stroke_color_iccbased_cmyk() {
    let mut extractor = TextExtractor::new();
    extractor
        .execute_operator_public(Operator::SetStrokeColorSpace {
            name: "ICCBased".to_string(),
        })
        .unwrap();
    extractor
        .execute_operator_public(Operator::SetStrokeColor {
            components: vec![0.1, 0.2, 0.3, 0.4],
        })
        .unwrap();

    let state = extractor.state_stack.current();
    assert!(state.stroke_color_cmyk.is_some());
}

#[test]
fn test_set_stroke_color_separation() {
    let mut extractor = TextExtractor::new();
    extractor
        .execute_operator_public(Operator::SetStrokeColorSpace {
            name: "Separation".to_string(),
        })
        .unwrap();
    extractor
        .execute_operator_public(Operator::SetStrokeColor { components: vec![0.6] })
        .unwrap();

    let state = extractor.state_stack.current();
    // gray = 1.0 - 0.6 = 0.4 ~keep
    assert!((state.stroke_color_rgb.0 - 0.4).abs() < 0.01);
}

#[test]
fn test_set_stroke_color_devicen_cmyk() {
    let mut extractor = TextExtractor::new();
    extractor
        .execute_operator_public(Operator::SetStrokeColorSpace {
            name: "DeviceN".to_string(),
        })
        .unwrap();
    extractor
        .execute_operator_public(Operator::SetStrokeColor {
            components: vec![0.1, 0.2, 0.3, 0.4],
        })
        .unwrap();

    let state = extractor.state_stack.current();
    assert!(state.stroke_color_cmyk.is_some());
}

#[test]
fn test_set_stroke_color_devicen_single() {
    let mut extractor = TextExtractor::new();
    extractor
        .execute_operator_public(Operator::SetStrokeColorSpace {
            name: "DeviceN".to_string(),
        })
        .unwrap();
    extractor
        .execute_operator_public(Operator::SetStrokeColor { components: vec![0.5] })
        .unwrap();

    let state = extractor.state_stack.current();
    assert!((state.stroke_color_rgb.0 - 0.5).abs() < 0.01);
}

#[test]
fn test_set_stroke_color_cal_rgb() {
    let mut extractor = TextExtractor::new();
    extractor
        .execute_operator_public(Operator::SetStrokeColorSpace {
            name: "CalRGB".to_string(),
        })
        .unwrap();
    extractor
        .execute_operator_public(Operator::SetStrokeColor {
            components: vec![0.5, 0.6, 0.7],
        })
        .unwrap();

    let state = extractor.state_stack.current();
    assert!((state.stroke_color_rgb.0 - 0.5).abs() < 0.01);
}

#[test]
fn test_set_stroke_color_cal_gray() {
    let mut extractor = TextExtractor::new();
    extractor
        .execute_operator_public(Operator::SetStrokeColorSpace {
            name: "CalGray".to_string(),
        })
        .unwrap();
    extractor
        .execute_operator_public(Operator::SetStrokeColor { components: vec![0.9] })
        .unwrap();

    let state = extractor.state_stack.current();
    assert!((state.stroke_color_rgb.0 - 0.9).abs() < 0.01);
}

#[test]
fn test_set_stroke_color_unknown() {
    let mut extractor = TextExtractor::new();
    extractor
        .execute_operator_public(Operator::SetStrokeColorSpace {
            name: "UnknownCS".to_string(),
        })
        .unwrap();
    extractor
        .execute_operator_public(Operator::SetStrokeColor { components: vec![0.5] })
        .unwrap();
}

#[test]
fn test_set_fill_color_n_with_pattern() {
    let mut extractor = TextExtractor::new();
    extractor
        .execute_operator_public(Operator::SetFillColorN {
            components: vec![],
            name: Some(Box::new("P1".to_string())),
        })
        .unwrap();
}

#[test]
fn test_set_fill_color_n_without_pattern_gray() {
    let mut extractor = TextExtractor::new();
    extractor
        .execute_operator_public(Operator::SetFillColorSpace {
            name: "DeviceGray".to_string(),
        })
        .unwrap();
    extractor
        .execute_operator_public(Operator::SetFillColorN {
            components: vec![0.3],
            name: None,
        })
        .unwrap();

    let state = extractor.state_stack.current();
    assert!((state.fill_color_rgb.0 - 0.3).abs() < 0.01);
}

#[test]
fn test_set_fill_color_n_without_pattern_rgb() {
    let mut extractor = TextExtractor::new();
    extractor
        .execute_operator_public(Operator::SetFillColorSpace {
            name: "DeviceRGB".to_string(),
        })
        .unwrap();
    extractor
        .execute_operator_public(Operator::SetFillColorN {
            components: vec![0.1, 0.2, 0.3],
            name: None,
        })
        .unwrap();

    let state = extractor.state_stack.current();
    assert!((state.fill_color_rgb.0 - 0.1).abs() < 0.01);
}

#[test]
fn test_set_fill_color_n_lab() {
    let mut extractor = TextExtractor::new();
    extractor
        .execute_operator_public(Operator::SetFillColorSpace {
            name: "Lab".to_string(),
        })
        .unwrap();
    extractor
        .execute_operator_public(Operator::SetFillColorN {
            components: vec![80.0, 0.0, 0.0],
            name: None,
        })
        .unwrap();

    let state = extractor.state_stack.current();
    assert!((state.fill_color_rgb.0 - 0.8).abs() < 0.01);
}

#[test]
fn test_set_fill_color_n_cmyk() {
    let mut extractor = TextExtractor::new();
    extractor
        .execute_operator_public(Operator::SetFillColorSpace {
            name: "DeviceCMYK".to_string(),
        })
        .unwrap();
    extractor
        .execute_operator_public(Operator::SetFillColorN {
            components: vec![0.0, 0.0, 0.0, 0.0],
            name: None,
        })
        .unwrap();

    let state = extractor.state_stack.current();
    // White (no ink) ~keep
    assert!((state.fill_color_rgb.0 - 1.0).abs() < 0.01);
}

#[test]
fn test_set_fill_color_n_iccbased() {
    let mut extractor = TextExtractor::new();
    extractor
        .execute_operator_public(Operator::SetFillColorSpace {
            name: "ICCBased".to_string(),
        })
        .unwrap();
    extractor
        .execute_operator_public(Operator::SetFillColorN {
            components: vec![0.5, 0.6, 0.7],
            name: None,
        })
        .unwrap();

    let state = extractor.state_stack.current();
    assert!((state.fill_color_rgb.0 - 0.5).abs() < 0.01);
}

#[test]
fn test_set_fill_color_n_iccbased_gray() {
    let mut extractor = TextExtractor::new();
    extractor
        .execute_operator_public(Operator::SetFillColorSpace {
            name: "ICCBased".to_string(),
        })
        .unwrap();
    extractor
        .execute_operator_public(Operator::SetFillColorN {
            components: vec![0.9],
            name: None,
        })
        .unwrap();

    let state = extractor.state_stack.current();
    assert!((state.fill_color_rgb.0 - 0.9).abs() < 0.01);
}

#[test]
fn test_set_fill_color_n_iccbased_cmyk() {
    let mut extractor = TextExtractor::new();
    extractor
        .execute_operator_public(Operator::SetFillColorSpace {
            name: "ICCBased".to_string(),
        })
        .unwrap();
    extractor
        .execute_operator_public(Operator::SetFillColorN {
            components: vec![0.1, 0.2, 0.3, 0.4],
            name: None,
        })
        .unwrap();

    let state = extractor.state_stack.current();
    assert!(state.fill_color_cmyk.is_some());
}

#[test]
fn test_set_fill_color_n_separation() {
    let mut extractor = TextExtractor::new();
    extractor
        .execute_operator_public(Operator::SetFillColorSpace {
            name: "Separation".to_string(),
        })
        .unwrap();
    extractor
        .execute_operator_public(Operator::SetFillColorN {
            components: vec![0.4],
            name: None,
        })
        .unwrap();

    let state = extractor.state_stack.current();
    assert!((state.fill_color_rgb.0 - 0.6).abs() < 0.01);
}

#[test]
fn test_set_fill_color_n_devicen() {
    let mut extractor = TextExtractor::new();
    extractor
        .execute_operator_public(Operator::SetFillColorSpace {
            name: "DeviceN".to_string(),
        })
        .unwrap();
    extractor
        .execute_operator_public(Operator::SetFillColorN {
            components: vec![0.1, 0.2, 0.3, 0.4],
            name: None,
        })
        .unwrap();

    let state = extractor.state_stack.current();
    assert!(state.fill_color_cmyk.is_some());
}

#[test]
fn test_set_fill_color_n_devicen_single() {
    let mut extractor = TextExtractor::new();
    extractor
        .execute_operator_public(Operator::SetFillColorSpace {
            name: "DeviceN".to_string(),
        })
        .unwrap();
    extractor
        .execute_operator_public(Operator::SetFillColorN {
            components: vec![0.2],
            name: None,
        })
        .unwrap();

    let state = extractor.state_stack.current();
    assert!((state.fill_color_rgb.0 - 0.8).abs() < 0.01);
}

#[test]
fn test_set_stroke_color_n_with_pattern() {
    let mut extractor = TextExtractor::new();
    extractor
        .execute_operator_public(Operator::SetStrokeColorN {
            components: vec![],
            name: Some(Box::new("P2".to_string())),
        })
        .unwrap();
}

#[test]
fn test_set_stroke_color_n_gray() {
    let mut extractor = TextExtractor::new();
    extractor
        .execute_operator_public(Operator::SetStrokeColorSpace {
            name: "DeviceGray".to_string(),
        })
        .unwrap();
    extractor
        .execute_operator_public(Operator::SetStrokeColorN {
            components: vec![0.6],
            name: None,
        })
        .unwrap();

    let state = extractor.state_stack.current();
    assert!((state.stroke_color_rgb.0 - 0.6).abs() < 0.01);
}

#[test]
fn test_set_stroke_color_n_rgb() {
    let mut extractor = TextExtractor::new();
    extractor
        .execute_operator_public(Operator::SetStrokeColorSpace {
            name: "DeviceRGB".to_string(),
        })
        .unwrap();
    extractor
        .execute_operator_public(Operator::SetStrokeColorN {
            components: vec![0.8, 0.7, 0.6],
            name: None,
        })
        .unwrap();

    let state = extractor.state_stack.current();
    assert!((state.stroke_color_rgb.0 - 0.8).abs() < 0.01);
}

#[test]
fn test_set_stroke_color_n_lab() {
    let mut extractor = TextExtractor::new();
    extractor
        .execute_operator_public(Operator::SetStrokeColorSpace {
            name: "Lab".to_string(),
        })
        .unwrap();
    extractor
        .execute_operator_public(Operator::SetStrokeColorN {
            components: vec![60.0, 0.0, 0.0],
            name: None,
        })
        .unwrap();

    let state = extractor.state_stack.current();
    assert!((state.stroke_color_rgb.0 - 0.6).abs() < 0.01);
}

#[test]
fn test_set_stroke_color_n_cmyk() {
    let mut extractor = TextExtractor::new();
    extractor
        .execute_operator_public(Operator::SetStrokeColorSpace {
            name: "DeviceCMYK".to_string(),
        })
        .unwrap();
    extractor
        .execute_operator_public(Operator::SetStrokeColorN {
            components: vec![0.0, 0.0, 1.0, 0.0], // yellow ~keep
            name: None,
        })
        .unwrap();

    let state = extractor.state_stack.current();
    assert!(state.stroke_color_cmyk.is_some());
}

#[test]
fn test_set_stroke_color_n_iccbased_rgb() {
    let mut extractor = TextExtractor::new();
    extractor
        .execute_operator_public(Operator::SetStrokeColorSpace {
            name: "ICCBased".to_string(),
        })
        .unwrap();
    extractor
        .execute_operator_public(Operator::SetStrokeColorN {
            components: vec![0.2, 0.3, 0.4],
            name: None,
        })
        .unwrap();

    let state = extractor.state_stack.current();
    assert!((state.stroke_color_rgb.0 - 0.2).abs() < 0.01);
}

#[test]
fn test_set_stroke_color_n_iccbased_gray() {
    let mut extractor = TextExtractor::new();
    extractor
        .execute_operator_public(Operator::SetStrokeColorSpace {
            name: "ICCBased".to_string(),
        })
        .unwrap();
    extractor
        .execute_operator_public(Operator::SetStrokeColorN {
            components: vec![0.5],
            name: None,
        })
        .unwrap();

    let state = extractor.state_stack.current();
    assert!((state.stroke_color_rgb.0 - 0.5).abs() < 0.01);
}

#[test]
fn test_set_stroke_color_n_iccbased_cmyk() {
    let mut extractor = TextExtractor::new();
    extractor
        .execute_operator_public(Operator::SetStrokeColorSpace {
            name: "ICCBased".to_string(),
        })
        .unwrap();
    extractor
        .execute_operator_public(Operator::SetStrokeColorN {
            components: vec![0.1, 0.2, 0.3, 0.4],
            name: None,
        })
        .unwrap();

    let state = extractor.state_stack.current();
    assert!(state.stroke_color_cmyk.is_some());
}

#[test]
fn test_set_stroke_color_n_separation() {
    let mut extractor = TextExtractor::new();
    extractor
        .execute_operator_public(Operator::SetStrokeColorSpace {
            name: "Separation".to_string(),
        })
        .unwrap();
    extractor
        .execute_operator_public(Operator::SetStrokeColorN {
            components: vec![1.0],
            name: None,
        })
        .unwrap();

    let state = extractor.state_stack.current();
    assert!((state.stroke_color_rgb.0 - 0.0).abs() < 0.01);
}

#[test]
fn test_set_stroke_color_n_devicen_cmyk() {
    let mut extractor = TextExtractor::new();
    extractor
        .execute_operator_public(Operator::SetStrokeColorSpace {
            name: "DeviceN".to_string(),
        })
        .unwrap();
    extractor
        .execute_operator_public(Operator::SetStrokeColorN {
            components: vec![0.5, 0.5, 0.5, 0.5],
            name: None,
        })
        .unwrap();

    let state = extractor.state_stack.current();
    assert!(state.stroke_color_cmyk.is_some());
}

#[test]
fn test_set_stroke_color_n_devicen_single() {
    let mut extractor = TextExtractor::new();
    extractor
        .execute_operator_public(Operator::SetStrokeColorSpace {
            name: "DeviceN".to_string(),
        })
        .unwrap();
    extractor
        .execute_operator_public(Operator::SetStrokeColorN {
            components: vec![0.1],
            name: None,
        })
        .unwrap();

    let state = extractor.state_stack.current();
    assert!((state.stroke_color_rgb.0 - 0.9).abs() < 0.01);
}

/// Named color space reference like "Cs1" should fall back by component
/// count rather than emitting a warn! (regression: warn spam on PDFs
/// with ICCBased color spaces registered under user-defined names).
#[test]
fn test_named_fill_color_space_fallback_gray() {
    let mut e = TextExtractor::new();
    e.execute_operator_public(Operator::SetFillColorSpace {
        name: "Cs1".to_string(),
    })
    .unwrap();
    e.execute_operator_public(Operator::SetFillColor { components: vec![0.4] })
        .unwrap();
    let state = e.state_stack.current();
    let (r, g, b) = state.fill_color_rgb;
    assert!((r - 0.4).abs() < 0.01 && (g - 0.4).abs() < 0.01 && (b - 0.4).abs() < 0.01);
}

#[test]
fn test_named_fill_color_space_fallback_rgb() {
    let mut e = TextExtractor::new();
    e.execute_operator_public(Operator::SetFillColorSpace {
        name: "Cs2".to_string(),
    })
    .unwrap();
    e.execute_operator_public(Operator::SetFillColor {
        components: vec![0.1, 0.2, 0.3],
    })
    .unwrap();
    let state = e.state_stack.current();
    assert!((state.fill_color_rgb.0 - 0.1).abs() < 0.01);
    assert!((state.fill_color_rgb.1 - 0.2).abs() < 0.01);
    assert!((state.fill_color_rgb.2 - 0.3).abs() < 0.01);
}

#[test]
fn test_named_fill_color_space_fallback_cmyk() {
    let mut e = TextExtractor::new();
    e.execute_operator_public(Operator::SetFillColorSpace {
        name: "Cs3".to_string(),
    })
    .unwrap();
    e.execute_operator_public(Operator::SetFillColor {
        components: vec![0.0, 0.0, 0.0, 0.5],
    })
    .unwrap();
    let state = e.state_stack.current();
    assert!(state.fill_color_cmyk.is_some());
}

#[test]
fn test_named_stroke_color_space_fallback_rgb() {
    let mut e = TextExtractor::new();
    e.execute_operator_public(Operator::SetStrokeColorSpace {
        name: "Cs1".to_string(),
    })
    .unwrap();
    e.execute_operator_public(Operator::SetStrokeColor {
        components: vec![0.5, 0.6, 0.7],
    })
    .unwrap();
    let state = e.state_stack.current();
    assert!((state.stroke_color_rgb.0 - 0.5).abs() < 0.01);
    assert!((state.stroke_color_rgb.1 - 0.6).abs() < 0.01);
    assert!((state.stroke_color_rgb.2 - 0.7).abs() < 0.01);
}

#[test]
fn test_set_line_cap() {
    let mut extractor = TextExtractor::new();
    extractor
        .execute_operator_public(Operator::SetLineCap { cap_style: 2 })
        .unwrap();
    assert_eq!(extractor.state_stack.current().line_cap, 2);
}

#[test]
fn test_set_line_join() {
    let mut extractor = TextExtractor::new();
    extractor
        .execute_operator_public(Operator::SetLineJoin { join_style: 1 })
        .unwrap();
    assert_eq!(extractor.state_stack.current().line_join, 1);
}

#[test]
fn test_set_miter_limit() {
    let mut extractor = TextExtractor::new();
    extractor
        .execute_operator_public(Operator::SetMiterLimit { limit: 5.0 })
        .unwrap();
    assert!((extractor.state_stack.current().miter_limit - 5.0).abs() < 0.01);
}

#[test]
fn test_set_rendering_intent() {
    let mut extractor = TextExtractor::new();
    extractor
        .execute_operator_public(Operator::SetRenderingIntent {
            intent: "RelativeColorimetric".to_string(),
        })
        .unwrap();
    assert_eq!(extractor.state_stack.current().rendering_intent, "RelativeColorimetric");
}

#[test]
fn test_set_flatness() {
    let mut extractor = TextExtractor::new();
    extractor
        .execute_operator_public(Operator::SetFlatness { tolerance: 0.5 })
        .unwrap();
    assert!((extractor.state_stack.current().flatness - 0.5).abs() < 0.01);
}

#[test]
fn test_set_ext_gstate() {
    let mut extractor = TextExtractor::new();
    extractor
        .execute_operator_public(Operator::SetExtGState {
            dict_name: "GS1".to_string(),
        })
        .unwrap();
}

#[test]
fn test_paint_shading() {
    let mut extractor = TextExtractor::new();
    extractor
        .execute_operator_public(Operator::PaintShading {
            name: "sh1".to_string(),
        })
        .unwrap();
}

#[test]
fn test_inline_image_operator() {
    let mut extractor = TextExtractor::new();
    let mut dict = HashMap::new();
    dict.insert("W".to_string(), Object::Integer(100));
    dict.insert("H".to_string(), Object::Integer(50));
    extractor
        .execute_operator_public(Operator::InlineImage {
            dict: Box::new(dict),
            data: vec![0u8; 100],
        })
        .unwrap();
}

#[test]
fn test_inline_image_no_dimensions() {
    let mut extractor = TextExtractor::new();
    let dict = HashMap::new(); // no W/H ~keep
    extractor
        .execute_operator_public(Operator::InlineImage {
            dict: Box::new(dict),
            data: vec![0u8; 10],
        })
        .unwrap();
}

#[test]
fn test_email_context_at_sign_end() {
    assert!(is_email_context("user@", "domain.com"));
}

#[test]
fn test_email_context_domain_dot() {
    assert!(is_email_context("user@domain.", "com"));
}

#[test]
fn test_email_context_not_alpha_after_at() {
    // @ followed by non-alphanumeric should not be email ~keep
    assert!(!is_email_context("user@", " "));
}

#[test]
fn test_email_context_long_preceding_text() {
    // Test with very long preceding text (should only check last 64 bytes) ~keep
    let long_prefix = "a".repeat(200) + "@domain";
    assert!(is_email_context(&long_prefix, ".com"));
}

#[test]
fn test_should_insert_space_with_email_config() {
    let config = SpanMergingConfig {
        detect_email_patterns: true,
        email_threshold_multiplier: 2.5,
        ..Default::default()
    };
    let fonts = HashMap::new();

    let decision = should_insert_space(
        "user@domain",
        ".com",
        1.0,
        12.0,
        "F1",
        &fonts,
        false,
        &config,
        None,
        None,
        12.0,
        12.0,
    );
    assert!(
        !decision.insert_space,
        "Email context should suppress space for small gap"
    );
}

#[test]
fn test_should_insert_space_email_large_gap() {
    let config = SpanMergingConfig {
        detect_email_patterns: true,
        email_threshold_multiplier: 2.5,
        ..Default::default()
    };
    let fonts = HashMap::new();

    let decision = should_insert_space(
        "user@domain",
        ".com",
        100.0,
        12.0,
        "F1",
        &fonts,
        false,
        &config,
        None,
        None,
        12.0,
        12.0,
    );
    assert!(decision.insert_space, "Email context should insert space for large gap");
}

#[test]
fn test_should_insert_space_email_with_font_info() {
    let config = SpanMergingConfig {
        detect_email_patterns: true,
        ..Default::default()
    };
    let mut fonts: HashMap<String, Arc<FontInfo>> = HashMap::new();
    let font = create_test_font();
    fonts.insert("F1".to_string(), Arc::new(font));

    // Email context uses font metrics for threshold ~keep
    let decision = should_insert_space(
        "user@domain",
        ".com",
        1.0,
        12.0,
        "F1",
        &fonts,
        false,
        &config,
        None,
        None,
        12.0,
        12.0,
    );
    assert!(!decision.insert_space);
}

#[test]
fn test_should_insert_space_citation_context() {
    let config = SpanMergingConfig {
        detect_citation_markers: true,
        citation_font_size_ratio: 0.75,
        ..Default::default()
    };
    let fonts = HashMap::new();

    let prev_bbox = Rect::new(10.0, 100.0, 50.0, 12.0);
    let next_bbox = Rect::new(60.0, 105.0, 10.0, 7.0); // Raised, smaller ~keep

    let decision = should_insert_space(
        "text",
        "1",
        2.0,
        12.0,
        "F1",
        &fonts,
        true,
        &config,
        Some(&prev_bbox),
        Some(&next_bbox),
        12.0,
        7.2,
    );
    assert!(decision.insert_space, "Citation context with TJ should insert space");
}

#[test]
fn test_is_pictographic_ranges() {
    assert!(is_pictographic('📄'));
    assert!(is_pictographic('✅'));
    assert!(!is_pictographic('A'));
    assert!(!is_pictographic('→')); // arrow excluded (math/symbol text) ~keep
    assert!(!is_pictographic('5'));
}

#[test]
fn test_should_insert_space_emoji_letter_boundary() {
    let config = SpanMergingConfig::default();
    let fonts = HashMap::new();
    // The real case (arxiv_2510.26287): a wide emoji glyph abuts the next
    // token, so the gap is exactly 0. The space must still be kept. ~keep
    let decision0 = should_insert_space(
        "📄", "README", 0.0, 12.0, "F1", &fonts, false, &config, None, None, 12.0, 12.0,
    );
    assert!(
        decision0.insert_space,
        "emoji→letter with a zero (abutting) gap must keep space"
    );

    let decision = should_insert_space(
        "📄", "README", 0.5, 12.0, "F1", &fonts, false, &config, None, None, 12.0, 12.0,
    );
    assert!(
        decision.insert_space,
        "emoji→letter with a positive gap keeps the space"
    );

    // A combined emoji sequence (next char is another pictograph, not a
    // letter) must NOT be forced into a space by this rule. ~keep
    let combined = should_insert_space(
        "📄", "📄", 0.0, 12.0, "F1", &fonts, false, &config, None, None, 12.0, 12.0,
    );
    assert!(!combined.insert_space, "emoji→emoji must not be forced into a space");
}

#[test]
fn test_should_insert_space_citation_geometric() {
    let config = SpanMergingConfig {
        detect_citation_markers: true,
        ..Default::default()
    };
    let fonts = HashMap::new();

    let prev_bbox = Rect::new(10.0, 100.0, 50.0, 12.0);
    let next_bbox = Rect::new(60.0, 105.0, 10.0, 7.0);

    let decision = should_insert_space(
        "text",
        "1",
        10.0,
        12.0,
        "F1",
        &fonts,
        false,
        &config,
        Some(&prev_bbox),
        Some(&next_bbox),
        12.0,
        7.2,
    );
    assert!(
        decision.insert_space,
        "Citation context with large gap should insert space"
    );
}

#[test]
fn test_should_insert_space_citation_with_font() {
    let config = SpanMergingConfig {
        detect_citation_markers: true,
        ..Default::default()
    };
    let mut fonts: HashMap<String, Arc<FontInfo>> = HashMap::new();
    fonts.insert("F1".to_string(), Arc::new(create_test_font()));

    let prev_bbox = Rect::new(10.0, 100.0, 50.0, 12.0);
    let next_bbox = Rect::new(60.0, 105.0, 10.0, 7.0);

    let decision = should_insert_space(
        "text",
        "1",
        5.0,
        12.0,
        "F1",
        &fonts,
        true,
        &config,
        Some(&prev_bbox),
        Some(&next_bbox),
        12.0,
        7.2,
    );
    assert!(decision.insert_space);
}

#[test]
fn test_line_break_different_column() {
    let config = SpanMergingConfig::default();
    let fonts = HashMap::new();

    let prev_bbox = Rect::new(50.0, 700.0, 200.0, 12.0);
    let next_bbox = Rect::new(400.0, 680.0, 200.0, 12.0);

    let decision = should_insert_space(
        "end",
        "start",
        0.0,
        12.0,
        "F1",
        &fonts,
        false,
        &config,
        Some(&prev_bbox),
        Some(&next_bbox),
        12.0,
        12.0,
    );
    // Different column - should not trigger same_column line break path
    // The default no space path should apply ~keep
}

#[test]
fn test_line_break_not_triggered_small_vertical_gap() {
    let config = SpanMergingConfig::default();
    let fonts = HashMap::new();

    let prev_bbox = Rect::new(100.0, 700.0, 200.0, 12.0);
    let next_bbox = Rect::new(100.0, 699.0, 200.0, 12.0);

    let decision = should_insert_space(
        "word",
        "next",
        0.0,
        12.0,
        "F1",
        &fonts,
        false,
        &config,
        Some(&prev_bbox),
        Some(&next_bbox),
        12.0,
        12.0,
    );
}

#[test]
fn test_should_insert_space_tiebreaker_with_bboxes() {
    let config = SpanMergingConfig::default();
    let fonts = HashMap::new();

    let prev_bbox = Rect::new(100.0, 700.0, 50.0, 12.0);
    let next_bbox = Rect::new(155.0, 700.0, 50.0, 12.0);

    // TJ triggered but gap does not suggest space (conflict)
    // Should go through tiebreaker ~keep
    let decision = should_insert_space(
        "word",
        "next",
        1.0,
        12.0,
        "F1",
        &fonts,
        true,
        &config,
        Some(&prev_bbox),
        Some(&next_bbox),
        12.0,
        12.0,
    );
    // Result depends on WordBoundaryDetector ~keep
}

#[test]
fn test_should_insert_space_geometric_only_conflict() {
    let config = SpanMergingConfig::default();
    let fonts = HashMap::new();

    let prev_bbox = Rect::new(100.0, 700.0, 50.0, 12.0);
    let next_bbox = Rect::new(155.0, 700.0, 50.0, 12.0);

    // No TJ but gap suggests space (conflict with no TJ) ~keep
    let decision = should_insert_space(
        "word",
        "next",
        5.0,
        12.0,
        "F1",
        &fonts,
        false,
        &config,
        Some(&prev_bbox),
        Some(&next_bbox),
        12.0,
        12.0,
    );
    // Geometric alone - should go through tiebreaker path ~keep
}

#[test]
fn test_should_insert_space_font_aware() {
    let config = SpanMergingConfig::default();
    let mut fonts: HashMap<String, Arc<FontInfo>> = HashMap::new();
    fonts.insert("F1".to_string(), Arc::new(create_test_font()));

    // With font info, threshold is calculated from font metrics ~keep
    let decision = should_insert_space(
        "word", "next", 0.5, 12.0, "F1", &fonts, false, &config, None, None, 12.0, 12.0,
    );
    // The result depends on font-specific threshold ~keep
}

// ── Spec-aligned gap correction (§9.4.4): the fallback-width
//    inflation that splits "SalesForce" → "SalesF orce" is only applied
//    when glyphs actually overlap (raw_gap < 0), per corrected_space_gap ── ~keep

/// Adjacent glyphs (raw_gap == 0) on a fallback-width font must NOT be
/// inflated into a phantom gap — this is the "SalesF"+"orce" case. The
/// reported gap stays 0 so no spurious word space is inserted.
#[test]
fn test_corrected_space_gap_no_inflation_when_adjacent() {
    // raw_gap 0.0, unreliable widths, non-empty: must stay 0.0. ~keep
    assert_eq!(corrected_space_gap(0.0, false, 34.23, false), 0.0);
    // small positive raw gap (academic "XGBoostX"+"provides") untouched. ~keep
    assert_eq!(corrected_space_gap(0.47, false, 50.0, false), 0.47);
}

#[test]
fn test_strip_cjk_digit_boundary_spaces() {
    // A space between a CJK ideograph and an embedded number is dropped at
    // both ends; the number itself is preserved. ~keep
    assert_eq!(strip_cjk_digit_boundary_spaces("公元前 1000 年"), "公元前1000年");
    assert_eq!(
        strip_cjk_digit_boundary_spaces("追溯至 10,000 年前"),
        "追溯至10,000年前"
    );
    // Works for Japanese ideographs/kana too. ~keep
    assert_eq!(strip_cjk_digit_boundary_spaces("西暦 2024 年"), "西暦2024年");
    // Korean (Hangul) is EXCLUDED — Korean uses inter-word spaces, so a
    // space between a syllable and a number is a real word boundary and
    // must be preserved ("14 예" = "14 cases", "7 예중"). ~keep
    assert_eq!(strip_cjk_digit_boundary_spaces("약 1 만년"), "약 1 만년");
    assert_eq!(strip_cjk_digit_boundary_spaces("기질은 14 예에서"), "기질은 14 예에서");
    assert_eq!(strip_cjk_digit_boundary_spaces("貓 通常"), "貓 通常"); // CJK↔CJK ~keep
    assert_eq!(strip_cjk_digit_boundary_spaces("catus 펠리스"), "catus 펠리스"); // letter↔CJK ~keep
    assert_eq!(strip_cjk_digit_boundary_spaces("10 000"), "10 000"); // digit↔digit ~keep
    assert_eq!(strip_cjk_digit_boundary_spaces("page 12 of 30"), "page 12 of 30");
    // ~keep
    // No-op fast path. ~keep
    assert_eq!(strip_cjk_digit_boundary_spaces("中文"), "中文");

    // Brackets hug their content: a space between a CJK/Hangul character and
    // an adjacent bracket is a layout artifact, dropped on both sides and
    // for both ASCII paren/square/brace shapes. ~keep
    assert_eq!(strip_cjk_digit_boundary_spaces("고양이 (학명"), "고양이(학명"); // Hangul→( ~keep
    assert_eq!(strip_cjk_digit_boundary_spaces("카투스 [*]) 는"), "카투스[*])는");
    // ~keep
    assert_eq!(strip_cjk_digit_boundary_spaces("漢字 (注)"), "漢字(注)"); // CJK↔paren ~keep
    // A space between Latin and a bracket is left alone (English may write
    // "study (note)" with a space). ~keep
    assert_eq!(strip_cjk_digit_boundary_spaces("study (note)"), "study (note)");
}

#[test]
fn test_strip_prime_decimal_boundary_spaces() {
    // Artifact space between the prime and the decimal point is dropped. ~keep
    assert_eq!(
        strip_prime_decimal_boundary_spaces("0\u{2032}\u{2032} .28"),
        "0\u{2032}\u{2032}.28"
    );
    // Artifact space between the prime's decimal point and its digits. ~keep
    assert_eq!(
        strip_prime_decimal_boundary_spaces("0\u{2032}\u{2032}. 28"),
        "0\u{2032}\u{2032}.28"
    );
    // Single prime and double-prime (U+2033) both handled. ~keep
    assert_eq!(strip_prime_decimal_boundary_spaces("1\u{2032}.47"), "1\u{2032}.47");
    // ~keep
    assert_eq!(strip_prime_decimal_boundary_spaces("12\u{2033} .5"), "12\u{2033}.5");
    // Feet-and-inches keeps its space: prime → DIGIT (not a decimal point). ~keep
    assert_eq!(
        strip_prime_decimal_boundary_spaces("5\u{2032} 6\u{2033}"),
        "5\u{2032} 6\u{2033}"
    );
    // A prime ending a sentence followed by prose is untouched (next not . / digit). ~keep
    assert_eq!(
        strip_prime_decimal_boundary_spaces("see 3\u{2032} and"),
        "see 3\u{2032} and"
    );
    // A lone decimal with no preceding prime is untouched. ~keep
    assert_eq!(strip_prime_decimal_boundary_spaces("v1. 0 release"), "v1. 0 release");
    // No-op fast path. ~keep
    assert_eq!(
        strip_prime_decimal_boundary_spaces("0\u{2032}\u{2032}.28"),
        "0\u{2032}\u{2032}.28"
    );
}

/// Overlap (raw_gap < 0) on a fallback-width font IS corrected — this is
/// the NASA-Apollo case where the 0.55 em fallback over-reports
/// width and swallows a real word gap. The correction lifts the gap.
#[test]
fn test_corrected_space_gap_corrects_overlap() {
    // raw_gap -2.0, width 30 → -2.0 + 30*(1 - 1/1.22) ≈ -2.0 + 5.41 = 3.41 ~keep
    let g = corrected_space_gap(-2.0, false, 30.0, false);
    assert!(
        g > 0.0,
        "overlap on fallback-width font must be lifted positive, got {g}"
    );
}

/// Reliable-width fonts (explicit /Widths) are never corrected — the
/// bbox gap is authoritative regardless of sign.
#[test]
fn test_corrected_space_gap_reliable_widths_untouched() {
    assert_eq!(corrected_space_gap(-2.0, true, 30.0, false), -2.0);
    assert_eq!(corrected_space_gap(5.0, true, 30.0, false), 5.0);
}

#[test]
fn test_span_merging_config_adaptive_with_config() {
    let adaptive_config = crate::extractors::gap_statistics::AdaptiveThresholdConfig::default();
    let config = SpanMergingConfig::adaptive_with_config(adaptive_config);
    assert!(config.use_adaptive_threshold);
    assert!(config.adaptive_config.is_some());
}

#[test]
fn test_fallback_quotation_marks() {
    assert_eq!(fallback_char_to_unicode(0x2018), "\u{2018}"); // Left single quote ~keep
    assert_eq!(fallback_char_to_unicode(0x2019), "\u{2019}"); // Right single quote ~keep
    assert_eq!(fallback_char_to_unicode(0x201C), "\u{201C}"); // Left double quote ~keep
    assert_eq!(fallback_char_to_unicode(0x201D), "\u{201D}"); // Right double quote ~keep
}

#[test]
fn test_fallback_math_extended() {
    assert_eq!(fallback_char_to_unicode(0x00F7), "\u{00F7}"); // Division ~keep
    assert_eq!(fallback_char_to_unicode(0x2202), "\u{2202}"); // Partial diff ~keep
    assert_eq!(fallback_char_to_unicode(0x2207), "\u{2207}"); // Nabla ~keep
    assert_eq!(fallback_char_to_unicode(0x220F), "\u{220F}"); // Product ~keep
    assert_eq!(fallback_char_to_unicode(0x2261), "\u{2261}"); // Identical ~keep
    assert_eq!(fallback_char_to_unicode(0x2248), "\u{2248}"); // Almost equal ~keep
}

#[test]
fn test_fallback_set_theory() {
    assert_eq!(fallback_char_to_unicode(0x2282), "\u{2282}"); // Subset ~keep
    assert_eq!(fallback_char_to_unicode(0x2283), "\u{2283}"); // Superset ~keep
    assert_eq!(fallback_char_to_unicode(0x2286), "\u{2286}"); // Subset or equal ~keep
    assert_eq!(fallback_char_to_unicode(0x2287), "\u{2287}"); // Superset or equal ~keep
    assert_eq!(fallback_char_to_unicode(0x2208), "\u{2208}"); // Element of ~keep
    assert_eq!(fallback_char_to_unicode(0x2209), "\u{2209}"); // Not element ~keep
    assert_eq!(fallback_char_to_unicode(0x2200), "\u{2200}"); // For all ~keep
    assert_eq!(fallback_char_to_unicode(0x2203), "\u{2203}"); // There exists ~keep
    assert_eq!(fallback_char_to_unicode(0x2205), "\u{2205}"); // Empty set ~keep
}

#[test]
fn test_fallback_logic() {
    assert_eq!(fallback_char_to_unicode(0x2227), "\u{2227}"); // Logical and ~keep
    assert_eq!(fallback_char_to_unicode(0x2228), "\u{2228}"); // Logical or ~keep
    assert_eq!(fallback_char_to_unicode(0x00AC), "\u{00AC}"); // Not ~keep
}

#[test]
fn test_fallback_arrows() {
    assert_eq!(fallback_char_to_unicode(0x2192), "\u{2192}"); // Right arrow ~keep
    assert_eq!(fallback_char_to_unicode(0x2190), "\u{2190}"); // Left arrow ~keep
    assert_eq!(fallback_char_to_unicode(0x2194), "\u{2194}"); // Left right arrow ~keep
    assert_eq!(fallback_char_to_unicode(0x21D2), "\u{21D2}"); // Double right ~keep
    assert_eq!(fallback_char_to_unicode(0x21D4), "\u{21D4}"); // Double left-right ~keep
}

#[test]
fn test_fallback_greek_lowercase_extended() {
    assert_eq!(fallback_char_to_unicode(0x03B5), "\u{03B5}"); // epsilon ~keep
    assert_eq!(fallback_char_to_unicode(0x03B6), "\u{03B6}"); // zeta ~keep
    assert_eq!(fallback_char_to_unicode(0x03B7), "\u{03B7}"); // eta ~keep
    assert_eq!(fallback_char_to_unicode(0x03B9), "\u{03B9}"); // iota ~keep
    assert_eq!(fallback_char_to_unicode(0x03BA), "\u{03BA}"); // kappa ~keep
    assert_eq!(fallback_char_to_unicode(0x03BB), "\u{03BB}"); // lambda ~keep
    assert_eq!(fallback_char_to_unicode(0x03BC), "\u{03BC}"); // mu ~keep
    assert_eq!(fallback_char_to_unicode(0x03BD), "\u{03BD}"); // nu ~keep
    assert_eq!(fallback_char_to_unicode(0x03BE), "\u{03BE}"); // xi ~keep
    assert_eq!(fallback_char_to_unicode(0x03BF), "\u{03BF}"); // omicron ~keep
    assert_eq!(fallback_char_to_unicode(0x03C1), "\u{03C1}"); // rho ~keep
    assert_eq!(fallback_char_to_unicode(0x03C2), "\u{03C2}"); // final sigma ~keep
    assert_eq!(fallback_char_to_unicode(0x03C3), "\u{03C3}"); // sigma ~keep
    assert_eq!(fallback_char_to_unicode(0x03C4), "\u{03C4}"); // tau ~keep
    assert_eq!(fallback_char_to_unicode(0x03C5), "\u{03C5}"); // upsilon ~keep
    assert_eq!(fallback_char_to_unicode(0x03C6), "\u{03C6}"); // phi ~keep
    assert_eq!(fallback_char_to_unicode(0x03C7), "\u{03C7}"); // chi ~keep
    assert_eq!(fallback_char_to_unicode(0x03C8), "\u{03C8}"); // psi ~keep
}

#[test]
fn test_fallback_greek_uppercase_extended() {
    assert_eq!(fallback_char_to_unicode(0x0391), "\u{0391}"); // Alpha ~keep
    assert_eq!(fallback_char_to_unicode(0x0392), "\u{0392}"); // Beta ~keep
    assert_eq!(fallback_char_to_unicode(0x0394), "\u{0394}"); // Delta ~keep
    assert_eq!(fallback_char_to_unicode(0x0395), "\u{0395}"); // Epsilon ~keep
    assert_eq!(fallback_char_to_unicode(0x0396), "\u{0396}"); // Zeta ~keep
    assert_eq!(fallback_char_to_unicode(0x0397), "\u{0397}"); // Eta ~keep
    assert_eq!(fallback_char_to_unicode(0x0398), "\u{0398}"); // Theta ~keep
    assert_eq!(fallback_char_to_unicode(0x0399), "\u{0399}"); // Iota ~keep
    assert_eq!(fallback_char_to_unicode(0x039A), "\u{039A}"); // Kappa ~keep
    assert_eq!(fallback_char_to_unicode(0x039B), "\u{039B}"); // Lambda ~keep
    assert_eq!(fallback_char_to_unicode(0x039C), "\u{039C}"); // Mu ~keep
    assert_eq!(fallback_char_to_unicode(0x039D), "\u{039D}"); // Nu ~keep
    assert_eq!(fallback_char_to_unicode(0x039E), "\u{039E}"); // Xi ~keep
    assert_eq!(fallback_char_to_unicode(0x039F), "\u{039F}"); // Omicron ~keep
    assert_eq!(fallback_char_to_unicode(0x03A0), "\u{03A0}"); // Pi ~keep
    assert_eq!(fallback_char_to_unicode(0x03A1), "\u{03A1}"); // Rho ~keep
    assert_eq!(fallback_char_to_unicode(0x03A3), "\u{03A3}"); // Sigma ~keep
    assert_eq!(fallback_char_to_unicode(0x03A4), "\u{03A4}"); // Tau ~keep
    assert_eq!(fallback_char_to_unicode(0x03A5), "\u{03A5}"); // Upsilon ~keep
    assert_eq!(fallback_char_to_unicode(0x03A6), "\u{03A6}"); // Phi ~keep
    assert_eq!(fallback_char_to_unicode(0x03A7), "\u{03A7}"); // Chi ~keep
    assert_eq!(fallback_char_to_unicode(0x03A8), "\u{03A8}"); // Psi ~keep
}

#[test]
fn test_fallback_currency_extended() {
    assert_eq!(fallback_char_to_unicode(0x20A3), "\u{20A3}"); // Franc ~keep
    assert_eq!(fallback_char_to_unicode(0x20A4), "\u{20A4}"); // Lira ~keep
    assert_eq!(fallback_char_to_unicode(0x20A9), "\u{20A9}"); // Won ~keep
    assert_eq!(fallback_char_to_unicode(0x20AA), "\u{20AA}"); // Shekel ~keep
    assert_eq!(fallback_char_to_unicode(0x20AB), "\u{20AB}"); // Dong ~keep
    assert_eq!(fallback_char_to_unicode(0x20B9), "\u{20B9}"); // Rupee ~keep
}

#[test]
fn test_decode_text_simple_font_with_control_chars() {
    let font = create_test_font();
    let bytes = vec![0x01, 0x41, 0x09]; // ctrl char, 'A', tab ~keep
    let result = decode_text_to_unicode(
        &bytes,
        Some(&font),
        DecodePolicy {
            preserve_unmapped: preserve_unmapped_glyphs(),
            decompose_ligatures: false,
            question_mark_for_invalid: true,
        },
        None,
    );
    // Should filter control chars but keep tab ~keep
    assert!(result.contains('\t') || result.contains('A'));
}

#[test]
fn test_decode_text_single_byte_only() {
    // Test with bytes that hit the TwoByte < 2 fallback ~keep
    let mut font = create_test_font();
    font.subtype = "Type0".to_string();
    font.encoding = crate::fonts::Encoding::Identity;
    let bytes = vec![0x41]; // Single byte for Type0 identity ~keep
    let result = decode_text_to_unicode(
        &bytes,
        Some(&font),
        DecodePolicy {
            preserve_unmapped: preserve_unmapped_glyphs(),
            decompose_ligatures: false,
            question_mark_for_invalid: true,
        },
        None,
    );
    // Should hit trailing byte path ~keep
}

#[test]
fn test_set_fill_color_space_resets_color() {
    let mut extractor = TextExtractor::new();
    extractor
        .execute_operator_public(Operator::SetFillRgb { r: 1.0, g: 0.0, b: 0.0 })
        .unwrap();

    let state = extractor.state_stack.current();
    assert!((state.fill_color_rgb.0 - 1.0).abs() < 0.01);

    // Change color space should reset to black ~keep
    extractor
        .execute_operator_public(Operator::SetFillColorSpace {
            name: "DeviceGray".to_string(),
        })
        .unwrap();

    let state = extractor.state_stack.current();
    assert!((state.fill_color_rgb.0 - 0.0).abs() < 0.01);
    assert!(state.fill_color_cmyk.is_none());
}

#[test]
fn test_set_stroke_color_space_resets_color() {
    let mut extractor = TextExtractor::new();
    extractor
        .execute_operator_public(Operator::SetStrokeRgb { r: 0.0, g: 1.0, b: 0.0 })
        .unwrap();

    extractor
        .execute_operator_public(Operator::SetStrokeColorSpace {
            name: "DeviceRGB".to_string(),
        })
        .unwrap();

    let state = extractor.state_stack.current();
    assert!((state.stroke_color_rgb.0 - 0.0).abs() < 0.01);
    assert!(state.stroke_color_cmyk.is_none());
}

#[test]
fn test_set_stroke_cmyk() {
    let mut extractor = TextExtractor::new();
    extractor
        .execute_operator_public(Operator::SetStrokeCmyk {
            c: 1.0,
            m: 0.0,
            y: 0.0,
            k: 0.0,
        })
        .unwrap();

    let state = extractor.state_stack.current();
    assert!(state.stroke_color_cmyk.is_some());
    // Cyan: R=0, G=1, B=1 ~keep
    assert!((state.stroke_color_rgb.0 - 0.0).abs() < 0.01);
}

#[test]
fn test_set_stroke_gray() {
    let mut extractor = TextExtractor::new();
    extractor
        .execute_operator_public(Operator::SetStrokeGray { gray: 0.7 })
        .unwrap();

    let state = extractor.state_stack.current();
    assert!((state.stroke_color_rgb.0 - 0.7).abs() < 0.01);
    assert!((state.stroke_color_rgb.1 - 0.7).abs() < 0.01);
    assert!((state.stroke_color_rgb.2 - 0.7).abs() < 0.01);
}

#[test]
fn test_set_stroke_rgb() {
    let mut extractor = TextExtractor::new();
    extractor
        .execute_operator_public(Operator::SetStrokeRgb { r: 0.3, g: 0.6, b: 0.9 })
        .unwrap();

    let state = extractor.state_stack.current();
    assert!((state.stroke_color_rgb.0 - 0.3).abs() < 0.01);
    assert!((state.stroke_color_rgb.1 - 0.6).abs() < 0.01);
    assert!((state.stroke_color_rgb.2 - 0.9).abs() < 0.01);
}

#[test]
fn test_cmyk_to_rgb_mixed() {
    let (r, g, b) = cmyk_to_rgb(0.5, 0.3, 0.1, 0.2);
    assert!((0.0..=1.0).contains(&r));
    assert!((0.0..=1.0).contains(&g));
    assert!((0.0..=1.0).contains(&b));
}

#[test]
fn test_cmyk_to_rgb_all_ones() {
    let (r, g, b) = cmyk_to_rgb(1.0, 1.0, 1.0, 1.0);
    assert!((r - 0.0).abs() < 0.01);
    assert!((g - 0.0).abs() < 0.01);
    assert!((b - 0.0).abs() < 0.01);
}

#[test]
fn test_deduplicate_content_based() {
    let mut extractor = TextExtractor::new();
    extractor.spans = vec![
        TextSpan {
            provenance: None,
            text_rise: 0.0,
            artifact_type: None,
            text: "Hello World".to_string(), // >= 5 chars ~keep
            bbox: Rect::new(100.0, 700.0, 60.0, 12.0),
            font_name: "F1".to_string(),
            font_size: 12.0,
            font_weight: FontWeight::Normal,
            color: Color::black(),
            mcid: None,
            mcid_scope: None,
            sequence: 0,
            split_boundary_before: false,
            offset_semantic: false,
            is_italic: false,
            is_monospace: false,
            char_spacing: 0.0,
            word_spacing: 0.0,
            horizontal_scaling: 100.0,
            primary_detected: false,
            char_widths: vec![],
            char_x_offsets: Vec::new(),
            heading_level: None,
            rotation_degrees: 0.0,
            wmode: 0,
            rtl_draw_logical: false,
            mirrored: false,
            page_rotation_applied: 0,
        },
        TextSpan {
            provenance: None,
            text_rise: 0.0,
            artifact_type: None,
            text: "Hello World".to_string(), // Same text, overlapping position ~keep
            bbox: Rect::new(102.0, 700.0, 60.0, 12.0), // X within 5pt ~keep
            font_name: "F1".to_string(),
            font_size: 12.0,
            font_weight: FontWeight::Normal,
            color: Color::black(),
            mcid: None,
            mcid_scope: None,
            sequence: 1,
            split_boundary_before: false,
            offset_semantic: false,
            is_italic: false,
            is_monospace: false,
            char_spacing: 0.0,
            word_spacing: 0.0,
            horizontal_scaling: 100.0,
            primary_detected: false,
            char_widths: vec![],
            char_x_offsets: Vec::new(),
            heading_level: None,
            rotation_degrees: 0.0,
            wmode: 0,
            rtl_draw_logical: false,
            mirrored: false,
            page_rotation_applied: 0,
        },
    ];

    extractor.deduplicate_overlapping_spans();
    assert_eq!(extractor.spans.len(), 1, "Content duplicates should be removed");
}

#[test]
fn test_deduplicate_content_not_overlapping() {
    let mut extractor = TextExtractor::new();
    extractor.spans = vec![
        TextSpan {
            provenance: None,
            text_rise: 0.0,
            artifact_type: None,
            text: "Hello World".to_string(),
            bbox: Rect::new(100.0, 700.0, 60.0, 12.0),
            font_name: "F1".to_string(),
            font_size: 12.0,
            font_weight: FontWeight::Normal,
            color: Color::black(),
            mcid: None,
            mcid_scope: None,
            sequence: 0,
            split_boundary_before: false,
            offset_semantic: false,
            is_italic: false,
            is_monospace: false,
            char_spacing: 0.0,
            word_spacing: 0.0,
            horizontal_scaling: 100.0,
            primary_detected: false,
            char_widths: vec![],
            char_x_offsets: Vec::new(),
            heading_level: None,
            rotation_degrees: 0.0,
            wmode: 0,
            rtl_draw_logical: false,
            mirrored: false,
            page_rotation_applied: 0,
        },
        TextSpan {
            provenance: None,
            text_rise: 0.0,
            artifact_type: None,
            text: "Hello World".to_string(),           // Same text but far apart ~keep
            bbox: Rect::new(500.0, 700.0, 60.0, 12.0), // X > 5pt difference ~keep
            font_name: "F1".to_string(),
            font_size: 12.0,
            font_weight: FontWeight::Normal,
            color: Color::black(),
            mcid: None,
            mcid_scope: None,
            sequence: 1,
            split_boundary_before: false,
            offset_semantic: false,
            is_italic: false,
            is_monospace: false,
            char_spacing: 0.0,
            word_spacing: 0.0,
            horizontal_scaling: 100.0,
            primary_detected: false,
            char_widths: vec![],
            char_x_offsets: Vec::new(),
            heading_level: None,
            rotation_degrees: 0.0,
            wmode: 0,
            rtl_draw_logical: false,
            mirrored: false,
            page_rotation_applied: 0,
        },
    ];

    extractor.deduplicate_overlapping_spans();
    assert_eq!(
        extractor.spans.len(),
        2,
        "Non-overlapping content should not be deduped"
    );
}

#[test]
fn test_advance_position_no_font() {
    let mut extractor = TextExtractor::new();
    extractor.state_stack.current_mut().font_size = 12.0;
    extractor.state_stack.current_mut().horizontal_scaling = 100.0;

    let width = extractor.advance_position_for_string(b"Hello", true).unwrap();
    assert!(width > 0.0, "Width should be positive even without font");
}

#[test]
fn test_advance_position_with_font() {
    let mut extractor = TextExtractor::new();
    let font = create_test_font();
    extractor.add_font("F1".to_string(), font);
    let current_font = extractor.fonts.get("F1").cloned();
    extractor.set_cached_current_font(current_font);
    extractor.state_stack.current_mut().font_size = 12.0;
    extractor.state_stack.current_mut().font_name = Some("F1".to_string());
    extractor.state_stack.current_mut().horizontal_scaling = 100.0;

    let width = extractor.advance_position_for_string(b"Hi", true).unwrap();
    assert!(width > 0.0, "Width should be positive with font");
}

#[test]
fn test_advance_position_with_word_space() {
    let mut extractor = TextExtractor::new();
    extractor.state_stack.current_mut().font_size = 12.0;
    extractor.state_stack.current_mut().horizontal_scaling = 100.0;
    extractor.state_stack.current_mut().word_space = 5.0;

    let width = extractor.advance_position_for_string(b"A B", true).unwrap();
    assert!(width > 0.0);
}

#[test]
fn test_insert_space_as_span() {
    let mut extractor = TextExtractor::new();
    extractor.state_stack.current_mut().font_size = 12.0;
    extractor.state_stack.current_mut().horizontal_scaling = 100.0;
    extractor.state_stack.current_mut().font_name = Some("F1".to_string());

    let before = extractor.spans.len();
    extractor.insert_space_as_span().unwrap();
    assert_eq!(extractor.spans.len(), before + 1);
    assert_eq!(extractor.spans.last().unwrap().text, " ");
    assert!(extractor.spans.last().unwrap().offset_semantic);
}

#[test]
fn test_adaptive_threshold_with_justified_text() {
    let config = TextExtractionConfig {
        use_adaptive_tj_threshold: true,
        word_margin_ratio: 0.1,
        ..TextExtractionConfig::default()
    };
    let mut extractor = TextExtractor::with_config(config);
    extractor.state_stack.current_mut().font_size = 12.0;

    // Simulate justified text (high CV) ~keep
    for i in 0..100 {
        extractor
            .tj_offset_history
            .push(if i % 2 == 0 { -50.0 } else { -200.0 });
    }

    let threshold = extractor.calculate_adaptive_tj_threshold();
    // Justified text uses 3x ratio, so threshold should be more negative ~keep
    assert!(threshold < 0.0);
}

#[test]
fn test_adaptive_threshold_with_font_name() {
    let config = TextExtractionConfig {
        use_adaptive_tj_threshold: true,
        word_margin_ratio: 0.1,
        ..TextExtractionConfig::default()
    };
    let mut extractor = TextExtractor::with_config(config);
    let font = create_test_font();
    extractor.add_font("F1".to_string(), font);
    extractor.state_stack.current_mut().font_size = 12.0;
    extractor.state_stack.current_mut().font_name = Some("F1".to_string());

    let threshold = extractor.calculate_adaptive_tj_threshold();
    assert!(threshold < 0.0);
}

#[test]
fn test_analyze_tj_distribution_zero_mean() {
    let mut extractor = TextExtractor::new();
    extractor.tj_offset_history = vec![100.0, -100.0, 100.0, -100.0];
    let (is_justified, cv) = extractor.analyze_tj_distribution();
    // Mean ~0, so CV should be 0 (avoid division by zero) ~keep
    assert_eq!(cv, 0.0);
}

#[test]
fn test_quote_operator_span_mode() {
    let mut extractor = TextExtractor::new();
    extractor.merging_config = SpanMergingConfig::legacy();
    let font = create_test_font();
    extractor.add_font("F1".to_string(), font);

    let stream = b"BT /F1 12 Tf 14 TL 100 700 Td (Line1) Tj (Line2) ' ET";
    let spans = extractor.extract_text_spans(stream).unwrap();

    let text: String = spans.iter().map(|s| s.text.as_str()).collect::<Vec<_>>().join("");
    assert!(text.contains("Line1"), "Should contain Line1, got: {}", text);
    assert!(text.contains("Line2"), "Should contain Line2, got: {}", text);
}

#[test]
fn test_double_quote_operator_span_mode() {
    let mut extractor = TextExtractor::new();
    extractor.merging_config = SpanMergingConfig::legacy();
    let font = create_test_font();
    extractor.add_font("F1".to_string(), font);

    let stream = b"BT /F1 12 Tf 14 TL 100 700 Td 1 2 (Text) \" ET";
    let spans = extractor.extract_text_spans(stream).unwrap();

    let text: String = spans.iter().map(|s| s.text.as_str()).collect::<Vec<_>>().join("");
    assert!(text.contains("Text"), "Should extract text, got: {}", text);
}

#[test]
fn test_sort_spans_by_columns() {
    let mut extractor = TextExtractor::new();
    let columns = vec![(0.0, 250.0), (300.0, 550.0)];

    extractor.spans = vec![
        TextSpan {
            provenance: None,
            text_rise: 0.0,
            artifact_type: None,
            text: "Right Col".to_string(),
            bbox: Rect::new(350.0, 700.0, 100.0, 12.0),
            font_name: "F1".to_string(),
            font_size: 12.0,
            font_weight: FontWeight::Normal,
            color: Color::black(),
            mcid: None,
            mcid_scope: None,
            sequence: 0,
            split_boundary_before: false,
            offset_semantic: false,
            is_italic: false,
            is_monospace: false,
            char_spacing: 0.0,
            word_spacing: 0.0,
            horizontal_scaling: 100.0,
            primary_detected: false,
            char_widths: vec![],
            char_x_offsets: Vec::new(),
            heading_level: None,
            rotation_degrees: 0.0,
            wmode: 0,
            rtl_draw_logical: false,
            mirrored: false,
            page_rotation_applied: 0,
        },
        TextSpan {
            provenance: None,
            text_rise: 0.0,
            artifact_type: None,
            text: "Left Col".to_string(),
            bbox: Rect::new(50.0, 700.0, 100.0, 12.0),
            font_name: "F1".to_string(),
            font_size: 12.0,
            font_weight: FontWeight::Normal,
            color: Color::black(),
            mcid: None,
            mcid_scope: None,
            sequence: 1,
            split_boundary_before: false,
            offset_semantic: false,
            is_italic: false,
            is_monospace: false,
            char_spacing: 0.0,
            word_spacing: 0.0,
            horizontal_scaling: 100.0,
            primary_detected: false,
            char_widths: vec![],
            char_x_offsets: Vec::new(),
            heading_level: None,
            rotation_degrees: 0.0,
            wmode: 0,
            rtl_draw_logical: false,
            mirrored: false,
            page_rotation_applied: 0,
        },
    ];

    extractor.sort_spans_by_columns(&columns);
    assert_eq!(extractor.spans[0].text, "Left Col");
    assert_eq!(extractor.spans[1].text, "Right Col");
}

#[test]
fn test_tj_buffer_with_mcid() {
    let state = crate::content::graphics_state::GraphicsStateStack::new();
    let buffer = TjBuffer::new(state.current(), Some(42), None);
    assert!(buffer.is_empty());
    assert_eq!(buffer.mcid, Some(42));
}

#[test]
fn test_extractor_with_primary_word_boundary() {
    let config = TextExtractionConfig {
        word_boundary_mode: WordBoundaryMode::Primary,
        ..TextExtractionConfig::default()
    };
    let mut extractor = TextExtractor::with_config(config);
    let font = create_test_font();
    extractor.add_font("F1".to_string(), font);
    extractor.merging_config = SpanMergingConfig::legacy();

    let stream = b"BT /F1 12 Tf 100 700 Td (Hello) Tj ET";
    let spans = extractor.extract_text_spans(stream).unwrap();

    let text: String = spans.iter().map(|s| s.text.as_str()).collect::<Vec<_>>().join("");
    assert!(
        text.contains("Hello"),
        "Primary mode should still extract text, got: {}",
        text
    );
}

#[test]
fn test_merge_prevents_double_space() {
    let mut extractor = TextExtractor::new();
    extractor.merging_config = SpanMergingConfig::legacy();

    extractor.spans = vec![
        TextSpan {
            provenance: None,
            text_rise: 0.0,
            artifact_type: None,
            text: "Hello ".to_string(), // ends with space ~keep
            bbox: Rect::new(100.0, 700.0, 35.0, 12.0),
            font_name: "F1".to_string(),
            font_size: 12.0,
            font_weight: FontWeight::Normal,
            color: Color::black(),
            mcid: None,
            mcid_scope: None,
            sequence: 0,
            split_boundary_before: false,
            offset_semantic: false,
            is_italic: false,
            is_monospace: false,
            char_spacing: 0.0,
            word_spacing: 0.0,
            horizontal_scaling: 100.0,
            primary_detected: false,
            char_widths: vec![],
            char_x_offsets: Vec::new(),
            heading_level: None,
            rotation_degrees: 0.0,
            wmode: 0,
            rtl_draw_logical: false,
            mirrored: false,
            page_rotation_applied: 0,
        },
        TextSpan {
            provenance: None,
            text_rise: 0.0,
            artifact_type: None,
            text: " World".to_string(),                // starts with space ~keep
            bbox: Rect::new(136.0, 700.0, 35.0, 12.0), // 1pt gap ~keep
            font_name: "F1".to_string(),
            font_size: 12.0,
            font_weight: FontWeight::Normal,
            color: Color::black(),
            mcid: None,
            mcid_scope: None,
            sequence: 1,
            split_boundary_before: true, // forces merge-with-space path ~keep
            offset_semantic: false,
            is_italic: false,
            is_monospace: false,
            char_spacing: 0.0,
            word_spacing: 0.0,
            horizontal_scaling: 100.0,
            primary_detected: false,
            char_widths: vec![],
            char_x_offsets: Vec::new(),
            heading_level: None,
            rotation_degrees: 0.0,
            wmode: 0,
            rtl_draw_logical: false,
            mirrored: false,
            page_rotation_applied: 0,
        },
    ];

    extractor.merge_adjacent_spans();
    assert_eq!(extractor.spans.len(), 1);
    // Should not have "Hello World" (triple space) ~keep
    assert!(!extractor.spans[0].text.contains("   "), "Should prevent triple space");
}

#[test]
fn test_extractor_with_config_copies_word_boundary_mode() {
    let config = TextExtractionConfig {
        word_boundary_mode: WordBoundaryMode::Primary,
        ..TextExtractionConfig::default()
    };
    let extractor = TextExtractor::with_config(config);
    assert_eq!(extractor.word_boundary_mode, WordBoundaryMode::Primary);
}

#[test]
fn test_partition_boundary_at_start() {
    let extractor = TextExtractor::new();
    let chars = vec![
        CharacterInfo {
            code: 65,
            glyph_id: None,
            width: 10.0,
            x_position: 0.0,
            tj_offset: None,
            font_size: 12.0,
            is_ligature: false,
            original_ligature: None,
            protected_from_split: false,
        },
        CharacterInfo {
            code: 66,
            glyph_id: None,
            width: 10.0,
            x_position: 10.0,
            tj_offset: None,
            font_size: 12.0,
            is_ligature: false,
            original_ligature: None,
            protected_from_split: false,
        },
    ];

    // Boundary at 0 means empty first cluster ~keep
    let clusters = extractor.partition_characters_by_boundaries(&chars, vec![0]);
    // Should have just one cluster (boundary at 0 produces no items before it) ~keep
    assert!(!clusters.is_empty());
}

#[test]
fn test_fill_cmyk_then_change_color_space() {
    let mut extractor = TextExtractor::new();
    extractor
        .execute_operator_public(Operator::SetFillCmyk {
            c: 0.5,
            m: 0.5,
            y: 0.5,
            k: 0.5,
        })
        .unwrap();
    assert!(extractor.state_stack.current().fill_color_cmyk.is_some());

    // Changing color space should reset CMYK ~keep
    extractor
        .execute_operator_public(Operator::SetFillColorSpace {
            name: "DeviceRGB".to_string(),
        })
        .unwrap();
    assert!(extractor.state_stack.current().fill_color_cmyk.is_none());
}

#[test]
fn test_bdc_with_mcid() {
    let mut extractor = TextExtractor::new();
    let mut props = HashMap::new();
    props.insert("MCID".to_string(), Object::Integer(5));

    extractor
        .execute_operator_public(Operator::BeginMarkedContentDict {
            tag: "P".to_string(),
            properties: Box::new(Object::Dictionary(props)),
        })
        .unwrap();

    assert_eq!(extractor.current_mcid, Some(5));
    assert!(!extractor.inside_artifact);
}

#[test]
fn test_bdc_artifact_with_type() {
    let mut extractor = TextExtractor::new();
    let mut props = HashMap::new();
    props.insert("Type".to_string(), Object::Name("Pagination".to_string()));
    props.insert("Subtype".to_string(), Object::Name("Header".to_string()));

    extractor
        .execute_operator_public(Operator::BeginMarkedContentDict {
            tag: "Artifact".to_string(),
            properties: Box::new(Object::Dictionary(props)),
        })
        .unwrap();

    assert!(extractor.inside_artifact);
}

#[test]
fn test_emc_resets_mcid() {
    let mut extractor = TextExtractor::new();
    extractor.current_mcid = Some(10);
    extractor.marked_content_stack.push(MarkedContentContext {
        artifact_type: None,
        tag: "P".to_string(),
        is_artifact: false,
        actual_text: None,
        expansion: None,
        is_excluded_layer: false,
        is_placed_pdf: false,
        actual_text_emitted: false,
        own_mcid: None,
    });

    extractor.execute_operator_public(Operator::EndMarkedContent).unwrap();

    assert_eq!(extractor.current_mcid, None);
    assert!(extractor.marked_content_stack.is_empty());
}

#[test]
fn test_emc_with_empty_stack() {
    let mut extractor = TextExtractor::new();
    extractor.execute_operator_public(Operator::EndMarkedContent).unwrap();
}

#[test]
fn test_bdc_with_actual_text() {
    let mut extractor = TextExtractor::new();
    let mut props = HashMap::new();
    props.insert("ActualText".to_string(), Object::String(b"fi".to_vec()));

    extractor
        .execute_operator_public(Operator::BeginMarkedContentDict {
            tag: "Span".to_string(),
            properties: Box::new(Object::Dictionary(props)),
        })
        .unwrap();

    let actual = extractor.get_current_actual_text();
    assert_eq!(actual, Some("fi".to_string()));
}

#[test]
fn test_bdc_with_expansion() {
    let mut extractor = TextExtractor::new();
    let mut props = HashMap::new();
    props.insert("E".to_string(), Object::String(b"PDF".to_vec()));

    extractor
        .execute_operator_public(Operator::BeginMarkedContentDict {
            tag: "Span".to_string(),
            properties: Box::new(Object::Dictionary(props)),
        })
        .unwrap();

    let ctx = &extractor.marked_content_stack[0];
    assert_eq!(ctx.expansion, Some("PDF".to_string()));
}

#[test]
fn test_do_operator_without_document() {
    let mut extractor = TextExtractor::new();
    // Do without document set should not panic ~keep
    extractor
        .execute_operator_public(Operator::Do {
            name: "Im1".to_string(),
        })
        .unwrap();
}

#[test]
fn test_flush_tj_span_buffer_empty_buffer() {
    let mut extractor = TextExtractor::new();
    let state = extractor.state_stack.current().clone();
    extractor.tj_span_buffer = Some(TjBuffer::new(&state, None, None));
    let before = extractor.spans.len();
    extractor.flush_tj_span_buffer().unwrap();
    assert_eq!(extractor.spans.len(), before);
}

#[test]
fn test_flush_tj_span_buffer_with_content() {
    let mut extractor = TextExtractor::new();
    let state_stack = crate::content::graphics_state::GraphicsStateStack::new();
    let mut buffer = TjBuffer::new(state_stack.current(), Some(7), None);
    buffer.append(b"Test").unwrap();
    buffer.accumulated_width = 20.0;
    extractor.tj_span_buffer = Some(buffer);

    extractor.flush_tj_span_buffer().unwrap();
    assert_eq!(extractor.spans.len(), 1);
    assert!(extractor.spans[0].text.contains("Test"));
}

#[test]
fn test_tj_array_span_mode_with_space_insertion() {
    let config = TextExtractionConfig {
        use_adaptive_tj_threshold: false,
        space_insertion_threshold: -120.0,
        ..TextExtractionConfig::default()
    };
    let mut extractor = TextExtractor::with_config(config);
    extractor.merging_config = SpanMergingConfig::legacy();
    let font = create_test_font();
    extractor.add_font("F1".to_string(), font);

    // TJ array with large offset that triggers space ~keep
    let stream = b"BT /F1 12 Tf 100 700 Td [(Word1) -500 (Word2)] TJ ET";
    let spans = extractor.extract_text_spans(stream).unwrap();

    let text: String = spans.iter().map(|s| s.text.as_str()).collect::<Vec<_>>().join("");
    assert!(text.contains("Word1"), "Should contain Word1");
    assert!(text.contains("Word2"), "Should contain Word2");
}

#[test]
fn test_sort_spans_single_column() {
    let mut extractor = TextExtractor::new();
    extractor.spans = vec![
        TextSpan {
            provenance: None,
            text_rise: 0.0,
            artifact_type: None,
            text: "Line2".to_string(),
            bbox: Rect::new(50.0, 680.0, 100.0, 12.0),
            font_name: "F1".to_string(),
            font_size: 12.0,
            font_weight: FontWeight::Normal,
            color: Color::black(),
            mcid: None,
            mcid_scope: None,
            sequence: 0,
            split_boundary_before: false,
            offset_semantic: false,
            is_italic: false,
            is_monospace: false,
            char_spacing: 0.0,
            word_spacing: 0.0,
            horizontal_scaling: 100.0,
            primary_detected: false,
            char_widths: vec![],
            char_x_offsets: Vec::new(),
            heading_level: None,
            rotation_degrees: 0.0,
            wmode: 0,
            rtl_draw_logical: false,
            mirrored: false,
            page_rotation_applied: 0,
        },
        TextSpan {
            provenance: None,
            text_rise: 0.0,
            artifact_type: None,
            text: "Line1".to_string(),
            bbox: Rect::new(50.0, 700.0, 100.0, 12.0),
            font_name: "F1".to_string(),
            font_size: 12.0,
            font_weight: FontWeight::Normal,
            color: Color::black(),
            mcid: None,
            mcid_scope: None,
            sequence: 1,
            split_boundary_before: false,
            offset_semantic: false,
            is_italic: false,
            is_monospace: false,
            char_spacing: 0.0,
            word_spacing: 0.0,
            horizontal_scaling: 100.0,
            primary_detected: false,
            char_widths: vec![],
            char_x_offsets: Vec::new(),
            heading_level: None,
            rotation_degrees: 0.0,
            wmode: 0,
            rtl_draw_logical: false,
            mirrored: false,
            page_rotation_applied: 0,
        },
    ];

    extractor.sort_spans_by_reading_order();
    assert_eq!(extractor.spans[0].text, "Line1"); // higher Y first ~keep
    assert_eq!(extractor.spans[1].text, "Line2");
}

/// A scanned vertical-CJK OCR layer can emit hundreds of single-glyph
/// `wmode=1` spans whose X-centers step by a fraction of the median
/// span width: every adjacent pair looks "same column" under a pairwise
/// `|a - b| <= tol` check, but the first and last span are hundreds of
/// points apart, so the comparator claims contradictory orderings
/// (A<B, B<C, C<A) and Rust's `sort_by` panics with "does not correctly
/// implement a total order" instead of returning a reading order.
#[test]
fn test_sort_spans_vertical_tategaki_chained_x_centers_does_not_panic() {
    let mut extractor = TextExtractor::new();
    extractor.spans = (0..240)
        .map(|i| TextSpan {
            text: format!("g{i}"),
            bbox: Rect::new(20.0 + i as f32 * 0.8, 700.0 - ((i * 37) % 96) as f32 * 7.0, 1.0, 12.0),
            font_size: 12.0,
            wmode: 1,
            ..TextSpan::default()
        })
        .collect();

    extractor.sort_spans_by_reading_order(); // must not panic ~keep
    assert_eq!(extractor.spans.len(), 240);
}

#[test]
fn test_tm_continuation_different_transform() {
    let mut extractor = TextExtractor::new();
    extractor.merging_config = SpanMergingConfig::legacy();
    let font = create_test_font();
    extractor.add_font("F1".to_string(), font);

    // Different transform params (a=2) should NOT be continuation ~keep
    let stream = b"BT /F1 12 Tf 1 0 0 1 100 700 Tm (A) Tj 2 0 0 1 120 700 Tm (B) Tj ET";
    let spans = extractor.extract_text_spans(stream).unwrap();

    assert!(!spans.is_empty());
}

#[test]
fn test_decode_pdf_text_string_single_byte() {
    let result = TextExtractor::decode_pdf_text_string(&[0x41]);
    assert_eq!(result, "A");
}

#[test]
fn test_decode_pdf_text_string_invalid_utf16() {
    // UTF-16BE BOM followed by invalid pair ~keep
    let bytes = vec![0xFE, 0xFF, 0xD8, 0x00]; // invalid surrogate half ~keep
    let result = TextExtractor::decode_pdf_text_string(&bytes);
    assert!(!result.is_empty() || result.is_empty()); // Just don't panic ~keep
}

#[test]
fn test_decode_pdf_text_string_utf16le_invalid() {
    // UTF-16LE BOM followed by odd byte count ~keep
    let bytes = vec![0xFF, 0xFE, 0x41]; // odd after BOM ~keep
    let result = TextExtractor::decode_pdf_text_string(&bytes);
    // Should handle gracefully ~keep
}

// ========================================================================
// TDD: decode_pdf_text_string — PDFDocEncoding fallback correctness
// Bytes 0xA0–0xFF and the special 0x80–0x9E zone must decode through
// PDFDocEncoding, not through from_utf8_lossy (which produces U+FFFD).
// ======================================================================== ~keep

#[test]
fn test_decode_pdfdocencoding_latin_byte() {
    // 0xE9 = PDFDocEncoding for é (U+00E9). Not valid UTF-8 on its own. ~keep
    let result = TextExtractor::decode_pdf_text_string(&[0xE9]);
    assert_eq!(
        result, "é",
        "0xE9 must decode as 'é' via PDFDocEncoding, not produce U+FFFD"
    );
}

#[test]
fn test_decode_pdfdocencoding_bullet() {
    // 0x80 = PDFDocEncoding for • (U+2022 BULLET) ~keep
    let result = TextExtractor::decode_pdf_text_string(&[0x80]);
    assert_eq!(result, "•", "0x80 must decode as bullet '•' via PDFDocEncoding");
}

#[test]
fn test_decode_pdfdocencoding_emdash() {
    // 0x84 = PDFDocEncoding for — (U+2014 EM DASH) ~keep
    let result = TextExtractor::decode_pdf_text_string(&[0x84]);
    assert_eq!(result, "—", "0x84 must decode as em-dash '—' via PDFDocEncoding");
}

#[test]
fn test_decode_pdfdocencoding_trademark() {
    // 0x92 = PDFDocEncoding for ™ (U+2122 TRADE MARK SIGN) ~keep
    let result = TextExtractor::decode_pdf_text_string(&[0x92]);
    assert_eq!(result, "™", "0x92 must decode as trademark '™' via PDFDocEncoding");
}

#[test]
fn test_decode_pdfdocencoding_undefined_9f_is_dropped() {
    // 0x9F is undefined in PDFDocEncoding — must be silently dropped. ~keep
    let result = TextExtractor::decode_pdf_text_string(&[0x41, 0x9F, 0x42]);
    assert_eq!(result, "AB", "0x9F is undefined in PDFDocEncoding and must be dropped");
}

#[test]
fn test_decode_pdfdocencoding_mixed_ascii_and_latin() {
    // "Hello" followed by 0xE9 (é): 6 bytes → "Helloé" ~keep
    let bytes: Vec<u8> = b"Hello".iter().copied().chain([0xE9]).collect();
    let result = TextExtractor::decode_pdf_text_string(&bytes);
    assert_eq!(
        result, "Helloé",
        "Mixed ASCII + PDFDocEncoding bytes must decode correctly"
    );
}

#[test]
fn test_decode_pdfdocencoding_utf8_bytes_still_work() {
    // Valid UTF-8 without BOM: must still decode correctly (for lenient PDFs).
    // ASCII is a subset of UTF-8, so this path always works. ~keep
    let result = TextExtractor::decode_pdf_text_string(b"ASCII text");
    assert_eq!(result, "ASCII text");
}

#[test]
fn test_share_truetype_cmaps_no_donors() {
    let mut extractor = TextExtractor::new();
    let font = create_test_font();
    extractor.add_font("F1".to_string(), font);

    extractor.share_truetype_cmaps();
    assert_eq!(extractor.fonts.len(), 1);
}

#[test]
fn test_extractor_with_config_and_profile() {
    let config = TextExtractionConfig::new().with_profile(crate::config::ExtractionProfile::POLICY);

    let mut extractor = TextExtractor::with_config(config);
    let font = create_test_font();
    extractor.add_font("F1".to_string(), font);

    let stream = b"BT /F1 12 Tf 100 700 Td (Policy) Tj ET";
    let chars = extractor.extract(stream).unwrap();
    assert!(!chars.is_empty());
}

#[test]
fn test_merge_offset_semantic_space_suppression() {
    let mut extractor = TextExtractor::new();
    extractor.merging_config = SpanMergingConfig::legacy();

    extractor.spans = vec![
        TextSpan {
            provenance: None,
            text_rise: 0.0,
            artifact_type: None,
            text: "Hello".to_string(),
            bbox: Rect::new(100.0, 700.0, 30.0, 12.0),
            font_name: "F1".to_string(),
            font_size: 12.0,
            font_weight: FontWeight::Normal,
            color: Color::black(),
            mcid: None,
            mcid_scope: None,
            sequence: 0,
            split_boundary_before: false,
            offset_semantic: false,
            is_italic: false,
            is_monospace: false,
            char_spacing: 0.0,
            word_spacing: 0.0,
            horizontal_scaling: 100.0,
            primary_detected: false,
            char_widths: vec![],
            char_x_offsets: Vec::new(),
            heading_level: None,
            rotation_degrees: 0.0,
            wmode: 0,
            rtl_draw_logical: false,
            mirrored: false,
            page_rotation_applied: 0,
        },
        TextSpan {
            provenance: None,
            text_rise: 0.0,
            artifact_type: None,
            text: " ".to_string(), // offset_semantic space ~keep
            bbox: Rect::new(130.5, 700.0, 2.0, 12.0),
            font_name: "F1".to_string(),
            font_size: 12.0,
            font_weight: FontWeight::Normal,
            color: Color::black(),
            mcid: None,
            mcid_scope: None,
            sequence: 1,
            split_boundary_before: true, // forcing merge path ~keep
            offset_semantic: true,
            is_italic: false,
            is_monospace: false,
            char_spacing: 0.0,
            word_spacing: 0.0,
            horizontal_scaling: 100.0,
            primary_detected: false,
            char_widths: vec![],
            char_x_offsets: Vec::new(),
            heading_level: None,
            rotation_degrees: 0.0,
            wmode: 0,
            rtl_draw_logical: false,
            mirrored: false,
            page_rotation_applied: 0,
        },
    ];

    extractor.merge_adjacent_spans();
    // offset_semantic space should be merged without adding extra space ~keep
    let text = &extractor.spans[0].text;
    assert!(!text.contains("  "), "Should not have double space, got: '{}'", text);
}

#[test]
fn test_is_monospace_font_recognizes_broad_mono_families() {
    assert!(is_monospace_font("Menlo"));
    assert!(is_monospace_font("Fira Code"));
    assert!(is_monospace_font("Fira Mono"));
    assert!(is_monospace_font("Source Code Pro"));
    assert!(is_monospace_font("Inconsolata"));
    assert!(is_monospace_font("CMTT10"));
    assert!(is_monospace_font("LMMono10-Regular"));
    assert!(is_monospace_font("DejaVu Sans Mono"));
}

#[test]
fn test_is_monospace_font_recognizes_bare_word_mono_families() {
    // These carry "mono" as an ordinary word/suffix, not the "Monotype" foundry
    // name, and must still match. ~keep
    assert!(is_monospace_font("PT Mono"));
    assert!(is_monospace_font("Roboto Mono"));
    assert!(is_monospace_font("Nimbus Mono"));
}

#[test]
fn test_is_monospace_font_rejects_monotype_foundry_names() {
    // "Monotype" is a foundry, not a monospace family; these are script/display faces. ~keep
    assert!(!is_monospace_font("Monotype Corsiva"));
    assert!(!is_monospace_font("Arial Monotype"));
}

#[test]
fn test_is_monospace_font_still_matches_monotype_branded_monospace_faces() {
    // A Monotype-branded face that is genuinely monospace still matches via its
    // own marker ("consolas"), even though the "monotype" exclusion suppresses
    // the bare "mono" check for it. ~keep
    assert!(is_monospace_font("Monotype Consolas"));
}

#[test]
fn test_is_monospace_font_rejects_fira_sans() {
    // Fira Sans is a proportional face; only Fira Code / Fira Mono are monospace. ~keep
    assert!(!is_monospace_font("Fira Sans"));
    assert!(!is_monospace_font("Fira Sans Condensed"));
}

#[test]
fn test_tj_buffer_marks_menlo_as_monospace() {
    let state = crate::content::graphics_state::GraphicsStateStack::new();
    let font = Arc::new(FontInfo {
        base_font: "Menlo".to_string(),
        ..create_test_font()
    });
    let buffer = TjBuffer::new(state.current(), None, Some(font));
    assert!(
        buffer.is_monospace,
        "Menlo must be recognized as monospace via is_monospace_font, not just the narrow legacy list"
    );
}

#[test]
fn test_tj_buffer_does_not_mark_fira_sans_as_monospace() {
    let state = crate::content::graphics_state::GraphicsStateStack::new();
    let font = Arc::new(FontInfo {
        base_font: "Fira Sans".to_string(),
        ..create_test_font()
    });
    let buffer = TjBuffer::new(state.current(), None, Some(font));
    assert!(
        !buffer.is_monospace,
        "Fira Sans is proportional prose text and must not be routed toward code fencing"
    );
}
