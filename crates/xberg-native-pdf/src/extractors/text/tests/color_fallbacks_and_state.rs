//! Named-color-space fallback and graphics-state (line cap/join, rendering intent) tests split out of `extractors/text/tests.rs` for file size. ~keep

use super::super::*;
use super::extraction_and_rotation::create_test_font;
use std::sync::Arc;

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
    assert_eq!(corrected_space_gap(0.0, false, 34.23, false, 0.0), 0.0);
    // small positive raw gap (academic "XGBoostX"+"provides") untouched. ~keep
    assert_eq!(corrected_space_gap(0.47, false, 50.0, false, 0.0), 0.47);
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
    let g = corrected_space_gap(-2.0, false, 30.0, false, 0.0);
    assert!(
        g > 0.0,
        "overlap on fallback-width font must be lifted positive, got {g}"
    );
}

/// Reliable-width fonts (explicit /Widths) are never corrected — the
/// bbox gap is authoritative regardless of sign.
#[test]
fn test_corrected_space_gap_reliable_widths_untouched() {
    assert_eq!(corrected_space_gap(-2.0, true, 30.0, false, 0.0), -2.0);
    assert_eq!(corrected_space_gap(5.0, true, 30.0, false, 0.0), 5.0);
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
fn corrected_space_gap_preserves_a_one_ulp_overlap() {
    let previous_end = 87.79001_f32;
    let next_start = 87.79_f32;
    let raw_gap = next_start - previous_end;
    assert!(raw_gap < 0.0);
    assert_eq!(corrected_space_gap(raw_gap, false, 16.67, false, previous_end), raw_gap);
}

#[test]
fn corrected_space_gap_uses_the_coordinates_rounding_scale() {
    for next_start in [-10_000.0_f32, 87.79, 10_000.0] {
        let previous_end = next_start.next_up();
        let raw_gap = next_start - previous_end;
        let magnitude = previous_end.abs().max(next_start.abs());
        assert_eq!(corrected_space_gap(raw_gap, false, 16.67, false, magnitude), raw_gap);
    }
    let true_overlap = corrected_space_gap(-2.0, false, 30.0, false, 87.79);
    assert_eq!(true_overlap, corrected_space_gap(-2.0, false, 30.0, false, 0.0));
    assert!(true_overlap > 0.0);
}
