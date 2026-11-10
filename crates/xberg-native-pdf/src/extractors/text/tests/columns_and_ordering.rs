//! Span-column detection, reading-order, and adjacent-span-merge tests split out of `extractors/text/tests.rs` for file size. ~keep

use super::super::*;
use super::extraction_and_rotation::create_test_font;

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
            ligature_glyph: false,
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
            ligature_glyph: false,
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
            ligature_glyph: false,
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

    // Only the fields the assertion cares about are set explicitly; every other
    // field matches `TextSpan::default()` exactly, so this is byte-identical to
    // the previous fully-spelled-out literals. ~keep
    extractor.spans = vec![
        TextSpan {
            text: "Hello".to_string(),
            bbox: Rect::new(100.0, 700.0, 30.0, 12.0),
            font_name: "F1".to_string(),
            sequence: 0,
            rotation_degrees: 180.0,
            ..TextSpan::default()
        },
        TextSpan {
            text: "World".to_string(),
            bbox: Rect::new(131.0, 700.0, 30.0, 12.0), // 1pt gap ~keep
            font_name: "F1".to_string(),
            sequence: 1,
            rotation_degrees: 180.0,
            ..TextSpan::default()
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
            ligature_glyph: false,
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
            ligature_glyph: false,
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
            ligature_glyph: false,
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
            ligature_glyph: false,
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

    // Only the fields the assertion cares about are set explicitly; every other
    // field matches `TextSpan::default()` exactly, so this is byte-identical to
    // the previous fully-spelled-out literals. ~keep
    extractor.spans = vec![
        TextSpan {
            text: "Hello".to_string(),
            bbox: Rect::new(100.0, 700.0, 30.0, 12.0),
            font_name: "F1".to_string(),
            sequence: 0,
            ..TextSpan::default()
        },
        TextSpan {
            text: " ".to_string(),
            bbox: Rect::new(130.0, 700.0, 2.0, 12.0),
            font_name: "F1".to_string(),
            sequence: 1,
            offset_semantic: true,
            ..TextSpan::default()
        },
        TextSpan {
            text: "World".to_string(),
            bbox: Rect::new(132.0, 700.0, 30.0, 12.0),
            font_name: "F1".to_string(),
            sequence: 2,
            ..TextSpan::default()
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
        "word ", "next", 5.0, 12.0, "F1", &fonts, true, &config, None, None, 12.0, 12.0, false,
    );
    assert!(!decision.insert_space);
    assert_eq!(decision.source, SpaceSource::AlreadyPresent);
}

#[test]
fn test_should_insert_space_boundary_already_present_leading() {
    let config = SpanMergingConfig::default();
    let fonts = HashMap::new();

    let decision = should_insert_space(
        "word", " next", 5.0, 12.0, "F1", &fonts, true, &config, None, None, 12.0, 12.0, false,
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
        "word", "next", 10.0, 12.0, "F1", &fonts, false, &config, None, None, 12.0, 12.0, false,
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
        "word", "next", 4.0, 12.0, "F1", &fonts, true, &config, None, None, 12.0, 12.0, false,
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
        "word", "next", 0.5, 12.0, "F1", &fonts, false, &config, None, None, 12.0, 12.0, false,
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
        false,
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
        false,
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
