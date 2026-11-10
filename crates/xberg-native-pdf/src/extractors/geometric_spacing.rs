//! Geometric space detection - pdfplumber-style single rule.
//!
//! This module implements position-based space detection without heuristics,
//! confidence scoring, or document-type awareness. The algorithm is:
//!
//! ```text
//! insert_space = (prev.x1 + margin) < next.x0
//! where margin = word_margin * max(prev.width, prev.height)
//! ```
//!
//! This matches pdfplumber and pdfminer.six LAParams approach, achieving
//! 0 spurious spaces on policy documents and maintaining quality on academic papers.
//!
//! Reference: pdfplumber (<https://github.com/jsvine/pdfplumber>)
//! Reference: pdfminer.six LAParams (word_margin parameter)

use crate::layout::TextSpan;

/// Result of geometric space detection.
///
/// Per pdfplumber architecture: spaces are determined purely by position.
/// No confidence scoring, no heuristics, no document-type awareness.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct SpaceInsertion {
    /// Whether to insert a space between characters/spans
    pub insert: bool,
}

impl SpaceInsertion {
    /// Create a decision to insert a space.
    #[inline]
    pub const fn yes() -> Self {
        Self { insert: true }
    }

    /// Create a decision to not insert a space.
    #[inline]
    pub const fn no() -> Self {
        Self { insert: false }
    }
}

/// Configuration for geometric space detection.
///
/// Matches pdfplumber/pdfminer.six LAParams approach. This single parameter
/// replaces the previous 5+ threshold parameters in the old system.
#[derive(Debug, Clone, Copy)]
pub struct SpacingConfig {
    /// Word margin as ratio of character size.
    ///
    /// Default: 0.1 (matches pdfminer.six default)
    ///
    /// - Lower values (0.05): More spaces inserted, catches tight kerning
    /// - Higher values (0.15): Fewer spaces, more conservative
    ///
    /// The actual margin threshold is calculated as:
    /// ```text
    /// margin = word_margin * max(prev_width, prev_height)
    /// ```
    pub word_margin: f32,
}

impl Default for SpacingConfig {
    fn default() -> Self {
        Self { word_margin: 0.1 }
    }
}

impl SpacingConfig {
    /// Create configuration for tight spacing (policy documents).
    ///
    /// Uses lower word_margin to avoid inserting spurious spaces
    /// in tightly-spaced text.
    pub fn tight() -> Self {
        Self { word_margin: 0.05 }
    }

    /// Create configuration for loose spacing (academic papers).
    ///
    /// Uses higher word_margin to properly detect word boundaries
    /// in loosely-spaced text.
    pub fn loose() -> Self {
        Self { word_margin: 0.15 }
    }
}

/// Determine if space should be inserted between consecutive spans.
///
/// Uses pdfplumber's single geometric rule: if the gap between spans
/// exceeds a margin relative to character size, insert a space.
///
/// # Algorithm
///
/// 1. Check if boundary already has whitespace (Rule 0: skip if so)
/// 2. Calculate gap: `gap = next.x0 - prev.x1`
/// 3. Calculate relative margin: `margin = word_margin * max(prev.width, prev.height)`
/// 4. Insert space if: `gap > margin`
///
/// # Arguments
///
/// * `prev` - Previous span in reading order
/// * `next` - Next span in reading order
/// * `config` - Spacing configuration
///
/// # Returns
///
/// `SpaceInsertion` indicating whether to insert a space.
pub fn should_insert_space(prev: &TextSpan, next: &TextSpan, config: &SpacingConfig) -> SpaceInsertion {
    if has_boundary_whitespace(&prev.text, &next.text) {
        return SpaceInsertion::no();
    }

    let prev_right = prev.bbox.right();
    let next_left = next.bbox.left();
    let gap = next_left - prev_right;

    let char_size = prev.bbox.width.max(prev.bbox.height);
    let margin = config.word_margin * char_size;

    if gap > margin {
        SpaceInsertion::yes()
    } else {
        SpaceInsertion::no()
    }
}

/// Check if boundary between texts already has whitespace.
#[inline]
fn has_boundary_whitespace(prev: &str, next: &str) -> bool {
    prev.chars().last().is_some_and(|c| c.is_whitespace()) || next.chars().next().is_some_and(|c| c.is_whitespace())
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::geometry::Rect;
    use crate::layout::{Color, FontWeight};

    fn make_span(text: &str, x: f32, width: f32) -> TextSpan {
        TextSpan {
            provenance: None,
            text_rise: 0.0,
            artifact_type: None,
            text: text.to_string(),
            bbox: Rect::new(x, 0.0, width, 12.0),
            font_name: "Arial".to_string(),
            font_size: 12.0,
            font_weight: FontWeight::Normal,
            is_italic: false,
            is_monospace: false,
            color: Color::new(0.0, 0.0, 0.0),
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
        }
    }

    #[test]
    fn test_clear_word_gap() {
        let prev = make_span("Hello", 0.0, 30.0);
        let next = make_span("World", 40.0, 30.0); // 10pt gap ~keep
        let config = SpacingConfig::default(); // margin = 0.1*30 = 3pt ~keep

        // gap (10pt) > margin (3pt) -> insert ~keep
        assert_eq!(should_insert_space(&prev, &next, &config), SpaceInsertion::yes());
    }

    #[test]
    fn test_tight_kerning() {
        let prev = make_span("Hel", 0.0, 20.0);
        let next = make_span("lo", 21.0, 10.0); // 1pt gap ~keep
        let config = SpacingConfig::default(); // margin = 0.1*20 = 2pt ~keep

        // gap (1pt) < margin (2pt) -> no insert ~keep
        assert_eq!(should_insert_space(&prev, &next, &config), SpaceInsertion::no());
    }

    #[test]
    fn test_existing_boundary_space() {
        let prev = make_span("Hello ", 0.0, 30.0);
        let next = make_span("World", 35.0, 30.0);
        let config = SpacingConfig::default();

        assert_eq!(should_insert_space(&prev, &next, &config), SpaceInsertion::no());
    }

    #[test]
    fn test_word_margin_variations() {
        let tight = SpacingConfig { word_margin: 0.05 };
        let loose = SpacingConfig { word_margin: 0.15 };

        let prev = make_span("Hello", 0.0, 30.0);
        let next = make_span("World", 33.0, 30.0); // 3pt gap ~keep

        // tight: margin = 0.05*30 = 1.5pt, gap > margin -> insert ~keep
        assert_eq!(should_insert_space(&prev, &next, &tight), SpaceInsertion::yes());

        // loose: margin = 0.15*30 = 4.5pt, gap < margin -> no insert ~keep
        assert_eq!(should_insert_space(&prev, &next, &loose), SpaceInsertion::no());
    }

    #[test]
    fn test_exactly_at_margin() {
        let prev = make_span("Hello", 0.0, 20.0);
        let config = SpacingConfig::default(); // margin = 0.1*20 = 2pt ~keep

        // gap = margin (exactly 2.0) -> no insert (must be strictly greater) ~keep
        let next = make_span("World", 22.0, 20.0);
        assert_eq!(should_insert_space(&prev, &next, &config), SpaceInsertion::no());

        // gap > margin (2.1 > 2.0) -> insert ~keep
        let next = make_span("World", 22.1, 20.0);
        assert_eq!(should_insert_space(&prev, &next, &config), SpaceInsertion::yes());
    }

    #[test]
    fn test_leading_space_in_next() {
        let prev = make_span("Hello", 0.0, 30.0);
        let next = make_span(" World", 40.0, 30.0);
        let config = SpacingConfig::default();

        assert_eq!(should_insert_space(&prev, &next, &config), SpaceInsertion::no());
    }
}
