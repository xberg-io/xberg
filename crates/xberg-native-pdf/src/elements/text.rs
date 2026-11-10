//! Text content element types.
//!
//! This module provides the `TextContent` type and related structures
//! for representing text in PDFs.

use crate::extractors::text::ArtifactType;
use crate::geometry::{Point, Rect};
use crate::layout::{Color, FontWeight, TextSpan};

/// Text content that can be extracted from or written to a PDF.
///
/// This is the unified text representation for both reading and writing.
/// Unlike `TextSpan` which is extraction-focused, `TextContent` is designed
/// to work symmetrically for both directions.
#[derive(Debug, Clone, Default)]
pub struct TextContent {
    /// The text string
    pub text: String,
    /// Bounding box of the text
    pub bbox: Rect,
    /// Font specification
    pub font: FontSpec,
    /// Text styling (bold, italic, color, etc.)
    pub style: TextStyle,
    /// Reading order index (for extraction) or write order (for generation)
    pub reading_order: Option<usize>,
    /// Artifact type classification
    pub artifact_type: Option<ArtifactType>,

    /// Baseline origin point (extracted from text matrix)
    pub origin: Option<Point>,
    /// Rotation angle in degrees (0-360)
    pub rotation_degrees: Option<f32>,
    /// Full transformation matrix [a, b, c, d, e, f]
    pub matrix: Option<[f32; 6]>,
}

impl TextContent {
    /// Create a new text content element.
    pub fn new(text: impl Into<String>, bbox: Rect, font: FontSpec, style: TextStyle) -> Self {
        Self {
            text: text.into(),
            bbox,
            font,
            style,
            reading_order: None,
            artifact_type: None,
            origin: None,
            rotation_degrees: None,
            matrix: None,
        }
    }

    /// Create text content with reading order.
    pub fn with_reading_order(mut self, order: usize) -> Self {
        self.reading_order = Some(order);
        self
    }

    /// Set the artifact type.
    pub fn with_artifact_type(mut self, artifact_type: ArtifactType) -> Self {
        self.artifact_type = Some(artifact_type);
        self
    }

    /// Check if this text is bold.
    pub fn is_bold(&self) -> bool {
        self.style.weight.is_bold()
    }

    /// Check if this text is italic.
    pub fn is_italic(&self) -> bool {
        self.style.italic
    }

    /// Get the font size in points.
    pub fn font_size(&self) -> f32 {
        self.font.size
    }

    /// Set the transformation matrix.
    pub fn with_matrix(mut self, matrix: [f32; 6]) -> Self {
        self.matrix = Some(matrix);
        self
    }

    /// Set the origin point.
    pub fn with_origin(mut self, origin: Point) -> Self {
        self.origin = Some(origin);
        self
    }

    /// Set the rotation angle in degrees.
    pub fn with_rotation(mut self, degrees: f32) -> Self {
        self.rotation_degrees = Some(degrees);
        self
    }

    /// Check if this text is rotated (non-zero rotation).
    pub fn is_rotated(&self) -> bool {
        self.rotation_degrees.map(|r| r.abs() > 0.1).unwrap_or(false)
    }

    /// Get rotation angle in radians.
    pub fn rotation_radians(&self) -> Option<f32> {
        self.rotation_degrees.map(|d| d.to_radians())
    }

    /// Get the transformation matrix if available.
    pub fn get_matrix(&self) -> Option<[f32; 6]> {
        self.matrix
    }
}

/// Convert from TextSpan (extraction result) to TextContent (unified representation).
impl From<TextSpan> for TextContent {
    fn from(span: TextSpan) -> Self {
        TextContent {
            text: span.text,
            bbox: span.bbox,
            font: FontSpec {
                name: span.font_name,
                size: span.font_size,
            },
            style: TextStyle {
                weight: span.font_weight,
                italic: span.is_italic,
                color: span.color,
                underline: false,
                strikethrough: false,
            },
            reading_order: Some(span.sequence),
            artifact_type: span.artifact_type,
            origin: None,
            rotation_degrees: None,
            matrix: None,
        }
    }
}

/// Convert from TextContent to TextSpan (for backward compatibility).
impl From<TextContent> for TextSpan {
    fn from(content: TextContent) -> Self {
        TextSpan {
            provenance: None,
            text: content.text,
            bbox: content.bbox,
            font_name: content.font.name,
            font_size: content.font.size,
            font_weight: content.style.weight,
            is_italic: content.style.italic,
            is_monospace: false,
            color: content.style.color,
            mcid: None,
            mcid_scope: None,
            sequence: content.reading_order.unwrap_or(0),
            split_boundary_before: false,
            offset_semantic: false,
            char_spacing: 0.0,
            word_spacing: 0.0,
            horizontal_scaling: 100.0,
            primary_detected: false,
            artifact_type: content.artifact_type,
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
}

/// Font specification for text rendering.
#[derive(Debug, Clone)]
pub struct FontSpec {
    /// Font name (e.g., "Helvetica")
    pub name: String,
    /// Font size in points
    pub size: f32,
}

impl Default for FontSpec {
    fn default() -> Self {
        Self {
            name: "Helvetica".to_string(),
            size: 12.0,
        }
    }
}

impl FontSpec {
    /// Create a new font specification.
    pub fn new(name: impl Into<String>, size: f32) -> Self {
        Self {
            name: name.into(),
            size,
        }
    }

    /// Create a Helvetica font specification.
    pub fn helvetica(size: f32) -> Self {
        Self::new("Helvetica", size)
    }

    /// Create a Times-Roman font specification.
    pub fn times(size: f32) -> Self {
        Self::new("Times-Roman", size)
    }

    /// Create a Courier font specification.
    pub fn courier(size: f32) -> Self {
        Self::new("Courier", size)
    }
}

/// Font style classification.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub enum FontStyle {
    /// Normal (upright) style
    #[default]
    Normal,
    /// Italic style
    Italic,
    /// Oblique style (slanted but not true italic)
    Oblique,
}

/// Text styling information.
#[derive(Debug, Clone, Default)]
pub struct TextStyle {
    /// Font weight (normal, bold, etc.)
    pub weight: FontWeight,
    /// Whether the text is italic
    pub italic: bool,
    /// Text color
    pub color: Color,
    /// Whether the text is underlined
    pub underline: bool,
    /// Whether the text has a strikethrough
    pub strikethrough: bool,
}

impl TextStyle {
    /// Create default text style.
    pub fn new() -> Self {
        Self::default()
    }

    /// Create a bold text style.
    pub fn bold() -> Self {
        Self {
            weight: FontWeight::Bold,
            ..Default::default()
        }
    }

    /// Create an italic text style.
    pub fn italic() -> Self {
        Self {
            italic: true,
            ..Default::default()
        }
    }

    /// Create a bold-italic text style.
    pub fn bold_italic() -> Self {
        Self {
            weight: FontWeight::Bold,
            italic: true,
            ..Default::default()
        }
    }

    /// Set font weight.
    pub fn with_weight(mut self, weight: FontWeight) -> Self {
        self.weight = weight;
        self
    }

    /// Set italic style.
    pub fn with_italic(mut self, italic: bool) -> Self {
        self.italic = italic;
        self
    }

    /// Set text color.
    pub fn with_color(mut self, color: Color) -> Self {
        self.color = color;
        self
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_text_content_creation() {
        let text = TextContent::new(
            "Hello",
            Rect::new(0.0, 0.0, 50.0, 12.0),
            FontSpec::default(),
            TextStyle::default(),
        );

        assert_eq!(text.text, "Hello");
        assert_eq!(text.font_size(), 12.0);
        assert!(!text.is_bold());
        assert!(!text.is_italic());
    }

    #[test]
    fn test_text_span_conversion() {
        let span = TextSpan {
            provenance: None,
            text_rise: 0.0,
            artifact_type: None,
            text: "Test".to_string(),
            bbox: Rect::new(10.0, 20.0, 40.0, 12.0),
            font_name: "Times".to_string(),
            font_size: 12.0,
            font_weight: FontWeight::Bold,
            is_italic: false,
            is_monospace: false,
            color: Color::black(),
            mcid: None,
            mcid_scope: None,
            sequence: 3,
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
        };

        let content: TextContent = span.into();

        assert_eq!(content.text, "Test");
        assert_eq!(content.font.name, "Times");
        assert_eq!(content.font.size, 12.0);
        assert!(content.is_bold());
        assert_eq!(content.reading_order, Some(3));
    }
}
