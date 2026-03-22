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
use crate::types::{ExtractionResult, Metadata};
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
    hints: Vec<Vec<crate::pdf::markdown::types::LayoutHint>>,
    images: Vec<image::DynamicImage>,
    results: Vec<crate::pdf::layout_runner::PageLayoutResult>,
}

#[cfg(all(feature = "pdf", feature = "layout-detection"))]
fn run_layout_detection(content: &[u8], config: &ExtractionConfig) -> Option<LayoutDetectionBundle> {
    let layout_config = config.layout.as_ref()?;

    // Only run for output formats that use the markdown pipeline.
    let needs_structured = matches!(
        config.output_format,
        OutputFormat::Markdown | OutputFormat::Djot | OutputFormat::Html
    );
    if !needs_structured {
        tracing::debug!("Layout detection skipped: output format does not use markdown pipeline");
        return None;
    }

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

/// Run layout detection on pre-rendered images, returning pixel-space results.
///
/// Used by the OCR path to share rendered images with layout detection.
/// Returns `None` when layout detection is not configured or fails.
#[cfg(all(feature = "pdf", feature = "layout-detection", feature = "ocr"))]
fn run_layout_detection_on_images(
    images: &[image::DynamicImage],
    config: &ExtractionConfig,
) -> Option<Vec<crate::layout::DetectionResult>> {
    let layout_config = config.layout.as_ref()?;

    let needs_structured = matches!(
        config.output_format,
        OutputFormat::Markdown | OutputFormat::Djot | OutputFormat::Html
    );
    if !needs_structured {
        return None;
    }

    let mut engine = match crate::layout::take_or_create_engine(layout_config) {
        Ok(e) => e,
        Err(e) => {
            tracing::warn!("Layout engine init failed for OCR path: {}", e);
            return None;
        }
    };

    let result = match crate::pdf::layout_runner::detect_layout_for_images(images, &mut engine) {
        Ok(results) => {
            let total_detections: usize = results.iter().map(|r| r.detections.len()).sum();
            tracing::info!(
                page_count = results.len(),
                total_detections,
                "Layout detection on OCR images completed"
            );
            Some(results)
        }
        Err(e) => {
            tracing::warn!("Layout detection on OCR images failed: {}", e);
            None
        }
    };

    // Return engine to cache for reuse by subsequent extractions
    crate::layout::return_engine(engine);
    result
}

/// Render PDF pages, optionally run layout detection, then run OCR.
///
/// Renders images once and shares them between layout detection and OCR
/// to avoid redundant PDF rendering. When a multi-backend pipeline is
/// configured (or auto-constructed), uses quality-based fallback across
/// backends.
#[cfg(feature = "ocr")]
async fn run_ocr_with_layout(
    content: &[u8],
    config: &ExtractionConfig,
    path: Option<&std::path::Path>,
) -> crate::Result<String> {
    let ocr_config = config.ocr.as_ref().ok_or_else(|| {
        crate::error::KreuzbergError::parsing("OCR configuration missing")
    })?;

    // Check for pipeline configuration
    if let Some(pipeline) = ocr_config.effective_pipeline() {
        return ocr::run_ocr_pipeline(
            Some(content),
            None, // No images rendered yet
            #[cfg(feature = "layout-detection")]
            None, // No layout detected yet
            config,
            &pipeline,
            path,
        )
        .await;
    }

    let images = ocr::render_pages_for_ocr(content)?;

    #[cfg(feature = "layout-detection")]
    let layout_detections = run_layout_detection_on_images(&images, config);

    let (text, _mean_conf) = extract_with_ocr(
        Some(content),
        Some(&images),
        #[cfg(feature = "layout-detection")]
        layout_detections.as_deref(),
        config,
        path,
    )
    .await?;
    Ok(text)
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
    ) -> Result<ExtractionResult> {
        self.extract_core(content, mime_type, config, None).await
    }

    #[cfg(feature = "tokio-runtime")]
    async fn extract_file(&self, path: &Path, mime_type: &str, config: &ExtractionConfig) -> Result<ExtractionResult> {
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
    #[cfg_attr(feature = "otel", tracing::instrument(skip(self, content)))]
    async fn extract_core(
        &self,
        content: &[u8],
        mime_type: &str,
        config: &ExtractionConfig,
        path: Option<&std::path::Path>,
    ) -> Result<ExtractionResult> {
        // Strip /Rotate from page dicts to work around pdfium text extraction bug
        // where FPDFText_CountChars returns 0 for 90°/270° rotated pages.
        #[cfg(feature = "pdf")]
        let derotated = crate::pdf::text::strip_page_rotation(content);
        #[cfg(feature = "pdf")]
        let content = &*derotated;

        #[cfg(feature = "pdf")]
        #[allow(unused_variables)]
        let (
            mut pdf_metadata,
            native_text,
            tables,
            page_contents,
            boundaries,
            pre_rendered_markdown,
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
                let layout_hints: Option<Vec<Vec<crate::pdf::markdown::types::LayoutHint>>> = None;

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
                            pre_rendered_markdown,
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
                            pre_rendered_markdown,
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
                    Option<Vec<Vec<crate::pdf::markdown::types::LayoutHint>>>,
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
        let (text, used_ocr) = if config.force_ocr {
            (run_ocr_with_layout(content, config, path).await?, true)
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

            // When pre-rendered markdown is available, the native text pipeline
            // has already produced structured output with layout guidance, heading
            // detection, table extraction, etc. OCR fallback on such documents
            // often DEGRADES quality because:
            // 1. OCR struggles with dot leaders (TOC pages), producing garbled text.
            // 2. OCR on multi-column pages can interleave columns.
            // 3. The per-page OCR trigger fires on a single weak page even when
            //    most pages have excellent native text.
            //
            // Skip OCR when pre-rendered markdown exists with substantive content,
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

            let has_substantive_markdown = pre_rendered_markdown.is_some()
                && total_chars >= thresholds.substantive_min_chars
                && alnum_ws_ratio >= thresholds.alnum_ws_ratio_threshold;

            let skip_ocr_for_non_text = pre_rendered_markdown.is_some()
                && total_chars >= thresholds.non_text_min_chars
                && alnum_ws_ratio < thresholds.alnum_ws_ratio_threshold;

            if skip_ocr_for_non_text {
                tracing::debug!(
                    alnum_ws_ratio,
                    total_chars,
                    alnum_ws_chars,
                    "Skipping OCR: font encoding issues but content is non-textual and pre-rendered markdown available"
                );
                (native_text, false)
            } else if has_substantive_markdown {
                tracing::debug!(
                    total_chars,
                    alnum_ws_ratio,
                    ocr_fallback = decision.fallback,
                    has_font_encoding_issues,
                    "Skipping OCR: pre-rendered markdown available with substantive native text"
                );
                (native_text, false)
            } else if decision.fallback || has_font_encoding_issues {
                (run_ocr_with_layout(content, config, path).await?, true)
            } else {
                (native_text, false)
            }
        } else {
            (native_text, false)
        };

        #[cfg(not(feature = "ocr"))]
        let (text, used_ocr) = (native_text, false);

        // Post-processing: use pre-rendered markdown from initial document load if available.
        // The markdown was rendered during the first document load to avoid redundant PDF parsing.
        // OCR results already produce markdown via the hOCR path, so this only applies
        // when native text extraction was used and markdown output was requested.
        // Note: we defer consumption of pre_rendered_markdown until after images are available
        // so that we can inject image placeholders into it before finalizing the text.
        #[cfg(feature = "pdf")]
        let use_pdf_markdown = !used_ocr && pre_rendered_markdown.is_some();
        tracing::debug!(
            used_ocr,
            has_pre_rendered = pre_rendered_markdown.is_some(),
            use_pdf_markdown,
            pre_rendered_len = pre_rendered_markdown.as_ref().map(|m| m.len()).unwrap_or(0),
            "PDF extractor: deciding whether to use pre-rendered markdown"
        );

        #[cfg(not(feature = "pdf"))]
        let use_pdf_markdown = false;

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

        let images = if config.images.as_ref().map(|c| c.extract_images).unwrap_or(false) {
            // Image extraction is enabled, extract images if present
            match crate::pdf::images::extract_images_from_pdf(content) {
                Ok(pdf_images) => Some(
                    pdf_images
                        .into_iter()
                        .enumerate()
                        .map(|(idx, img)| {
                            // Use the decoded format (e.g. "jpeg", "png") rather than the
                            // PDF filter name (e.g. "DCTDecode", "FlateDecode").
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
                            }
                        })
                        .collect(),
                ),
                // If extraction fails, return empty vector instead of None
                Err(_) => Some(vec![]),
            }
        } else {
            // Image extraction is not enabled
            None
        };

        // Run OCR on extracted images if configured (same pattern as docx/pptx)
        #[cfg(all(feature = "ocr", feature = "tokio-runtime"))]
        let images = if let Some(imgs) = images {
            match crate::extraction::image_ocr::process_images_with_ocr(imgs, config).await {
                Ok(processed) => Some(processed),
                Err(e) => {
                    tracing::warn!(
                        "Image OCR on embedded PDF images failed: {:?}, continuing without image OCR",
                        e
                    );
                    None
                }
            }
        } else {
            None
        };

        // Finalize text: apply pre-rendered markdown (with image placeholder injection) if available.
        // Quality gate: if pre-rendered markdown has >2x the words of native text,
        // the layout pipeline added garbage (e.g., text extracted from decorative
        // elements). Fall back to native text in that case.
        #[cfg(feature = "pdf")]
        let use_pdf_markdown = if use_pdf_markdown {
            if let Some(ref md) = pre_rendered_markdown {
                let md_words = md.split_whitespace().count();
                let native_words = text.split_whitespace().count();
                if native_words > 50 && md_words > native_words * 2 {
                    tracing::debug!(
                        md_words,
                        native_words,
                        "Layout quality gate: markdown has >2x native text words, falling back"
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
        let (text, used_pdf_markdown) = if use_pdf_markdown {
            if let Some(md) = pre_rendered_markdown {
                let should_inject = config.images.as_ref().is_none_or(|img_cfg| img_cfg.inject_placeholders);
                let final_md = if should_inject {
                    if let Some(ref imgs) = images {
                        if !imgs.is_empty() {
                            crate::pdf::markdown::inject_image_placeholders(&md, imgs)
                        } else {
                            md
                        }
                    } else {
                        md
                    }
                } else {
                    md
                };
                (final_md, true)
            } else {
                (text, false)
            }
        } else {
            (text, false)
        };

        #[cfg(not(feature = "pdf"))]
        let used_pdf_markdown = false;

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
        let effective_mime_type = mime_type.to_string();

        // Signal pre-formatted markdown so the pipeline doesn't double-convert.
        // Only skip conversion for Markdown; Djot and HTML get the quality markdown
        // content but still need apply_output_format() for format-specific conversion.
        #[cfg(feature = "pdf")]
        let pre_formatted_output = if used_pdf_markdown && config.output_format == OutputFormat::Markdown {
            tracing::trace!("PDF extractor: signaling pre-formatted markdown to pipeline");
            Some("markdown".to_string())
        } else {
            tracing::trace!(
                used_pdf_markdown,
                output_format = ?config.output_format,
                "PDF extractor: NOT signaling pre-formatted markdown"
            );
            None
        };
        #[cfg(not(feature = "pdf"))]
        let pre_formatted_output: Option<String> = None;

        Ok(ExtractionResult {
            content: text,
            mime_type: effective_mime_type.into(),
            metadata: Metadata {
                output_format: pre_formatted_output,
                #[cfg(feature = "pdf")]
                title: pdf_metadata.title.clone(),
                #[cfg(feature = "pdf")]
                subject: pdf_metadata.subject.clone(),
                #[cfg(feature = "pdf")]
                authors: pdf_metadata.authors.clone(),
                #[cfg(feature = "pdf")]
                keywords: pdf_metadata.keywords.clone(),
                #[cfg(feature = "pdf")]
                created_at: pdf_metadata.created_at.clone(),
                #[cfg(feature = "pdf")]
                modified_at: pdf_metadata.modified_at.clone(),
                #[cfg(feature = "pdf")]
                created_by: pdf_metadata.created_by.clone(),
                #[cfg(feature = "pdf")]
                pages: pdf_metadata.page_structure.clone(),
                #[cfg(feature = "pdf")]
                format: Some(crate::types::FormatMetadata::Pdf(pdf_metadata.pdf_specific)),
                ..Default::default()
            },
            pages: final_pages,
            tables,
            detected_languages: None,
            chunks: None,
            images,
            djot_content: None,
            elements: None,
            ocr_elements: None,
            document: None,
            #[cfg(any(feature = "keywords-yake", feature = "keywords-rake"))]
            extracted_keywords: None,
            quality_score: None,
            processing_warnings: Vec::new(),
            #[cfg(feature = "pdf")]
            annotations: pdf_annotations,
            #[cfg(not(feature = "pdf"))]
            annotations: None,
        })
    }

    fn priority(&self) -> i32 {
        50
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

    #[test]
    fn test_pdf_extractor_priority() {
        let extractor = PdfExtractor::new();
        assert_eq!(extractor.priority(), 50);
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
            assert!(
                extraction_result.pages.is_some(),
                "Pages should be extracted when extract_pages is true"
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
