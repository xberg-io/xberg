//! Conversion helpers from layout-detection bounding boxes to `LayoutHint`s.
//!
//! Two coordinate-space conversions live here, one per consumer:
//!
//! * **OCR path** — `detection_to_layout_hints_pixel_space`: paragraphs that
//!   reach `apply_layout_overrides` come from `ocr_doc_to_paragraphs`, which
//!   keeps everything in rendered pixel space. The hint conversion only
//!   Y-flips (image y=0 at top → PDF y=0 at bottom) without scaling.
//!
//! * **Markdown layout-for-markdown path** —
//!   `pixel_detection_to_layout_hints_pdf_space`: paragraphs come from oxide
//!   text extraction in PDF point space. The hint conversion must scale x
//!   and y from rendered pixel dimensions to PDF point dimensions AND
//!   Y-flip.
//!
//! Mixing the two — e.g., feeding the OCR helper into the markdown path —
//! produces hint bboxes that don't overlap with paragraphs, so
//! `apply_layout_overrides` becomes a silent no-op (the bug fixed by this
//! module's introduction).

#![cfg(feature = "layout-detection")]

use crate::layout::{DetectionResult, LayoutClass};
use crate::pdf::structure::types::{LayoutHint, LayoutHintClass};

/// Map a model-emitted `LayoutClass` to the simplified `LayoutHintClass`
/// consumed by `apply_layout_overrides`.
fn map_class(class: LayoutClass) -> LayoutHintClass {
    match class {
        LayoutClass::Title => LayoutHintClass::Title,
        LayoutClass::SectionHeader => LayoutHintClass::SectionHeader,
        LayoutClass::Code => LayoutHintClass::Code,
        LayoutClass::Formula => LayoutHintClass::Formula,
        LayoutClass::ListItem => LayoutHintClass::ListItem,
        LayoutClass::Caption => LayoutHintClass::Caption,
        LayoutClass::Footnote => LayoutHintClass::Footnote,
        LayoutClass::PageHeader => LayoutHintClass::PageHeader,
        LayoutClass::PageFooter => LayoutHintClass::PageFooter,
        LayoutClass::Table => LayoutHintClass::Table,
        LayoutClass::Picture => LayoutHintClass::Picture,
        LayoutClass::Text => LayoutHintClass::Text,
        _ => LayoutHintClass::Other,
    }
}

/// Convert pixel-space layout detections to PDF-point-space `LayoutHint`s.
///
/// Detections come back from RT-DETR / YOLO in **rendered image pixel
/// space** (origin at top-left, units = pixels at the rendering DPI).
/// Paragraphs in the layout-for-markdown path reach
/// `apply_layout_overrides` in **PDF point space** (origin at bottom-left,
/// units = points). This function bridges the two by:
///
/// 1. Scaling x by `page_width_pts / image_width_px`.
/// 2. Scaling y by `page_height_pts / image_height_px`.
/// 3. Flipping y so the PDF-space y axis grows upward.
///
/// `image_width_px` / `image_height_px` are the actual rendered image
/// dimensions for the page, **not** the model's internal grid (e.g., 640×640).
/// Pass 1 as the minimum to avoid division by zero on degenerate inputs.
#[cfg(feature = "layout-detection")]
pub(crate) fn pixel_detection_to_layout_hints_pdf_space(
    detection: &DetectionResult,
    image_width_px: u32,
    image_height_px: u32,
    page_width_pts: f32,
    page_height_pts: f32,
) -> Vec<LayoutHint> {
    let sx = page_width_pts / image_width_px.max(1) as f32;
    let sy = page_height_pts / image_height_px.max(1) as f32;
    detection
        .detections
        .iter()
        .map(|det| LayoutHint {
            class_name: map_class(det.class_name),
            confidence: det.confidence,
            left: det.bbox.x1 * sx,
            right: det.bbox.x2 * sx,
            top: page_height_pts - det.bbox.y1 * sy,
            bottom: page_height_pts - det.bbox.y2 * sy,
        })
        .collect()
}

/// Convert pixel-space layout detections to `LayoutHint`s for the **OCR
/// path**, where paragraphs are also in pixel space.
///
/// Y-flip only — no x/y scaling — because the caller passes
/// `page_height_px` as the rendered pixel height matching the OCR's own
/// pixel-space paragraphs.
#[cfg(all(feature = "ocr", feature = "layout-detection"))]
pub(crate) fn detection_to_layout_hints_pixel_space(
    detection: &DetectionResult,
    page_height_px: f32,
) -> Vec<LayoutHint> {
    detection
        .detections
        .iter()
        .map(|det| LayoutHint {
            class_name: map_class(det.class_name),
            confidence: det.confidence,
            left: det.bbox.x1,
            right: det.bbox.x2,
            top: page_height_px - det.bbox.y1,
            bottom: page_height_px - det.bbox.y2,
        })
        .collect()
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::layout::types::{BBox, LayoutDetection};

    fn detection_at(class: LayoutClass, x1: f32, y1: f32, x2: f32, y2: f32) -> LayoutDetection {
        LayoutDetection::new(class, 0.9, BBox::new(x1, y1, x2, y2))
    }

    /// A4 page rendered at 150 DPI: 595×842 pt → 1240×1754 px.
    /// A heading detection in the top-left of the rendered image
    /// (pixel y=100..200) must map to a hint near the *top* of the
    /// PDF-space page (PDF y near page_height_pts).
    #[test]
    fn pixel_to_pdf_scales_and_flips_y_for_a4_at_150dpi() {
        let det = DetectionResult::new(
            1240,
            1754,
            vec![detection_at(LayoutClass::SectionHeader, 124.0, 100.0, 1116.0, 200.0)],
        );
        let hints = pixel_detection_to_layout_hints_pdf_space(&det, 1240, 1754, 595.0, 842.0);

        assert_eq!(hints.len(), 1);
        let h = &hints[0];

        // Scale factors: sx = 595/1240 ≈ 0.47984, sy = 842/1754 ≈ 0.48005.
        let sx = 595.0_f32 / 1240.0;
        let sy = 842.0_f32 / 1754.0;

        // x is scaled but not flipped.
        assert!((h.left - 124.0 * sx).abs() < 0.01, "left scaling: got {}", h.left);
        assert!((h.right - 1116.0 * sx).abs() < 0.01, "right scaling: got {}", h.right);

        // y is scaled AND flipped: image_y → page_height_pts - image_y * sy.
        let expected_top = 842.0 - 100.0 * sy;
        let expected_bottom = 842.0 - 200.0 * sy;
        assert!(
            (h.top - expected_top).abs() < 0.01,
            "top scale+flip: got {}, want {}",
            h.top,
            expected_top
        );
        assert!(
            (h.bottom - expected_bottom).abs() < 0.01,
            "bottom scale+flip: got {}, want {}",
            h.bottom,
            expected_bottom
        );

        // Sanity: a heading in the top of the image should land near the top
        // of the PDF page (large y in PDF-space), not near the bottom.
        assert!(
            h.top > 700.0,
            "heading at image y=100 should map to PDF y near top (≈ {}), got {}",
            expected_top,
            h.top
        );
    }

    /// Identical pixel and point dimensions → only y-flip happens, no scaling.
    #[test]
    fn pixel_to_pdf_no_scaling_when_dims_equal() {
        let det = DetectionResult::new(
            612,
            792,
            vec![detection_at(LayoutClass::Title, 50.0, 50.0, 562.0, 100.0)],
        );
        let hints = pixel_detection_to_layout_hints_pdf_space(&det, 612, 792, 612.0, 792.0);

        assert_eq!(hints.len(), 1);
        let h = &hints[0];
        assert!((h.left - 50.0).abs() < 0.001);
        assert!((h.right - 562.0).abs() < 0.001);
        assert!((h.top - (792.0 - 50.0)).abs() < 0.001);
        assert!((h.bottom - (792.0 - 100.0)).abs() < 0.001);
    }

    /// Class mapping must preserve heading-like classes for downstream
    /// classification (`apply_layout_overrides` only acts on specific classes).
    #[test]
    fn class_mapping_preserves_heading_classes() {
        let det = DetectionResult::new(
            100,
            100,
            vec![
                detection_at(LayoutClass::Title, 0.0, 0.0, 50.0, 10.0),
                detection_at(LayoutClass::SectionHeader, 0.0, 20.0, 50.0, 30.0),
            ],
        );
        let hints = pixel_detection_to_layout_hints_pdf_space(&det, 100, 100, 100.0, 100.0);
        assert_eq!(hints.len(), 2);
        assert_eq!(hints[0].class_name, LayoutHintClass::Title);
        assert_eq!(hints[1].class_name, LayoutHintClass::SectionHeader);
    }

    /// Degenerate input: image_width_px=0 must not panic via division-by-zero
    /// (we clamp to 1 internally). The output bbox values are meaningless in
    /// this case but the function must return cleanly.
    #[test]
    fn pixel_to_pdf_zero_dim_does_not_panic() {
        let det = DetectionResult::new(0, 0, vec![detection_at(LayoutClass::Text, 0.0, 0.0, 10.0, 10.0)]);
        let _ = pixel_detection_to_layout_hints_pdf_space(&det, 0, 0, 595.0, 842.0);
    }
}
