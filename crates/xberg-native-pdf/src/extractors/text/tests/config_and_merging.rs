//! Extraction-config and span-merging-config tests split out of `extractors/text/tests.rs` for file size. ~keep

use super::super::*;
use super::extraction_and_rotation::create_test_font;
use std::sync::Arc;

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
