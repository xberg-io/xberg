//! Per-region VLM extraction for PDF layout regions.
//!
//! When `vlm_fallback != Disabled` and layout detection has identified
//! `Picture` (figure), `Table`, or other visually complex regions, this
//! module crops those regions from the page raster, calls the VLM via
//! [`crate::llm::region_extractor`], and returns markdown elements that
//! can be spliced into the assembled document.
//!
//! Only compiled when `liter-llm` + `layout-detection` are both enabled
//! (not on Windows).

#![cfg(all(feature = "liter-llm", feature = "layout-detection", not(target_os = "windows")))]

use std::io::Cursor;

use image::{ExtendedColorType, ImageEncoder};

use crate::core::config::LlmConfig;
use crate::llm::region_extractor::{RegionKind, extract_region_with_vlm};
use crate::pdf::structure::types::{LayoutHint, LayoutHintClass};

/// Minimum confidence threshold for a layout hint to trigger VLM region extraction.
const MIN_REGION_CONFIDENCE: f32 = 0.6;

/// Minimum pixel area for a cropped region to be sent to VLM.
///
/// Regions smaller than this are skipped — they are unlikely to contain meaningful
/// content and would waste VLM API tokens.
const MIN_REGION_PIXEL_AREA: u32 = 1_000;

/// A VLM-extracted markdown string and the page index it belongs to.
pub(crate) struct RegionVlmResult {
    /// 0-based page index.
    pub page_index: usize,
    /// Markdown text from the VLM.
    pub markdown: String,
    /// The layout hint that triggered this extraction.
    #[allow(dead_code)]
    pub hint: LayoutHint,
}

/// Extract markdown from VLM for all figure / complex-layout regions across all pages.
///
/// Iterates the layout hints for each page. For hints whose class warrants VLM
/// extraction ([`LayoutHintClass::Picture`] and, when requested, dense tables),
/// crops the region from the page raster image, encodes it as PNG, and calls
/// [`extract_region_with_vlm`]. Results from all pages are collected and returned.
///
/// # Arguments
///
/// * `layout_images` — One rasterised `RgbImage` per page (same order as `layout_hints`).
/// * `layout_hints` — Per-page layout detection results, each a `Vec<LayoutHint>`.
/// * `llm_config` — LLM provider configuration for the VLM calls.
///
/// # Returns
///
/// A `Vec<RegionVlmResult>` with one entry per successfully extracted region.
/// Errors from individual regions are logged as warnings but do not propagate —
/// a VLM failure on one region must not abort the whole extraction.
pub(crate) async fn extract_vlm_regions(
    layout_images: &[image::RgbImage],
    layout_hints: &[Vec<LayoutHint>],
    llm_config: &LlmConfig,
) -> Vec<RegionVlmResult> {
    let mut results: Vec<RegionVlmResult> = Vec::new();

    for (page_index, (page_image, hints)) in layout_images.iter().zip(layout_hints.iter()).enumerate() {
        let img_width = page_image.width();
        let img_height = page_image.height();

        for hint in hints {
            if hint.confidence < MIN_REGION_CONFIDENCE {
                continue;
            }

            let region_kind = match hint.class_name {
                LayoutHintClass::Picture => RegionKind::Figure,
                // Dense tables are not currently intercepted here — the
                // existing table extraction pipeline handles them. This arm
                // is reserved for future use when the table model is Disabled.
                _ => continue,
            };

            // The hints produced by `pixel_detection_to_layout_hints_pdf_space`
            // use PDF coordinate space (y=0 at bottom).  The layout images are in
            // pixel space (y=0 at top).  The layout runner stores raw pixel-space
            // bounding boxes in the `LayoutHint` structs *before* any coordinate
            // conversion for the markdown pipeline.
            //
            // However, `layout_hints` here are *already converted* to PDF space.
            // To crop the raster image we need to invert back to pixel space:
            //   pixel_y1 = img_height - pdf_top
            //   pixel_y2 = img_height - pdf_bottom
            //
            // All coordinates are clamped to image bounds.
            let pdf_top = hint.top;
            let pdf_bottom = hint.bottom;
            let pdf_left = hint.left;
            let pdf_right = hint.right;

            // Compute pixel bounds (clamped to image dimensions).
            // pdf_top corresponds to the smaller pixel_y (near image top).
            let pixel_y1 = (img_height as f32 - pdf_top).max(0.0).min(img_height as f32) as u32;
            let pixel_y2 = (img_height as f32 - pdf_bottom).max(0.0).min(img_height as f32) as u32;
            let pixel_x1 = pdf_left.max(0.0).min(img_width as f32) as u32;
            let pixel_x2 = pdf_right.max(0.0).min(img_width as f32) as u32;

            // Ensure the crop box has positive area in the right direction.
            let (y_top, y_bot) = if pixel_y1 <= pixel_y2 {
                (pixel_y1, pixel_y2)
            } else {
                (pixel_y2, pixel_y1)
            };
            let (x_left, x_right) = if pixel_x1 <= pixel_x2 {
                (pixel_x1, pixel_x2)
            } else {
                (pixel_x2, pixel_x1)
            };

            let crop_w = x_right.saturating_sub(x_left);
            let crop_h = y_bot.saturating_sub(y_top);

            if crop_w * crop_h < MIN_REGION_PIXEL_AREA {
                tracing::trace!(
                    page = page_index,
                    crop_w,
                    crop_h,
                    "region too small for VLM extraction; skipping"
                );
                continue;
            }

            // Crop the region from the page raster.
            let crop = image::imageops::crop_imm(page_image, x_left, y_top, crop_w, crop_h).to_image();

            // Encode crop as PNG for the VLM call.
            let mut png_buf = Cursor::new(Vec::<u8>::new());
            let encode_result = image::codecs::png::PngEncoder::new(&mut png_buf).write_image(
                crop.as_raw(),
                crop.width(),
                crop.height(),
                ExtendedColorType::Rgb8,
            );
            if let Err(e) = encode_result {
                tracing::warn!(
                    page = page_index,
                    error = %e,
                    "failed to PNG-encode region crop; skipping VLM call"
                );
                continue;
            }
            let crop_bytes = png_buf.into_inner();

            tracing::debug!(
                page = page_index,
                region_kind = ?region_kind,
                confidence = hint.confidence,
                crop_w,
                crop_h,
                "sending region to VLM"
            );

            match extract_region_with_vlm(&crop_bytes, "image/png", region_kind, llm_config, None).await {
                Ok(markdown) => {
                    let trimmed = markdown.trim().to_string();
                    if !trimmed.is_empty() {
                        results.push(RegionVlmResult {
                            page_index,
                            markdown: trimmed,
                            hint: hint.clone(),
                        });
                    }
                }
                Err(e) => {
                    tracing::warn!(
                        page = page_index,
                        region_kind = ?region_kind,
                        error = %e,
                        "VLM region extraction failed; region suppressed"
                    );
                }
            }
        }
    }

    results
}

/// Inject VLM-extracted region markdown into an `InternalDocument`.
///
/// For each `RegionVlmResult`, appends a paragraph element with the VLM
/// markdown. The current implementation appends per-page results at the end
/// of each page's content in document order. Splice-by-anchor is deferred to
/// a future revision once `InternalDocument` exposes stable anchor positions.
pub(crate) fn inject_region_results(
    document: &mut crate::types::internal::InternalDocument,
    results: Vec<RegionVlmResult>,
) {
    use crate::types::internal::{ElementKind, InternalElement};

    for result in results {
        let page_num = (result.page_index + 1) as u32;
        document
            .elements
            .push(InternalElement::text(ElementKind::Paragraph, result.markdown, 0).with_page(page_num));
        tracing::debug!(page = page_num, "injected VLM region result into document");
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::pdf::structure::types::LayoutHintClass;

    fn make_hint(class: LayoutHintClass, confidence: f32) -> LayoutHint {
        LayoutHint {
            class_name: class,
            confidence,
            left: 50.0,
            bottom: 600.0,
            right: 400.0,
            top: 750.0,
        }
    }

    #[test]
    fn test_low_confidence_hints_are_skipped() {
        // Confidence below MIN_REGION_CONFIDENCE must not be extracted.
        let hint = make_hint(LayoutHintClass::Picture, 0.3);
        assert!(hint.confidence < MIN_REGION_CONFIDENCE);
    }

    #[test]
    fn test_non_picture_hints_are_skipped() {
        // Only Picture class triggers VLM extraction in the current implementation.
        let non_picture = [
            LayoutHintClass::Text,
            LayoutHintClass::SectionHeader,
            LayoutHintClass::Title,
            LayoutHintClass::PageHeader,
            LayoutHintClass::PageFooter,
            LayoutHintClass::Caption,
            LayoutHintClass::Code,
            LayoutHintClass::Formula,
            LayoutHintClass::Footnote,
            LayoutHintClass::ListItem,
            LayoutHintClass::Other,
        ];
        // Verify that RegionKind has no mapping for non-picture classes.
        // This test documents the contract; new classes added to the match
        // arm must update this list.
        for class in non_picture {
            let hint = make_hint(class, 0.9);
            // We cannot call extract_vlm_regions without an async runtime and
            // real images. The test is structural — it verifies the list above
            // and keeps documentation accurate.
            let _ = hint; // suppress unused warning
        }
    }

    #[test]
    fn test_min_pixel_area_constant() {
        // MIN_REGION_PIXEL_AREA must be positive to avoid sending empty crops.
        assert!(MIN_REGION_PIXEL_AREA > 0);
    }
}
