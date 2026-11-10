//! Unit tests for [`super`].
//!
//! Split out of `gap_statistics.rs` purely for file size, mirroring the split used for
//! `document.rs`/`document/tests.rs` and `extractors/text/mod.rs`/`extractors/text/tests.rs`. ~keep

use super::*;

#[test]
fn test_percentile_single_value() {
    let values = vec![5.0];
    assert_eq!(percentile(&values, 0.5), 5.0);
}

#[test]
fn test_percentile_two_values() {
    let values = vec![1.0, 3.0];
    assert_eq!(percentile(&values, 0.0), 1.0);
    assert_eq!(percentile(&values, 1.0), 3.0);
    assert_eq!(percentile(&values, 0.5), 2.0);
}

#[test]
fn test_percentile_many_values() {
    let values = vec![1.0, 2.0, 3.0, 4.0, 5.0, 6.0, 7.0, 8.0, 9.0, 10.0];
    assert_eq!(percentile(&values, 0.0), 1.0);
    assert_eq!(percentile(&values, 1.0), 10.0);
    assert_eq!(percentile(&values, 0.5), 5.5);
}

#[test]
fn test_extract_gaps() {
    use crate::geometry::Rect;

    let spans = vec![
        TextSpan {
            provenance: None,
            text_rise: 0.0,
            artifact_type: None,
            text: "Hello".to_string(),
            bbox: Rect::new(0.0, 0.0, 30.0, 12.0),
            font_name: "Arial".to_string(),
            font_size: 12.0,
            font_weight: crate::layout::FontWeight::Normal,
            is_italic: false,
            is_monospace: false,
            color: crate::layout::Color::new(0.0, 0.0, 0.0),
            mcid: None,
            mcid_scope: None,
            sequence: 0,
            split_boundary_before: false,
            offset_semantic: false,
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
            bbox: Rect::new(35.0, 0.0, 30.0, 12.0),
            font_name: "Arial".to_string(),
            font_size: 12.0,
            font_weight: crate::layout::FontWeight::Normal,
            is_italic: false,
            is_monospace: false,
            color: crate::layout::Color::new(0.0, 0.0, 0.0),
            mcid: None,
            mcid_scope: None,
            sequence: 1,
            split_boundary_before: false,
            offset_semantic: false,
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

    let gaps = extract_gaps(&spans);
    assert_eq!(gaps.len(), 1);
    assert_eq!(gaps[0], 5.0);
}

#[test]
fn test_extract_gaps_empty() {
    let gaps = extract_gaps(&[]);
    assert!(gaps.is_empty());
}

#[test]
fn test_calculate_statistics() {
    let gaps = vec![0.1, 0.2, 0.15, 0.25, 0.3];
    let stats = calculate_statistics(gaps).unwrap();

    assert_eq!(stats.count, 5);
    assert_eq!(stats.min, 0.1);
    assert_eq!(stats.max, 0.3);
    assert!(stats.mean > 0.19 && stats.mean < 0.21);
}

#[test]
fn test_calculate_statistics_empty() {
    let gaps = vec![];
    assert!(calculate_statistics(gaps).is_none());
}

#[test]
fn test_gap_statistics_iqr() {
    let gaps = vec![1.0, 2.0, 3.0, 4.0, 5.0];
    let stats = calculate_statistics(gaps).unwrap();
    let iqr = stats.iqr();
    assert!(iqr > 0.0);
}

#[test]
fn test_adaptive_threshold_config_defaults() {
    let config = AdaptiveThresholdConfig::default();
    assert_eq!(config.median_multiplier, 1.5);
    assert_eq!(config.min_threshold_pt, 0.05);
    // Phase 7 FIX: max_threshold_pt was increased from 1.0 to 100.0
    // to allow computed thresholds for documents with larger word spacing ~keep
    assert_eq!(config.max_threshold_pt, 100.0);
    assert!(!config.use_iqr);
    assert_eq!(config.min_samples, 10);
}

#[test]
fn test_adaptive_threshold_config_aggressive() {
    let config = AdaptiveThresholdConfig::aggressive();
    assert_eq!(config.median_multiplier, 1.2);
}

#[test]
fn test_adaptive_threshold_config_conservative() {
    let config = AdaptiveThresholdConfig::conservative();
    assert_eq!(config.median_multiplier, 2.0);
}

#[test]
fn test_determine_threshold_clamping() {
    let gaps = vec![0.01, 0.01, 0.01, 0.01, 0.01, 0.01, 0.01, 0.01, 0.01, 0.01];
    let stats = calculate_statistics(gaps).unwrap();
    let config = AdaptiveThresholdConfig::default();

    let threshold = determine_adaptive_threshold(&stats, &config);
    assert!(threshold >= config.min_threshold_pt);
    assert!(threshold <= config.max_threshold_pt);
}

#[test]
fn test_analyze_document_gaps_empty() {
    let result = analyze_document_gaps(&[], None);
    assert_eq!(result.threshold_pt, 0.1);
    assert!(result.stats.is_none());
}

#[test]
fn test_analyze_document_gaps_insufficient_samples() {
    use crate::geometry::Rect;

    let spans = vec![
        TextSpan {
            provenance: None,
            text_rise: 0.0,
            artifact_type: None,
            text: "A".to_string(),
            bbox: Rect::new(0.0, 0.0, 10.0, 12.0),
            font_name: "Arial".to_string(),
            font_size: 12.0,
            font_weight: crate::layout::FontWeight::Normal,
            is_italic: false,
            is_monospace: false,
            color: crate::layout::Color::new(0.0, 0.0, 0.0),
            mcid: None,
            mcid_scope: None,
            sequence: 0,
            split_boundary_before: false,
            offset_semantic: false,
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
            text: "B".to_string(),
            bbox: Rect::new(15.0, 0.0, 10.0, 12.0),
            font_name: "Arial".to_string(),
            font_size: 12.0,
            font_weight: crate::layout::FontWeight::Normal,
            is_italic: false,
            is_monospace: false,
            color: crate::layout::Color::new(0.0, 0.0, 0.0),
            mcid: None,
            mcid_scope: None,
            sequence: 1,
            split_boundary_before: false,
            offset_semantic: false,
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

    let result = analyze_document_gaps(&spans, None);
    assert_eq!(result.threshold_pt, 0.1);
    assert!(result.stats.is_none());
}
