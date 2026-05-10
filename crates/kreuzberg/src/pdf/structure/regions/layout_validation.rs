//! Pixel-level validation of layout predictions using connected component analysis.
//!
//! Validates Table/Picture regions by checking whether the predicted region
//! actually contains text-like content in the rendered page image. Regions
//! with very few connected components are flagged as "empty" — segments
//! overlapping them should not be suppressed.

/// Result of validating a single layout region.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
#[allow(dead_code)] // Variants used when layout-detection feature is enabled
pub(in crate::pdf::structure) enum RegionValidation {
    /// Region contains text-like connected components; suppress normally.
    HasContent,
    /// Region has very few text CCs; likely a false positive — don't suppress.
    Empty,
    /// Validation was skipped (not a suppressible class, or error).
    Skipped,
}

/// Minimum connected components for a region to be considered text-bearing.
#[cfg(all(feature = "layout-detection", feature = "ocr"))]
const MIN_TEXT_CC_COUNT: i32 = 3;

/// Validate whether a layout region contains text by analyzing the rendered
/// page image with leptonica's connected component analysis.
///
/// Crops the region from the page image, binarizes, counts CCs. If the
/// count is below `MIN_TEXT_CC_COUNT`, the region is flagged as Empty.
///
/// `region_x/y/w/h` are in pixel coordinates (image space, y=0 at top).
///
/// Requires the `ocr` feature (leptonica via kreuzberg-tesseract). Returns
/// `Skipped` unconditionally when `ocr` is not enabled.
#[cfg(all(feature = "layout-detection", feature = "ocr"))]
pub(in crate::pdf::structure) fn validate_region_has_text(
    page_rgb: &[u8],
    page_width: u32,
    page_height: u32,
    region_x: u32,
    region_y: u32,
    region_w: u32,
    region_h: u32,
) -> RegionValidation {
    // Skip tiny regions
    if region_w < 5 || region_h < 5 {
        return RegionValidation::Empty;
    }

    // Build a Pix from just the cropped region (avoid building full-page Pix)
    // Extract the RGB sub-rectangle manually
    let mut crop_data = Vec::with_capacity((region_w * region_h * 3) as usize);
    for row in region_y..(region_y + region_h).min(page_height) {
        let row_start = (row * page_width + region_x) as usize * 3;
        let row_end = (row * page_width + (region_x + region_w).min(page_width)) as usize * 3;
        if row_end <= page_rgb.len() && row_start <= page_rgb.len() {
            crop_data.extend_from_slice(&page_rgb[row_start..row_end]);
        }
    }

    let actual_w = region_w.min(page_width.saturating_sub(region_x));
    let actual_h = region_h.min(page_height.saturating_sub(region_y));
    if actual_w < 5 || actual_h < 5 || crop_data.len() != (actual_w * actual_h * 3) as usize {
        return RegionValidation::Skipped;
    }

    // Build Pix from cropped RGB data
    let pix = match kreuzberg_tesseract::Pix::from_raw_rgb(&crop_data, actual_w, actual_h) {
        Ok(p) => p,
        Err(_) => return RegionValidation::Skipped,
    };

    // Grayscale → binarize → count CCs
    let gray = match pix.to_grayscale() {
        Ok(g) => g,
        Err(_) => return RegionValidation::Skipped,
    };

    let binary = match gray.adaptive_threshold(16, 16) {
        Ok(b) => b,
        Err(_) => return RegionValidation::Skipped,
    };

    let cc_count = match binary.count_connected_components(4) {
        Ok(c) => c,
        Err(_) => return RegionValidation::Skipped,
    };

    if cc_count >= MIN_TEXT_CC_COUNT {
        RegionValidation::HasContent
    } else {
        tracing::trace!(
            cc_count,
            region_w,
            region_h,
            "layout validation: region flagged as empty (few CCs)"
        );
        RegionValidation::Empty
    }
}

/// Fallback: when `ocr` feature is absent, leptonica is unavailable — every
/// region is treated as having content (conservative; no false suppressions).
#[cfg(all(feature = "layout-detection", not(feature = "ocr")))]
pub(in crate::pdf::structure) fn validate_region_has_text(
    _page_rgb: &[u8],
    _page_width: u32,
    _page_height: u32,
    _region_x: u32,
    _region_y: u32,
    _region_w: u32,
    _region_h: u32,
) -> RegionValidation {
    RegionValidation::HasContent
}

/// Validate all suppressible layout regions on a page.
///
/// Returns a Vec parallel to `hints`, indicating which regions have validated
/// text content. Only validates Table and Picture regions (those that cause
/// segment suppression). Other classes return `Skipped`.
#[cfg(feature = "layout-detection")]
pub(in crate::pdf::structure) fn validate_page_regions(
    page_image: &image::DynamicImage,
    hints: &[super::super::types::LayoutHint],
    page_result: &crate::pdf::structure::types::PageLayoutResult,
) -> Vec<RegionValidation> {
    use super::super::types::LayoutHintClass;

    let rgb = page_image.to_rgb8();
    let img_w = rgb.width();
    let img_h = rgb.height();
    let rgb_data = rgb.as_raw();

    // Scale factors: PDF points → rendered image pixels
    let sx = img_w as f32 / page_result.page_width_pts;
    let sy = img_h as f32 / page_result.page_height_pts;

    hints
        .iter()
        .map(|hint| {
            // Only validate Table and Picture (suppressible classes)
            if !matches!(hint.class_name, LayoutHintClass::Table | LayoutHintClass::Picture) {
                return RegionValidation::Skipped;
            }

            // Convert PDF coords to pixel coords (same as table_recognition.rs)
            let px_left = (hint.left * sx).round().max(0.0) as u32;
            let px_top = ((page_result.page_height_pts - hint.top) * sy).round().max(0.0) as u32;
            let px_right = (hint.right * sx).round().min(img_w as f32) as u32;
            let px_bottom = ((page_result.page_height_pts - hint.bottom) * sy)
                .round()
                .min(img_h as f32) as u32;

            let crop_w = px_right.saturating_sub(px_left);
            let crop_h = px_bottom.saturating_sub(px_top);

            // Skip very large regions (>50% page area) — likely intentional
            if (crop_w as f32 * crop_h as f32) > (img_w as f32 * img_h as f32 * 0.5) {
                return RegionValidation::Skipped;
            }

            validate_region_has_text(rgb_data, img_w, img_h, px_left, px_top, crop_w, crop_h)
        })
        .collect()
}
