//! Shared data types for PDF hierarchy extraction (backend-agnostic).

use super::bounding_box::BoundingBox;

/// A block of text with spatial and semantic information.
#[derive(Debug, Clone, PartialEq)]
pub struct TextBlock {
    /// The text content
    pub text: String,
    /// The bounding box of the block
    pub bbox: BoundingBox,
    /// The font size of the text in this block
    pub font_size: f32,
}

/// Text segment data extracted from PDF.
///
/// Backend-agnostic: populated by either the pdf_oxide or another extractor.
#[derive(Debug, Clone)]
pub struct SegmentData {
    /// The segment text content (may contain spaces / multiple words)
    pub text: String,
    /// Left x position in PDF units
    pub x: f32,
    /// Bottom y position in PDF units (PDF coordinate system, y=0 at bottom)
    pub y: f32,
    /// Width of the segment bounding box
    pub width: f32,
    /// Height of the segment bounding box
    pub height: f32,
    /// Font size in points
    pub font_size: f32,
    /// Whether the font is bold
    pub is_bold: bool,
    /// Whether the font is italic
    pub is_italic: bool,
    /// Whether the font is monospace
    pub is_monospace: bool,
    /// Baseline Y position
    pub baseline_y: f32,
    /// Pre-assigned heading level from the PDF structure tree (1-6), or `None`
    /// when the heading level is unknown and must be inferred via font-size clustering.
    pub assigned_role: Option<u8>,
}
