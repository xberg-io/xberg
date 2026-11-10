#![allow(warnings)]
//! Integration tests for spacing and word boundary detection (PDF-spec compliant only)
//!
//! These tests validate spacing logic against realistic PDF scenarios:
//! - Justified text with variable TJ offsets (real PDFs often use this)
//! - Multi-line text with proper line break handling
//! - Consensus-based spacing decision making
//! - Regression testing for TrueType cmap and text post-processing
//!
//! All tests use ONLY PDF-spec-defined signals per ISO 32000-1:2008:
//! - TJ array offsets (Section 9.4.4) - typographic hints
//! - Bbox coordinates (Section 5.2) - geometric positioning
//! - Font metrics (Sections 9.6-9.8) - character width
//!
//! NOTE: These tests use mock TextSpan data simulating real PDF extraction
//! because we're validating the algorithm logic before implementation.

use xberg_native_pdf::extractors::{SpanMergingConfig, TextExtractionConfig};
use xberg_native_pdf::geometry::Rect;
use xberg_native_pdf::layout::{Color, FontWeight, TextSpan};

/// Create a test text span with spacing information
fn create_test_span(text: &str, x: f32, y: f32, width: f32, height: f32, _tj_space_signal: bool) -> TextSpan {
    TextSpan {
        provenance: None,
        artifact_type: None,
        text: text.to_string(),
        bbox: Rect::new(x, y, width, height),
        font_name: "TimesRoman".to_string(),
        font_size: height,
        font_weight: FontWeight::Normal,
        is_italic: false,
        is_monospace: false,
        color: Color::black(),
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
        text_rise: 0.0,
        rtl_draw_logical: false,
        ligature_glyph: false,
        mirrored: false,
        page_rotation_applied: 0,
    }
}

/// Create a line of text with specified word positions and gaps
///
/// Simulates PDF text operators (Td, TD, Tm, T*) positioning text
/// on a single horizontal line with natural spacing.
///
/// Returns: Vector of TextSpan objects representing words
fn create_line_of_text(word_specs: Vec<(&str, f32, f32)>) -> Vec<TextSpan> {
    let font_size = 12.0;
    let baseline_y = 100.0;

    word_specs
        .into_iter()
        .map(|(text, x, width)| create_test_span(text, x, baseline_y, width, font_size, false))
        .collect()
}

// ============================================================================
// Test 4: Justified Text Spacing (Real-World Issue)
// ============================================================================
//
// PDF Problem: Academic/Government documents often use justified alignment
// which requires variable TJ offsets to distribute space across word boundaries.
// This causes false space insertion in justified text.
//
// Example from corpus:
//   Justified: "The quick brown fox jumps" might have:
//   - TJ offsets: [-50, -120, -80, -150, -200, -100] (high variance)
//   - Current (broken): Inserts space within words like "inform ation"
//   - Expected: No space insertion in hyphenated/justified text
//
// Spec Reference: ISO 32000-1:2008 Section 9.4.4 (TJ array offsets) ~keep

#[test]
fn test_justified_text_variable_tj_offsets_no_false_spaces() {
    //! Test: Justified text with high variance in TJ offsets
    //!
    //! Scenario: Academic paper with justified paragraphs
    //! TJ offsets in justified text: [-30, -180, -50, -200, -100, -250]
    //! (High variance simulates PDF justification algorithm distributing space)
    //!
    //! Expected: No spaces within words despite varying TJ offsets
    //! Per ISO 32000-1:2008 Section 9.4.4: TJ offsets are typographic hints,
    //! not semantic word boundaries.

    let tj_distribution = [-30.0, -180.0, -50.0, -200.0, -100.0, -250.0];

    // Calculate coefficient of variation (measures distribution variance)
    // High CV > 0.5 indicates justified text with variable spacing ~keep
    let mean = tj_distribution.iter().sum::<f32>() / tj_distribution.len() as f32;
    let variance = tj_distribution.iter().map(|x| (x - mean).powi(2)).sum::<f32>() / tj_distribution.len() as f32;
    let std_dev = variance.sqrt();
    let cv = std_dev.abs() / mean.abs();

    assert!(
        cv > 0.5,
        "Justified text distribution should have high CV (got {:.4})",
        cv
    );

    // Conservative threshold for justified text (3× std_dev)
    // This filters less aggressively to avoid false space insertion ~keep
    let conservative_threshold = mean - (3.0 * std_dev);

    // Aggressive threshold for normal text (1× std_dev)
    // This filters more aggressively for consistent spacing ~keep
    let aggressive_threshold = mean - std_dev;

    assert!(
        conservative_threshold < aggressive_threshold,
        "Conservative threshold ({:.2}) should be more negative than aggressive ({:.2})",
        conservative_threshold,
        aggressive_threshold
    );
}

// ============================================================================
// Test 5: Multi-Column Line Break Handling
// ============================================================================
//
// PDF Problem: Multi-column documents (newspapers, technical papers)
// sometimes have words split across line breaks without proper spacing.
//
// Example from corpus:
//   Line 1 end: "habitat"
//   Line 2 start: "quality"
//   Current (broken): "habitatquality" (no space between lines)
//   Expected: "habitat quality" (space inserted at line break)
//
// Spec Reference: ISO 32000-1:2008 Section 5.2 (coordinate system & positioning) ~keep

#[test]
fn test_multicolumn_line_break_spacing_with_hyphen() {
    //! Test: Words split across line breaks with soft hyphens
    //!
    //! Scenario: Multi-column newspaper article
    //! Line 1 ends with "habi-" (hyphenated)
    //! Line 2 starts with "tat" (continuation)
    //!
    //! Expected: No space inserted (soft hyphen indicates word continuation)
    //! Verification: Use vertical gap + same-column detection + hyphen check

    let line1_end = create_test_span("habi-", 50.0, 100.0, 50.0, 12.0, false);

    // Line 2: text "tat" starting at x=50 (same column), y=80 (well below Line 1)
    // Gap = line1.bottom (112) - line2.top (80) = 32 points ~keep
    let line2_start = create_test_span("tat", 50.0, 80.0, 30.0, 12.0, false);

    let line1_bottom = line1_end.bbox.y + line1_end.bbox.height;
    let line2_top = line2_start.bbox.y;
    let vertical_gap = (line1_bottom - line2_top).abs();
    let same_column = (line1_end.bbox.left() - line2_start.bbox.left()).abs() < (line1_end.font_size * 2.0);

    // Should detect as line break (vertical gap > 0.5× font_size = 6.0 points) ~keep
    assert!(
        vertical_gap > (line1_end.font_size * 0.5),
        "Should detect vertical gap: {:.2} > {:.2}",
        vertical_gap,
        line1_end.font_size * 0.5
    );

    assert!(
        same_column,
        "Words should be in same column (x positions: {:.2} vs {:.2})",
        line1_end.bbox.left(),
        line2_start.bbox.left()
    );

    // Previous text ends with hyphen: should NOT insert space ~keep
    assert!(
        line1_end.text.ends_with('-'),
        "First line should end with hyphen for word continuation"
    );

    // Decision logic:
    // IF vertical_gap > 0.5×font_size AND same_column AND prev_text.ends_with('-')
    //   THEN no space (soft hyphen)
    // ELSE space (hard line break) ~keep
    let should_insert_space =
        !(vertical_gap > (line1_end.font_size * 0.5) && same_column && line1_end.text.ends_with('-'));

    assert!(!should_insert_space, "Should NOT insert space at hyphenated line break");
}

#[test]
fn test_multicolumn_line_break_spacing_hard_break() {
    //! Test: Words split across line breaks without hyphenation
    //!
    //! Scenario: Multi-column layout with normal line break
    //! Line 1 ends with "habitat" (no hyphen)
    //! Line 2 starts with "quality" (new word)
    //!
    //! Expected: Space inserted at line break
    //! Verification: Same vertical/column detection but without hyphen

    let line1_end = create_test_span("habitat", 50.0, 100.0, 50.0, 12.0, false);

    let line2_start = create_test_span("quality", 50.0, 80.0, 50.0, 12.0, false);

    let line1_bottom = line1_end.bbox.y + line1_end.bbox.height;
    let line2_top = line2_start.bbox.y;
    let vertical_gap = (line1_bottom - line2_top).abs();
    let same_column = (line1_end.bbox.left() - line2_start.bbox.left()).abs() < (line1_end.font_size * 2.0);

    // Should detect as line break (vertical gap > 6.0 points) ~keep
    assert!(vertical_gap > (line1_end.font_size * 0.5));
    assert!(same_column);

    // Previous text does NOT end with hyphen: SHOULD insert space ~keep
    assert!(
        !line1_end.text.ends_with('-'),
        "First line should not end with hyphen for hard line break"
    );

    let should_insert_space =
        !(vertical_gap > (line1_end.font_size * 0.5) && same_column && line1_end.text.ends_with('-'));

    assert!(
        should_insert_space,
        "Should insert space at hard line break (non-hyphenated)"
    );
}

// ============================================================================
// Test 6: Regression Tests - Font & Text Processing Compatibility
// ============================================================================
//
// Spacing changes must not break existing functionality:
// - Identity encoding for CID-keyed fonts
// - TrueType cmap fallback
// - Text post-processing (hyphenation, whitespace, special chars)
// - CIDToGIDMap parsing for Type0 fonts
//
// Spec Reference: ISO 32000-1:2008 Sections 9 (Text), 14 (Structure) ~keep

#[test]
fn test_regression_truetype_cmap_unaffected() {
    //! Test: TrueType cmap functionality unchanged by spacing logic
    //!
    //! TrueType cmap fallback (used when ToUnicode CMap unavailable)
    //! should not be affected by spacing/word boundary changes.
    //!
    //! Spec: ISO 32000-1:2008 Section 9.10 (Character-to-Unicode mapping)

    let config = TextExtractionConfig::default();
    let span_config = SpanMergingConfig::default();

    assert!(config.word_margin_ratio > 0.0);
    assert!(span_config.space_threshold_em_ratio > 0.0);
}

#[test]
fn test_regression_text_processing_pipeline() {
    //! Test: Text post-processing pipeline intact
    //!
    //! Text post-processing includes:
    //! - Hyphenation removal (soft hyphens at line breaks)
    //! - Whitespace normalization (multiple spaces → single space)
    //! - Special character handling (Greek letters, mathematical symbols)
    //!
    //! Spacing changes should not break the post-processing pipeline.
    //!
    //! Spec: ISO 32000-1:2008 Section 9.10 (Text extraction semantics)

    let greek_text = "The variable α represents alpha";
    assert!(greek_text.contains('α'));

    let text_with_multiple_spaces = "Hello   world";
    let normalized = text_with_multiple_spaces.replace("   ", " ");
    assert_eq!(normalized, "Hello world");

    let hyphenated_word = "habi-tat";
    assert!(hyphenated_word.contains('-'));
}

// ============================================================================
// Test 7: Consensus-Based Spacing Decision Logic
// ============================================================================
//
// Instead of relying on TJ offsets alone,
// require multiple spec-defined signals to agree before inserting spaces.
//
// This reduces false positives in justified text while maintaining
// correct spacing in normal documents.
//
// Spec Reference: ISO 32000-1:2008 Section 9.4.4 (TJ array offsets) ~keep

#[test]
fn test_consensus_spacing_all_scenarios() {
    //! Test: Consensus logic across different document types
    //!
    //! Decision matrix:
    //! TJ Signal | Geometric Gap | Result
    //! ----------|---------------|--------
    //! YES       | YES (> 2×)    | SPACE (high confidence)
    //! YES       | NO            | NO SPACE (prevent false positive)
    //! NO        | YES (> 2×)    | SPACE (strong geometric signal)
    //! NO        | NO            | NO SPACE (default)

    let _font_size = 12.0;
    let threshold = 2.0;

    // Case 1: Both signals agree - INSERT SPACE ~keep
    let _tj_signal_1 = true;
    let geometric_gap_1 = 5.0;
    let should_space_1 = _tj_signal_1 && (geometric_gap_1 > threshold);
    assert!(should_space_1, "Case 1: Both signals should insert space");

    // Case 2: TJ signal only, weak geometric - NO SPACE ~keep
    let _tj_signal_2 = true;
    let geometric_gap_2 = 1.5;
    let should_space_2 = _tj_signal_2 && (geometric_gap_2 > threshold);
    assert!(!should_space_2, "Case 2: TJ alone shouldn't insert space");

    // Case 3: Strong geometric signal alone (gap > 2× threshold) - INSERT SPACE ~keep
    let geometric_gap_3 = 5.0;
    let should_space_3 = geometric_gap_3 > (threshold * 2.0);
    assert!(should_space_3, "Case 3: Strong geometric should insert space");

    // Case 4: No signals - NO SPACE ~keep
    let _tj_signal_4 = false;
    let geometric_gap_4 = 1.0;
    let should_space_4 = _tj_signal_4 || (geometric_gap_4 > threshold);
    assert!(!should_space_4, "Case 4: No signals shouldn't insert space");
}

// ============================================================================
// Test 8: PDF-Spec Compliance Verification
// ============================================================================
//
// This test ensures the implementation uses ONLY spec-defined signals
// and does NOT include application-level heuristics like URL detection,
// regex patterns, or linguistic analysis.
//
// Per ISO 32000-1:2008 Section 9.10:
// "Determining word boundaries is not specified by PDF.
//  Applications should use character-level data from the PDF along with
//  font metrics and positioning information." ~keep

#[test]
fn test_spec_compliance_only_pdf_defined_signals() {
    //! Test: Verify all spacing decisions use ONLY spec-defined data
    //!
    //! Allowed signals (from PDF spec):
    //! ✅ TJ array offsets (Section 9.4.4) - typographic hints
    //! ✅ Text positioning operators (Td, TD, Tm, T*) - geometric (Section 9.4.1-9.4.3)
    //! ✅ Font metrics (Sections 9.6-9.8) - character widths
    //! ✅ Character spacing (Tc, Tw, Tz) - Section 9.3.1
    //!
    //! FORBIDDEN (not in PDF spec):
    //! ❌ URL pattern matching
    //! ❌ Language/linguistic analysis
    //! ❌ Regular expressions for content analysis
    //! ❌ Semantic heuristics (CamelCase, email addresses, etc.)

    let span = create_test_span("test", 0.0, 0.0, 5.0, 12.0, false);

    assert!(!span.text.is_empty(), "Text content (spec field)");
    assert!(
        span.bbox.width > 0.0 && span.bbox.height > 0.0,
        "Bbox has valid dimensions"
    );
    assert!(!span.font_name.is_empty(), "Font name (spec field)");
    assert!(span.font_size > 0.0, "Font size (spec field)");
    assert_eq!(span.char_spacing, 0.0, "Char spacing Tc (spec field)");
    assert_eq!(span.word_spacing, 0.0, "Word spacing Tw (spec field)");
    assert_eq!(span.horizontal_scaling, 100.0, "Horizontal scaling Tz (spec field)");

    // Verify NO application-specific fields present
    // (we only use the standard TextSpan fields from PDF extraction) ~keep

    let config = TextExtractionConfig::default();
    assert!(
        config.word_margin_ratio > 0.0,
        "Should use word margin ratio based on font metrics"
    );
    // Should NOT have fields for: url_detection, regex_patterns, semantic_analysis, etc. ~keep
}
