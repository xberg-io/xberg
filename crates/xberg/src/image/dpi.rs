/// PDF points per inch constant
const PDF_POINTS_PER_INCH: f64 = 72.0;

#[cfg(feature = "pdf")]
pub(crate) const OCR_PNG_ENCODE_BYTES_PER_PIXEL: u64 = 4;
#[cfg(feature = "pdf")]
pub(crate) const OCR_PNG_ENCODE_FIXED_BYTES: u64 = 256 * 1024;
#[cfg(feature = "pdf")]
const OCR_RENDER_SOURCE_BYTES_PER_PIXEL: u64 = 4;
#[cfg(feature = "pdf")]
const OCR_RGB_CONVERSION_BYTES_PER_PIXEL: u64 = 3;

/// Calculate smart DPI based on page dimensions, memory constraints, and target DPI
// The only non-test caller is `image::preprocessing`, which is `ocr-pipeline`-gated.
// `layout-detection` pulls this module in for `effective_pdf_render_dpi` alone (#1577), so
// under `pdf + layout-detection` without `ocr-pipeline` this function is genuinely
// unreachable and `-D warnings` fails the build on that leg. ~keep
#[cfg_attr(not(any(feature = "ocr-pipeline", test)), allow(dead_code))]
#[allow(clippy::cast_possible_truncation)]
pub(crate) fn calculate_smart_dpi(
    page_width: f64,
    page_height: f64,
    target_dpi: i32,
    max_dimension: i32,
    max_memory_mb: f64,
) -> i32 {
    let width_inches = page_width / PDF_POINTS_PER_INCH;
    let height_inches = page_height / PDF_POINTS_PER_INCH;

    let max_pixels = (max_memory_mb * 1024.0 * 1024.0 / 3.0).sqrt().round() as i32;

    let max_dpi_for_memory_width = if width_inches > 0.0 {
        (f64::from(max_pixels) / width_inches).round() as i32
    } else {
        target_dpi
    };

    let max_dpi_for_memory_height = if height_inches > 0.0 {
        (f64::from(max_pixels) / height_inches).round() as i32
    } else {
        target_dpi
    };

    let memory_constrained_dpi = max_dpi_for_memory_width.min(max_dpi_for_memory_height);

    let dimension_constrained_dpi =
        calculate_dimension_constrained_dpi(width_inches, height_inches, target_dpi, max_dimension);

    let final_dpi = target_dpi.min(memory_constrained_dpi).min(dimension_constrained_dpi);

    final_dpi.max(72)
}

/// Calculate DPI constrained by maximum dimension
#[allow(clippy::cast_possible_truncation)]
fn calculate_dimension_constrained_dpi(
    width_inches: f64,
    height_inches: f64,
    target_dpi: i32,
    max_dimension: i32,
) -> i32 {
    let target_width_pixels = (width_inches * f64::from(target_dpi)).round() as i32;
    let target_height_pixels = (height_inches * f64::from(target_dpi)).round() as i32;
    let max_pixel_dimension = target_width_pixels.max(target_height_pixels);

    if max_pixel_dimension > max_dimension {
        let max_dpi_for_width = if width_inches > 0.0 {
            (f64::from(max_dimension) / width_inches).round() as i32
        } else {
            target_dpi
        };

        let max_dpi_for_height = if height_inches > 0.0 {
            (f64::from(max_dimension) / height_inches).round() as i32
        } else {
            target_dpi
        };

        max_dpi_for_width.min(max_dpi_for_height)
    } else {
        target_dpi
    }
}

/// Default PDF page render DPI when the caller supplies no `ImageExtractionConfig`.
///
/// Deliberately the historical literal `150`, not `ImageDpiConfig::default().target_dpi` (300).
/// Rendering natively at 300 would be defensible -- the downstream normalization step already
/// upscales 150 -> 300 before OCR, so true detail would replace interpolation at the same final
/// pixel count -- but it quadruples the peak render allocation, changes OCR output for every
/// existing caller, and was observed to trip `SecurityLimits` on a page that renders fine today.
/// #1577 asks only that a *configured* `target_dpi` stop being ignored; raising the default is a
/// separate quality change that needs an A/B before it ships. ~keep
#[cfg(feature = "pdf")]
pub(crate) const DEFAULT_PDF_RENDER_DPI: i32 = 150;

/// Ceiling for rendering a full-page raster at its own density (#1786).
///
/// The OCR preprocessor resamples every page to `ImagePreprocessingConfig::default().target_dpi`
/// (300) before recognition, so detail above 300 is discarded anyway, and a Letter page at 300
/// stays inside `SecurityLimits::default().max_content_size` at the PNG-encode step. Keep the
/// two in step: a higher ceiling here buys nothing until that target moves. ~keep
#[cfg(feature = "pdf")]
pub(crate) const SCAN_PAGE_MAX_RENDER_DPI: i32 = 300;

/// The render DPI for a page that is one full-page raster of `native_density` dots per inch.
///
/// The raster's own density, never below [`DEFAULT_PDF_RENDER_DPI`] and never above
/// [`SCAN_PAGE_MAX_RENDER_DPI`]. Rendering a 196 dpi scan at 150 discards a quarter of its
/// pixels before the OCR preprocessor upscales the page to 300 again; rendering it at 196
/// keeps every pixel for the same final size (#1786). A density that is not finite is treated
/// as unknown and gets the default.
#[cfg(feature = "pdf")]
pub(crate) fn scan_page_render_dpi(native_density: f64) -> i32 {
    if !native_density.is_finite() {
        return DEFAULT_PDF_RENDER_DPI;
    }
    (native_density.round() as i32).clamp(DEFAULT_PDF_RENDER_DPI, SCAN_PAGE_MAX_RENDER_DPI)
}

/// Resolve the DPI to render a PDF page at for OCR (#1577), honoring `ImageExtractionConfig`'s
/// `target_dpi`, `min_dpi`, `max_dpi`, `max_image_dimension`, and `auto_adjust_dpi`.
///
/// Without an `ImageExtractionConfig` this resolves to [`DEFAULT_PDF_RENDER_DPI`]. With one,
/// `target_dpi` is clamped to `[min_dpi, max_dpi]`; when `auto_adjust_dpi` is also set, the
/// clamped target is further reduced (never raised) to keep the render within
/// `max_image_dimension` pixels on its longest side, via the same
/// [`calculate_dimension_constrained_dpi`] helper the standalone-image path uses. Downstream,
/// `render::choose_safe_dpi` still applies its own absolute rasterizer pixel ceiling
/// regardless of what this returns, so a caller cannot push a render past that safety limit
/// through this function alone. ~keep
#[cfg(feature = "pdf")]
pub(crate) fn effective_pdf_render_dpi(
    images_config: Option<&crate::core::config::ImageExtractionConfig>,
    page_width_pt: f64,
    page_height_pt: f64,
) -> i32 {
    let Some(images_config) = images_config else {
        return DEFAULT_PDF_RENDER_DPI;
    };
    let clamped_target = images_config
        .target_dpi
        .clamp(images_config.min_dpi, images_config.max_dpi);
    if !images_config.auto_adjust_dpi {
        return clamped_target;
    }
    let dimension_constrained = calculate_dimension_constrained_dpi(
        page_width_pt / PDF_POINTS_PER_INCH,
        page_height_pt / PDF_POINTS_PER_INCH,
        clamped_target,
        images_config.max_image_dimension,
    );
    dimension_constrained.clamp(images_config.min_dpi, images_config.max_dpi)
}

/// The DPI to render `page_idx` of `doc` at for OCR.
///
/// Shared by every PDF OCR render route -- the whole-document/`force_ocr_pages` route
/// (`extractors::pdf::ocr::rendering`) and the layout-detection route
/// (`extractors::pdf::layout_runner`), which renders its own OCR input independently of
/// them. Before this was shared, only the non-layout routes made the scan-density decision
/// below, so a scanned page's OCR render DPI depended on whether layout detection was on
/// (#1828, follow-up to #1786).
///
/// A caller's `images_config` decides as before (#1577). Without one, a page that is a single
/// full-page raster renders at that raster's own density, bounded by
/// [`SCAN_PAGE_MAX_RENDER_DPI`], instead of the [`DEFAULT_PDF_RENDER_DPI`] default: a 196 dpi
/// scan rendered at 150 and then upscaled to 300 by the OCR preprocessor lost table values
/// that OCR of the same page as an image read (#1786). Every other page keeps the default.
#[cfg(feature = "pdf")]
pub(crate) fn planned_pdf_ocr_render_dpi(
    doc: &xberg_native_pdf::PdfDocument,
    page_idx: usize,
    images_config: Option<&crate::core::config::ImageExtractionConfig>,
    security_limits: &crate::extractors::security::SecurityLimits,
) -> (i32, i32) {
    planned_pdf_ocr_render_dpi_with_preprocessing(doc, page_idx, images_config, security_limits, None)
}

/// Preserve an explicit OCR normalization density in the source render before detail is lost.
/// Explicit render configuration and a scan's original density retain their precedence. ~keep
#[cfg(feature = "pdf")]
pub(crate) fn planned_pdf_ocr_render_dpi_with_preprocessing(
    doc: &xberg_native_pdf::PdfDocument,
    page_idx: usize,
    images_config: Option<&crate::core::config::ImageExtractionConfig>,
    security_limits: &crate::extractors::security::SecurityLimits,
    preprocessing_dpi: Option<i32>,
) -> (i32, i32) {
    let requested_dpi = if images_config.is_none()
        && let Some(density) = crate::pdf::scan_detect::full_page_raster_density(doc, page_idx)
    {
        scan_page_render_dpi(density)
    } else {
        let (page_width_pt, page_height_pt) = crate::pdf::render::get_page_dimensions_pt(doc, page_idx);
        if images_config.is_none() {
            preprocessing_dpi
                .unwrap_or(DEFAULT_PDF_RENDER_DPI)
                .max(DEFAULT_PDF_RENDER_DPI)
        } else {
            effective_pdf_render_dpi(images_config, f64::from(page_width_pt), f64::from(page_height_pt))
        }
    };
    let (page_width_pt, page_height_pt) = crate::pdf::render::get_page_dimensions_pt(doc, page_idx);
    let capped_dpi = cap_pdf_ocr_render_dpi(
        requested_dpi,
        f64::from(page_width_pt),
        f64::from(page_height_pt),
        security_limits.max_content_size,
    );
    (requested_dpi, capped_dpi)
}

#[cfg(feature = "pdf")]
fn cap_pdf_ocr_render_dpi(requested_dpi: i32, page_width_pt: f64, page_height_pt: f64, max_content_size: usize) -> i32 {
    let fits = |dpi: i32| {
        let width = ((page_width_pt / PDF_POINTS_PER_INCH) * f64::from(dpi)).ceil().max(1.0) as u128;
        let height = ((page_height_pt / PDF_POINTS_PER_INCH) * f64::from(dpi))
            .ceil()
            .max(1.0) as u128;
        let bytes_per_pixel = u128::from(
            OCR_RENDER_SOURCE_BYTES_PER_PIXEL + OCR_RGB_CONVERSION_BYTES_PER_PIXEL + OCR_PNG_ENCODE_BYTES_PER_PIXEL,
        );
        width
            .checked_mul(height)
            .and_then(|pixels| pixels.checked_mul(bytes_per_pixel))
            .and_then(|bytes| bytes.checked_add(u128::from(OCR_PNG_ENCODE_FIXED_BYTES)))
            .is_some_and(|bytes| bytes <= max_content_size as u128)
    };
    if fits(requested_dpi) {
        return requested_dpi;
    }

    let mut low = 1;
    let mut high = requested_dpi.max(1);
    while low < high {
        let middle = low + (high - low + 1) / 2;
        if fits(middle) {
            low = middle;
        } else {
            high = middle - 1;
        }
    }
    low
}

/// Calculate optimal DPI with min/max constraints
#[cfg(test)]
pub(crate) fn calculate_optimal_dpi(
    page_width: f64,
    page_height: f64,
    target_dpi: i32,
    max_dimension: i32,
    min_dpi: i32,
    max_dpi: i32,
) -> i32 {
    let smart_dpi = calculate_smart_dpi(page_width, page_height, target_dpi, max_dimension, 2048.0);

    min_dpi.max(smart_dpi.min(max_dpi))
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_calculate_smart_dpi_basic() {
        let dpi = calculate_smart_dpi(612.0, 792.0, 300, 4096, 2048.0);
        assert!(dpi >= 72);
        assert!(dpi <= 300);
    }

    #[test]
    fn test_calculate_smart_dpi_memory_constrained() {
        let dpi = calculate_smart_dpi(1224.0, 1584.0, 300, 8192, 10.0);
        assert!(dpi < 300);
        assert!(dpi >= 72);
    }

    #[test]
    fn test_calculate_smart_dpi_dimension_constrained() {
        let dpi = calculate_smart_dpi(612.0, 792.0, 300, 1000, 2048.0);
        assert!(dpi < 300);
    }

    #[test]
    fn test_calculate_smart_dpi_minimum_dpi() {
        let dpi = calculate_smart_dpi(10000.0, 10000.0, 300, 100, 1.0);
        assert_eq!(dpi, 72);
    }

    #[test]
    fn test_calculate_smart_dpi_zero_dimensions() {
        let dpi = calculate_smart_dpi(0.0, 792.0, 300, 4096, 2048.0);
        assert!(dpi >= 72);

        let dpi = calculate_smart_dpi(612.0, 0.0, 300, 4096, 2048.0);
        assert!(dpi >= 72);

        let dpi = calculate_smart_dpi(0.0, 0.0, 300, 4096, 2048.0);
        assert_eq!(dpi, 300);
    }

    #[test]
    fn test_calculate_dimension_constrained_dpi() {
        let dpi = calculate_dimension_constrained_dpi(8.5, 11.0, 300, 4096);
        assert!(dpi <= 300);

        let dpi = calculate_dimension_constrained_dpi(8.5, 11.0, 600, 2000);
        assert!(dpi < 600);
    }

    #[test]
    fn test_calculate_optimal_dpi() {
        let dpi = calculate_optimal_dpi(612.0, 792.0, 300, 4096, 72, 600);
        assert!(dpi >= 72);
        assert!(dpi <= 600);

        let dpi = calculate_optimal_dpi(10000.0, 10000.0, 300, 100, 100, 600);
        assert_eq!(dpi, 100);

        let dpi = calculate_optimal_dpi(72.0, 72.0, 1000, 10000, 72, 600);
        assert_eq!(dpi, 600);
    }

    #[test]
    fn test_memory_calculation() {
        let dpi = calculate_smart_dpi(612.0, 792.0, 10000, 100000, 2048.0);
        assert!(dpi < 10000);
        assert!(dpi >= 72);
    }

    #[test]
    fn test_aspect_ratio_preservation() {
        let wide_dpi = calculate_smart_dpi(1224.0, 396.0, 300, 4096, 2048.0);
        let tall_dpi = calculate_smart_dpi(396.0, 1224.0, 300, 4096, 2048.0);

        assert!(wide_dpi >= 72);
        assert!(tall_dpi >= 72);
    }

    /// #1577: without an `ImageExtractionConfig`, PDF page render stays at the historical
    /// default. The bug is a *configured* DPI being ignored, not the default being wrong, so
    /// this pins the default against an accidental change.
    #[cfg(feature = "pdf")]
    #[test]
    fn effective_pdf_render_dpi_without_config_is_the_default() {
        assert_eq!(effective_pdf_render_dpi(None, 612.0, 792.0), DEFAULT_PDF_RENDER_DPI);
        assert_eq!(DEFAULT_PDF_RENDER_DPI, 150);
    }

    /// #1786: a scan renders at its own density inside `[150, 300]`; outside that band the
    /// nearer bound applies, and an unknown density falls back to the default.
    #[cfg(feature = "pdf")]
    #[test]
    fn scan_page_render_dpi_keeps_the_native_density_inside_the_band() {
        assert_eq!(scan_page_render_dpi(196.0), 196);
        assert_eq!(scan_page_render_dpi(195.6), 196);
        assert_eq!(scan_page_render_dpi(100.0), DEFAULT_PDF_RENDER_DPI);
        assert_eq!(scan_page_render_dpi(600.0), SCAN_PAGE_MAX_RENDER_DPI);
        assert_eq!(scan_page_render_dpi(f64::NAN), DEFAULT_PDF_RENDER_DPI);
    }

    /// A caller's `target_dpi` reaches the render step verbatim when it fits inside
    /// `[min_dpi, max_dpi]` and `auto_adjust_dpi` is off (#1577's exact repro: `target_dpi=600`
    /// on a Letter page must not stay pinned at whatever the render historically used).
    #[cfg(feature = "pdf")]
    #[test]
    fn effective_pdf_render_dpi_honours_target_dpi_without_auto_adjust() {
        let images_config = crate::core::config::ImageExtractionConfig {
            target_dpi: 600,
            auto_adjust_dpi: false,
            min_dpi: 72,
            max_dpi: 600,
            ..Default::default()
        };
        assert_eq!(effective_pdf_render_dpi(Some(&images_config), 612.0, 792.0), 600);
    }

    /// `target_dpi` is clamped into `[min_dpi, max_dpi]` before anything else, both when it
    /// overshoots the ceiling and when it undershoots the floor.
    #[cfg(feature = "pdf")]
    #[test]
    fn effective_pdf_render_dpi_clamps_target_dpi_to_min_max() {
        let over_max = crate::core::config::ImageExtractionConfig {
            target_dpi: 1200,
            auto_adjust_dpi: false,
            min_dpi: 72,
            max_dpi: 600,
            ..Default::default()
        };
        assert_eq!(effective_pdf_render_dpi(Some(&over_max), 612.0, 792.0), 600);

        let under_min = crate::core::config::ImageExtractionConfig {
            target_dpi: 50,
            auto_adjust_dpi: false,
            min_dpi: 72,
            max_dpi: 600,
            ..Default::default()
        };
        assert_eq!(effective_pdf_render_dpi(Some(&under_min), 612.0, 792.0), 72);
    }

    /// With `auto_adjust_dpi` on, a page whose `target_dpi` render would exceed
    /// `max_image_dimension` on its longest side is reduced (never raised) to fit -- the same
    /// dimension-constrained-DPI rule the standalone-image path applies, now reachable from a
    /// rendered PDF page.
    #[cfg(feature = "pdf")]
    #[test]
    fn effective_pdf_render_dpi_auto_adjusts_for_max_image_dimension() {
        let images_config = crate::core::config::ImageExtractionConfig {
            target_dpi: 600,
            auto_adjust_dpi: true,
            max_image_dimension: 2000,
            min_dpi: 72,
            max_dpi: 600,
            ..Default::default()
        };
        // An 8.5x11in Letter page at 600 DPI is 5100x6600px, well past a 2000px cap; the
        // long (11in) side must be the one that determines the reduced DPI:
        // round(2000 / 11) = 182.
        let dpi = effective_pdf_render_dpi(Some(&images_config), 612.0, 792.0);
        assert_eq!(dpi, 182);
        assert!(dpi < 600, "auto_adjust_dpi must reduce, not raise, an oversized target");
    }
}

#[cfg(all(test, feature = "pdf", feature = "ocr"))]
#[path = "dpi/preprocessing_density_tests.rs"]
mod preprocessing_density_tests;
