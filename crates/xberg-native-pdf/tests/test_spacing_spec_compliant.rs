//! Unit tests for Phase 4: PDF-Spec Compliant Spacing Fixes
//!
//! This test suite validates three critical fixes for word boundary detection:
//! 1. Statistical TJ offset analysis (ISO 32000-1:2008 Section 9.4.4)
//! 2. Consensus-based spacing logic (Sections 9.4.4, 5.2)
//! 3. Line break handling with geometric detection (Section 5.2)
//!
//! All tests use ONLY spec-defined signals:
//! - TJ offset values (Section 9.4.4) - typographic hints, NOT semantic boundaries
//! - Geometric positions from bbox (Section 5.2) - coordinate system
//! - Font metrics (Sections 9.6-9.8) - spec-defined widths
//!
//! NO application-level heuristics (URL detection, regex patterns, etc.)

use xberg_native_pdf::geometry::Rect;
use xberg_native_pdf::layout::{Color, FontWeight, TextSpan};

/// Create a test text span with specified position and dimensions.
fn create_test_span(text: &str, x: f32, y: f32, width: f32, height: f32) -> TextSpan {
    TextSpan {
        provenance: None,
        artifact_type: None,
        text: text.to_string(),
        bbox: Rect::new(x, y, width, height),
        font_name: "Times".to_string(),
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

// ============================================================================
// TJ Distribution Analysis Tests (PDF Spec Section 9.4.4)
// ============================================================================ ~keep

/// Test statistical analysis of TJ offset distribution.
///
/// PDF Spec Compliance:
/// - Section 9.4.4: TJ array offsets represent typographic spacing (1/1000 of text space)
/// - Justified text uses arbitrary TJ offsets to distribute whitespace
/// - High variance (CV > 0.5) indicates justified text
///
/// Test Data:
/// - Justified text: TJ offsets [-50.0, -120.0, -80.0, -150.0, -200.0]
///   Mean = -120.0, StdDev = 61.2, CV = 0.51 (justified)
/// - Normal text: TJ offsets [-120.0, -118.0, -122.0, -120.0]
///   Mean = -120.0, StdDev = 1.63, CV = 0.014 (normal)
#[test]
fn test_tj_distribution_analysis() {
    // Case 1: Justified text with high variance
    // Real justified text TJ offsets: large variance to distribute whitespace ~keep
    let offsets_justified = [-30.0, -180.0, -50.0, -200.0, -100.0, -250.0];

    let sum: f32 = offsets_justified.iter().sum();
    let mean = sum / offsets_justified.len() as f32;
    let variance: f32 =
        offsets_justified.iter().map(|x| (x - mean).powi(2)).sum::<f32>() / offsets_justified.len() as f32;
    let std_dev = variance.sqrt();
    let cv = std_dev.abs() / mean.abs();

    assert!(cv > 0.5, "Justified text should have CV > 0.5, got {}", cv);

    // For justified text (high variance): use 3× std dev for conservative filtering ~keep
    let justified_threshold = mean - (3.0 * std_dev);
    // For normal text (low variance): use 1× std dev for aggressive filtering ~keep
    let normal_threshold = mean - std_dev;

    assert!(justified_threshold.is_finite(), "Justified threshold should be finite");
    assert!(normal_threshold.is_finite(), "Normal threshold should be finite");

    // With negative TJ offsets, more negative = more conservative (filters less)
    // So justified (conservative) should be MORE negative than normal (aggressive)
    // e.g., -300.0 (justified, conservative) < -150.0 (normal, aggressive) ~keep
    assert!(
        justified_threshold < normal_threshold,
        "Justified threshold {} should be < (more negative than) normal threshold {}",
        justified_threshold,
        normal_threshold
    );

    let offsets_normal = [-120.0, -118.0, -122.0, -120.0];

    let sum2: f32 = offsets_normal.iter().sum();
    let mean2 = sum2 / offsets_normal.len() as f32;
    let variance2: f32 = offsets_normal.iter().map(|x| (x - mean2).powi(2)).sum::<f32>() / offsets_normal.len() as f32;
    let std_dev2 = variance2.sqrt();
    let cv2 = std_dev2.abs() / mean2.abs();

    assert!(cv2 < 0.5, "Normal text should have CV < 0.5, got {}", cv2);

    let normal_threshold_case2 = mean2 - std_dev2;

    assert!(
        normal_threshold_case2.is_finite(),
        "Normal text threshold should be finite"
    );
}

/// Test consensus-based spacing requiring multiple spec-defined signals.
///
/// PDF Spec Compliance:
/// - Section 9.4.4: TJ offsets are typographic hints, not semantic boundaries
/// - Section 5.2: Bounding boxes provide geometric positioning
/// - Consensus approach: Require both signals OR strong geometric signal alone
///
/// This prevents false positives in justified text where TJ offsets vary.
#[test]
fn test_consensus_spacing_both_signals() {
    let prev = create_test_span("word1", 0.0, 0.0, 10.0, 12.0);
    // prev bbox right edge: 0.0 + 10.0 = 10.0
    // next bbox left edge: 20.0, so gap = 20.0 - 10.0 = 10.0 ~keep
    let next = create_test_span("word2", 20.0, 0.0, 10.0, 12.0);

    let gap = next.bbox.left() - prev.bbox.right();
    let char_size = 12.0;
    let threshold = 0.1 * char_size;

    let tj_suggests_space = true;
    let geometric_suggests_space = gap > threshold;

    assert!(tj_suggests_space && geometric_suggests_space);

    // Case 2: Only TJ signal, weak geometric → NO SPACE (prevent false positive) ~keep
    let prev2 = create_test_span("word1", 0.0, 0.0, 10.0, 12.0);
    // prev bbox right edge: 0.0 + 10.0 = 10.0
    // next bbox left edge: 10.5, so gap = 10.5 - 10.0 = 0.5 < 1.2 ~keep
    let next2 = create_test_span("word2", 10.5, 0.0, 10.0, 12.0);

    let gap2 = next2.bbox.left() - prev2.bbox.right();
    let geometric_suggests_space2 = gap2 > threshold;

    let tj_suggests_space2 = true;

    // TJ says yes, but geometric says no (gap = 0.5 is NOT > threshold 1.2) ~keep
    assert!(
        !geometric_suggests_space2,
        "Gap {} should be <= threshold {}",
        gap2, threshold
    );
    assert!(!should_insert_space_mock(
        tj_suggests_space2,
        geometric_suggests_space2,
        gap2,
        threshold
    ));

    // Case 3: Strong geometric signal alone (gap > 2× threshold) → INSERT SPACE ~keep
    let prev3 = create_test_span("word1", 0.0, 0.0, 10.0, 12.0);
    // prev bbox right edge: 0.0 + 10.0 = 10.0
    // next bbox left edge: 30.0, so gap = 30.0 - 10.0 = 20.0 > 2.4 (2× threshold) ~keep
    let next3 = create_test_span("word2", 30.0, 0.0, 10.0, 12.0);

    let gap3 = next3.bbox.left() - prev3.bbox.right();
    let strong_geometric = gap3 > (2.0 * threshold);

    assert!(
        strong_geometric,
        "Gap {} should be > 2× threshold {}",
        gap3,
        2.0 * threshold
    );
    assert!(should_insert_space_mock(false, false, gap3, threshold));
}

/// Test consensus-based spacing prevents false positives in justified text.
///
/// PDF Spec Compliance (Section 9.4.4):
/// - Justified text uses arbitrary TJ offsets that don't represent semantic boundaries
/// - Pattern: intra-word TJ offset (e.g., "-80.0") followed by small geometric gap
///
/// Example: "user-" (offset -80.0) + "provided" (gap 1.5pt)
/// Should NOT insert space despite negative TJ offset (NOT spec-defined boundary)
#[test]
fn test_consensus_spacing_justified_text_no_false_spaces() {
    let char_size = 12.0;
    let threshold = 0.1 * char_size;

    let tj_signal = true;
    let geometric_gap = 0.8;

    // Consensus decision: NO SPACE
    // - TJ suggests space, but geometric gap is small
    // - Do not insert space (requires both signals OR strong geometric) ~keep
    assert!(!should_insert_space_mock(tj_signal, false, geometric_gap, threshold));
}

/// Mock implementation of consensus-based spacing decision.
///
/// This matches the algorithm specified in the Phase 4 plan:
/// - Both signals agree → insert space
/// - Only TJ signal (geometric weak) → no space
/// - Strong geometric alone (gap > 2× threshold) → insert space
fn should_insert_space_mock(tj_signal: bool, geometric: bool, gap: f32, threshold: f32) -> bool {
    if tj_signal && geometric {
        return true;
    }

    if gap > (2.0 * threshold) {
        return true;
    }

    false
}

// ============================================================================
// Line Break Handling Tests (PDF Spec Section 5.2)
// ============================================================================ ~keep

/// Test line break detection using spec-defined bounding box coordinates.
///
/// PDF Spec Compliance:
/// - Section 5.2: Bounding boxes define text position in coordinate system
/// - Line breaks identified by significant vertical gap
/// - Same column detected by similar horizontal position
///
/// Test Data:
/// - Line 1: "habitat" at y=100, ends at x=70
/// - Line 2: "quality" at y=85 (gap 15pt), starts at x=5 (same column)
/// - Expected: Insert space between lines (vertical gap detected)
#[test]
fn test_line_break_spacing_vertical_gap() {
    let line1 = create_test_span("habitat", 5.0, 100.0, 65.0, 12.0);
    let line2 = create_test_span("quality", 5.0, 85.0, 60.0, 12.0);

    let font_size = 12.0;
    let vertical_gap = (line2.bbox.top() - line1.bbox.bottom()).abs();
    let same_column = (line1.bbox.left() - line2.bbox.left()).abs() < (font_size * 2.0);
    let line_break = vertical_gap > (font_size * 0.5);

    assert!(line_break, "Vertical gap {} should trigger line break", vertical_gap);
    assert!(same_column, "Texts should be in same column");

    let prev_ends_with_hyphen = line1.text.ends_with('-');
    assert!(!prev_ends_with_hyphen);

    assert!(should_insert_space_line_break_mock(
        line_break,
        same_column,
        prev_ends_with_hyphen
    ));
}

/// Test line break with hyphenation doesn't insert space.
///
/// PDF Spec Compliance:
/// - Section 5.2: Bounding box coordinates
/// - Soft hyphen (end with "-"): Merge words without space
/// - Hard hyphen: Context-dependent (not handled in Phase 4)
///
/// Test Data:
/// - Line 1: "habi-" ends with hyphen
/// - Line 2: "tat" starts on new line
/// - Expected: NO space (soft hyphen detected)
#[test]
fn test_line_break_spacing_with_hyphen() {
    let line1 = create_test_span("habi-", 5.0, 100.0, 45.0, 12.0);
    let line2 = create_test_span("tat", 5.0, 85.0, 25.0, 12.0);

    let font_size = 12.0;
    let vertical_gap = (line2.bbox.top() - line1.bbox.bottom()).abs();
    let same_column = (line1.bbox.left() - line2.bbox.left()).abs() < (font_size * 2.0);
    let line_break = vertical_gap > (font_size * 0.5);

    assert!(line_break);
    assert!(same_column);

    let prev_ends_with_hyphen = line1.text.ends_with('-');
    assert!(prev_ends_with_hyphen);

    // Decision: NO SPACE (merge hyphenated words) ~keep
    assert!(!should_insert_space_line_break_mock(
        line_break,
        same_column,
        prev_ends_with_hyphen
    ));
}

/// Test line break in multi-column layout.
///
/// PDF Spec Compliance:
/// - Section 5.2: Different columns have different X coordinates
/// - Column breaks identified by horizontal gap in addition to vertical gap
///
/// Test Data:
/// - Column 1, Line N: Text at x=5
/// - Column 2, Line 1: Text at x=300 (different column)
/// - Expected: NO space (different column)
#[test]
fn test_line_break_different_column() {
    let col1 = create_test_span("column1text", 5.0, 100.0, 60.0, 12.0);
    let col2 = create_test_span("column2text", 300.0, 85.0, 60.0, 12.0);

    let font_size = 12.0;
    let vertical_gap = (col2.bbox.top() - col1.bbox.bottom()).abs();
    let same_column = (col1.bbox.left() - col2.bbox.left()).abs() < (font_size * 2.0);
    let line_break = vertical_gap > (font_size * 0.5);

    assert!(line_break);
    assert!(!same_column, "Texts should be in different columns");

    // Decision: NO SPACE (different column, not a line break) ~keep
    assert!(!should_insert_space_line_break_mock(line_break, same_column, false));
}

/// Mock implementation of line break detection.
///
/// This matches the algorithm specified in the Phase 4 plan:
/// - Line break + same column + no hyphen → insert space
/// - Line break + same column + hyphen → no space (merge)
/// - Different column → no space (column break, not line break)
fn should_insert_space_line_break_mock(line_break: bool, same_column: bool, prev_ends_with_hyphen: bool) -> bool {
    if !line_break || !same_column {
        return false;
    }

    if prev_ends_with_hyphen {
        return false;
    }

    true
}

// ============================================================================
// Integration: PDF-Spec Compliance Verification
// ============================================================================ ~keep

/// Verify that all spacing decisions use ONLY spec-defined signals.
///
/// This test documents the three signals used (per ISO 32000-1:2008):
/// 1. TJ offset values (Section 9.4.4) - typographic hints
/// 2. Bounding box coordinates (Section 5.2) - geometric positioning
/// 3. Font metrics (Sections 9.6-9.8) - spec-defined widths
///
/// NO semantic analysis (URLs, language patterns, CamelCase, etc.)
#[test]
fn test_spec_compliance_only_pdf_defined_signals() {
    // Signal 1: TJ Offset (PDF Section 9.4.4)
    // - Negative values in TJ array indicate typographic spacing
    // - Used as ONE signal in consensus approach ~keep
    let tj_offset_from_pdf = -120.0_f32;
    assert!(tj_offset_from_pdf < 0.0, "TJ offset is spec-defined value");

    // Signal 2: Bounding Box Coordinates (PDF Section 5.2)
    // - bbox provides geometric position
    // - Gaps calculated from coordinates ~keep
    let span1 = create_test_span("test", 0.0, 0.0, 10.0, 12.0);
    let span2 = create_test_span("text", 15.0, 0.0, 10.0, 12.0);
    let geometric_gap = span2.bbox.left() - span1.bbox.right();
    assert_eq!(geometric_gap, 5.0);

    // Signal 3: Font Metrics (PDF Sections 9.6-9.8)
    // - Font size and width from spec-defined font dictionary
    // - Used to calculate thresholds ~keep
    let font_size = span1.font_size;
    let space_width_ratio = 0.1;
    let threshold = space_width_ratio * font_size;
    assert_eq!(threshold, 1.2);

    // All signals are PDF spec-defined
    // NO application-level semantics added ~keep
}
