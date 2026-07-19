//! Layout detection runner for PDF pages.
//!
//! Renders pages of a PDF document in chunks, runs the layout engine on each
//! chunk in sequence, and converts pixel-space detections to PDF
//! coordinate–space [`PageLayoutResult`] values.
//!
//! Chunked rendering+detection keeps peak memory proportional to
//! `LAYOUT_BATCH_CHUNK_SIZE` images plus the accumulated output images,
//! rather than requiring the whole document's rasterised frames and the full
//! ONNX batch tensor to be live simultaneously.
//!
//! The resulting images, page metadata, layout hints, and raw detections feed
//! both native markdown structure recovery and OCR layout assembly.

/// Maximum number of pages sent to the layout model in a single ONNX call.
///
/// Each chunk requires:  chunk_size × 3 × 640 × 640 × 4 bytes ≈ 4.9 MB × chunk_size
/// for the batch tensor.  8 pages ≈ 39 MB vs 214 pages ≈ 1.05 GB without chunking.
#[cfg(all(feature = "pdf", feature = "layout-detection"))]
const LAYOUT_BATCH_CHUNK_SIZE: usize = 8;

/// Small white raster used when one page cannot be rendered.
///
/// Keeping a page-aligned placeholder lets downstream OCR reuse every other
/// rendered page without retrying the entire document. The matching detection
/// result is empty and uses these same dimensions.
#[cfg(all(feature = "pdf", feature = "layout-detection"))]
const FAILED_RENDER_PLACEHOLDER_SIDE: u32 = 64;

#[cfg(all(feature = "pdf", feature = "layout-detection"))]
use crate::{
    Result, XbergError,
    core::config::{ExtractionConfig, layout::LayoutDetectionConfig},
    extractors::pdf::layout_hints::pixel_detection_to_layout_hints_pdf_space,
    pdf::structure::types::{LayoutHint, PageLayoutResult},
};

/// Render every page of `content` to RGB (in chunks) and run layout detection.
///
/// Returns `(images, results, hints_per_page, detections_per_page)` where:
/// - `images[i]` is the rendered RGB image for page `i` (or a small white placeholder
///   if the page failed to render).
/// - `results[i]` holds per-region detection metadata in PDF coordinate space.
/// - `hints_per_page[i]` holds the layout hints derived from detections on
///   page `i` (empty for pages that failed to render or produced no detections).
/// - `detections_per_page[i]` preserves the pixel-space detections for OCR
///   layout assembly (empty for pages that failed to render).
///
/// # Memory behaviour
///
/// Pages are rendered and detected in chunks of [`LAYOUT_BATCH_CHUNK_SIZE`]
/// so the peak ONNX batch tensor size is bounded.  The returned `images` vec
/// accumulates all page images for downstream table recognition.
///
/// # Errors
///
/// Returns an error if the PDF cannot be opened, the layout engine cannot be
/// initialised, or detection fails on any chunk.  Individual page render
/// failures are logged and produce empty layout for that page without aborting
/// the whole document.
#[cfg(all(feature = "pdf", feature = "layout-detection"))]
type LayoutForMarkdownOutput = (
    Vec<image::RgbImage>,
    Vec<PageLayoutResult>,
    Vec<Vec<LayoutHint>>,
    Vec<crate::layout::DetectionResult>,
);

#[cfg(all(feature = "pdf", feature = "layout-detection"))]
async fn run_layout_for_pdf_pages_async(
    content: &[u8],
    layout_config: &LayoutDetectionConfig,
) -> Result<LayoutForMarkdownOutput> {
    #[cfg(feature = "tokio-runtime")]
    {
        let owned_content = content.to_vec();
        let owned_config = layout_config.clone();
        tokio::task::spawn_blocking(move || run_layout_for_pdf_pages(&owned_content, &owned_config))
            .await
            .map_err(|error| XbergError::Other(format!("layout runner task failed: {error}")))?
    }

    #[cfg(not(feature = "tokio-runtime"))]
    run_layout_for_pdf_pages(content, layout_config)
}

#[cfg(all(feature = "pdf", feature = "layout-detection"))]
fn validate_batch_cardinality(expected: usize, actual: usize) -> Result<()> {
    if actual == expected {
        return Ok(());
    }

    Err(XbergError::Other(format!(
        "layout runner: batch detection returned {actual} results for {expected} rendered pages"
    )))
}

#[cfg(all(feature = "pdf", feature = "layout-detection"))]
fn render_failure_placeholder() -> image::RgbImage {
    image::RgbImage::from_pixel(
        FAILED_RENDER_PLACEHOLDER_SIDE,
        FAILED_RENDER_PLACEHOLDER_SIDE,
        image::Rgb([u8::MAX; 3]),
    )
}

#[cfg(all(feature = "pdf", feature = "layout-detection"))]
fn displayed_page_dimensions(width: f32, height: f32, rotation_degrees: u32) -> (f32, f32) {
    match rotation_degrees % 360 {
        90 | 270 => (height, width),
        _ => (width, height),
    }
}

#[cfg(all(feature = "pdf", feature = "layout-detection"))]
pub(super) fn run_layout_for_pdf_pages(
    content: &[u8],
    layout_config: &LayoutDetectionConfig,
) -> Result<LayoutForMarkdownOutput> {
    let doc = pdf_oxide::PdfDocument::from_bytes(content.to_vec()).map_err(|e| XbergError::Parsing {
        message: format!("layout runner: failed to open PDF: {e}"),
        source: None,
    })?;

    let page_count = doc.page_count().map_err(|e| XbergError::Parsing {
        message: format!("layout runner: failed to get page count: {e}"),
        source: None,
    })?;

    if page_count == 0 {
        return Ok((Vec::new(), Vec::new(), Vec::new(), Vec::new()));
    }

    let mut engine = crate::layout::take_or_create_engine(layout_config)
        .map_err(|e| XbergError::Other(format!("layout runner: engine init failed: {e}")))?;

    let page_rotations = crate::pdf::render::get_page_rotations(content, page_count);

    let mut all_images: Vec<image::RgbImage> = Vec::with_capacity(page_count);
    let mut all_layout_results: Vec<PageLayoutResult> = Vec::with_capacity(page_count);
    let mut all_hints: Vec<Vec<LayoutHint>> = Vec::with_capacity(page_count);
    let mut all_detections: Vec<crate::layout::DetectionResult> = Vec::with_capacity(page_count);

    let total_chunks = page_count.div_ceil(LAYOUT_BATCH_CHUNK_SIZE);

    for (chunk_idx, chunk_start) in (0..page_count).step_by(LAYOUT_BATCH_CHUNK_SIZE).enumerate() {
        let chunk_end = (chunk_start + LAYOUT_BATCH_CHUNK_SIZE).min(page_count);
        let chunk_size = chunk_end - chunk_start;

        let mut chunk_page_meta: Vec<(f32, f32)> = Vec::with_capacity(chunk_size);
        let mut chunk_images: Vec<Option<image::RgbImage>> = Vec::with_capacity(chunk_size);

        for page_idx in chunk_start..chunk_end {
            let (media_width, media_height) = doc
                .get_page_media_box(page_idx)
                .map(|(llx, lly, urx, ury)| ((urx - llx).abs(), (ury - lly).abs()))
                .unwrap_or((612.0, 792.0));
            let rotation = page_rotations.get(page_idx).copied().unwrap_or(0);
            let (pw, ph) = displayed_page_dimensions(media_width, media_height, rotation);
            chunk_page_meta.push((pw, ph));

            let rgb_opt = match crate::pdf::render::render_page_with_safeguards(&doc, page_idx, 150) {
                Err(e) => {
                    tracing::warn!(
                        page = page_idx + 1,
                        page_width_pts = pw,
                        page_height_pts = ph,
                        error = %e,
                        "layout runner: skipping page with render failure, returning empty detections"
                    );
                    None
                }
                Ok(rendered) => match image::load_from_memory(&rendered.data) {
                    Err(e) => {
                        tracing::warn!(
                            page = page_idx + 1,
                            page_width_pts = pw,
                            page_height_pts = ph,
                            error = %e,
                            "layout runner: skipping page (PNG decode failed), returning empty detections"
                        );
                        None
                    }
                    // pdf_oxide applies inherited page rotation while rendering.
                    // Keep this raster unchanged so its coordinates stay aligned
                    // with layout detections and reused OCR input.
                    Ok(img) => Some(img.into_rgb8()),
                },
            };
            chunk_images.push(rgb_opt);
        }

        let rendered_positions: Vec<usize> = chunk_images
            .iter()
            .enumerate()
            .filter_map(|(k, opt)| opt.as_ref().map(|_| k))
            .collect();

        let detection_results = if rendered_positions.is_empty() {
            tracing::debug!(
                chunk_idx,
                total_chunks,
                "layout runner: all pages in chunk failed to render, skipping detection"
            );
            Vec::new()
        } else {
            let rgb_refs: Vec<&image::RgbImage> = rendered_positions
                .iter()
                .map(|&k| chunk_images[k].as_ref().expect("filtered to Some above"))
                .collect();

            tracing::debug!(
                chunk_idx,
                total_chunks,
                chunk_start,
                chunk_end,
                rendered = rgb_refs.len(),
                "layout runner: detecting chunk"
            );

            let results = match engine.detect_batch(&rgb_refs) {
                Ok(results) => results,
                Err(e) => {
                    crate::layout::return_engine(engine);
                    return Err(XbergError::Other(format!("layout runner: batch detection failed: {e}")));
                }
            };
            if let Err(error) = validate_batch_cardinality(rgb_refs.len(), results.len()) {
                crate::layout::return_engine(engine);
                return Err(error);
            }
            results
        };

        let mut detected_by_pos: Vec<Option<_>> = (0..chunk_size).map(|_| None).collect();
        for (&pos, result) in rendered_positions.iter().zip(detection_results) {
            detected_by_pos[pos] = Some(result);
        }

        for k in 0..chunk_size {
            let (pw, ph) = chunk_page_meta[k];
            let rotation = page_rotations.get(chunk_start + k).copied().unwrap_or(0);
            let img = chunk_images[k].take().unwrap_or_else(render_failure_placeholder);

            if let Some((detection, _timings)) = detected_by_pos[k].take() {
                let image_width_px = img.width();
                let image_height_px = img.height();

                let hints: Vec<LayoutHint> =
                    pixel_detection_to_layout_hints_pdf_space(&detection, image_width_px, image_height_px, pw, ph);

                tracing::debug!(
                    detections = detection.detections.len(),
                    hints = hints.len(),
                    page_width_pts = pw,
                    page_height_pts = ph,
                    rotation,
                    image_width_px,
                    image_height_px,
                    "layout runner: page detections"
                );

                all_hints.push(hints);
                all_detections.push(detection);
            } else {
                all_hints.push(Vec::new());
                all_detections.push(crate::layout::DetectionResult {
                    page_width: img.width(),
                    page_height: img.height(),
                    detections: Vec::new(),
                });
            }

            all_layout_results.push(PageLayoutResult {
                page_width_pts: pw,
                page_height_pts: ph,
            });
            all_images.push(img);
        }
    }

    crate::layout::return_engine(engine);

    Ok((all_images, all_layout_results, all_hints, all_detections))
}

/// Convenience wrapper that reads `use_layout_for_markdown` and other gate
/// conditions from `config` and, when they are all satisfied, runs
/// [`run_layout_for_pdf_pages`].
///
/// Returns four `None` values when the feature is not requested, or on soft
/// failure (logged as a warning so the markdown path can continue without
/// layout hints). Rendering and inference run off the async executor when a
/// Tokio runtime is enabled.
#[cfg(all(feature = "pdf", feature = "layout-detection"))]
type LayoutForMarkdownOptional = (
    Option<Vec<image::RgbImage>>,
    Option<Vec<PageLayoutResult>>,
    Option<Vec<Vec<LayoutHint>>>,
    Option<Vec<crate::layout::DetectionResult>>,
);

#[cfg(all(feature = "pdf", feature = "layout-detection"))]
pub(super) async fn maybe_run_layout_for_markdown(
    content: &[u8],
    config: &ExtractionConfig,
) -> LayoutForMarkdownOptional {
    if !config.use_layout_for_markdown {
        return (None, None, None, None);
    }
    let Some(ref layout_config) = config.layout else {
        return (None, None, None, None);
    };
    if config.force_ocr {
        return (None, None, None, None);
    }
    match run_layout_for_pdf_pages_async(content, layout_config).await {
        Ok((images, results, hints, detections)) => {
            let total_hints: usize = hints.iter().map(|h| h.len()).sum();
            tracing::info!(
                pages = images.len(),
                total_hints,
                "layout-for-markdown: detection succeeded"
            );
            (Some(images), Some(results), Some(hints), Some(detections))
        }
        Err(e) => {
            tracing::warn!(
                error = %e,
                "layout-for-markdown: detection failed, continuing without layout hints"
            );
            (None, None, None, None)
        }
    }
}

/// Run the layout pass used by OCR without blocking a Tokio worker thread.
#[cfg(all(
    feature = "pdf",
    feature = "layout-detection",
    any(feature = "ocr", feature = "ocr-pipeline")
))]
pub(super) async fn run_layout_for_ocr(
    content: &[u8],
    layout_config: &LayoutDetectionConfig,
) -> Result<LayoutForMarkdownOutput> {
    run_layout_for_pdf_pages_async(content, layout_config).await
}

#[cfg(all(test, feature = "pdf", feature = "layout-detection"))]
mod tests {
    use super::{
        FAILED_RENDER_PLACEHOLDER_SIDE, displayed_page_dimensions, render_failure_placeholder,
        validate_batch_cardinality,
    };

    fn rotated_pdf(inherited: bool) -> Vec<u8> {
        use lopdf::{Document, Object, Stream, dictionary};

        let mut document = Document::with_version("1.5");
        let pages_id = document.new_object_id();
        let page_id = document.new_object_id();
        let content_id = document.add_object(Stream::new(dictionary! {}, Vec::new()));

        let mut page = dictionary! {
            "Type" => "Page",
            "Parent" => pages_id,
            "MediaBox" => vec![0.into(), 0.into(), 200.into(), 100.into()],
            "Resources" => dictionary! {},
            "Contents" => content_id,
        };
        if !inherited {
            page.set("Rotate", 90);
        }
        document.objects.insert(page_id, Object::Dictionary(page));

        let mut pages = dictionary! {
            "Type" => "Pages",
            "Kids" => vec![page_id.into()],
            "Count" => 1,
        };
        if inherited {
            pages.set("Rotate", 90);
        }
        document.objects.insert(pages_id, Object::Dictionary(pages));

        let catalog_id = document.add_object(dictionary! {
            "Type" => "Catalog",
            "Pages" => pages_id,
        });
        document.trailer.set("Root", catalog_id);

        let mut bytes = Vec::new();
        document.save_to(&mut bytes).expect("fixture PDF must serialize");
        bytes
    }

    fn assert_pdf_oxide_applies_rotation(bytes: Vec<u8>) {
        let document = pdf_oxide::PdfDocument::from_bytes(bytes.clone()).expect("fixture PDF must open");
        let rendered =
            crate::pdf::render::render_page_with_safeguards(&document, 0, 72).expect("rotated fixture must render");
        let rotations = crate::pdf::render::get_page_rotations(&bytes, 1);
        let (media_width, media_height) = document
            .get_page_media_box(0)
            .map(|(llx, lly, urx, ury)| ((urx - llx).abs(), (ury - lly).abs()))
            .expect("fixture must have a MediaBox");

        assert_eq!(rotations, vec![90]);
        assert!(
            rendered.height > rendered.width,
            "rotated landscape page must render as portrait"
        );
        assert_eq!(
            displayed_page_dimensions(media_width, media_height, rotations[0]),
            (100.0, 200.0)
        );
    }

    #[test]
    fn batch_cardinality_accepts_one_result_per_rendered_page() {
        assert!(validate_batch_cardinality(3, 3).is_ok());
    }

    #[test]
    fn batch_cardinality_rejects_truncated_results() {
        let error = validate_batch_cardinality(3, 2).expect_err("truncated results must fail");
        assert!(error.to_string().contains("2 results for 3 rendered pages"));
    }

    #[test]
    fn render_failure_placeholder_is_nonempty_and_white() {
        let image = render_failure_placeholder();

        assert_eq!(
            image.dimensions(),
            (FAILED_RENDER_PLACEHOLDER_SIDE, FAILED_RENDER_PLACEHOLDER_SIDE)
        );
        assert!(image.pixels().all(|pixel| pixel.0 == [u8::MAX; 3]));
    }

    #[test]
    fn pdf_oxide_applies_direct_page_rotation() {
        assert_pdf_oxide_applies_rotation(rotated_pdf(false));
    }

    #[test]
    fn pdf_oxide_applies_inherited_page_rotation() {
        assert_pdf_oxide_applies_rotation(rotated_pdf(true));
    }
}
