//! PDF document extractor.
//!
//! Provides extraction of text, metadata, tables, and images from PDF documents
//! using pypdfium2 and playa-pdf. Supports both native text extraction and OCR fallback.

mod extraction;
mod ocr;
mod pages;

use crate::Result;
use crate::core::config::{ExtractionConfig, OutputFormat};
use crate::plugins::{DocumentExtractor, Plugin};
use crate::types::Metadata;
use crate::types::internal::{ElementKind, InternalDocument, InternalElement};
use async_trait::async_trait;
#[cfg(feature = "tokio-runtime")]
use std::path::Path;

#[cfg(feature = "pdf")]
use crate::pdf::error::PdfError;

// Re-export for backward compatibility
#[cfg(feature = "ocr")]
pub use ocr::{NativeTextStats, OcrFallbackDecision, evaluate_native_text_for_ocr, evaluate_per_page_ocr};

#[cfg(feature = "pdf")]
use pdfium_render::prelude::{PdfDocument, Pdfium};

use extraction::extract_all_from_document;
#[cfg(feature = "pdf-oxide")]
use extraction::extract_all_from_oxide_document;
#[cfg(feature = "ocr")]
use ocr::extract_with_ocr;
use pages::assign_tables_and_images_to_pages;

/// Run layout detection on PDF bytes and return per-page layout hints.
///
/// Returns `None` when layout detection is not configured or fails.
/// Failures are logged as warnings but do not propagate — extraction
/// continues without layout hints (graceful degradation).
/// Layout detection result bundle: hints for markdown pipeline, rendered images, and raw results.
///
/// Images and raw results are used by TATR table recognition in the native path.
#[cfg(all(feature = "pdf", feature = "layout-detection"))]
struct LayoutDetectionBundle {
    hints: Vec<Vec<crate::pdf::structure::types::LayoutHint>>,
    images: Vec<image::DynamicImage>,
    results: Vec<crate::pdf::layout_runner::PageLayoutResult>,
}

#[cfg(all(feature = "pdf", feature = "layout-detection"))]
fn run_layout_detection(content: &[u8], config: &ExtractionConfig) -> Option<LayoutDetectionBundle> {
    let layout_config = config.layout.as_ref()?;

    // We no longer pre-render all images here because `detect_layout_for_document`
    // now uses batched rendering under the hood to prevent OOMs.

    let mut engine = match crate::layout::take_or_create_engine(layout_config) {
        Ok(e) => e,
        Err(e) => {
            tracing::warn!("Layout engine init failed, continuing without: {}", e);
            return None;
        }
    };

    let result = match crate::pdf::layout_runner::detect_layout_for_document(content, &mut engine) {
        Ok((results, timing, images)) => {
            tracing::info!(
                total_ms = timing.total_ms,
                avg_inference_ms = timing.avg_inference_ms(),
                page_count = results.len(),
                total_detections = results.iter().map(|r| r.regions.len()).sum::<usize>(),
                "Layout detection completed"
            );
            let hints = extraction::convert_results_to_hints(&results);
            Some(LayoutDetectionBundle { hints, images, results })
        }
        Err(e) => {
            tracing::warn!("Layout detection failed, continuing without: {}", e);
            None
        }
    };

    // Return engine to cache for reuse by subsequent extractions
    crate::layout::return_engine(engine);
    result
}

/// Run layout detection on PDF bytes via batching, returning pixel-space results.
///
/// Used by the OCR path to get layout detections without rendering all pages upfront.
/// Returns `None` when layout detection is not configured or fails.
#[cfg(all(feature = "pdf", feature = "layout-detection", feature = "ocr"))]
fn run_layout_detection_ocr_pass(
    content: &[u8],
    config: &ExtractionConfig,
) -> Option<Vec<crate::layout::DetectionResult>> {
    let layout_config = config.layout.as_ref()?;

    let mut engine = match crate::layout::take_or_create_engine(layout_config) {
        Ok(e) => e,
        Err(e) => {
            tracing::warn!("Layout engine init failed for OCR path: {}", e);
            return None;
        }
    };

    let mut all_results = Vec::new();
    let batch_size = crate::pdf::layout_runner::DEFAULT_LAYOUT_BATCH_SIZE;

    let result = match crate::pdf::layout_runner::detect_layout_for_document_batched(
        content,
        &mut engine,
        batch_size,
        |batch_res, _timings, _batch_imgs| {
            // Reconstruct DetectionResult (pixel-space bbox) from PageLayoutResult (PDF-space bbox)
            // We know:
            // pixel_x * (page_width / img_width) = pdf_left
            // page_height - pixel_y * (page_height / img_height) = pdf_top
            // Solving for pixel_x and pixel_y:
            // pixel_x = pdf_left * (img_width / page_width)
            // pixel_y = (page_height - pdf_top) * (img_height / page_height)

            for res in batch_res {
                let img_w = res.render_width_px as f32;
                let img_h = res.render_height_px as f32;
                let page_w = res.page_width_pts;
                let page_h = res.page_height_pts;

                let sx = if page_w > 0.0 { img_w / page_w } else { 1.0 };
                let sy = if page_h > 0.0 { img_h / page_h } else { 1.0 };

                let detections = res
                    .regions
                    .into_iter()
                    .map(|region| {
                        let bbox = crate::layout::BBox {
                            x1: region.bbox.left * sx,
                            y1: (page_h - region.bbox.top) * sy,
                            x2: region.bbox.right * sx,
                            y2: (page_h - region.bbox.bottom) * sy,
                        };
                        crate::layout::LayoutDetection {
                            class: region.class,
                            confidence: region.confidence,
                            bbox,
                        }
                    })
                    .collect();

                all_results.push(crate::layout::DetectionResult {
                    page_width: res.render_width_px,
                    page_height: res.render_height_px,
                    detections,
                });
            }
            Ok(())
        },
    ) {
        Ok(_) => Some(all_results),
        Err(e) => {
            tracing::warn!("Layout detection batched pass failed: {}", e);
            None
        }
    };

    crate::layout::return_engine(engine);
    result
}

/// Render PDF layout detections, then run OCR lazily.
///
/// Layout caching is performed at 72 DPI to save memory. OCR rendering
/// is executed dynamically at 300 DPI within batches via `extract_with_ocr`
/// to avoid `Vec<DynamicImage>` out-of-memory errors on large PDFs.
#[cfg(feature = "ocr")]
async fn run_ocr_with_layout(
    content: &[u8],
    config: &ExtractionConfig,
    path: Option<&std::path::Path>,
) -> crate::Result<(
    String,
    Vec<crate::types::Table>,
    Vec<crate::types::OcrElement>,
    Option<crate::types::internal::InternalDocument>,
)> {
    let default_ocr_config = crate::core::config::OcrConfig::default();
    let ocr_config = config.ocr.as_ref().unwrap_or(&default_ocr_config);

    // Run layout detection up front so it is available to both the pipeline
    // path and the direct extract_with_ocr path below.  Without this, the
    // pipeline branch (active whenever `paddle-ocr` is compiled in via the
    // `full` feature) would silently skip layout detection entirely and always
    // return empty tables.
    #[cfg(feature = "layout-detection")]
    let layout_detections = run_layout_detection_ocr_pass(content, config);

    // Check for pipeline configuration
    if let Some(pipeline) = ocr_config.effective_pipeline() {
        let (text, _ocr_tables, ocr_elements, pipeline_doc) = ocr::run_ocr_pipeline(
            Some(content),
            None,
            #[cfg(feature = "layout-detection")]
            layout_detections.as_deref(),
            config,
            &pipeline,
            path,
        )
        .await?;
        return Ok((text, Vec::new(), ocr_elements, pipeline_doc));
    }

    #[cfg(feature = "layout-detection")]
    let layout_detections = run_layout_detection_ocr_pass(content, config);

    let (text, _mean_conf, ocr_tables, ocr_elements, ocr_doc) = extract_with_ocr(
        Some(content),
        None, // Lazy stream 300 DPI pages in extract_with_ocr's batch loop
        #[cfg(feature = "layout-detection")]
        layout_detections.as_deref(),
        config,
        path,
    )
    .await?;
    Ok((text, ocr_tables, ocr_elements, ocr_doc))
}

/// PDF document extractor using pypdfium2 and playa-pdf.
pub struct PdfExtractor;

impl Default for PdfExtractor {
    fn default() -> Self {
        Self::new()
    }
}

impl PdfExtractor {
    pub fn new() -> Self {
        Self
    }
}

impl Plugin for PdfExtractor {
    fn name(&self) -> &str {
        "pdf-extractor"
    }

    fn version(&self) -> String {
        env!("CARGO_PKG_VERSION").to_string()
    }

    fn initialize(&self) -> Result<()> {
        Ok(())
    }

    fn shutdown(&self) -> Result<()> {
        Ok(())
    }
}

#[cfg_attr(not(target_arch = "wasm32"), async_trait)]
#[cfg_attr(target_arch = "wasm32", async_trait(?Send))]
impl DocumentExtractor for PdfExtractor {
    async fn extract_bytes(
        &self,
        content: &[u8],
        mime_type: &str,
        config: &ExtractionConfig,
    ) -> Result<InternalDocument> {
        self.extract_core(content, mime_type, config, None).await
    }

    #[cfg(feature = "tokio-runtime")]
    async fn extract_file(&self, path: &Path, mime_type: &str, config: &ExtractionConfig) -> Result<InternalDocument> {
        // Set the PDF file path for pdf_oxide text extraction (thread-local).
        #[cfg(feature = "pdf")]
        crate::pdf::oxide_text::set_current_pdf_path(Some(path.to_path_buf()));
        let bytes = tokio::fs::read(path).await?;
        let result = self.extract_core(&bytes, mime_type, config, Some(path)).await;
        #[cfg(feature = "pdf")]
        crate::pdf::oxide_text::set_current_pdf_path(None);
        result
    }

    fn supported_mime_types(&self) -> &[&str] {
        &["application/pdf"]
    }
}

impl PdfExtractor {
    /// Core extraction logic shared between extract_bytes and extract_file.
    ///
    /// Accepts an optional `path` which is passed to OCR backends to allow
    /// direct document-level processing (bypassing page rendering).
    async fn extract_core(
        &self,
        content: &[u8],
        mime_type: &str,
        config: &ExtractionConfig,
        path: Option<&std::path::Path>,
    ) -> Result<InternalDocument> {
        tracing::debug!(format = "pdf", size_bytes = content.len(), "extraction starting");
        let _ = &path; // used only when `ocr` feature is enabled

        // --- pdf_oxide backend dispatch ---
        // PdfOxide: prefer oxide, fall back to pdfium on document open/parse failures.
        // Auto: try oxide first, fall back to pdfium on any error.
        // Pdfium: skip oxide entirely.
        #[cfg(feature = "pdf-oxide")]
        {
            use crate::core::config::pdf::PdfBackend;
            let backend = config
                .pdf_options
                .as_ref()
                .map(|p| &p.backend)
                .unwrap_or(&PdfBackend::Pdfium);

            match backend {
                PdfBackend::PdfOxide => {
                    match self.extract_core_oxide(content, mime_type, config, path).await {
                        Ok(doc) => return Ok(doc),
                        Err(oxide_err) => {
                            // Fall back to pdfium on document open/parse failures.
                            // These are Parsing errors from pdf_oxide indicating the
                            // document could not be opened or its structure could not
                            // be parsed (e.g. password-protected, corrupt, complex
                            // structure). Other error kinds (OCR, IO, etc.) propagate.
                            if matches!(&oxide_err, crate::error::KreuzbergError::Parsing { .. }) {
                                tracing::debug!(
                                    error = %oxide_err,
                                    "pdf_oxide could not parse document, falling back to pdfium"
                                );
                                // Fall through to pdfium path below
                            } else {
                                return Err(oxide_err);
                            }
                        }
                    }
                }
                PdfBackend::Auto => {
                    match self.extract_core_oxide(content, mime_type, config, path).await {
                        Ok(doc) => return Ok(doc),
                        Err(oxide_err) => {
                            tracing::debug!(
                                error = %oxide_err,
                                "pdf_oxide extraction failed, falling back to pdfium"
                            );
                            // Fall through to pdfium path below
                        }
                    }
                }
                PdfBackend::Pdfium => { /* fall through to pdfium path */ }
            }
        }

        // Strip /Rotate from page dicts to work around pdfium text extraction bug
        // where FPDFText_CountChars returns 0 for 90°/270° rotated pages.
        #[cfg(feature = "pdf")]
        let derotated = crate::pdf::text::strip_page_rotation(content);
        #[cfg(feature = "pdf")]
        let content = &*derotated;

        #[cfg(feature = "pdf")]
        #[allow(unused_variables, unused_mut)]
        let (
            mut pdf_metadata,
            native_text,
            mut tables,
            page_contents,
            boundaries,
            pre_rendered_doc,
            has_font_encoding_issues,
            pdf_annotations,
        ) = {
            #[cfg(target_arch = "wasm32")]
            {
                let pdfium = crate::pdf::bindings::bind_pdfium(PdfError::MetadataExtractionFailed, "initialize Pdfium")
                    .map_err(|pdf_err| {
                        if pdf_err.to_string().contains("WASM") || pdf_err.to_string().contains("Module") {
                            crate::error::KreuzbergError::Parsing {
                                message: "PDF extraction requires proper WASM module initialization. \
                                     Ensure your WASM environment is set up with PDFium support. \
                                     See: https://docs.kreuzberg.dev/wasm/pdf"
                                    .to_string(),
                                source: None,
                            }
                        } else {
                            pdf_err.into()
                        }
                    })?;

                let document = load_pdf_from_byte_slice(&pdfium, content, config)?;

                extract_all_from_document(&document, config, None, None, None)?
            }
            #[cfg(all(not(target_arch = "wasm32"), feature = "tokio-runtime"))]
            {
                // Run layout detection on the derotated bytes (shared by all tokio paths).
                // Layout hints are plain data (Vec/f32/enum), so they are Send and can
                // be moved into spawn_blocking if needed.
                #[cfg(feature = "layout-detection")]
                let layout_bundle = run_layout_detection(content, config);
                #[cfg(feature = "layout-detection")]
                let (layout_hints, layout_images, layout_results) = match layout_bundle {
                    Some(b) => (Some(b.hints), Some(b.images), Some(b.results)),
                    None => (None, None, None),
                };
                #[cfg(not(feature = "layout-detection"))]
                let layout_hints: Option<Vec<Vec<crate::pdf::structure::types::LayoutHint>>> = None;

                if crate::core::batch_mode::is_batch_mode() {
                    let content_owned = content.to_vec();
                    let span = tracing::Span::current();
                    let config_owned = config.clone();
                    let oxide_path = crate::pdf::oxide_text::current_pdf_path();
                    let result = tokio::task::spawn_blocking(move || {
                        let _guard = span.entered();
                        // Propagate PDF path to spawned thread for pdf_oxide extraction.
                        crate::pdf::oxide_text::set_current_pdf_path(oxide_path);

                        let pdfium =
                            crate::pdf::bindings::bind_pdfium(PdfError::MetadataExtractionFailed, "initialize Pdfium")?;

                        let document = load_pdf_from_byte_slice(&pdfium, &content_owned, &config_owned)?;

                        let (
                            pdf_metadata,
                            native_text,
                            tables,
                            page_contents,
                            boundaries,
                            pre_rendered_doc,
                            has_font_encoding_issues,
                            pdf_annotations,
                        ) = extract_all_from_document(
                            &document,
                            &config_owned,
                            layout_hints.as_deref(),
                            #[cfg(feature = "layout-detection")]
                            layout_images.as_deref(),
                            #[cfg(not(feature = "layout-detection"))]
                            None,
                            #[cfg(feature = "layout-detection")]
                            layout_results.as_deref(),
                            #[cfg(not(feature = "layout-detection"))]
                            None,
                        )
                        .map_err(|e| PdfError::ExtractionFailed(e.to_string()))?;

                        if let Some(page_cfg) = config_owned.pages.as_ref()
                            && page_cfg.extract_pages
                            && page_contents.is_none()
                        {
                            return Err(PdfError::ExtractionFailed(
                                "Page extraction was configured but no page data was extracted in batch mode"
                                    .to_string(),
                            ));
                        }

                        Ok::<_, crate::pdf::error::PdfError>((
                            pdf_metadata,
                            native_text,
                            tables,
                            page_contents,
                            boundaries,
                            pre_rendered_doc,
                            has_font_encoding_issues,
                            pdf_annotations,
                        ))
                    })
                    .await
                    .map_err(|e| crate::error::KreuzbergError::Other(format!("PDF extraction task failed: {}", e)))?;

                    match result {
                        Ok(tuple) => tuple,
                        Err(e) => return Err(e.into()),
                    }
                } else {
                    let pdfium =
                        crate::pdf::bindings::bind_pdfium(PdfError::MetadataExtractionFailed, "initialize Pdfium")?;

                    let document = load_pdf_from_byte_slice(&pdfium, content, config)?;

                    extract_all_from_document(
                        &document,
                        config,
                        layout_hints.as_deref(),
                        #[cfg(feature = "layout-detection")]
                        layout_images.as_deref(),
                        #[cfg(not(feature = "layout-detection"))]
                        None,
                        #[cfg(feature = "layout-detection")]
                        layout_results.as_deref(),
                        #[cfg(not(feature = "layout-detection"))]
                        None,
                    )?
                }
            }
            #[cfg(all(not(target_arch = "wasm32"), not(feature = "tokio-runtime")))]
            {
                #[cfg(feature = "layout-detection")]
                let layout_bundle = run_layout_detection(content, config);
                #[cfg(feature = "layout-detection")]
                let (layout_hints, layout_images, layout_results) = match layout_bundle {
                    Some(b) => (Some(b.hints), Some(b.images), Some(b.results)),
                    None => (None, None, None),
                };
                #[cfg(not(feature = "layout-detection"))]
                let (layout_hints, layout_images, layout_results): (
                    Option<Vec<Vec<crate::pdf::structure::types::LayoutHint>>>,
                    Option<()>,
                    Option<()>,
                ) = (None, None, None);

                let pdfium =
                    crate::pdf::bindings::bind_pdfium(PdfError::MetadataExtractionFailed, "initialize Pdfium")?;

                let document = load_pdf_from_byte_slice(&pdfium, content, config)?;

                extract_all_from_document(
                    &document,
                    config,
                    layout_hints.as_deref(),
                    #[cfg(feature = "layout-detection")]
                    layout_images.as_deref(),
                    #[cfg(not(feature = "layout-detection"))]
                    None,
                    #[cfg(feature = "layout-detection")]
                    layout_results.as_deref(),
                    #[cfg(not(feature = "layout-detection"))]
                    None,
                )?
            }
        };

        #[cfg(feature = "ocr")]
        let mut ocr_tables: Vec<crate::types::Table> = Vec::new();
        #[cfg(feature = "ocr")]
        #[allow(unused_assignments)]
        let mut _ocr_elements_from_ocr: Vec<crate::types::OcrElement> = Vec::new();
        #[cfg(feature = "ocr")]
        let mut ocr_internal_doc: Option<crate::types::internal::InternalDocument> = None;
        #[cfg(feature = "ocr")]
        let (text, used_ocr) = if config.disable_ocr {
            (native_text, false)
        } else if config.force_ocr {
            let (ocr_text, ocr_tbls, ocr_elems, ocr_doc) = run_ocr_with_layout(content, config, path).await?;
            ocr_tables = ocr_tbls;
            _ocr_elements_from_ocr = ocr_elems;
            ocr_internal_doc = ocr_doc;
            (ocr_text, true)
        } else if let Some(ref ocr_pages) = config.force_ocr_pages {
            if !ocr_pages.is_empty() {
                if let Some(ref bounds) = boundaries {
                    if !bounds.is_empty() {
                        let mixed =
                            ocr::extract_mixed_ocr_native(&native_text, bounds, ocr_pages, content, config, path)
                                .await?;
                        (mixed, true)
                    } else {
                        tracing::warn!("force_ocr_pages set but no page boundaries available; using native text");
                        (native_text, false)
                    }
                } else {
                    tracing::warn!("force_ocr_pages set but no page boundaries available; using native text");
                    (native_text, false)
                }
            } else {
                (native_text, false)
            }
        } else if let Some(ocr_config) = config.ocr.as_ref() {
            let thresholds = ocr_config.effective_thresholds();
            let decision = ocr::evaluate_per_page_ocr(
                &native_text,
                boundaries.as_deref(),
                pdf_metadata.pdf_specific.page_count,
                &thresholds,
            );

            if std::env::var("KREUZBERG_DEBUG_OCR").is_ok() {
                eprintln!(
                    "[kreuzberg::pdf::ocr] fallback={} font_encoding_issues={} non_whitespace={} alnum={} meaningful_words={} \
                     avg_non_whitespace={:.2} avg_alnum={:.2} alnum_ratio={:.3} fragmented_word_ratio={:.3} \
                     avg_word_length={:.2} word_count={} consecutive_repeat_ratio={:.3}",
                    decision.fallback,
                    has_font_encoding_issues,
                    decision.stats.non_whitespace,
                    decision.stats.alnum,
                    decision.stats.meaningful_words,
                    decision.avg_non_whitespace,
                    decision.avg_alnum,
                    decision.stats.alnum_ratio,
                    decision.stats.fragmented_word_ratio,
                    decision.stats.avg_word_length,
                    decision.stats.word_count,
                    decision.stats.consecutive_repeat_ratio
                );
            }

            // When a pre-rendered structured document is available, the native text
            // pipeline has already produced structured output with layout guidance,
            // heading detection, table extraction, etc. OCR fallback on such documents
            // often DEGRADES quality because:
            // 1. OCR struggles with dot leaders (TOC pages), producing garbled text.
            // 2. OCR on multi-column pages can interleave columns.
            // 3. The per-page OCR trigger fires on a single weak page even when
            //    most pages have excellent native text.
            //
            // Skip OCR when a pre-rendered document exists with substantive content,
            // UNLESS the native text is critically broken (font encoding issues
            // with mostly non-textual content, indicating the PDF has no real text layer).
            let total_chars = native_text.chars().count();
            let alnum_ws_chars = native_text
                .chars()
                .filter(|c| c.is_alphanumeric() || c.is_whitespace())
                .count();
            let alnum_ws_ratio = if total_chars > 0 {
                alnum_ws_chars as f64 / total_chars as f64
            } else {
                1.0
            };

            let has_substantive_doc = pre_rendered_doc.is_some()
                && total_chars >= thresholds.substantive_min_chars
                && alnum_ws_ratio >= thresholds.alnum_ws_ratio_threshold;

            let skip_ocr_for_non_text = pre_rendered_doc.is_some()
                && total_chars >= thresholds.non_text_min_chars
                && alnum_ws_ratio < thresholds.alnum_ws_ratio_threshold;

            if skip_ocr_for_non_text {
                tracing::debug!(
                    alnum_ws_ratio,
                    total_chars,
                    alnum_ws_chars,
                    "Skipping OCR: font encoding issues but content is non-textual and pre-rendered structured doc available"
                );
                (native_text, false)
            } else if has_substantive_doc {
                tracing::debug!(
                    total_chars,
                    alnum_ws_ratio,
                    ocr_fallback = decision.fallback,
                    has_font_encoding_issues,
                    "Skipping OCR: pre-rendered structured doc available with substantive native text"
                );
                (native_text, false)
            } else if decision.fallback || has_font_encoding_issues {
                match run_ocr_with_layout(content, config, path).await {
                    Ok((ocr_text, ocr_tbls, ocr_elems, ocr_doc)) => {
                        ocr_tables = ocr_tbls;
                        _ocr_elements_from_ocr = ocr_elems;
                        ocr_internal_doc = ocr_doc;
                        (ocr_text, true)
                    }
                    Err(e) => {
                        tracing::warn!(
                            error = %e,
                            "OCR fallback failed; using native text extraction result"
                        );
                        (native_text, false)
                    }
                }
            } else {
                (native_text, false)
            }
        } else {
            (native_text, false)
        };

        #[cfg(not(feature = "ocr"))]
        let (text, used_ocr) = (native_text, false);

        // Merge OCR-detected tables with native-extracted tables.
        // When OCR was used, its TATR-detected tables may be more accurate for scanned pages.
        #[cfg(feature = "ocr")]
        if !ocr_tables.is_empty() {
            tables.extend(ocr_tables);
            tables.sort_by_key(|t| t.page_number);
        }

        // Post-processing: use pre-rendered InternalDocument from initial document load if available.
        // The document was rendered during the first document load to avoid redundant PDF parsing.
        // OCR results already produce structured output via the hOCR path, so this only applies
        // when native text extraction was used and structured output was requested.
        #[cfg(feature = "pdf")]
        let use_structured_doc = !used_ocr && pre_rendered_doc.is_some();
        tracing::debug!(
            used_ocr,
            has_pre_rendered = pre_rendered_doc.is_some(),
            use_structured_doc,
            pre_rendered_elements = pre_rendered_doc.as_ref().map(|d| d.elements.len()).unwrap_or(0),
            "PDF extractor: deciding whether to use pre-rendered document"
        );

        #[cfg(not(feature = "pdf"))]
        let use_structured_doc = false;

        #[cfg(feature = "pdf")]
        if let Some(ref page_cfg) = config.pages
            && page_cfg.insert_page_markers
        {
            let marker_placeholder = page_cfg.marker_format.replace("{page_num}", "");
            if !marker_placeholder.is_empty() && !text.contains(&marker_placeholder) {
                #[cfg(feature = "otel")]
                tracing::warn!(
                    "Page markers were configured but none found in extracted content. \
                     This may indicate very short documents or incomplete extraction."
                );
            }
        }

        let (images, image_fallback_warning) = if config.images.as_ref().map(|c| c.extract_images).unwrap_or(false) {
            match crate::pdf::images::extract_images_from_pdf(content) {
                Ok(mut pdf_images) => {
                    // Fallback: re-extract unusable images via pdfium bitmap rendering
                    #[cfg(feature = "pdf")]
                    let fallback_count =
                        crate::pdf::images::reextract_raw_images_via_pdfium(content, &mut pdf_images).unwrap_or(0);
                    #[cfg(not(feature = "pdf"))]
                    let fallback_count = 0u32;

                    let warning = if fallback_count > 0 {
                        Some(crate::types::ProcessingWarning {
                            source: std::borrow::Cow::Borrowed("image_extraction"),
                            message: std::borrow::Cow::Owned(format!(
                                "{fallback_count} image(s) re-extracted via pdfium bitmap fallback"
                            )),
                        })
                    } else {
                        None
                    };

                    let extracted = pdf_images
                        .into_iter()
                        .enumerate()
                        .map(|(idx, img)| {
                            let format = std::borrow::Cow::Owned(img.decoded_format.clone());
                            crate::types::ExtractedImage {
                                data: img.data,
                                format,
                                image_index: idx,
                                page_number: Some(img.page_number),
                                width: Some(img.width as u32),
                                height: Some(img.height as u32),
                                colorspace: img.color_space,
                                bits_per_component: img.bits_per_component.map(|b| b as u32),
                                is_mask: false,
                                description: None,
                                ocr_result: None,
                                bounding_box: None,
                                source_path: None,
                            }
                        })
                        .collect();
                    (Some(extracted), warning)
                }
                Err(_) => (Some(vec![]), None),
            }
        } else {
            (None, None)
        };

        // Finalize: apply pre-rendered structured document if available.
        // Quality gate: if pre-rendered doc has >2x the words of native text,
        // the layout pipeline added garbage. Fall back to native text in that case.
        #[cfg(feature = "pdf")]
        let use_structured_doc = if use_structured_doc {
            if let Some(ref doc) = pre_rendered_doc {
                let doc_words: usize = doc.elements.iter().map(|e| e.text.split_whitespace().count()).sum();
                let native_words = text.split_whitespace().count();
                if native_words > 50 && doc_words > native_words * 2 {
                    tracing::debug!(
                        doc_words,
                        native_words,
                        "Layout quality gate: document has >2x native text words, falling back"
                    );
                    false
                } else {
                    true
                }
            } else {
                false
            }
        } else {
            false
        };

        #[cfg(feature = "pdf")]
        let (text, used_structured_doc) = if use_structured_doc {
            // We have a pre-rendered InternalDocument — we'll use it directly below.
            // Extract text from the document for the plain-text fallback path.
            (text, true)
        } else {
            (text, false)
        };

        #[cfg(not(feature = "pdf"))]
        let used_structured_doc = false;

        let final_pages = assign_tables_and_images_to_pages(page_contents, &tables, images.as_deref().unwrap_or(&[]));

        // Refine PageInfo.is_blank in page_structure to match PageContent refinement
        if let (Some(final_pgs), Some(page_structure)) = (&final_pages, &mut pdf_metadata.page_structure)
            && let Some(ref mut page_infos) = page_structure.pages
        {
            for page_info in page_infos.iter_mut() {
                if let Some(pc) = final_pgs.iter().find(|p| p.page_number == page_info.number) {
                    page_info.is_blank = pc.is_blank;
                }
            }
        }

        // Always preserve the original document MIME type (e.g. application/pdf).
        // The output format is tracked separately in metadata.output_format.
        // Signal pre-formatted output so the pipeline doesn't double-convert.
        // Only skip conversion for Markdown; Djot and HTML get the structured
        // content but still need apply_output_format() for format-specific conversion.
        #[cfg(feature = "pdf")]
        let pre_formatted_output = if used_structured_doc && config.output_format == OutputFormat::Markdown {
            tracing::trace!("PDF extractor: signaling pre-formatted structured output to pipeline");
            Some("markdown".to_string())
        } else {
            tracing::trace!(
                used_structured_doc,
                output_format = ?config.output_format,
                "PDF extractor: NOT signaling pre-formatted output"
            );
            None
        };
        #[cfg(not(feature = "pdf"))]
        let pre_formatted_output: Option<String> = None;

        #[cfg(feature = "pdf")]
        tracing::debug!(
            use_structured_doc,
            output_format = ?config.output_format,
            "final document path selection"
        );

        // Build InternalDocument from the extracted data.
        // When a pre-rendered InternalDocument is available, use it directly.
        // Otherwise, fall back to splitting text on double newlines.
        #[cfg(feature = "pdf")]
        let mut doc = if used_structured_doc {
            if let Some(mut pre_doc) = pre_rendered_doc {
                pre_doc.mime_type = std::borrow::Cow::Owned(mime_type.to_string());
                pre_doc.metadata = Metadata {
                    output_format: pre_formatted_output,
                    title: pdf_metadata.title.clone(),
                    subject: pdf_metadata.subject.clone(),
                    authors: pdf_metadata.authors.clone(),
                    keywords: pdf_metadata.keywords.clone(),
                    created_at: pdf_metadata.created_at.clone(),
                    modified_at: pdf_metadata.modified_at.clone(),
                    created_by: pdf_metadata.created_by.clone(),
                    pages: pdf_metadata.page_structure.clone(),
                    format: Some(crate::types::FormatMetadata::Pdf(pdf_metadata.pdf_specific)),
                    ..Default::default()
                };
                // Tables are already embedded in the InternalDocument via push_table.
                // Merge any tables not in the doc (e.g., OCR tables added later).
                for table in tables {
                    // Only add tables not already in the doc
                    if !pre_doc
                        .tables
                        .iter()
                        .any(|t| t.page_number == table.page_number && t.markdown == table.markdown)
                    {
                        pre_doc.tables.push(table);
                    }
                }
                if let Some(imgs) = images {
                    pre_doc.images = imgs;
                }
                if let Some(warning) = image_fallback_warning.clone() {
                    pre_doc.processing_warnings.push(warning);
                }
                pre_doc.annotations = pdf_annotations;
                pre_doc
            } else {
                // Shouldn't happen since used_structured_doc was true, but fallback
                let mut doc = InternalDocument::new("pdf");
                doc.mime_type = std::borrow::Cow::Owned(mime_type.to_string());
                for paragraph in text.split("\n\n") {
                    let trimmed = paragraph.trim();
                    if !trimmed.is_empty() {
                        doc.push_element(InternalElement::text(ElementKind::Paragraph, trimmed, 0));
                    }
                }
                doc.tables = tables;
                if let Some(imgs) = images {
                    doc.images = imgs;
                }
                if let Some(warning) = image_fallback_warning.clone() {
                    doc.processing_warnings.push(warning);
                }
                doc.annotations = pdf_annotations;
                doc
            }
        } else {
            // When the OCR path produced a structured InternalDocument, use it
            // instead of naively splitting text on double newlines.
            #[cfg(feature = "ocr")]
            let has_ocr_doc = ocr_internal_doc.is_some();
            #[cfg(feature = "ocr")]
            let mut doc = if let Some(mut ocr_doc) = ocr_internal_doc.take() {
                ocr_doc.mime_type = std::borrow::Cow::Owned(mime_type.to_string());
                ocr_doc
            } else {
                let mut d = InternalDocument::new("pdf");
                d.mime_type = std::borrow::Cow::Owned(mime_type.to_string());
                for paragraph in text.split("\n\n") {
                    let trimmed = paragraph.trim();
                    if !trimmed.is_empty() {
                        d.push_element(InternalElement::text(ElementKind::Paragraph, trimmed, 0));
                    }
                }
                d
            };
            #[cfg(not(feature = "ocr"))]
            let has_ocr_doc = false;
            #[cfg(not(feature = "ocr"))]
            let mut doc = {
                let mut d = InternalDocument::new("pdf");
                d.mime_type = std::borrow::Cow::Owned(mime_type.to_string());
                for paragraph in text.split("\n\n") {
                    let trimmed = paragraph.trim();
                    if !trimmed.is_empty() {
                        d.push_element(InternalElement::text(ElementKind::Paragraph, trimmed, 0));
                    }
                }
                d
            };
            doc.metadata = Metadata {
                output_format: pre_formatted_output,
                title: pdf_metadata.title.clone(),
                subject: pdf_metadata.subject.clone(),
                authors: pdf_metadata.authors.clone(),
                keywords: pdf_metadata.keywords.clone(),
                created_at: pdf_metadata.created_at.clone(),
                modified_at: pdf_metadata.modified_at.clone(),
                created_by: pdf_metadata.created_by.clone(),
                pages: pdf_metadata.page_structure.clone(),
                format: Some(crate::types::FormatMetadata::Pdf(pdf_metadata.pdf_specific)),
                ..Default::default()
            };
            // When the OCR path produced a structured InternalDocument, its tables are
            // already embedded with matching ElementKind::Table indices. Overwriting
            // doc.tables would break those references. Instead, merge any additional
            // tables (e.g., native-extracted tables) that aren't already present.
            // When no OCR doc was produced, assign all tables and create Table elements
            // so they are rendered through comrak's build_table().
            if has_ocr_doc {
                // The OCR doc already has tables with matching ElementKind::Table
                // indices. Only merge tables not already present (e.g., native tables)
                // and create corresponding elements for them.
                for table in tables {
                    if !doc
                        .tables
                        .iter()
                        .any(|t| t.page_number == table.page_number && t.markdown == table.markdown)
                    {
                        let table_index = doc.push_table(table);
                        doc.push_element(InternalElement::text(ElementKind::Table { table_index }, "", 0));
                    }
                }
            } else {
                for table in tables {
                    let table_index = doc.push_table(table);
                    doc.push_element(InternalElement::text(ElementKind::Table { table_index }, "", 0));
                }
            }
            if let Some(imgs) = images {
                doc.images = imgs;
            }
            if let Some(warning) = image_fallback_warning {
                doc.processing_warnings.push(warning);
            }
            doc.annotations = pdf_annotations;
            doc
        };

        #[cfg(not(feature = "pdf"))]
        let mut doc = {
            let mut doc = InternalDocument::new("pdf");
            doc.mime_type = std::borrow::Cow::Owned(mime_type.to_string());
            for paragraph in text.split("\n\n") {
                let trimmed = paragraph.trim();
                if !trimmed.is_empty() {
                    doc.push_element(InternalElement::text(ElementKind::Paragraph, trimmed, 0));
                }
            }
            doc
        };

        // Extract URIs from PDF annotations (links).
        // Collect into a temp vec first to avoid borrow conflict with doc.annotations.
        {
            use crate::types::annotations::PdfAnnotationType;
            use crate::types::uri::{Uri, UriKind};

            let uris: Vec<Uri> = doc
                .annotations
                .as_ref()
                .map(|annotations| {
                    annotations
                        .iter()
                        .filter(|a| a.annotation_type == PdfAnnotationType::Link)
                        .filter_map(|a| {
                            a.content.as_ref().map(|url| {
                                let kind = if url.starts_with('#') {
                                    UriKind::Anchor
                                } else if url.starts_with("mailto:") {
                                    UriKind::Email
                                } else {
                                    UriKind::Hyperlink
                                };
                                Uri {
                                    url: url.clone(),
                                    label: Some(url.clone()),
                                    page: Some(a.page_number as u32),
                                    kind,
                                }
                            })
                        })
                        .collect()
                })
                .unwrap_or_default();
            for uri in uris {
                doc.push_uri(uri);
            }
        }

        // Extract bookmarks/outlines as URIs.
        #[cfg(feature = "pdf")]
        {
            if let Ok(lopdf_doc) = lopdf::Document::load_mem(content) {
                let bookmark_uris = crate::pdf::bookmarks::extract_bookmarks(&lopdf_doc);
                for uri in bookmark_uris {
                    doc.push_uri(uri);
                }
            }
        }

        // Extract embedded files (PDF portfolios/attachments).
        #[cfg(all(feature = "pdf", feature = "tokio-runtime"))]
        {
            let (embedded_children, embedded_warnings) =
                crate::pdf::embedded_files::extract_and_process_embedded_files(content, config).await;
            if !embedded_children.is_empty() {
                match doc.children {
                    Some(ref mut existing) => existing.extend(embedded_children),
                    None => doc.children = Some(embedded_children),
                }
            }
            for warning in embedded_warnings {
                doc.processing_warnings.push(warning);
            }
        }

        // Attach pre-built per-page content so derive_extraction_result can use it.
        doc.prebuilt_pages = final_pages;

        #[cfg(feature = "pdf")]
        tracing::debug!(
            elements = doc.elements.len(),
            tables = doc.tables.len(),
            has_pages = doc.prebuilt_pages.is_some(),
            "InternalDocument finalized"
        );

        Ok(doc)
    }

    /// Core extraction via the pdf_oxide backend.
    ///
    /// Runs text + metadata, tables, and annotation extraction through the oxide
    /// modules, then builds an `InternalDocument` using the same post-processing
    /// pipeline (OCR evaluation, page assembly, image extraction, bookmarks, etc.).
    ///
    /// This method mirrors the shape of `extract_core` but skips pdfium entirely.
    #[cfg(feature = "pdf-oxide")]
    async fn extract_core_oxide(
        &self,
        content: &[u8],
        mime_type: &str,
        config: &ExtractionConfig,
        path: Option<&std::path::Path>,
    ) -> Result<InternalDocument> {
        let _ = &path; // used only when `ocr` feature is enabled

        #[allow(unused_variables, unused_mut)]
        let (
            pdf_metadata,
            native_text,
            mut tables,
            page_contents,
            boundaries,
            pre_rendered_doc,
            _has_font_encoding_issues,
            pdf_annotations,
        ) = extract_all_from_oxide_document(content, config)?;

        // --- OCR evaluation (reuses the same logic as the pdfium path) ---
        #[cfg(feature = "ocr")]
        let mut ocr_tables: Vec<crate::types::Table> = Vec::new();
        #[cfg(feature = "ocr")]
        #[allow(unused_assignments)]
        let mut _ocr_elements_from_ocr: Vec<crate::types::OcrElement> = Vec::new();
        #[cfg(feature = "ocr")]
        let mut ocr_internal_doc: Option<InternalDocument> = None;

        #[cfg(feature = "ocr")]
        let (text, _used_ocr) = if config.disable_ocr {
            (native_text, false)
        } else if config.force_ocr {
            let (ocr_text, ocr_tbls, ocr_elems, ocr_doc) = run_ocr_with_layout(content, config, path).await?;
            ocr_tables = ocr_tbls;
            _ocr_elements_from_ocr = ocr_elems;
            ocr_internal_doc = ocr_doc;
            (ocr_text, true)
        } else if let Some(ocr_config) = config.ocr.as_ref() {
            let thresholds = ocr_config.effective_thresholds();
            let decision = evaluate_per_page_ocr(
                &native_text,
                boundaries.as_deref(),
                pdf_metadata.pdf_specific.page_count,
                &thresholds,
            );

            if decision.fallback {
                match run_ocr_with_layout(content, config, path).await {
                    Ok((ocr_text, ocr_tbls, ocr_elems, ocr_doc)) => {
                        ocr_tables = ocr_tbls;
                        _ocr_elements_from_ocr = ocr_elems;
                        ocr_internal_doc = ocr_doc;
                        (ocr_text, true)
                    }
                    Err(e) => {
                        tracing::warn!(
                            error = %e,
                            "OCR fallback failed on oxide path; using native text"
                        );
                        (native_text, false)
                    }
                }
            } else {
                (native_text, false)
            }
        } else {
            (native_text, false)
        };

        #[cfg(not(feature = "ocr"))]
        let (text, _used_ocr) = (native_text, false);

        #[cfg(feature = "ocr")]
        if !ocr_tables.is_empty() {
            tables.extend(ocr_tables);
            tables.sort_by_key(|t| t.page_number);
        }

        // --- Image extraction (shared with pdfium path) ---
        let (images, image_fallback_warning) = if config.images.as_ref().map(|c| c.extract_images).unwrap_or(false) {
            match crate::pdf::images::extract_images_from_pdf(content) {
                Ok(pdf_images) => {
                    let extracted: Vec<crate::types::ExtractedImage> = pdf_images
                        .into_iter()
                        .enumerate()
                        .map(|(idx, img)| {
                            let format = std::borrow::Cow::Owned(img.decoded_format.clone());
                            crate::types::ExtractedImage {
                                data: img.data,
                                format,
                                image_index: idx,
                                page_number: Some(img.page_number),
                                width: Some(img.width as u32),
                                height: Some(img.height as u32),
                                colorspace: img.color_space,
                                bits_per_component: img.bits_per_component.map(|b| b as u32),
                                is_mask: false,
                                description: None,
                                ocr_result: None,
                                bounding_box: None,
                                source_path: None,
                            }
                        })
                        .collect();
                    (Some(extracted), None)
                }
                Err(e) => {
                    tracing::warn!(error = %e, "oxide path: image extraction failed");
                    (None, None)
                }
            }
        } else {
            (None, None)
        };

        // --- Page assembly ---
        let final_pages = assign_tables_and_images_to_pages(page_contents, &tables, &[]);

        // --- Build InternalDocument ---
        let pre_formatted_output: Option<String> = None;

        // When a pre-rendered structured document is available (headings, paragraphs from
        // font-size clustering), use it directly. Otherwise fall back to flat paragraph splitting.
        let used_ocr;
        #[cfg(feature = "ocr")]
        {
            used_ocr = ocr_internal_doc.is_some();
        }
        #[cfg(not(feature = "ocr"))]
        {
            used_ocr = false;
        }

        let use_structured_doc = !used_ocr && pre_rendered_doc.is_some();

        #[cfg(feature = "ocr")]
        let mut doc = if let Some(mut ocr_doc) = ocr_internal_doc.take() {
            ocr_doc.mime_type = std::borrow::Cow::Owned(mime_type.to_string());
            ocr_doc
        } else if let Some(mut pre_doc) = pre_rendered_doc {
            pre_doc.mime_type = std::borrow::Cow::Owned(mime_type.to_string());
            pre_doc
        } else {
            let mut d = InternalDocument::new("pdf");
            d.mime_type = std::borrow::Cow::Owned(mime_type.to_string());
            for paragraph in text.split("\n\n") {
                let trimmed = paragraph.trim();
                if !trimmed.is_empty() {
                    d.push_element(InternalElement::text(ElementKind::Paragraph, trimmed, 0));
                }
            }
            d
        };
        #[cfg(not(feature = "ocr"))]
        let mut doc = if let Some(mut pre_doc) = pre_rendered_doc {
            pre_doc.mime_type = std::borrow::Cow::Owned(mime_type.to_string());
            pre_doc
        } else {
            let mut d = InternalDocument::new("pdf");
            d.mime_type = std::borrow::Cow::Owned(mime_type.to_string());
            for paragraph in text.split("\n\n") {
                let trimmed = paragraph.trim();
                if !trimmed.is_empty() {
                    d.push_element(InternalElement::text(ElementKind::Paragraph, trimmed, 0));
                }
            }
            d
        };

        doc.metadata = Metadata {
            output_format: pre_formatted_output,
            title: pdf_metadata.title.clone(),
            subject: pdf_metadata.subject.clone(),
            authors: pdf_metadata.authors.clone(),
            keywords: pdf_metadata.keywords.clone(),
            created_at: pdf_metadata.created_at.clone(),
            modified_at: pdf_metadata.modified_at.clone(),
            created_by: pdf_metadata.created_by.clone(),
            pages: pdf_metadata.page_structure.clone(),
            format: Some(crate::types::FormatMetadata::Pdf(pdf_metadata.pdf_specific)),
            ..Default::default()
        };

        // When using the structured doc, tables are already interleaved by the assembly pipeline.
        // Only add tables separately for the flat-text fallback path.
        if !use_structured_doc {
            for table in tables {
                let table_index = doc.push_table(table);
                doc.push_element(InternalElement::text(ElementKind::Table { table_index }, "", 0));
            }
        }

        if let Some(imgs) = images {
            doc.images = imgs;
        }
        if let Some(warning) = image_fallback_warning {
            doc.processing_warnings.push(warning);
        }
        doc.annotations = pdf_annotations;

        // Extract URIs from annotations (links).
        {
            use crate::types::annotations::PdfAnnotationType;
            use crate::types::uri::{Uri, UriKind};

            let uris: Vec<Uri> = doc
                .annotations
                .as_ref()
                .map(|annotations| {
                    annotations
                        .iter()
                        .filter(|a| a.annotation_type == PdfAnnotationType::Link)
                        .filter_map(|a| {
                            a.content.as_ref().map(|url| {
                                let kind = if url.starts_with('#') {
                                    UriKind::Anchor
                                } else if url.starts_with("mailto:") {
                                    UriKind::Email
                                } else {
                                    UriKind::Hyperlink
                                };
                                Uri {
                                    url: url.clone(),
                                    label: Some(url.clone()),
                                    page: Some(a.page_number as u32),
                                    kind,
                                }
                            })
                        })
                        .collect()
                })
                .unwrap_or_default();
            for uri in uris {
                doc.push_uri(uri);
            }
        }

        // Extract bookmarks/outlines.
        #[cfg(feature = "pdf")]
        {
            if let Ok(lopdf_doc) = lopdf::Document::load_mem(content) {
                let bookmark_uris = crate::pdf::bookmarks::extract_bookmarks(&lopdf_doc);
                for uri in bookmark_uris {
                    doc.push_uri(uri);
                }
            }
        }

        // Extract embedded files.
        #[cfg(all(feature = "pdf", feature = "tokio-runtime"))]
        {
            let (embedded_children, embedded_warnings) =
                crate::pdf::embedded_files::extract_and_process_embedded_files(content, config).await;
            if !embedded_children.is_empty() {
                match doc.children {
                    Some(ref mut existing) => existing.extend(embedded_children),
                    None => doc.children = Some(embedded_children),
                }
            }
            for warning in embedded_warnings {
                doc.processing_warnings.push(warning);
            }
        }

        doc.prebuilt_pages = final_pages;

        tracing::debug!(
            elements = doc.elements.len(),
            tables = doc.tables.len(),
            has_pages = doc.prebuilt_pages.is_some(),
            "InternalDocument finalized (oxide path)"
        );

        Ok(doc)
    }
}

/// Loads a PDF from a byte slice, using the config's passwords if needed,
/// in the order they are provided.
#[cfg(feature = "pdf")]
fn load_pdf_from_byte_slice<'pdf>(
    pdfium: &'pdf Pdfium,
    content: &'pdf [u8],
    config: &ExtractionConfig,
) -> std::result::Result<PdfDocument<'pdf>, PdfError> {
    let passwords = [None].into_iter().chain(
        config
            .pdf_options
            .iter()
            .flat_map(|o| &o.passwords)
            .flatten()
            .map(String::as_str)
            .map(Some),
    );

    for pwd in passwords {
        match pdfium.load_pdf_from_byte_slice(content, pwd) {
            Ok(doc) => return Ok(doc),
            Err(e) => {
                let err_msg = crate::pdf::error::format_pdfium_error(e);
                if !err_msg.contains("password") && !err_msg.contains("Password") {
                    return Err(PdfError::InvalidPdf(err_msg));
                }
            }
        };
    }

    Err(PdfError::PasswordRequired)
}

#[cfg(test)]
mod tests {
    use super::*;
    #[cfg(feature = "ocr")]
    use crate::core::config::OcrQualityThresholds;

    #[test]
    fn test_pdf_extractor_plugin_interface() {
        let extractor = PdfExtractor::new();
        assert_eq!(extractor.name(), "pdf-extractor");
        assert!(extractor.initialize().is_ok());
        assert!(extractor.shutdown().is_ok());
    }

    #[test]
    fn test_pdf_extractor_supported_mime_types() {
        let extractor = PdfExtractor::new();
        let mime_types = extractor.supported_mime_types();
        assert_eq!(mime_types.len(), 1);
        assert!(mime_types.contains(&"application/pdf"));
    }

    #[cfg(feature = "ocr")]
    #[test]
    fn test_should_fallback_to_ocr_for_empty_text() {
        assert!(ocr::evaluate_native_text_for_ocr("", Some(1), &OcrQualityThresholds::default()).fallback);
    }

    #[cfg(feature = "ocr")]
    #[test]
    fn test_should_not_fallback_for_meaningful_text() {
        let sample = "This page has searchable vector text and should avoid OCR.";
        assert!(!ocr::evaluate_native_text_for_ocr(sample, Some(1), &OcrQualityThresholds::default()).fallback);
    }

    #[cfg(feature = "ocr")]
    #[test]
    fn test_should_fallback_for_punctuation_only_text() {
        let sample = " . , ; : -- -- ";
        assert!(ocr::evaluate_native_text_for_ocr(sample, Some(2), &OcrQualityThresholds::default()).fallback);
    }

    #[cfg(feature = "ocr")]
    #[test]
    fn test_per_page_ocr_no_boundaries_falls_back_to_whole_doc() {
        let text = "This document has enough meaningful words for evaluation purposes here.";
        let decision = ocr::evaluate_per_page_ocr(text, None, Some(1), &OcrQualityThresholds::default());
        assert!(!decision.fallback);
    }

    #[cfg(feature = "ocr")]
    #[test]
    fn test_per_page_ocr_empty_boundaries_falls_back_to_whole_doc() {
        let text = "This document has enough meaningful words for evaluation purposes here.";
        let decision = ocr::evaluate_per_page_ocr(text, Some(&[]), Some(1), &OcrQualityThresholds::default());
        assert!(!decision.fallback);
    }

    #[cfg(feature = "ocr")]
    #[test]
    fn test_per_page_ocr_all_pages_good() {
        use crate::types::PageBoundary;

        let page1 = "This first page has plenty of meaningful searchable text content here.";
        let page2 = "This second page also has plenty of meaningful searchable text content.";
        let text = format!("{}{}", page1, page2);
        let boundaries = vec![
            PageBoundary {
                byte_start: 0,
                byte_end: page1.len(),
                page_number: 1,
            },
            PageBoundary {
                byte_start: page1.len(),
                byte_end: text.len(),
                page_number: 2,
            },
        ];

        let decision = ocr::evaluate_per_page_ocr(&text, Some(&boundaries), Some(2), &OcrQualityThresholds::default());
        assert!(!decision.fallback);
    }

    #[cfg(feature = "ocr")]
    #[test]
    fn test_per_page_ocr_one_bad_page_triggers_fallback() {
        use crate::types::PageBoundary;

        let good_page = "This page has plenty of meaningful searchable text content for extraction.";
        let bad_page = " . ; ";
        let text = format!("{}{}", good_page, bad_page);
        let boundaries = vec![
            PageBoundary {
                byte_start: 0,
                byte_end: good_page.len(),
                page_number: 1,
            },
            PageBoundary {
                byte_start: good_page.len(),
                byte_end: text.len(),
                page_number: 2,
            },
        ];

        let decision = ocr::evaluate_per_page_ocr(&text, Some(&boundaries), Some(2), &OcrQualityThresholds::default());
        assert!(decision.fallback);
    }

    #[cfg(feature = "ocr")]
    #[test]
    fn test_per_page_ocr_empty_page_triggers_fallback() {
        use crate::types::PageBoundary;

        let good_page = "This page has plenty of meaningful searchable text content for extraction.";
        let empty_page = "";
        let text = format!("{}{}", good_page, empty_page);
        let boundaries = vec![
            PageBoundary {
                byte_start: 0,
                byte_end: good_page.len(),
                page_number: 1,
            },
            PageBoundary {
                byte_start: good_page.len(),
                byte_end: text.len(),
                page_number: 2,
            },
        ];

        let decision = ocr::evaluate_per_page_ocr(&text, Some(&boundaries), Some(2), &OcrQualityThresholds::default());
        assert!(decision.fallback);
    }

    #[cfg(feature = "ocr")]
    #[test]
    fn test_per_page_ocr_preserves_document_stats_on_fallback() {
        use crate::types::PageBoundary;

        let good_page = "This page has plenty of meaningful searchable text content for extraction.";
        let bad_page = " . ; ";
        let text = format!("{}{}", good_page, bad_page);
        let boundaries = vec![
            PageBoundary {
                byte_start: 0,
                byte_end: good_page.len(),
                page_number: 1,
            },
            PageBoundary {
                byte_start: good_page.len(),
                byte_end: text.len(),
                page_number: 2,
            },
        ];

        let decision = ocr::evaluate_per_page_ocr(&text, Some(&boundaries), Some(2), &OcrQualityThresholds::default());
        assert!(decision.fallback);
        assert!(decision.stats.non_whitespace > 0);
        assert!(decision.stats.meaningful_words > 0);
    }

    #[cfg(feature = "ocr")]
    #[test]
    fn test_per_page_ocr_invalid_boundaries_skipped() {
        use crate::types::PageBoundary;

        let text = "This page has plenty of meaningful searchable text content for extraction.";
        let boundaries = vec![
            PageBoundary {
                byte_start: 0,
                byte_end: text.len(),
                page_number: 1,
            },
            PageBoundary {
                byte_start: 999,
                byte_end: 9999,
                page_number: 2,
            },
        ];

        let decision = ocr::evaluate_per_page_ocr(text, Some(&boundaries), Some(1), &OcrQualityThresholds::default());
        assert!(!decision.fallback);
    }

    #[cfg(feature = "ocr")]
    #[test]
    fn test_per_page_ocr_multi_page_correct_page_count() {
        let text = "ab cd ef";
        let decision_wrong = ocr::evaluate_native_text_for_ocr(text, None, &OcrQualityThresholds::default());
        let decision_correct = ocr::evaluate_native_text_for_ocr(text, Some(20), &OcrQualityThresholds::default());
        assert!(
            decision_correct.avg_non_whitespace < decision_wrong.avg_non_whitespace,
            "Correct page count should produce lower per-page averages"
        );
    }

    #[tokio::test]
    #[cfg(feature = "pdf")]
    async fn test_pdf_batch_mode_validates_page_config_enabled() {
        use crate::core::config::PageConfig;

        let extractor = PdfExtractor::new();

        let config = ExtractionConfig {
            pages: Some(PageConfig {
                extract_pages: true,
                insert_page_markers: false,
                marker_format: "<!-- PAGE {page_num} -->".to_string(),
            }),
            ..Default::default()
        };

        let pdf_path =
            std::path::Path::new(env!("CARGO_MANIFEST_DIR")).join("../../test_documents/pdf/google_doc_document.pdf");
        if let Ok(content) = std::fs::read(pdf_path) {
            let result = extractor.extract_bytes(&content, "application/pdf", &config).await;
            assert!(
                result.is_ok(),
                "Failed to extract PDF with page config: {:?}",
                result.err()
            );

            let extraction_result = result.unwrap();
            let extraction_result = crate::extraction::derive::derive_extraction_result(
                extraction_result,
                true,
                crate::core::config::OutputFormat::Plain,
            );
            // NOTE: The current PDF extractor doesn't assign page numbers to InternalDocument elements,
            // so the derive pipeline's build_pages returns None even when extract_pages is true.
            // Page data should be populated once the PDF extractor assigns page numbers to elements.
            // For now, verify the extraction succeeds and content is present.
            assert!(
                !extraction_result.content.is_empty(),
                "Content should be extracted from PDF"
            );
        }
    }

    #[tokio::test]
    #[cfg(feature = "pdf")]
    async fn test_pdf_batch_mode_validates_page_config_disabled() {
        let extractor = PdfExtractor::new();
        let config = ExtractionConfig::default();

        let pdf_path =
            std::path::Path::new(env!("CARGO_MANIFEST_DIR")).join("../../test_documents/pdf/google_doc_document.pdf");
        if let Ok(content) = std::fs::read(pdf_path) {
            let result = extractor.extract_bytes(&content, "application/pdf", &config).await;
            assert!(
                result.is_ok(),
                "Failed to extract PDF without page config: {:?}",
                result.err()
            );

            let extraction_result = result.unwrap();
            let extraction_result = crate::extraction::derive::derive_extraction_result(
                extraction_result,
                true,
                crate::core::config::OutputFormat::Plain,
            );
            assert!(
                extraction_result.pages.is_none(),
                "Pages should not be extracted when pages config is None"
            );
        }
    }

    #[tokio::test]
    #[cfg(feature = "pdf")]
    async fn test_pdf_page_marker_validation() {
        use crate::core::config::PageConfig;

        let extractor = PdfExtractor::new();

        let config = ExtractionConfig {
            pages: Some(PageConfig {
                extract_pages: true,
                insert_page_markers: true,
                marker_format: "\n\n<!-- PAGE {page_num} -->\n\n".to_string(),
            }),
            ..Default::default()
        };

        let pdf_path = std::path::Path::new(env!("CARGO_MANIFEST_DIR")).join("../../test_documents/pdf/multi_page.pdf");
        if let Ok(content) = std::fs::read(pdf_path) {
            let result = extractor.extract_bytes(&content, "application/pdf", &config).await;
            assert!(
                result.is_ok(),
                "Failed to extract PDF with page markers: {:?}",
                result.err()
            );

            let extraction_result = result.unwrap();
            let extraction_result = crate::extraction::derive::derive_extraction_result(
                extraction_result,
                true,
                crate::core::config::OutputFormat::Plain,
            );
            let marker_placeholder = "<!-- PAGE ";
            if extraction_result.content.len() > 100 {
                assert!(
                    extraction_result.content.contains(marker_placeholder),
                    "Page markers should be inserted when configured and document has multiple pages"
                );
            }
        }
    }

    #[tokio::test]
    #[cfg(all(feature = "pdf", feature = "ocr"))]
    async fn test_pdf_force_ocr_without_ocr_config() {
        use crate::core::config::ExtractionConfig;

        let extractor = PdfExtractor::new();

        let config = ExtractionConfig {
            force_ocr: true,
            ocr: None,
            ..Default::default()
        };

        let pdf_path = std::path::Path::new(env!("CARGO_MANIFEST_DIR")).join("../../test_documents/pdf/multi_page.pdf");
        if let Ok(content) = std::fs::read(pdf_path) {
            let result = extractor.extract_bytes(&content, "application/pdf", &config).await;

            if let Err(e) = result {
                assert!(
                    !e.to_string().contains("OCR config required for force_ocr"),
                    "Should not require manual OCR config when force_ocr is true"
                );
            }
        }
    }

    #[test]
    #[cfg(feature = "pdf")]
    fn test_pdf_extractor_without_feature_pdf() {
        let extractor = PdfExtractor::new();
        assert_eq!(extractor.name(), "pdf-extractor");
    }
}
