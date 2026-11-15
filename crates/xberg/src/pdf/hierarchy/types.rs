//! Shared data types for PDF hierarchy extraction (backend-agnostic).

use super::bounding_box::BoundingBox;

/// A block of text with spatial and semantic information.
#[cfg_attr(alef, alef(skip))]
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
/// Backend-agnostic: populated by either the xberg_native_pdf or another extractor.
#[cfg_attr(alef, alef(skip))]
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
    /// Text-matrix rotation reported by the PDF backend, in degrees.
    ///
    /// The backend's original geometry remains in `x`, `y`, `width`, and
    /// `height` for layout/table projection; consumers doing reading-order or
    /// spacing arithmetic must use the upright-frame helpers below.
    pub rotation_degrees: f32,
    /// Pre-assigned heading level from the PDF structure tree (1-6), or `None`
    /// when the heading level is unknown and must be inferred via font-size clustering.
    pub assigned_role: Option<u8>,
    pub(crate) ocr_heading_spans: Option<Box<[crate::types::internal::OcrSourceSpan]>>,
}

impl SegmentData {
    /// Whether the segment is painted on the page's upright text axis.
    pub(crate) fn is_unrotated(&self) -> bool {
        self.rotation_degrees.abs() <= f32::EPSILON
    }

    /// Whether two segments share a reading frame.
    pub(crate) fn has_same_rotation(&self, other: &Self) -> bool {
        (self.rotation_degrees - other.rotation_degrees).abs() <= f32::EPSILON
    }

    /// Page-space origin rotated into this segment's upright reading frame.
    ///
    /// Returns `(advance, cross)`, where advance follows the baseline and cross
    /// follows the direction in which visual lines stack.
    pub(crate) fn upright_origin(&self) -> (f32, f32) {
        if self.is_unrotated() {
            return (self.x, self.y);
        }
        let (sin, cos) = (-self.rotation_degrees).to_radians().sin_cos();
        (self.x * cos - self.y * sin, self.x * sin + self.y * cos)
    }

    /// `(start, end)` along this segment's reading direction.
    pub(crate) fn upright_advance_extent(&self) -> (f32, f32) {
        let (start, _) = self.upright_origin();
        (start, start + self.width)
    }

    /// `(low, high)` along the axis on which visual lines stack.
    pub(crate) fn upright_cross_extent(&self) -> (f32, f32) {
        let (_, low) = self.upright_origin();
        (low, low + self.height)
    }

    /// Baseline coordinate in this segment's upright reading frame.
    pub(crate) fn upright_baseline(&self) -> f32 {
        if self.is_unrotated() {
            self.baseline_y
        } else {
            self.upright_origin().1
        }
    }
}

#[cfg(test)]
mod tests {
    use super::SegmentData;

    fn segment(rotation_degrees: f32) -> SegmentData {
        SegmentData {
            text: "x".to_string(),
            x: 100.0,
            y: 700.0,
            width: 40.0,
            height: 10.0,
            font_size: 10.0,
            is_bold: false,
            is_italic: false,
            is_monospace: false,
            baseline_y: 700.0,
            rotation_degrees,
            assigned_role: None,
            ocr_heading_spans: None,
        }
    }

    #[test]
    fn should_leave_unrotated_segment_geometry_unchanged() {
        let segment = segment(0.0);

        assert_eq!(segment.upright_origin(), (100.0, 700.0));
        assert_eq!(segment.upright_advance_extent(), (100.0, 140.0));
        assert_eq!(segment.upright_cross_extent(), (700.0, 710.0));
    }

    #[test]
    fn should_rotate_ninety_degree_segment_into_its_reading_frame() {
        let segment = segment(90.0);

        let (advance, cross) = segment.upright_origin();
        assert!((advance - 700.0).abs() < 1e-3, "advance axis was {advance}");
        assert!((cross + 100.0).abs() < 1e-3, "cross axis was {cross}");
        let (start, end) = segment.upright_advance_extent();
        assert!((start - 700.0).abs() < 1e-3 && (end - 740.0).abs() < 1e-3);
    }
}
