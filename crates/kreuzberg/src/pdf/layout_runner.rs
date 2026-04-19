//! Layout detection runner for PDF documents.
//!
//! Renders PDF pages to images, runs layout detection via [`LayoutEngine`],
//! and maps pixel-space bounding boxes to PDF coordinate space (points).

use std::cell::RefCell;
use std::time::Instant;

#[cfg(not(target_arch = "wasm32"))]
use rayon::prelude::*;

use crate::layout::{DetectionResult, LayoutClass, LayoutEngine};
use crate::pdf::error::Result;

/// Default number of pages per layout-detection batch.
///
/// A 640×640 RGB image is ~1.2 MB, so 10 pages ≈ 12 MB of raw pixel data per batch.
/// Used by both `detect_layout_for_document` and the OCR layout pass in
/// `run_layout_detection_ocr_pass` so that both paths share the same default.
pub(crate) const DEFAULT_LAYOUT_BATCH_SIZE: usize = 10;

/// Bounding box in PDF coordinate space (points, y=0 at bottom of page).
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct PdfLayoutBBox {
    pub left: f32,
    pub bottom: f32,
    pub right: f32,
    pub top: f32,
}

impl PdfLayoutBBox {
    pub fn width(&self) -> f32 {
        (self.right - self.left).max(0.0)
    }

    pub fn height(&self) -> f32 {
        (self.top - self.bottom).max(0.0)
    }
}

/// A detected layout region mapped to PDF coordinate space.
#[derive(Debug, Clone)]
pub struct PageLayoutRegion {
    pub class: LayoutClass,
    pub confidence: f32,
    pub bbox: PdfLayoutBBox,
}

/// Layout detection results for a single page.
#[derive(Debug, Clone)]
pub struct PageLayoutResult {
    pub page_index: usize,
    pub regions: Vec<PageLayoutRegion>,
    pub page_width_pts: f32,
    pub page_height_pts: f32,
    /// Width of the rendered image used for layout detection (pixels).
    pub render_width_px: u32,
    /// Height of the rendered image used for layout detection (pixels).
    pub render_height_px: u32,
}

/// Timing breakdown for a single page.
#[derive(Debug, Clone)]
pub struct PageTiming {
    /// Time to render the PDF page to a raster image (amortized from batch render).
    pub render_ms: f64,
    /// Time spent in image preprocessing (resize, normalize, tensor construction).
    pub preprocess_ms: f64,
    /// Time for the ONNX model session.run() call (actual neural network inference).
    pub onnx_ms: f64,
    /// Total model inference time (preprocess + onnx), as measured by the engine.
    pub inference_ms: f64,
    /// Time spent in postprocessing (confidence filtering, overlap resolution).
    pub postprocess_ms: f64,
    /// Time to map pixel-space bounding boxes to PDF coordinate space.
    pub mapping_ms: f64,
}

/// Timing breakdown for the entire layout detection run.
#[derive(Debug, Clone)]
pub struct LayoutTimingReport {
    pub total_ms: f64,
    pub per_page: Vec<PageTiming>,
}

impl LayoutTimingReport {
    pub fn avg_render_ms(&self) -> f64 {
        if self.per_page.is_empty() {
            return 0.0;
        }
        self.per_page.iter().map(|p| p.render_ms).sum::<f64>() / self.per_page.len() as f64
    }

    pub fn avg_inference_ms(&self) -> f64 {
        if self.per_page.is_empty() {
            return 0.0;
        }
        self.per_page.iter().map(|p| p.inference_ms).sum::<f64>() / self.per_page.len() as f64
    }

    pub fn avg_preprocess_ms(&self) -> f64 {
        if self.per_page.is_empty() {
            return 0.0;
        }
        self.per_page.iter().map(|p| p.preprocess_ms).sum::<f64>() / self.per_page.len() as f64
    }

    pub fn avg_onnx_ms(&self) -> f64 {
        if self.per_page.is_empty() {
            return 0.0;
        }
        self.per_page.iter().map(|p| p.onnx_ms).sum::<f64>() / self.per_page.len() as f64
    }

    pub fn avg_postprocess_ms(&self) -> f64 {
        if self.per_page.is_empty() {
            return 0.0;
        }
        self.per_page.iter().map(|p| p.postprocess_ms).sum::<f64>() / self.per_page.len() as f64
    }

    pub fn total_inference_ms(&self) -> f64 {
        self.per_page.iter().map(|p| p.inference_ms).sum()
    }

    pub fn total_render_ms(&self) -> f64 {
        self.per_page.iter().map(|p| p.render_ms).sum()
    }

    pub fn total_preprocess_ms(&self) -> f64 {
        self.per_page.iter().map(|p| p.preprocess_ms).sum()
    }

    pub fn total_onnx_ms(&self) -> f64 {
        self.per_page.iter().map(|p| p.onnx_ms).sum()
    }

    pub fn total_postprocess_ms(&self) -> f64 {
        self.per_page.iter().map(|p| p.postprocess_ms).sum()
    }
}

/// Convert a pixel-space bounding box to PDF coordinate space.
///
/// Pixel coordinates: (x1, y1) top-left, (x2, y2) bottom-right, y increases downward.
/// PDF coordinates: (left, bottom, right, top), y=0 at bottom of page, y increases upward.
fn pixel_to_pdf_bbox(
    pixel: &crate::layout::BBox,
    img_width: u32,
    img_height: u32,
    page_width_pts: f32,
    page_height_pts: f32,
) -> PdfLayoutBBox {
    let sx = page_width_pts / img_width as f32;
    let sy = page_height_pts / img_height as f32;
    PdfLayoutBBox {
        left: pixel.x1 * sx,
        right: pixel.x2 * sx,
        // Pixel y1 (top) maps to PDF top (higher y value)
        top: page_height_pts - (pixel.y1 * sy),
        // Pixel y2 (bottom) maps to PDF bottom (lower y value)
        bottom: page_height_pts - (pixel.y2 * sy),
    }
}

/// Convert a [`DetectionResult`] to [`PageLayoutResult`] with PDF coordinates.
fn detection_to_page_result(
    page_index: usize,
    detection: &DetectionResult,
    page_width_pts: f32,
    page_height_pts: f32,
) -> PageLayoutResult {
    let regions = detection
        .detections
        .iter()
        .map(|det| PageLayoutRegion {
            class: det.class,
            confidence: det.confidence,
            bbox: pixel_to_pdf_bbox(
                &det.bbox,
                detection.page_width,
                detection.page_height,
                page_width_pts,
                page_height_pts,
            ),
        })
        .collect();

    PageLayoutResult {
        page_index,
        regions,
        page_width_pts,
        page_height_pts,
        render_width_px: detection.page_width,
        render_height_px: detection.page_height,
    }
}

// Thread-local layout engine for parallel detection.
//
// Each rayon worker thread creates its own `LayoutEngine` on first use,
// amortising the ~1-2 s model-load cost across the pages it processes.
// Memory cost is ~250 MB per active rayon worker thread.
thread_local! {
    static TL_ENGINE: RefCell<Option<LayoutEngine>> = const { RefCell::new(None) };
}

/// Run layout detection on all pages of a PDF document, yielding results in batches.
///
/// This avoids rendering all pages into memory at once. It yields `PageLayoutResult`,
/// the pre-rendered image, and the timings for that batch via a callback.
#[tracing::instrument(skip_all)]
pub fn detect_layout_for_document_batched<F>(
    pdf_bytes: &[u8],
    engine: &mut LayoutEngine,
    batch_size: usize,
    mut callback: F,
) -> Result<LayoutTimingReport>
where
    F: FnMut(Vec<PageLayoutResult>, Vec<PageTiming>, Vec<image::DynamicImage>) -> Result<()>,
{
    let total_start = Instant::now();

    use super::bindings::bind_pdfium;
    use pdfium_render::prelude::*;

    let pdfium = bind_pdfium(
        crate::pdf::error::PdfError::RenderingFailed,
        "layout detection render + dimensions",
        None,
    )?;
    let document = pdfium.load_pdf_from_byte_slice(pdf_bytes, None).map_err(|e| {
        crate::pdf::error::PdfError::InvalidPdf(format!("Failed to load PDF for layout detection: {:?}", e))
    })?;

    let pages = document.pages();
    let page_count = pages.len() as usize;

    // Capture the engine config so each rayon worker can create its own
    // LayoutEngine on first use (thread-local, ~250 MB per worker).
    let engine_config = engine.config().clone();

    // Time budget: 30 s wall-clock overall.
    const MAX_LAYOUT_MS: f64 = 30_000.0;

    let mut all_timings = Vec::with_capacity(page_count);

    // We'll process in chunks of `batch_size`
    for batch_start in (0..page_count).step_by(batch_size) {
        let batch_end = (batch_start + batch_size).min(page_count);

        let elapsed_before = total_start.elapsed().as_secs_f64() * 1000.0;
        if elapsed_before > MAX_LAYOUT_MS {
            tracing::warn!(
                elapsed_ms = elapsed_before,
                total_pages = page_count,
                "Layout detection time budget already exceeded before inference"
            );

            // Just return empty results for the remaining pages
            let mut empty_results = Vec::with_capacity(page_count - batch_start);
            let mut empty_timings = Vec::with_capacity(page_count - batch_start);
            let empty_images = Vec::with_capacity(page_count - batch_start);

            for i in batch_start..page_count {
                // To avoid getting dimensions for dummy return, we just guess or try to get it if cheap
                let (page_w, page_h) = if let Ok(page) = pages.get(i as i32) {
                    (page.width().value, page.height().value)
                } else {
                    (612.0, 792.0)
                };

                empty_results.push(PageLayoutResult {
                    page_index: i,
                    regions: Vec::new(),
                    page_width_pts: page_w,
                    page_height_pts: page_h,
                    render_width_px: 0,
                    render_height_px: 0,
                });

                empty_timings.push(PageTiming {
                    render_ms: 0.0,
                    preprocess_ms: 0.0,
                    onnx_ms: 0.0,
                    inference_ms: 0.0,
                    postprocess_ms: 0.0,
                    mapping_ms: 0.0,
                });
            }

            callback(empty_results, empty_timings.clone(), empty_images)?;
            all_timings.extend(empty_timings);
            break;
        }

        let render_start = Instant::now();
        let mut batch_images = Vec::with_capacity(batch_end - batch_start);
        let mut batch_dimensions = Vec::with_capacity(batch_end - batch_start);

        for i in batch_start..batch_end {
            let page = pages.get(i as i32).map_err(|e| {
                crate::pdf::error::PdfError::RenderingFailed(format!("Failed to get page {}: {:?}", i, e))
            })?;

            let width_pts = page.width().value;
            let height_pts = page.height().value;
            batch_dimensions.push((width_pts, height_pts));

            const MODEL_SIZE: f32 = 640.0;
            let scale = (MODEL_SIZE / width_pts).min(MODEL_SIZE / height_pts);
            let render_w = (width_pts * scale).round() as i32;
            let render_h = (height_pts * scale).round() as i32;

            let config = PdfRenderConfig::new()
                .set_target_width(render_w.max(1))
                .set_target_height(render_h.max(1))
                .rotate_if_landscape(PdfPageRenderRotation::None, false);

            let bitmap = page.render_with_config(&config).map_err(|e| {
                crate::pdf::error::PdfError::RenderingFailed(format!("Failed to render page {}: {}", i, e))
            })?;

            let image = bitmap
                .as_image()
                .map_err(|e| {
                    crate::pdf::error::PdfError::RenderingFailed(format!(
                        "Failed to convert bitmap to image for page {}: {}",
                        i, e
                    ))
                })?
                .into_rgb8();

            batch_images.push(image::DynamicImage::ImageRgb8(image));
        }

        let batch_render_ms = render_start.elapsed().as_secs_f64() * 1000.0;
        let render_ms_per_page = if !batch_images.is_empty() {
            batch_render_ms / batch_images.len() as f64
        } else {
            0.0
        };

        // Run inference in parallel for this batch
        #[cfg(not(target_arch = "wasm32"))]
        let mut parallel_results: Vec<std::result::Result<(PageLayoutResult, PageTiming), String>> = batch_images
            .par_iter()
            .enumerate()
            .map(|(offset, img)| {
                let page_idx = batch_start + offset;
                let rgb = match img {
                    image::DynamicImage::ImageRgb8(r) => std::borrow::Cow::Borrowed(r),
                    other => std::borrow::Cow::Owned(other.to_rgb8()),
                };

                TL_ENGINE.with(|cell| {
                    let mut engine_ref = cell.borrow_mut();
                    if engine_ref.is_none() {
                        let engine = LayoutEngine::from_config(engine_config.clone())
                            .map_err(|e| format!("thread-local LayoutEngine init failed: {e}"))?;
                        *engine_ref = Some(engine);
                    }
                    let tl_engine = engine_ref
                        .as_mut()
                        .ok_or_else(|| "thread-local LayoutEngine missing after init".to_string())?;

                    let inference_start = Instant::now();
                    let (detection, detect_timings) = tl_engine
                        .detect_timed(&rgb)
                        .map_err(|e| format!("Layout detection failed on page {page_idx}: {e}"))?;
                    let inference_ms = inference_start.elapsed().as_secs_f64() * 1000.0;

                    let mapping_start = Instant::now();
                    let (page_w, page_h) = batch_dimensions[offset];
                    let page_result = detection_to_page_result(page_idx, &detection, page_w, page_h);
                    let mapping_ms = mapping_start.elapsed().as_secs_f64() * 1000.0;

                    let timing = PageTiming {
                        render_ms: render_ms_per_page,
                        preprocess_ms: detect_timings.preprocess_ms,
                        onnx_ms: detect_timings.onnx_ms,
                        inference_ms,
                        postprocess_ms: detect_timings.postprocess_ms,
                        mapping_ms,
                    };

                    Ok((page_result, timing))
                })
            })
            .collect();
        #[cfg(target_arch = "wasm32")]
        let mut parallel_results: Vec<std::result::Result<(PageLayoutResult, PageTiming), String>> = batch_images
            .iter()
            .enumerate()
            .map(|(offset, img)| {
                let page_idx = batch_start + offset;
                let rgb = match img {
                    image::DynamicImage::ImageRgb8(r) => std::borrow::Cow::Borrowed(r),
                    other => std::borrow::Cow::Owned(other.to_rgb8()),
                };

                TL_ENGINE.with(|cell| {
                    let mut engine_ref = cell.borrow_mut();
                    if engine_ref.is_none() {
                        let engine = LayoutEngine::from_config(engine_config.clone())
                            .map_err(|e| format!("thread-local LayoutEngine init failed: {e}"))?;
                        *engine_ref = Some(engine);
                    }
                    let tl_engine = engine_ref
                        .as_mut()
                        .ok_or_else(|| "thread-local LayoutEngine missing after init".to_string())?;

                    let inference_start = Instant::now();
                    let (detection, detect_timings) = tl_engine
                        .detect_timed(&rgb)
                        .map_err(|e| format!("Layout detection failed on page {page_idx}: {e}"))?;
                    let inference_ms = inference_start.elapsed().as_secs_f64() * 1000.0;

                    let mapping_start = Instant::now();
                    let (page_w, page_h) = batch_dimensions[offset];
                    let page_result = detection_to_page_result(page_idx, &detection, page_w, page_h);
                    let mapping_ms = mapping_start.elapsed().as_secs_f64() * 1000.0;

                    let timing = PageTiming {
                        render_ms: render_ms_per_page,
                        preprocess_ms: detect_timings.preprocess_ms,
                        onnx_ms: detect_timings.onnx_ms,
                        inference_ms,
                        postprocess_ms: detect_timings.postprocess_ms,
                        mapping_ms,
                    };

                    Ok((page_result, timing))
                })
            })
            .collect();

        parallel_results.sort_by_key(|r| match r {
            Ok((pr, _)) => pr.page_index,
            Err(_) => usize::MAX,
        });

        let mut batch_res = Vec::with_capacity(parallel_results.len());
        let mut batch_timings = Vec::with_capacity(parallel_results.len());

        for r in parallel_results {
            let (pr, pt) = r.map_err(crate::pdf::error::PdfError::RenderingFailed)?;
            batch_res.push(pr);
            batch_timings.push(pt);
        }

        all_timings.extend(batch_timings.clone());
        callback(batch_res, batch_timings, batch_images)?;
    }

    let total_ms = total_start.elapsed().as_secs_f64() * 1000.0;

    let report = LayoutTimingReport {
        total_ms,
        per_page: all_timings,
    };

    Ok(report)
}

/// Run layout detection on all pages of a PDF document.
///
/// Under the hood, this uses batched layout detection to prevent holding too many
/// full-resolution page images in memory simultaneously before detection.
#[tracing::instrument(skip_all)]
pub fn detect_layout_for_document(
    pdf_bytes: &[u8],
    engine: &mut LayoutEngine,
) -> Result<(Vec<PageLayoutResult>, LayoutTimingReport, Vec<image::DynamicImage>)> {
    let mut all_results = Vec::new();
    let mut all_images = Vec::new();

    let batch_size = DEFAULT_LAYOUT_BATCH_SIZE;

    let report = detect_layout_for_document_batched(
        pdf_bytes,
        engine,
        batch_size,
        |batch_res, _batch_timings, batch_imgs| {
            all_results.extend(batch_res);
            all_images.extend(batch_imgs);
            Ok(())
        },
    )?;

    tracing::info!(
        page_count = all_results.len(),
        total_ms = report.total_ms,
        total_render_ms = report.total_render_ms(),
        total_inference_ms = report.total_inference_ms(),
        total_preprocess_ms = report.total_preprocess_ms(),
        total_onnx_ms = report.total_onnx_ms(),
        total_postprocess_ms = report.total_postprocess_ms(),
        avg_render_ms = report.avg_render_ms(),
        avg_preprocess_ms = report.avg_preprocess_ms(),
        avg_onnx_ms = report.avg_onnx_ms(),
        avg_inference_ms = report.avg_inference_ms(),
        avg_postprocess_ms = report.avg_postprocess_ms(),
        "Layout detection complete for document"
    );

    Ok((all_results, report, all_images))
}

/// Run layout detection on pre-rendered images.
///
/// Returns pixel-space [`DetectionResult`]s — no PDF coordinate conversion.
/// Use this when images are already available (e.g., from the OCR rendering
/// path) to avoid redundant PDF re-rendering.
pub fn detect_layout_for_images(
    images: &[image::DynamicImage],
    engine: &mut LayoutEngine,
) -> Result<Vec<DetectionResult>> {
    const LAYOUT_BATCH_SIZE: usize = 4;

    // Pre-convert any non-RGB8 images once so we can borrow them in chunks.
    let rgb_owned: Vec<Option<image::RgbImage>> = images
        .iter()
        .map(|img| match img {
            image::DynamicImage::ImageRgb8(_) => None,
            other => Some(other.to_rgb8()),
        })
        .collect();
    let rgb_refs: Vec<&image::RgbImage> = images
        .iter()
        .zip(rgb_owned.iter())
        .map(|(img, owned)| match owned {
            Some(r) => r,
            None => match img {
                image::DynamicImage::ImageRgb8(r) => r,
                _ => unreachable!(),
            },
        })
        .collect();

    let mut results = Vec::with_capacity(images.len());

    for (chunk_start, chunk) in rgb_refs.chunks(LAYOUT_BATCH_SIZE).enumerate() {
        let page_base = chunk_start * LAYOUT_BATCH_SIZE;
        let batch_results = engine.detect_batch(chunk).map_err(|e| {
            crate::pdf::error::PdfError::RenderingFailed(format!(
                "Layout detection failed on pages {}–{}: {}",
                page_base,
                page_base + chunk.len() - 1,
                e
            ))
        })?;

        for (offset, (detection, _timings)) in batch_results.into_iter().enumerate() {
            tracing::debug!(
                page = page_base + offset,
                detections = detection.detections.len(),
                "Layout detection complete for pre-rendered page"
            );
            results.push(detection);
        }
    }

    Ok(results)
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::layout::BBox;

    #[test]
    fn test_pixel_to_pdf_bbox_full_page() {
        // Full page bounding box: pixel (0,0)-(612,792) for a 612x792 image
        // Should map to PDF (0,0)-(612,792) at 72 DPI (1:1 mapping)
        let pixel = BBox::new(0.0, 0.0, 612.0, 792.0);
        let pdf = pixel_to_pdf_bbox(&pixel, 612, 792, 612.0, 792.0);
        assert!((pdf.left - 0.0).abs() < 0.01);
        assert!((pdf.bottom - 0.0).abs() < 0.01);
        assert!((pdf.right - 612.0).abs() < 0.01);
        assert!((pdf.top - 792.0).abs() < 0.01);
    }

    #[test]
    fn test_pixel_to_pdf_bbox_top_quarter() {
        // Top-left quarter in pixel space: (0,0)-(306,396)
        // In PDF space: left=0, right=306, bottom=396, top=792
        let pixel = BBox::new(0.0, 0.0, 306.0, 396.0);
        let pdf = pixel_to_pdf_bbox(&pixel, 612, 792, 612.0, 792.0);
        assert!((pdf.left - 0.0).abs() < 0.01);
        assert!((pdf.right - 306.0).abs() < 0.01);
        assert!((pdf.top - 792.0).abs() < 0.01, "top should be page top: {}", pdf.top);
        assert!(
            (pdf.bottom - 396.0).abs() < 0.01,
            "bottom should be mid-page: {}",
            pdf.bottom
        );
    }

    #[test]
    fn test_pixel_to_pdf_bbox_bottom_quarter() {
        // Bottom-right quarter in pixel space: (306,396)-(612,792)
        // In PDF space: left=306, right=612, bottom=0, top=396
        let pixel = BBox::new(306.0, 396.0, 612.0, 792.0);
        let pdf = pixel_to_pdf_bbox(&pixel, 612, 792, 612.0, 792.0);
        assert!((pdf.left - 306.0).abs() < 0.01);
        assert!((pdf.right - 612.0).abs() < 0.01);
        assert!((pdf.top - 396.0).abs() < 0.01, "top should be mid-page: {}", pdf.top);
        assert!(
            (pdf.bottom - 0.0).abs() < 0.01,
            "bottom should be page bottom: {}",
            pdf.bottom
        );
    }

    #[test]
    fn test_pixel_to_pdf_bbox_scaled_image() {
        // Image rendered at different resolution than page points
        // Image: 640x640, Page: 612x792
        let pixel = BBox::new(0.0, 0.0, 640.0, 640.0);
        let pdf = pixel_to_pdf_bbox(&pixel, 640, 640, 612.0, 792.0);
        assert!((pdf.left - 0.0).abs() < 0.01);
        assert!((pdf.right - 612.0).abs() < 0.01);
        assert!((pdf.top - 792.0).abs() < 0.01);
        assert!((pdf.bottom - 0.0).abs() < 0.01);
    }

    #[test]
    fn test_pixel_to_pdf_bbox_center_region() {
        // Center region: pixel (160,160)-(480,480) on 640x640 image, page 612x792
        let pixel = BBox::new(160.0, 160.0, 480.0, 480.0);
        let pdf = pixel_to_pdf_bbox(&pixel, 640, 640, 612.0, 792.0);
        // sx = 612/640 = 0.95625, sy = 792/640 = 1.2375
        let sx = 612.0 / 640.0;
        let sy = 792.0 / 640.0;
        assert!((pdf.left - 160.0 * sx).abs() < 0.01);
        assert!((pdf.right - 480.0 * sx).abs() < 0.01);
        assert!((pdf.top - (792.0 - 160.0 * sy)).abs() < 0.01);
        assert!((pdf.bottom - (792.0 - 480.0 * sy)).abs() < 0.01);
    }

    #[test]
    fn test_pixel_to_pdf_bbox_preserves_width() {
        let pixel = BBox::new(100.0, 200.0, 400.0, 500.0);
        let pdf = pixel_to_pdf_bbox(&pixel, 612, 792, 612.0, 792.0);
        // Width should be preserved at 1:1 scale
        let pixel_width = 300.0; // 400 - 100
        assert!((pdf.width() - pixel_width).abs() < 0.01);
    }

    #[test]
    fn test_pixel_to_pdf_bbox_y_flip() {
        // A box near the top in pixel space should be near the top in PDF space (high y)
        let top_pixel = BBox::new(0.0, 0.0, 100.0, 50.0);
        let top_pdf = pixel_to_pdf_bbox(&top_pixel, 612, 792, 612.0, 792.0);
        assert!(
            top_pdf.top > 700.0,
            "Box at pixel-top should have high PDF y: {}",
            top_pdf.top
        );

        // A box near the bottom in pixel space should be near the bottom in PDF space (low y)
        let bottom_pixel = BBox::new(0.0, 742.0, 100.0, 792.0);
        let bottom_pdf = pixel_to_pdf_bbox(&bottom_pixel, 612, 792, 612.0, 792.0);
        assert!(
            bottom_pdf.bottom < 50.0,
            "Box at pixel-bottom should have low PDF y: {}",
            bottom_pdf.bottom
        );
    }

    #[test]
    fn test_pdf_layout_bbox_dimensions() {
        let bbox = PdfLayoutBBox {
            left: 10.0,
            bottom: 20.0,
            right: 110.0,
            top: 120.0,
        };
        assert!((bbox.width() - 100.0).abs() < 0.01);
        assert!((bbox.height() - 100.0).abs() < 0.01);
    }

    #[test]
    fn test_detection_to_page_result() {
        use crate::layout::{DetectionResult, LayoutDetection};

        let detection = DetectionResult::new(
            640,
            640,
            vec![
                LayoutDetection::new(LayoutClass::Title, 0.95, BBox::new(50.0, 30.0, 590.0, 80.0)),
                LayoutDetection::new(LayoutClass::Text, 0.88, BBox::new(50.0, 100.0, 590.0, 600.0)),
            ],
        );

        let result = detection_to_page_result(0, &detection, 612.0, 792.0);
        assert_eq!(result.page_index, 0);
        assert_eq!(result.regions.len(), 2);
        assert_eq!(result.regions[0].class, LayoutClass::Title);
        assert!((result.regions[0].confidence - 0.95).abs() < 0.001);
        // Title should be near the top of the page (high y)
        assert!(result.regions[0].bbox.top > 700.0);
        assert_eq!(result.regions[1].class, LayoutClass::Text);
        assert_eq!(result.render_width_px, 640);
        assert_eq!(result.render_height_px, 640);
    }

    #[test]
    fn test_layout_timing_report() {
        let report = LayoutTimingReport {
            total_ms: 500.0,
            per_page: vec![
                PageTiming {
                    render_ms: 10.0,
                    preprocess_ms: 5.0,
                    onnx_ms: 70.0,
                    inference_ms: 80.0,
                    postprocess_ms: 0.5,
                    mapping_ms: 0.1,
                },
                PageTiming {
                    render_ms: 12.0,
                    preprocess_ms: 6.0,
                    onnx_ms: 74.0,
                    inference_ms: 85.0,
                    postprocess_ms: 0.5,
                    mapping_ms: 0.1,
                },
                PageTiming {
                    render_ms: 11.0,
                    preprocess_ms: 5.5,
                    onnx_ms: 72.0,
                    inference_ms: 82.0,
                    postprocess_ms: 0.5,
                    mapping_ms: 0.1,
                },
            ],
        };
        assert!((report.avg_render_ms() - 11.0).abs() < 0.01);
        assert!((report.avg_inference_ms() - 82.333).abs() < 0.1);
        assert!((report.total_inference_ms() - 247.0).abs() < 0.01);
        assert!((report.total_render_ms() - 33.0).abs() < 0.01);
        assert!((report.avg_preprocess_ms() - 5.5).abs() < 0.01);
        assert!((report.avg_onnx_ms() - 72.0).abs() < 0.01);
        assert!((report.avg_postprocess_ms() - 0.5).abs() < 0.001);
        assert!((report.total_preprocess_ms() - 16.5).abs() < 0.01);
        assert!((report.total_onnx_ms() - 216.0).abs() < 0.01);
        assert!((report.total_postprocess_ms() - 1.5).abs() < 0.001);
    }

    #[test]
    fn test_layout_timing_report_empty() {
        let report = LayoutTimingReport {
            total_ms: 0.0,
            per_page: vec![],
        };
        assert!((report.avg_render_ms()).abs() < 0.001);
        assert!((report.avg_inference_ms()).abs() < 0.001);
    }
}
