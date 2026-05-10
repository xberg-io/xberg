//! Layout detection runner for PDF pages.
//!
//! Renders all pages of a PDF document at default resolution, runs the layout
//! engine once across the whole batch, and converts pixel-space detections to
//! PDF coordinate–space [`PageLayoutResult`] values.
//!
//! The resulting `(Vec<RgbImage>, Vec<PageLayoutResult>)` pair is consumed
//! by [`super::extraction::extract_all_from_oxide_document`] via
//! `layout_images` / `layout_results`, which feeds the segment structure
//! pipeline with layout hints for heading / table / list / figure
//! classification (the "layout-for-markdown" path).

#[cfg(all(feature = "pdf", feature = "layout-detection"))]
use crate::{
    KreuzbergError, Result,
    core::config::{ExtractionConfig, layout::LayoutDetectionConfig},
    extractors::pdf::layout_hints::pixel_detection_to_layout_hints_pdf_space,
    pdf::structure::types::{LayoutHint, PageLayoutResult},
};

/// Render every page of `content` to RGB at the pdf_oxide default DPI and run
/// layout detection on the full batch.
///
/// Returns `(images, results)` where:
/// - `images[i]` is the rendered RGB image for page `i` (owned, used by table
///   recognition and region validation downstream).
/// - `results[i]` holds per-region detections in PDF coordinate space (points).
///
/// # Errors
///
/// Returns an error if the PDF cannot be opened, any page fails to render, or
/// the layout engine cannot be initialised.  Callers should treat detection
/// failures as soft errors (log a warning and continue without layout) rather
/// than propagating them — hence the engine is returned to the global cache
/// before any error path exits.
#[cfg(all(feature = "pdf", feature = "layout-detection"))]
type LayoutForMarkdownOutput = (Vec<image::RgbImage>, Vec<PageLayoutResult>, Vec<Vec<LayoutHint>>);

#[cfg(all(feature = "pdf", feature = "layout-detection"))]
pub(super) fn run_layout_for_pdf_pages(
    content: &[u8],
    layout_config: &LayoutDetectionConfig,
) -> Result<LayoutForMarkdownOutput> {
    use pdf_oxide::rendering::RenderOptions;

    // --- 1. Open document and render all pages ---
    let mut doc = pdf_oxide::PdfDocument::from_bytes(content.to_vec()).map_err(|e| KreuzbergError::Parsing {
        message: format!("layout runner: failed to open PDF: {e}"),
        source: None,
    })?;

    let page_count = doc.page_count().map_err(|e| KreuzbergError::Parsing {
        message: format!("layout runner: failed to get page count: {e}"),
        source: None,
    })?;

    if page_count == 0 {
        return Ok((Vec::new(), Vec::new(), Vec::new()));
    }

    let render_opts = RenderOptions::default();

    // Collect (page_width_pts, page_height_pts, RgbImage) for every page.
    //
    // We decode each page to DynamicImage only long enough to call `.to_rgb8()`,
    // then drop the DynamicImage immediately. At 150 DPI an A4 RGB image is ~6 MB;
    // keeping DynamicImage and RgbImage alive simultaneously would double peak RSS.
    let mut page_data: Vec<(f32, f32, image::RgbImage)> = Vec::with_capacity(page_count);
    for page_idx in 0..page_count {
        // Get page dimensions in PDF points (0-based index).
        let (page_width_pts, page_height_pts) = doc
            .get_page_media_box(page_idx)
            .map(|(llx, lly, urx, ury)| ((urx - llx).abs(), (ury - lly).abs()))
            .unwrap_or((612.0, 792.0)); // Letter fallback

        let rendered = pdf_oxide::rendering::render_page(&mut doc, page_idx, &render_opts).map_err(|e| {
            KreuzbergError::Parsing {
                message: format!("layout runner: failed to render page {}: {e}", page_idx + 1),
                source: None,
            }
        })?;

        // Decode to DynamicImage, convert to RGB immediately, then drop DynamicImage
        // so its pixel buffer is freed before processing the next page.
        let rgb = image::load_from_memory(&rendered.data)
            .map_err(|e| KreuzbergError::Parsing {
                message: format!("layout runner: failed to decode page {} PNG: {e}", page_idx + 1),
                source: None,
            })?
            .into_rgb8();

        page_data.push((page_width_pts, page_height_pts, rgb));
    }

    // --- 2. Run layout detection across all rendered images ---
    let mut engine = crate::layout::take_or_create_engine(layout_config)
        .map_err(|e| KreuzbergError::Other(format!("layout runner: engine init failed: {e}")))?;

    let rgb_refs: Vec<&image::RgbImage> = page_data.iter().map(|(_, _, img)| img).collect();

    let batch_results = match engine.detect_batch(&rgb_refs) {
        Ok(r) => {
            crate::layout::return_engine(engine);
            r
        }
        Err(e) => {
            crate::layout::return_engine(engine);
            return Err(KreuzbergError::Other(format!(
                "layout runner: batch detection failed: {e}"
            )));
        }
    };

    // --- 3. Convert pixel detections → PDF coordinate space + build hints ---
    let mut images: Vec<image::RgbImage> = Vec::with_capacity(page_count);
    let mut layout_results: Vec<PageLayoutResult> = Vec::with_capacity(page_count);
    let mut hints_per_page: Vec<Vec<LayoutHint>> = Vec::with_capacity(page_count);

    for ((page_width_pts, page_height_pts, img), (detection, _timings)) in page_data.into_iter().zip(batch_results) {
        let image_width_px = img.width();
        let image_height_px = img.height();

        let hints = pixel_detection_to_layout_hints_pdf_space(
            &detection,
            image_width_px,
            image_height_px,
            page_width_pts,
            page_height_pts,
        );

        tracing::debug!(
            detections = detection.detections.len(),
            hints = hints.len(),
            page_width_pts,
            page_height_pts,
            image_width_px,
            image_height_px,
            "layout runner: page detections"
        );

        layout_results.push(PageLayoutResult {
            page_width_pts,
            page_height_pts,
        });
        hints_per_page.push(hints);
        images.push(img);
    }

    Ok((images, layout_results, hints_per_page))
}

/// Convenience wrapper that reads `use_layout_for_markdown` and other gate
/// conditions from `config` and, when they are all satisfied, runs
/// [`run_layout_for_pdf_pages`].
///
/// Returns `(None, None)` when the feature is not requested, or on soft
/// failure (logged as a warning so the markdown path can continue without
/// layout hints).
#[cfg(all(feature = "pdf", feature = "layout-detection"))]
type LayoutForMarkdownOptional = (
    Option<Vec<image::RgbImage>>,
    Option<Vec<PageLayoutResult>>,
    Option<Vec<Vec<LayoutHint>>>,
);

#[cfg(all(feature = "pdf", feature = "layout-detection"))]
pub(super) fn maybe_run_layout_for_markdown(content: &[u8], config: &ExtractionConfig) -> LayoutForMarkdownOptional {
    if !config.use_layout_for_markdown {
        return (None, None, None);
    }
    let Some(ref layout_config) = config.layout else {
        return (None, None, None);
    };
    if config.force_ocr {
        // force_ocr runs every page through OCR, which has its own layout detection path.
        // Running layout here too would be wasteful and produce conflicting hints.
        return (None, None, None);
    }
    match run_layout_for_pdf_pages(content, layout_config) {
        Ok((images, results, hints)) => {
            let total_hints: usize = hints.iter().map(|h| h.len()).sum();
            tracing::info!(
                pages = images.len(),
                total_hints,
                "layout-for-markdown: detection succeeded"
            );
            (Some(images), Some(results), Some(hints))
        }
        Err(e) => {
            tracing::warn!(
                error = %e,
                "layout-for-markdown: detection failed, continuing without layout hints"
            );
            (None, None, None)
        }
    }
}
