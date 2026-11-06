use super::super::*;
use super::common::*;

#[test]
fn test_push_line_breaks_table_row_single_newline() {
    // A table-row boundary (single_break = true) emits exactly one newline
    // regardless of the geometric row pitch. ~keep
    let prev = make_test_span("North", 72.0, 700.0, 30.0, 12.0);
    let span = make_test_span("South", 72.0, 676.0, 30.0, 12.0); // 24pt gap ≈ 1.7em ~keep
    let mut out = String::new();
    PdfDocument::push_line_breaks(&mut out, &prev, &span, 24.0, true);
    assert_eq!(out, "\n", "table row boundary must be a single newline");
    // The same gap WITHOUT the table flag rounds to a blank line (2). ~keep
    let mut out2 = String::new();
    PdfDocument::push_line_breaks(&mut out2, &prev, &span, 24.0, false);
    assert_eq!(out2, "\n\n", "non-table ~1.7em gap keeps the geometric blank line");
    // A single-line gap stays one newline either way. ~keep
    let mut out3 = String::new();
    PdfDocument::push_line_breaks(&mut out3, &prev, &span, 14.0, false);
    assert_eq!(out3, "\n");
}

#[test]
fn test_should_insert_space_same_line_with_gap() {
    let prev = make_test_span("Hello", 0.0, 100.0, 50.0, 12.0);
    let current = make_test_span("World", 56.0, 100.0, 50.0, 12.0);
    // 6pt gap (> 0.25 * 12 = 3pt) ~keep
    assert!(PdfDocument::should_insert_space(&prev, &current));
}

/// A single word drawn as two same-font runs with real (varying) per-glyph
/// metrics that overlap by a fraction of a point ("PLANAL"+"TINA", the
/// planaltina kerning-split repro) is a reliable kerning overlap: the
/// assembler must NOT insert a space, reconstructing "PLANALTINA".
#[test]
fn test_reliable_kerning_overlap_recognizes_split_word() {
    let mut prev = make_test_span("PLANAL", 100.0, 700.0, 38.35, 10.0);
    prev.char_widths = vec![6.67, 5.56, 6.67, 7.22, 6.67, 5.56]; // real Helvetica ~keep
    let span = make_test_span("TINA", 136.64, 700.0, 22.78, 10.0);
    let gap = span.bbox.x - (prev.bbox.x + prev.bbox.width); // ≈ -1.71pt overlap ~keep
    assert!(
        PdfDocument::is_reliable_kerning_overlap(&prev, &span, gap),
        "varying-width same-font runs overlapping by <1em must read as one word"
    );
}

/// A font with no /Widths array falls back to a uniform advance per glyph,
/// over-reporting each width and manufacturing a fake overlap between two
/// SEPARATE words ("STATION"+"FREEDOM"). Uniform char_widths must NOT be
/// treated as a kerning overlap — the assembler keeps the word-boundary
/// space.
#[test]
fn test_reliable_kerning_overlap_rejects_uniform_fallback_widths() {
    let mut prev = make_test_span("STATION", 100.0, 700.0, 42.0, 10.0);
    prev.char_widths = vec![6.0; 7]; // uniform missing-/Widths fallback ~keep
    let span = make_test_span("FREEDOM", 141.0, 700.0, 42.0, 10.0);
    let gap = span.bbox.x - (prev.bbox.x + prev.bbox.width); // -1.0pt fake overlap ~keep
    assert!(
        !PdfDocument::is_reliable_kerning_overlap(&prev, &span, gap),
        "uniform fallback widths are an inflated-width artifact, not kerning"
    );
}

/// A coarse width table with only two distinct advances (e.g. a font that
/// reports one width for wide glyphs and one for narrow) is not genuine
/// proportional metrics — it manufactures fake overlaps between SEPARATE
/// words ("território"+"e"). Two distinct advances must NOT qualify.
#[test]
fn test_reliable_kerning_overlap_rejects_coarse_two_value_widths() {
    let mut prev = make_test_span("território", 32.0, 700.0, 68.0, 13.6);
    prev.char_widths = vec![6.8, 6.8, 6.8, 6.8, 6.8, 6.8, 3.4, 3.4, 6.8, 6.8];
    let span = make_test_span("e", 66.0, 700.0, 6.8, 13.6);
    let gap = span.bbox.x - (prev.bbox.x + prev.bbox.width);
    assert!(!PdfDocument::is_reliable_kerning_overlap(&prev, &span, gap));
}

/// A lowercase→uppercase transition at an overlapping join is a
/// word/sentence boundary ("...with"+"Gp53"), not one word split by
/// kerning — it must NOT be treated as a reliable kerning overlap even with
/// real varying widths.
#[test]
fn test_reliable_kerning_overlap_rejects_lowercase_to_uppercase_boundary() {
    let mut prev = make_test_span("with", 100.0, 700.0, 20.0, 12.0);
    prev.char_widths = vec![6.7, 3.3, 4.8, 6.7];
    let span = make_test_span("Gp53", 118.0, 700.0, 24.0, 12.0);
    let gap = span.bbox.x - (prev.bbox.x + prev.bbox.width); // -2pt overlap ~keep
    assert!(!PdfDocument::is_reliable_kerning_overlap(&prev, &span, gap));
}

#[test]
fn merge_sub_superscript_accepts_fstatistic() {
    let mut spans = vec![
        make_test_span("F", 0.0, 100.0, 8.0, 12.0),
        make_test_span("4,176", 8.0, 98.0, 12.0, 8.0),
    ];
    PdfDocument::merge_sub_superscript_spans(&mut spans);
    assert_eq!(spans.len(), 1, "index cluster must merge into base");
    assert_eq!(spans[0].text, "F4,176");
}

#[test]
fn merge_sub_superscript_accepts_text_rise_flagged() {
    let mut base = make_test_span("M", 0.0, 100.0, 10.0, 12.0);
    base.text_rise = 0.0;
    let mut sup = make_test_span("\u{22C6}", 10.5, 103.0, 6.0, 12.0);
    sup.text_rise = 0.30;
    let mut spans = vec![base, sup];
    PdfDocument::merge_sub_superscript_spans(&mut spans);
    assert_eq!(spans.len(), 1, "Ts-flagged superscript must merge into base");
    assert_eq!(spans[0].text, "M\u{22C6}");
}

#[test]
fn merge_sub_superscript_accepts_same_baseline_numeric() {
    let base = make_test_span("[", 0.0, 100.0, 6.0, 18.0);
    let sup = make_test_span("123", 6.0, 100.0, 20.0, 13.0);
    let mut spans = vec![base, sup];
    PdfDocument::merge_sub_superscript_spans(&mut spans);
    assert_eq!(spans.len(), 1, "same-baseline numeric superscript must merge");
    assert_eq!(spans[0].text, "[123");
}

#[test]
fn merge_sub_superscript_rejects_same_baseline_alpha() {
    let base = make_test_span("A", 0.0, 100.0, 12.0, 18.0);
    let sub = make_test_span("bc", 12.0, 100.0, 8.0, 13.0);
    let mut spans = vec![base, sub];
    PdfDocument::merge_sub_superscript_spans(&mut spans);
    assert_eq!(spans.len(), 2, "same-baseline alpha run must not merge");
}

#[test]
fn merge_sub_superscript_keeps_table_number_separate() {
    // Guard: a bare figure/table number after a WORD base is not an index
    // cluster (no comma) and the word base is invalid — stays separate. ~keep
    let mut spans = vec![
        make_test_span("Table", 0.0, 100.0, 30.0, 12.0),
        make_test_span("3", 31.0, 100.0, 6.0, 12.0),
    ];
    PdfDocument::merge_sub_superscript_spans(&mut spans);
    assert_eq!(spans.len(), 2, "Table 3 must not merge");
}

#[test]
fn merge_sub_superscript_keeps_numeric_base_and_marker_separate() {
    let mut spans = vec![
        make_test_span("3", 250.14, 317.57, 6.48, 12.96),
        make_test_span("5", 256.73, 317.57, 4.26, 8.52),
    ];
    PdfDocument::merge_sub_superscript_spans(&mut spans);
    assert_eq!(spans.len(), 2, "a smaller marker must not turn 3 into 35");
}

#[test]
fn small_numeric_span_after_numeric_prose_gets_a_separator() {
    // Circular 6/2025, p. 5: the smaller footnote marker 5 abuts "comma 3".
    // Both extracted spans have the same bbox bottom and only a 0.11pt gap.
    let body = make_test_span("Il successivo comma 3", 121.1, 317.57, 135.52274, 12.96);
    let marker = make_test_span("5", 256.73, 317.57, 4.26, 8.52);
    assert!(PdfDocument::should_insert_space(&body, &marker));

    let ordinary_digit = make_test_span("5", 256.73, 317.57, 4.26, 12.96);
    assert!(!PdfDocument::should_insert_space(&body, &ordinary_digit));
}

#[test]
fn test_should_insert_space_same_line_no_gap() {
    let prev = make_test_span("Hello", 0.0, 100.0, 50.0, 12.0);
    let current = make_test_span("World", 51.0, 100.0, 50.0, 12.0);
    // 1pt gap (< 0.25 * 12 = 3pt) ~keep
    assert!(!PdfDocument::should_insert_space(&prev, &current));
}

#[test]
fn test_should_insert_space_different_lines() {
    let prev = make_test_span("Hello", 0.0, 100.0, 50.0, 12.0);
    let current = make_test_span("World", 56.0, 120.0, 50.0, 12.0);
    // Different lines = false (no space needed, line break instead) ~keep
    assert!(!PdfDocument::should_insert_space(&prev, &current));
}

#[test]
fn test_should_insert_space_column_gap() {
    let prev = make_test_span("Hello", 0.0, 100.0, 50.0, 12.0);
    let current = make_test_span("World", 200.0, 100.0, 50.0, 12.0);
    // Issue 487 (pr-138-example.pdf rate tables): a very large
    // same-line gap (here 150 pt > 5 em) must still produce a single
    // space. The earlier `gap < font_size * 5.0` upper bound made
    // this return false, after which the caller concatenated the two
    // spans without a separator and `3.80%` + `4.41%` came out as
    // `3.80%4.41%`. Large gap = different column = still a space. ~keep
    assert!(PdfDocument::should_insert_space(&prev, &current));
}

/// Stacked two-line column/table-header cell: `Comparison` drawn over
/// `rate` at a baseline drop that stays just under `same_line_threshold`,
/// so the caller treats them as one line and defers here. The two spans
/// horizontally OVERLAP (negative gap), which the positive-gap test would
/// reject — fusing them into `Comparisonrate`. A negative gap combined with
/// a real baseline shift is two stacked tokens (never intra-word kerning,
/// which shares a baseline), so a space must be inserted.
#[test]
fn test_stacked_cell_needs_space_overlapping_rows() {
    // fs=12 → same_line_threshold = max(14.4, 3.6) = 14.4; y_diff = 8 stays
    // under it (one line), gap = 20 - 60 = -40 (overlap). ~keep
    let upper = make_test_span("Comparison", 0.0, 108.0, 60.0, 12.0);
    let lower = make_test_span("rate", 20.0, 100.0, 25.0, 12.0);
    assert!(
        PdfDocument::stacked_cell_needs_space(&upper, &lower),
        "stacked overlapping cells with a baseline shift must be separated by a space"
    );
}

/// Guard: two spans on the SAME baseline that overlap by a couple points
/// (real intra-word kerning, e.g. `eigen`+`value` split by a font's tight
/// side-bearings) must NOT be flagged — the baseline shift is what
/// distinguishes a stacked cell from kerning.
#[test]
fn test_stacked_cell_same_baseline_overlap_is_kerning() {
    let prev = make_test_span("eigen", 0.0, 100.0, 30.0, 12.0);
    let current = make_test_span("value", 28.0, 100.0, 30.0, 12.0);
    assert!(
        !PdfDocument::stacked_cell_needs_space(&prev, &current),
        "same-baseline overlap is intra-word kerning, not a word boundary"
    );
}

/// Two glyphs of the same complex Brahmic script with an intra-word
/// gap (a Bengali matra-cluster `ছো` followed by `ট`, ~9pt apart at 13pt)
/// must NOT get a heuristic space — word breaks in these scripts are
/// carried by explicit SPACE glyphs (§14.8.2.5), and the Latin-tuned gap
/// test otherwise splits syllables (`ছো ট`). Mirrors the CJK guard.
#[test]
fn test_should_insert_space_suppressed_within_complex_script() {
    // Bengali: prev ends in matra ো (U+09CB), next is consonant ট (U+09AF…
    // here U+099F) — same script, ~9pt gap. ~keep
    let prev = make_test_span("\u{099B}\u{09CB}", 0.0, 100.0, 7.0, 13.0);
    let current = make_test_span("\u{099F}", 16.0, 100.0, 5.0, 13.0);
    assert!(
        !PdfDocument::should_insert_space(&prev, &current),
        "intra-word complex-script gap must not insert a space"
    );
    // Tamil likewise (prev ends in vowel sign ை U+0BC8, next consonant). ~keep
    let p2 = make_test_span("\u{0B87}\u{0BA9}\u{0BC8}", 0.0, 100.0, 20.0, 13.0);
    let c2 = make_test_span("\u{0B9A}\u{0BCD}", 30.0, 100.0, 8.0, 13.0);
    assert!(!PdfDocument::should_insert_space(&p2, &c2));
}

/// The guard is script-specific: a complex-script glyph meeting a Latin
/// glyph across a real gap still gets its boundary space (only *same*-script
/// intra-word gaps are suppressed).
#[test]
fn test_should_insert_space_kept_across_script_boundary() {
    let prev = make_test_span("\u{0B95}", 0.0, 100.0, 12.0, 13.0);
    let current = make_test_span("A", 18.0, 100.0, 8.0, 13.0);
    assert!(
        PdfDocument::should_insert_space(&prev, &current),
        "complex↔Latin boundary gap must keep its space"
    );
}

#[test]
fn test_column_spanning_decimal_wide_bbox() {
    // "1.10": 4 chars, cw=[3.98], expected=15.92, gap=9.8 > fs(7.0) → split ~keep
    let span = make_decimal_span("1.10", vec![3.9811199], 25.72, 7.0);
    assert!(PdfDocument::is_column_spanning_decimal(&span));
}

#[test]
fn test_column_spanning_decimal_5char_span() {
    // "12.11": 5 chars, cw=[3.98,3.98], expected=19.91, gap=7.73 > fs(7.0) → split ~keep
    let span = make_decimal_span("12.11", vec![3.9811199, 3.9811199], 27.64, 7.0);
    assert!(PdfDocument::is_column_spanning_decimal(&span));
}

#[test]
fn test_column_spanning_decimal_normal_bbox() {
    // "1.5" with 3 entries matching 3 chars; bbox_w = expected → gap ≈ 0 → no split ~keep
    let span = make_decimal_span("1.5", vec![3.0, 3.0, 3.0], 9.0, 7.0);
    assert!(!PdfDocument::is_column_spanning_decimal(&span));
}

#[test]
fn test_column_spanning_decimal_non_digit() {
    // "hello.world" — letters, not digits → no split ~keep
    let span = make_decimal_span("hello.world", vec![], 60.0, 12.0);
    assert!(!PdfDocument::is_column_spanning_decimal(&span));
}

#[test]
fn test_column_spanning_decimal_multiple_dots() {
    // "1.2.3" — two dots → no split ~keep
    let span = make_decimal_span("1.2.3", vec![3.0], 25.0, 7.0);
    assert!(!PdfDocument::is_column_spanning_decimal(&span));
}

#[test]
fn test_push_span_text_splits_wide_decimal() {
    let span = make_decimal_span("1.10", vec![3.9811199], 25.72, 7.0);
    let mut out = String::new();
    PdfDocument::push_span_text(&mut out, &span);
    assert_eq!(out, "1 10");
}

#[test]
fn test_push_span_text_leaves_normal_decimal() {
    let span = make_decimal_span("3.14", vec![4.0, 4.0, 4.0, 4.0], 16.0, 12.0);
    let mut out = String::new();
    PdfDocument::push_span_text(&mut out, &span);
    assert_eq!(out, "3.14");
}

#[test]
fn test_push_span_text_strips_soft_hyphen_mid_word() {
    // ISO 32000-1 §14.8.2.2.3: U+00AD marks a discretionary line-break
    // point only — it must never survive into extract_text/to_markdown/
    // to_html output, even mid-word with no adjacent line break (the
    // span was drawn as a single reflowed run, not split across lines). ~keep
    let span = make_decimal_span("recon\u{00AD}struction", vec![], 80.0, 12.0);
    let mut out = String::new();
    PdfDocument::push_span_text(&mut out, &span);
    assert_eq!(out, "reconstruction");
}

#[test]
fn test_push_span_text_strips_multiple_soft_hyphens() {
    let span = make_decimal_span("un\u{00AD}be\u{00AD}liev\u{00AD}able", vec![], 100.0, 12.0);
    let mut out = String::new();
    PdfDocument::push_span_text(&mut out, &span);
    assert_eq!(out, "unbelievable");
}

#[test]
fn test_cw_boundary_split_theorem_number() {
    // "Theorem1.7": 10 chars, 7 widths → split before '1' ~keep
    let span = make_decimal_span("Theorem1.7", vec![11.2, 8.9, 7.4, 8.1, 6.6, 7.4, 13.4], 83.8, 14.3);
    let result = PdfDocument::char_widths_boundary_split(&span);
    assert_eq!(result, Some(7)); // byte 7 = '1' ~keep
}

#[test]
fn test_cw_boundary_split_let_capital() {
    // "LetC": 4 chars, 3 widths — lower→upper boundary → split at 'C'
    // (represents two CID text runs "Let" + "C" concatenated) ~keep
    let span = make_decimal_span("LetC", vec![7.3, 5.2, 4.5], 26.7, 12.0);
    let result = PdfDocument::char_widths_boundary_split(&span);
    assert_eq!(result, Some(3)); // byte 3 = 'C' ~keep
}

#[test]
fn test_cw_boundary_no_split_already_space() {
    // "Theorem 1.1": 7 widths, char at idx 7 is space → no split ~keep
    let span = make_decimal_span("Theorem 1.1", vec![9.3, 7.5, 6.1, 6.7, 5.5, 6.1, 11.2], 80.0, 12.0);
    assert!(PdfDocument::char_widths_boundary_split(&span).is_none());
}

#[test]
fn test_cw_boundary_no_split_matching_count() {
    // "hello" with 5 widths: no mismatch ~keep
    let span = make_decimal_span("hello", vec![5.0, 5.0, 5.0, 5.0, 5.0], 25.0, 12.0);
    assert!(PdfDocument::char_widths_boundary_split(&span).is_none());
}

#[test]
fn test_cw_boundary_no_split_nonascii_boundary() {
    // "Marysia Prus-Gł": boundary char is 'ł' (non-ASCII) → no split ~keep
    let span = make_decimal_span("Marysia Prus-Gł", vec![5.0; 14], 80.0, 12.0);
    assert!(PdfDocument::char_widths_boundary_split(&span).is_none());
}

#[test]
fn test_push_span_text_splits_let_capital() {
    // Lower→upper boundary: "LetC" splits to "Let C" (space inserted at 'C') ~keep
    let span = make_decimal_span("LetC", vec![7.3, 5.2, 4.5], 26.7, 12.0);
    let mut out = String::new();
    PdfDocument::push_span_text(&mut out, &span);
    assert_eq!(out, "Let C");
}

#[test]
fn test_push_span_text_splits_theorem_number() {
    let span = make_decimal_span("Theorem1.7", vec![11.2, 8.9, 7.4, 8.1, 6.6, 7.4, 13.4], 83.8, 14.3);
    let mut out = String::new();
    PdfDocument::push_span_text(&mut out, &span);
    assert_eq!(out, "Theorem 1.7");
}

#[test]
fn test_should_insert_space_overlapping() {
    let prev = make_test_span("Hello", 0.0, 100.0, 50.0, 12.0);
    let current = make_test_span("World", 40.0, 100.0, 50.0, 12.0);
    assert!(!PdfDocument::should_insert_space(&prev, &current));
}

#[test]
fn test_should_insert_space_zero_font_size() {
    let prev = make_test_span("A", 0.0, 100.0, 10.0, 0.0);
    let current = make_test_span("B", 15.0, 100.0, 10.0, 0.0);
    let _ = PdfDocument::should_insert_space(&prev, &current);
}

#[test]
fn test_should_insert_space_large_font() {
    let prev = make_test_span("A", 0.0, 100.0, 100.0, 72.0);
    let current = make_test_span("B", 120.0, 100.0, 100.0, 72.0);
    assert!(PdfDocument::should_insert_space(&prev, &current));
}

// SEG-KO: a Sino-Korean numeral hugs its counter ("1만년"), so a tightly
// typeset Hangul↔digit boundary must NOT get a forced space. ~keep
#[test]
fn test_should_insert_space_hangul_digit_no_space() {
    let one = make_test_span("1", 0.0, 100.0, 8.0, 12.0);
    let man = make_test_span("만", 8.5, 100.0, 12.0, 12.0);
    assert!(!PdfDocument::should_insert_space(&one, &man));
    let nyeon = make_test_span("년", 0.0, 100.0, 12.0, 12.0);
    let two = make_test_span("2", 12.5, 100.0, 8.0, 12.0);
    assert!(!PdfDocument::should_insert_space(&nyeon, &two));
}

// The Hangul exception must NOT relax the Chinese ideograph↔digit split
// ("神鹰集团" + "2015" → separate tokens, issue 484). ~keep
#[test]
fn test_should_insert_space_ideograph_digit_still_splits() {
    let tuan = make_test_span("团", 0.0, 100.0, 12.0, 12.0);
    let year = make_test_span("2", 12.5, 100.0, 8.0, 12.0);
    assert!(PdfDocument::should_insert_space(&tuan, &year));
}

// SEG-INDIC: clause punctuation hugs the preceding Brahmic-script word, so a
// wide post-syllable advance must not float a danda / comma / colon off as
// its own token. Latin keeps its spacing (no regression). ~keep
#[test]
fn test_should_insert_space_indic_clause_punct_hugs() {
    let beng = make_test_span("ী", 0.0, 100.0, 12.0, 12.0);
    let danda = make_test_span("।", 15.0, 100.0, 6.0, 12.0);
    assert!(!PdfDocument::should_insert_space(&beng, &danda));
    let deva = make_test_span("ी", 0.0, 100.0, 12.0, 12.0);
    let comma = make_test_span(",", 15.0, 100.0, 5.0, 12.0);
    assert!(!PdfDocument::should_insert_space(&deva, &comma));
    // Latin word + comma at the same gap STILL gets a space (Indic-scoped). ~keep
    let latin = make_test_span("word", 0.0, 100.0, 12.0, 12.0);
    let comma2 = make_test_span(",", 15.0, 100.0, 5.0, 12.0);
    assert!(PdfDocument::should_insert_space(&latin, &comma2));
}

// SEG-KO: a Hangul eojeol that wrapped mid-syllable rejoins with no break;
// an eojeol-boundary wrap (text already ends with a space) still separates. ~keep
#[test]
fn test_hangul_midword_line_wrap() {
    let prev = make_test_span("집고양", 480.0, 110.0, 36.0, 12.0);
    let next = make_test_span("이의", 50.0, 95.0, 24.0, 12.0);
    assert!(PdfDocument::hangul_midword_line_wrap("…집고양", &prev, &next));
    assert!(!PdfDocument::hangul_midword_line_wrap("…했다 ", &prev, &next));
    let latin = make_test_span("the", 50.0, 95.0, 24.0, 12.0);
    assert!(!PdfDocument::hangul_midword_line_wrap("…집고양", &prev, &latin));
}

// -----------------------------------------------------------------
// PdfDocument::contains_rect_with_tolerance
//
// Pins the table-retain tolerance behaviour: spans whose f32
// right-edge drifts a fraction of a point past the table bbox
// (due to accumulated width-sum error) must still count as
// contained, but spans that actually extend beyond the table
// must not. Each test's first block is a geometry sanity check
// so a Rect::new construction mistake fails loudly rather than
// silently exercising the wrong geometry.
// ----------------------------------------------------------------- ~keep
#[test]
fn contains_rect_with_tolerance_absorbs_subpixel_drift() {
    use crate::geometry::Rect;
    let table = Rect::new(0.0, 0.0, 100.0, 100.0);
    let drifted = Rect::new(10.0, 10.0, 90.02, 80.0);

    // Geometry sanity: drifted span right-edge should sit ~0.02pt
    // past table right-edge. If this fails, the test construction
    // is wrong, not the tolerance logic. Tolerance is 1e-4pt
    // because `0.02f32` is not representable exactly — the
    // observed drift lands within ~4e-6 of 0.02. ~keep
    assert!(
        (drifted.right() - table.right() - 0.02).abs() < 1e-4,
        "drifted span right-edge should be 0.02pt past table right-edge; got drift = {}",
        drifted.right() - table.right()
    );
    assert_eq!(drifted.left(), 10.0, "span should start at x=10");
    assert_eq!(drifted.top(), 10.0, "span should start at y=10");
    assert_eq!(drifted.bottom(), 90.0, "span should end at y=90");

    assert!(PdfDocument::contains_rect_with_tolerance(&table, &drifted, 0.1));
}

#[test]
fn contains_rect_with_tolerance_rejects_genuinely_outside() {
    use crate::geometry::Rect;
    let table = Rect::new(0.0, 0.0, 100.0, 100.0);
    let outside = Rect::new(10.0, 10.0, 91.0, 80.0);

    assert!(
        (outside.right() - table.right() - 1.0).abs() < 1e-6,
        "outside span right-edge should be 1.0pt past table right-edge; got drift = {}",
        outside.right() - table.right()
    );

    assert!(!PdfDocument::contains_rect_with_tolerance(&table, &outside, 0.1));
}

#[test]
fn contains_rect_with_tolerance_accepts_fully_inside() {
    use crate::geometry::Rect;
    let table = Rect::new(0.0, 0.0, 100.0, 100.0);
    let inside = Rect::new(10.0, 10.0, 80.0, 80.0);

    assert!(
        inside.left() > table.left()
            && inside.right() < table.right()
            && inside.top() > table.top()
            && inside.bottom() < table.bottom(),
        "control span should be strictly inside the table"
    );

    assert!(PdfDocument::contains_rect_with_tolerance(&table, &inside, 0.1));
}

/// Regression test (pdfa_036): span filtering must use per-cell
/// bboxes, not the coarser outer table bbox.
///
/// Before the fix, `span_in_table` filtered by `table.bbox`, which could
/// be wider than the union of the actual cell bboxes. Paragraph text that
/// happened to fall inside the table's outer bbox was silently dropped even
/// though no cell claimed it, causing content loss (the "(HLA)/(KSL)"
/// paragraph in pdfa_036 disappeared).
///
/// After the fix, only spans inside at least one *cell* bbox are removed
/// from the flow. Spans inside the outer table bbox but outside all cells
/// (i.e. in a gap or margin) are preserved.
#[test]
fn cell_bbox_filter_preserves_span_in_outer_bbox_gap() {
    use crate::geometry::Rect;
    use crate::structure::table_extractor::{Table, TableCell, TableRow};

    // A table whose outer bbox is [0, 0] – [200, 100].
    // Two non-adjacent cells leave a horizontal gap at x=90..110 — that
    // gap is inside the outer bbox but not inside any cell. ~keep
    let mut table = Table::new();
    let mut row = TableRow::new(false);
    row.cells.push(TableCell {
        text: "left".to_string(),
        spans: vec![],
        colspan: 1,
        rowspan: 1,
        mcids: vec![],
        bbox: Some(Rect::new(0.0, 0.0, 90.0, 100.0)),
        is_header: false,
    });
    row.cells.push(TableCell {
        text: "right".to_string(),
        spans: vec![],
        colspan: 1,
        rowspan: 1,
        mcids: vec![],
        bbox: Some(Rect::new(110.0, 0.0, 90.0, 100.0)),
        is_header: false,
    });
    table.add_row(row);
    table.bbox = Some(Rect::new(0.0, 0.0, 200.0, 100.0));

    const TOL: f32 = 0.1;

    let span_cell = Rect::new(10.0, 10.0, 50.0, 20.0);
    let in_any_cell = table.rows.iter().any(|r| {
        r.cells.iter().any(|c| {
            c.bbox
                .is_some_and(|b| PdfDocument::contains_rect_with_tolerance(&b, &span_cell, TOL))
        })
    });
    assert!(in_any_cell, "span inside a cell bbox must be identified as in-table");

    let span_gap = Rect::new(95.0, 10.0, 10.0, 20.0);

    // 1. Outer-bbox filter (the OLD, incorrect approach) would classify it as in-table. ~keep
    let in_outer_bbox = PdfDocument::contains_rect_with_tolerance(&table.bbox.unwrap(), &span_gap, TOL);
    assert!(
        in_outer_bbox,
        "gap span must be inside the outer table bbox (precondition for the bug to trigger)"
    );

    // 2. Cell-bbox filter (the NEW, correct approach) must NOT classify it as in-table. ~keep
    let in_any_cell_gap = table.rows.iter().any(|r| {
        r.cells.iter().any(|c| {
            c.bbox
                .is_some_and(|b| PdfDocument::contains_rect_with_tolerance(&b, &span_gap, TOL))
        })
    });
    assert!(
        !in_any_cell_gap,
        "gap span must NOT be inside any cell bbox — cell-bbox filter must preserve it"
    );
}

#[test]
fn reorder_same_line_runs_preserves_disjoint_x_rows() {
    use crate::geometry::Rect;
    use crate::layout::TextSpan;

    // Two rows close enough in Y to pass the existing same_line_threshold:
    // Δy = 4.5 and fs = 10, so threshold = 5.0.
    // They are disjoint in X (gap of 225pt = 22.5 * fs, well over the
    // SAME_LINE_REORDER_MAX_GAP_FACTOR = 3.0 ceiling). The helper must
    // not X-sort them into [skersey, VerDate]; it must preserve the
    // row-aware order. ~keep
    let mut spans = vec![
        TextSpan {
            text: "VerDate".to_string(),
            bbox: Rect::new(350.0, 200.0, 85.0, 10.0),
            font_size: 10.0,
            sequence: 0,
            ..Default::default()
        },
        TextSpan {
            text: "skersey".to_string(),
            bbox: Rect::new(50.0, 195.5, 75.0, 10.0),
            font_size: 10.0,
            sequence: 1,
            ..Default::default()
        },
    ];

    PdfDocument::reorder_same_line_runs(&mut spans);

    let texts: Vec<&str> = spans.iter().map(|s| s.text.as_str()).collect();
    assert_eq!(texts, vec!["VerDate", "skersey"]);
}

#[test]
fn reorder_same_line_runs_orders_suffix_superscript_by_x() {
    use crate::geometry::Rect;
    use crate::layout::TextSpan;

    // Row-aware/Y-desc order can put the superscript first because it
    // sits higher. The tentative X-gap validation must not reject this
    // legitimate mixed-baseline run; the X-sorted gaps are 15pt and 0pt
    // at max_fs=14, both well under 3.0 * 14 = 42. Final order should
    // be normal left-to-right text. ~keep
    let mut spans = vec![
        TextSpan {
            text: "th".to_string(),
            bbox: Rect::new(180.0, 205.0, 10.0, 10.0),
            font_size: 10.0,
            sequence: 0,
            ..Default::default()
        },
        TextSpan {
            text: "September".to_string(),
            bbox: Rect::new(100.0, 200.0, 50.0, 14.0),
            font_size: 14.0,
            sequence: 1,
            ..Default::default()
        },
        TextSpan {
            text: "11".to_string(),
            bbox: Rect::new(165.0, 200.0, 15.0, 14.0),
            font_size: 14.0,
            sequence: 2,
            ..Default::default()
        },
    ];

    PdfDocument::reorder_same_line_runs(&mut spans);

    let texts: Vec<&str> = spans.iter().map(|s| s.text.as_str()).collect();
    assert_eq!(texts, vec!["September", "11", "th"]);
}

#[test]
fn reorder_same_line_runs_de_interleaves_two_stacked_lines() {
    use crate::geometry::Rect;
    use crate::layout::TextSpan;

    // Two lines the same-line tolerance merged into one band: at fs=10 the
    // threshold is max(10*1.2, 10*0.3)=12, and the lines are 8pt apart — so
    // they group as one run, yet 8 > 0.5*fs (=5) makes them TWO stacked rows
    // of two spans each. Their X-extents overlap, so a flat X-sort would
    // interleave them word-by-word ("The Story Book Review"). The
    // de-interleave path must instead order (Y-desc, then X) so each real
    // line stays contiguous: line one ("The Book") then line two
    // ("Story Review"). Input is given in the interleaved X order the
    // row-aware sort would produce. ~keep
    let span = |t: &str, x: f32, y: f32, w: f32, seq: usize| TextSpan {
        text: t.to_string(),
        bbox: Rect::new(x, y, w, 10.0),
        font_size: 10.0,
        sequence: seq,
        ..Default::default()
    };
    let mut spans = vec![
        span("The", 100.0, 200.0, 30.0, 0),
        span("Story", 110.0, 192.0, 40.0, 1),
        span("Book", 140.0, 200.0, 40.0, 2),
        span("Review", 150.0, 192.0, 55.0, 3),
    ];

    PdfDocument::reorder_same_line_runs(&mut spans);

    let texts: Vec<&str> = spans.iter().map(|s| s.text.as_str()).collect();
    assert_eq!(
        texts,
        vec!["The", "Book", "Story", "Review"],
        "stacked lines must de-interleave, not X-sort into one fake line"
    );
}

/// A ligature glyph drawn as its own run inside one word ("di" + "ﬀ" + "erent") is
/// separated from its neighbours by kerning slightly above the ordinary word-gap
/// threshold; it must not read as a word break (#2322).
#[test]
fn test_should_insert_space_keeps_a_tight_ligature_run_inside_a_word() {
    let before = make_test_span("di", 72.0, 700.0, 10.0, 12.0);
    let ligature = make_test_span("\u{FB00}", 84.0, 700.0, 7.0, 12.0); // 2pt = 0.17em gap ~keep
    let after = make_test_span("erent", 93.0, 700.0, 30.0, 12.0); // 2pt gap ~keep
    assert!(!PdfDocument::should_insert_space(&before, &ligature));
    assert!(!PdfDocument::should_insert_space(&ligature, &after));
}

/// The same relaxed threshold must not weld two words: a real space-sized gap
/// after a word-final ligature still separates them.
#[test]
fn test_should_insert_space_still_separates_words_around_a_ligature() {
    let word = make_test_span("sta\u{FB00}", 72.0, 700.0, 25.0, 12.0);
    let next = make_test_span("only", 100.0, 700.0, 20.0, 12.0); // 3pt = 0.25em gap ~keep
    assert!(PdfDocument::should_insert_space(&word, &next));
    let ligature_start = make_test_span("\u{FB01}nal", 100.0, 700.0, 20.0, 12.0);
    let prev = make_test_span("the", 72.0, 700.0, 25.0, 12.0);
    assert!(PdfDocument::should_insert_space(&prev, &ligature_start));
}

/// A word-final ligature inside a longer run ("staﬀ") is not a bare ligature run, so
/// a gap only slightly above the ordinary threshold still separates the next word.
#[test]
fn test_should_insert_space_after_a_word_final_ligature_uses_the_ordinary_gap() {
    let word = make_test_span("sta\u{FB00}", 72.0, 700.0, 25.0, 12.0);
    let next = make_test_span("only", 99.5, 700.0, 20.0, 12.0); // 2.5pt = 0.21em gap ~keep
    assert!(PdfDocument::should_insert_space(&word, &next));
}
