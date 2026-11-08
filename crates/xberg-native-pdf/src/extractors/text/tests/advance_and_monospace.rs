//! Glyph-advance, adaptive-threshold, and monospace-font-detection tests split out of `extractors/text/tests.rs` for file size. ~keep

use super::super::*;
use super::extraction_and_rotation::create_test_font;
use std::sync::Arc;

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

/// A fixed-pitch run drawn one glyph per text matrix keeps its empty cell as a word gap when the text
/// matrix scales the glyphs: the same layout in user space reads the same at either scale.
#[test]
fn fixed_pitch_empty_cell_under_a_scaled_text_matrix_is_a_word_gap() {
    let text_of = |stream: &[u8]| -> String {
        let mut font = create_test_font();
        font.base_font = "Courier".to_string();
        font.widths = Some(vec![600.0; 0x7F - 0x20]);
        font.first_char = Some(0x20);
        font.last_char = Some(0x7E);
        let mut extractor = TextExtractor::new();
        extractor.add_font("F1".to_string(), font);
        let spans = extractor.extract_text_spans(stream).unwrap();
        spans.iter().map(|s| s.text.as_str()).collect::<Vec<_>>().join("")
    };
    // Each cell is 6 points wide in user space; the third glyph skips one cell. ~keep
    let stream = |scale: f32| {
        let tm = |x: u32| format!("{scale} 0 0 {scale} {x} 700 Tm");
        format!(
            "BT /F1 {} Tf {} (A) Tj {} (B) Tj {} (C) Tj ET",
            10.0 / scale,
            tm(72),
            tm(78),
            tm(90)
        )
    };
    let unscaled = text_of(stream(1.0).as_bytes());
    let scaled = text_of(stream(0.5).as_bytes());
    assert_eq!(unscaled, "AB C");
    assert_eq!(scaled, unscaled);
    assert_eq!(text_of(stream(10.0).as_bytes()), unscaled);
}
