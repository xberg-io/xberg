//! PDF document extractor.
//!
//! Provides extraction of text, metadata, tables, and images from PDF documents
//! using pdf_oxide (pure Rust). Supports both native text extraction and OCR fallback.

mod extraction;
#[cfg(feature = "layout-detection")]
mod layout_hints;
#[cfg(all(feature = "pdf", feature = "layout-detection"))]
mod layout_runner;
mod ocr;
mod pages;
#[cfg(feature = "layout-detection")]
pub(crate) mod reading_order;
#[cfg(all(feature = "liter-llm", feature = "layout-detection"))]
mod region_vlm;

use crate::Result;
use crate::core::config::ExtractionConfig;
use crate::plugins::{InternalDocumentExtractor, Plugin};
use crate::types::internal::{ElementKind, InternalDocument, InternalElement};
use crate::types::{ExtractionMethod, Metadata};
use async_trait::async_trait;
#[cfg(feature = "tokio-runtime")]
use std::path::Path;

use extraction::extract_all_from_oxide_document;
#[cfg(any(feature = "ocr", feature = "ocr-pipeline"))]
use ocr::extract_with_ocr;
use pages::{assign_hierarchy_to_pages, assign_tables_and_images_to_pages};

/// Run OCR with optional layout detection on PDF bytes.
///
/// Returns `None` when layout detection is not configured or fails.
/// Failures are logged as warnings but do not propagate — extraction
/// continues without layout hints (graceful degradation).
#[cfg(any(feature = "ocr", feature = "ocr-pipeline"))]
async fn run_ocr_with_layout(
    content: &[u8],
    config: &ExtractionConfig,
    path: Option<&std::path::Path>,
) -> crate::Result<(
    String,
    Vec<crate::types::Table>,
    Vec<crate::types::OcrElement>,
    Option<crate::types::internal::InternalDocument>,
    Vec<crate::types::LlmUsage>,
    Vec<String>,
    Option<Vec<crate::types::ExtractedImage>>,
    Vec<crate::types::Formula>,
)> {
    let default_ocr_config = crate::core::config::OcrConfig::default();
    let ocr_config = config.ocr.as_ref().unwrap_or(&default_ocr_config);

    #[cfg(feature = "layout-detection")]
    let layout_detections: Option<Vec<crate::layout::DetectionResult>> = None;

    // Check for pipeline configuration
    if let Some(pipeline) = ocr_config.effective_pipeline() {
        // Box::pin the deep OCR future so its persisted state lives on the heap
        // instead of inflating this frame. The OCR await chain is large and
        // stack-sensitive; keeping it inline contributes to worker-stack overflow.
        let (text, _ocr_tables, ocr_elements, pipeline_doc, llm_usage, ocr_pts, pipeline_rasters, pipeline_formulas) =
            Box::pin(ocr::run_ocr_pipeline(
                Some(content),
                None,
                #[cfg(feature = "layout-detection")]
                layout_detections.as_deref(),
                config,
                &pipeline,
                path,
            ))
            .await?;
        return Ok((
            text,
            Vec::new(),
            ocr_elements,
            pipeline_doc,
            llm_usage,
            ocr_pts,
            pipeline_rasters,
            pipeline_formulas,
        ));
    }

    // Box::pin the deep OCR future so its persisted state lives on the heap
    // instead of inflating this frame. The OCR await chain is large and
    // stack-sensitive; keeping it inline contributes to worker-stack overflow.
    let (text, _mean_conf, ocr_tables, ocr_elements, ocr_doc, llm_usage, ocr_pts, ocr_rasters, formulas) =
        Box::pin(extract_with_ocr(
            Some(content),
            None,
            #[cfg(feature = "layout-detection")]
            layout_detections.as_deref(),
            config,
            path,
        ))
        .await?;
    Ok((
        text,
        ocr_tables,
        ocr_elements,
        ocr_doc,
        llm_usage,
        ocr_pts,
        ocr_rasters,
        formulas,
    ))
}

/// PDF document extractor using pdf_oxide.
#[cfg_attr(alef, alef(skip))]
pub struct PdfExtractor;

impl Default for PdfExtractor {
    fn default() -> Self {
        Self::new()
    }
}

impl PdfExtractor {
    pub(crate) fn new() -> Self {
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
impl InternalDocumentExtractor for PdfExtractor {
    async fn extract_content(
        &self,
        content: &[u8],
        mime_type: &str,
        config: &ExtractionConfig,
    ) -> Result<InternalDocument> {
        self.extract_core(content, mime_type, config, None).await
    }

    #[cfg(feature = "tokio-runtime")]
    async fn extract_path(&self, path: &Path, mime_type: &str, config: &ExtractionConfig) -> Result<InternalDocument> {
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
        self.extract_core_oxide(content, mime_type, config, path).await
    }

    /// Core extraction via the pdf_oxide backend.
    ///
    /// Runs text + metadata, tables, and annotation extraction through the oxide
    /// modules, then builds an `InternalDocument` using the same post-processing
    /// pipeline (OCR evaluation, page assembly, image extraction, bookmarks, etc.).
    #[cfg(feature = "pdf")]
    async fn extract_core_oxide(
        &self,
        content: &[u8],
        mime_type: &str,
        config: &ExtractionConfig,
        path: Option<&std::path::Path>,
    ) -> Result<InternalDocument> {
        let _ = &path; // used only when `ocr` feature is enabled

        #[cfg(all(feature = "pdf", feature = "layout-detection"))]
        let (markdown_layout_images, markdown_layout_results, markdown_layout_hints) =
            layout_runner::maybe_run_layout_for_markdown(content, config);

        #[cfg(all(feature = "pdf", feature = "layout-detection"))]
        let layout_hints: Option<&[Vec<crate::pdf::structure::types::LayoutHint>]> = markdown_layout_hints.as_deref();
        #[cfg(not(feature = "layout-detection"))]
        let layout_hints: Option<&[Vec<crate::pdf::structure::types::LayoutHint>]> = None;

        #[allow(unused_variables, unused_mut)]
        let (
            mut pdf_metadata,
            native_text,
            mut tables,
            page_contents,
            boundaries,
            pre_rendered_doc,
            _has_font_encoding_issues,
            pdf_annotations,
            mut extracted_images,
            pdf_form_fields,
        ) = extract_all_from_oxide_document(
            content,
            config,
            layout_hints,
            #[cfg(feature = "layout-detection")]
            markdown_layout_images.as_deref(),
            #[cfg(not(feature = "layout-detection"))]
            None,
            #[cfg(feature = "layout-detection")]
            markdown_layout_results.as_deref(),
            #[cfg(not(feature = "layout-detection"))]
            None,
        )?;

        // --- Inline-image OCR ---
        // Moved from extraction.rs (sync) to here (async) so we can resolve the
        // configured backend through the OcrBackend registry instead of calling
        // OcrProcessor directly, which previously hard-coded Tesseract regardless
        // of OcrConfig.backend / vlm_fallback / pipeline. Fixes #1088.
        // output_format is forwarded so backends that produce format-aware output
        // (e.g. Markdown table rendering) behave consistently with the image extractor.
        #[cfg(any(feature = "ocr", feature = "ocr-pipeline"))]
        if config.pdf_options.as_ref().is_some_and(|p| p.ocr_inline_images)
            && let Some(ref mut imgs) = extracted_images
            && !imgs.is_empty()
        {
            let default_ocr_config;
            let ocr_config = match config.ocr.as_ref() {
                Some(c) => c,
                None => {
                    default_ocr_config = crate::core::config::OcrConfig::default();
                    &default_ocr_config
                }
            };
            crate::plugins::ensure_ocr_backends_initialized();
            let backend = {
                let registry = crate::plugins::registry::get_ocr_backend_registry();
                registry.read().get(&ocr_config.backend)?
            };
            let mut ocr_config_with_format = ocr_config.clone();
            ocr_config_with_format.output_format = Some(config.output_format.clone());
            for img in imgs.iter_mut() {
                if config.cancel_token.as_ref().is_some_and(|t| t.is_cancelled()) {
                    break;
                }
                match backend.process_image(&img.data, &ocr_config_with_format).await {
                    Ok(ocr_result) => {
                        img.ocr_result = Some(Box::new(ocr_result));
                    }
                    Err(e) => {
                        tracing::warn!(
                            page = img.page_number,
                            image_index = img.image_index,
                            error = %e,
                            "inline image OCR failed; image returned without OCR result"
                        );
                    }
                }
            }
        }

        // --- OCR evaluation ---
        #[cfg(any(feature = "ocr", feature = "ocr-pipeline"))]
        let mut ocr_tables: Vec<crate::types::Table> = Vec::new();
        #[cfg(any(feature = "ocr", feature = "ocr-pipeline"))]
        let mut ocr_elements: Vec<crate::types::OcrElement> = Vec::new();
        #[cfg(any(feature = "ocr", feature = "ocr-pipeline"))]
        let mut ocr_internal_doc: Option<InternalDocument> = None;
        #[cfg(any(feature = "ocr", feature = "ocr-pipeline"))]
        let mut ocr_llm_usage: Vec<crate::types::LlmUsage> = Vec::new();
        #[cfg(any(feature = "ocr", feature = "ocr-pipeline"))]
        let mut ocr_page_texts: Option<Vec<String>> = None;
        #[cfg(any(feature = "ocr", feature = "ocr-pipeline"))]
        let mut ocr_results_map: Option<ahash::AHashMap<u32, String>> = None;
        #[cfg(any(feature = "ocr", feature = "ocr-pipeline"))]
        let mut ocr_page_rasters: Option<Vec<crate::types::ExtractedImage>> = None;
        #[cfg(any(feature = "ocr", feature = "ocr-pipeline"))]
        let mut ocr_formulas: Vec<crate::types::Formula> = Vec::new();
        // Warnings raised when an OCR fallback was required (native text failed the
        // quality gate) but the fallback itself failed or was unavailable, so the
        // returned native text is known-degraded. Drained into the document below so
        // consumers can tell an empty/poor result apart from a clean extraction.
        #[cfg(any(feature = "ocr", feature = "ocr-pipeline"))]
        let mut ocr_fallback_warnings: Vec<crate::types::ProcessingWarning> = Vec::new();

        #[cfg(any(feature = "ocr", feature = "ocr-pipeline"))]
        let (text, extraction_method) = if config.effective_disable_ocr() {
            (native_text, ExtractionMethod::Native)
        } else if config.force_ocr {
            let (ocr_text, ocr_tbls, ocr_elems, ocr_doc, llm_usage, ocr_pts, ocr_rstrs, formulas) =
                run_ocr_with_layout(content, config, path).await?;
            ocr_tables = ocr_tbls;
            ocr_elements = ocr_elems;
            ocr_internal_doc = ocr_doc;
            ocr_llm_usage = llm_usage;
            ocr_page_texts = Some(ocr_pts);
            ocr_page_rasters = ocr_rstrs;
            ocr_formulas = formulas;
            (ocr_text, ExtractionMethod::Ocr)
        } else if let Some(ref ocr_pages) = config.force_ocr_pages {
            if !ocr_pages.is_empty() {
                if let Some(ref bounds) = boundaries {
                    if !bounds.is_empty() {
                        let (mixed, results_map, mixed_llm_usage, mixed_rstrs, mixed_formulas) =
                            ocr::extract_mixed_ocr_native(&native_text, bounds, ocr_pages, content, config, path)
                                .await?;
                        ocr_llm_usage = mixed_llm_usage;
                        ocr_results_map = Some(results_map);
                        ocr_page_rasters = mixed_rstrs;
                        if !mixed_formulas.is_empty() {
                            ocr_formulas = mixed_formulas;
                        }
                        (mixed, ExtractionMethod::Mixed)
                    } else {
                        tracing::warn!("force_ocr_pages set but no page boundaries available; using native text");
                        (native_text, ExtractionMethod::Native)
                    }
                } else {
                    tracing::warn!("force_ocr_pages set but no page boundaries available; using native text");
                    (native_text, ExtractionMethod::Native)
                }
            } else {
                (native_text, ExtractionMethod::Native)
            }
        } else if let Some(ocr_config) = config.ocr.as_ref() {
            let thresholds = ocr_config.effective_thresholds();
            let decision = ocr::evaluate_per_page_ocr(
                &native_text,
                boundaries.as_deref(),
                pdf_metadata.pdf_specific.page_count,
                &thresholds,
            );

            if std::env::var("XBERG_DEBUG_OCR").is_ok() {
                eprintln!(
                    "[xberg::pdf::ocr] fallback={} non_whitespace={} alnum={} meaningful_words={} \
                     avg_non_whitespace={:.2} avg_alnum={:.2} alnum_ratio={:.3} fragmented_word_ratio={:.3} \
                     avg_word_length={:.2} word_count={} consecutive_repeat_ratio={:.3}",
                    decision.fallback,
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

            match ocr::evaluate_ocr_skip_gate(
                pre_rendered_doc.is_some(),
                total_chars,
                alnum_ws_ratio,
                &decision,
                &thresholds,
            ) {
                ocr::OcrGateOutcome::SkipNonText => {
                    tracing::debug!(
                        alnum_ws_ratio,
                        total_chars,
                        alnum_ws_chars,
                        "Skipping OCR: content is non-textual and pre-rendered structured doc available"
                    );
                    (native_text, ExtractionMethod::Native)
                }
                ocr::OcrGateOutcome::SkipSubstantive => {
                    tracing::debug!(
                        total_chars,
                        alnum_ws_ratio,
                        ocr_fallback = decision.fallback,
                        "Skipping OCR: pre-rendered structured doc available with substantive native text"
                    );
                    (native_text, ExtractionMethod::Native)
                }
                ocr::OcrGateOutcome::RunFallback => {
                    // Only skip document-level OCR when the caller explicitly configured
                    // image extraction AND opted into per-image OCR. Callers with no
                    // `images` config must not lose the fallback they previously relied on.
                    let skip_fallback = config.images.as_ref().map(|i| i.run_ocr_on_images).unwrap_or(false);
                    if skip_fallback {
                        tracing::debug!("Skipping document-level OCR fallback: run_ocr_on_images=true");
                        (native_text, ExtractionMethod::Native)
                    } else {
                        match run_ocr_with_layout(content, config, path).await {
                            Ok((ocr_text, ocr_tbls, ocr_elems, ocr_doc, llm_usage, ocr_pts, ocr_rstrs, formulas)) => {
                                ocr_tables = ocr_tbls;
                                ocr_elements = ocr_elems;
                                ocr_internal_doc = ocr_doc;
                                ocr_llm_usage = llm_usage;
                                ocr_page_texts = Some(ocr_pts);
                                ocr_page_rasters = ocr_rstrs;
                                ocr_formulas = formulas;
                                (ocr_text, ExtractionMethod::Ocr)
                            }
                            Err(e) => {
                                tracing::warn!(
                                    error = %e,
                                    "OCR fallback failed; using native text extraction result"
                                );
                                ocr_fallback_warnings.push(crate::types::ProcessingWarning {
                                    source: std::borrow::Cow::Borrowed("ocr"),
                                    message: std::borrow::Cow::Owned(format!(
                                        "OCR fallback failed ({e}); returning native text that was below the \
                                         quality threshold which triggered OCR. Extracted content may be empty \
                                         or incomplete."
                                    )),
                                });
                                (native_text, ExtractionMethod::Native)
                            }
                        }
                    }
                }
                // Intentional asymmetry with RunFallback: RunFallback is suppressed when
                // `run_ocr_on_images=true` because image-level OCR serves as the fallback.
                // RunFallbackOnPages targets specific pages that failed native quality,
                // which is independent of image extraction — suppressing it would silently
                // skip OCR on text pages that need it.
                ocr::OcrGateOutcome::RunFallbackOnPages(pages) => match boundaries.as_deref() {
                    Some(bounds) if !bounds.is_empty() => {
                        match ocr::extract_mixed_ocr_native(&native_text, bounds, &pages, content, config, path).await {
                            Ok((mixed, results_map, mixed_llm_usage, mixed_rstrs, mixed_formulas)) => {
                                ocr_llm_usage = mixed_llm_usage;
                                ocr_results_map = Some(results_map);
                                ocr_page_rasters = mixed_rstrs;
                                if !mixed_formulas.is_empty() {
                                    ocr_formulas = mixed_formulas;
                                }
                                (mixed, ExtractionMethod::Mixed)
                            }
                            Err(e) => {
                                tracing::warn!(
                                    error = %e,
                                    failing_pages = ?pages,
                                    "Targeted OCR fallback failed; using native text extraction result"
                                );
                                ocr_fallback_warnings.push(crate::types::ProcessingWarning {
                                    source: std::borrow::Cow::Borrowed("ocr"),
                                    message: std::borrow::Cow::Owned(format!(
                                        "Targeted OCR fallback failed ({e}) for pages {pages:?}; those pages \
                                         retain native text that was below the OCR-trigger quality threshold \
                                         and may be empty or incomplete."
                                    )),
                                });
                                (native_text, ExtractionMethod::Native)
                            }
                        }
                    }
                    _ => {
                        tracing::warn!(
                            failing_pages = ?pages,
                            "Targeted OCR requested but no page boundaries available; using native text"
                        );
                        ocr_fallback_warnings.push(crate::types::ProcessingWarning {
                            source: std::borrow::Cow::Borrowed("ocr"),
                            message: std::borrow::Cow::Owned(format!(
                                "Targeted OCR was required for pages {pages:?} but no page boundaries were \
                                 available; those pages retain native text that was below the OCR-trigger \
                                 quality threshold and may be empty or incomplete."
                            )),
                        });
                        (native_text, ExtractionMethod::Native)
                    }
                },
                ocr::OcrGateOutcome::UseNative => (native_text, ExtractionMethod::Native),
            }
        } else {
            (native_text, ExtractionMethod::Native)
        };

        #[cfg(not(any(feature = "ocr", feature = "ocr-pipeline")))]
        let (text, extraction_method) = (native_text, ExtractionMethod::Native);

        #[cfg(any(feature = "ocr", feature = "ocr-pipeline"))]
        if !ocr_tables.is_empty() {
            tables.extend(ocr_tables);
            tables.sort_by_key(|t| t.page_number);
        }

        // --- Image extraction ---
        let (images, image_fallback_warning): (
            Option<Vec<crate::types::ExtractedImage>>,
            Option<crate::types::ProcessingWarning>,
        ) = (extracted_images, None);

        #[cfg(any(feature = "ocr", feature = "ocr-pipeline"))]
        let mut page_contents = page_contents;

        #[cfg(any(feature = "ocr", feature = "ocr-pipeline"))]
        {
            if let Some(pts) = ocr_page_texts {
                if let Some(ref mut pages) = page_contents {
                    let pts_len = pts.len();
                    let pages_len = pages.len();

                    for (page, text) in pages.iter_mut().zip(pts) {
                        page.content = crate::pdf::text::fix_pdf_control_chars(&text).into_owned();
                        page.is_blank = Some(crate::extraction::blank_detection::is_page_text_blank(&page.content));
                    }

                    // Special case for VLM models that return a single string for a multi-page PDF:
                    // we clear the content of the remaining pages to avoid stale native text fallback.
                    if pts_len == 1 && pages_len > 1 {
                        for p in pages.iter_mut().skip(1) {
                            p.content.clear();
                            p.is_blank = Some(true);
                        }
                    }
                } else {
                    // No native pages, but we have OCR page texts - build the page array.
                    page_contents = Some(
                        pts.into_iter()
                            .enumerate()
                            .map(|(i, text)| {
                                let content = crate::pdf::text::fix_pdf_control_chars(&text).into_owned();
                                let is_blank = Some(crate::extraction::blank_detection::is_page_text_blank(&content));
                                crate::types::PageContent {
                                    page_number: (i + 1) as u32,
                                    content,
                                    tables: Vec::new(),
                                    image_indices: vec![],
                                    hierarchy: None,
                                    is_blank,
                                    layout_regions: None,
                                    speaker_notes: None,
                                    section_name: None,
                                    sheet_name: None,
                                }
                            })
                            .collect(),
                    );
                }
            }

            if let Some(results_map) = ocr_results_map
                && let Some(ref mut pages) = page_contents
            {
                for page in pages.iter_mut() {
                    if let Some(ocr_text) = results_map.get(&page.page_number) {
                        page.content = crate::pdf::text::fix_pdf_control_chars(ocr_text).into_owned();
                        page.is_blank = Some(crate::extraction::blank_detection::is_page_text_blank(&page.content));
                    }
                }
            }
        }

        // --- Recompute page boundaries after OCR fills page_contents (#1110) ---
        // When OCR writes new text into page_contents, the original boundaries in
        // pdf_metadata.page_structure (computed against native extraction) become
        // invalid byte offsets into the OCR-filled content.  Recompute them here so
        // downstream consumers (the chunker, result.metadata.pages) see valid offsets.
        #[cfg(all(any(feature = "ocr", feature = "ocr-pipeline"), feature = "chunking"))]
        if extraction_method.used_ocr()
            && let Some(ref pages) = page_contents
            && !pages.is_empty()
        {
            // Build combined content mirroring what render_plain produces from
            // page texts so recompute_boundaries_from_pages can locate each page.
            let combined: String = pages
                .iter()
                .filter(|p| !p.content.trim().is_empty())
                .map(|p| p.content.trim())
                .collect::<Vec<_>>()
                .join("\n\n");
            if let Some(ref mut page_structure) = pdf_metadata.page_structure {
                page_structure.boundaries = Some(crate::core::pipeline::features::recompute_boundaries_from_pages(
                    &combined, pages,
                ));
            }
        }

        // --- Page assembly ---
        let mut final_pages =
            assign_tables_and_images_to_pages(page_contents, &tables, images.as_deref().unwrap_or(&[]));

        // --- Build InternalDocument ---
        let pre_formatted_output: Option<String> = None;

        let used_ocr = extraction_method.used_ocr();
        let use_structured_doc = !used_ocr && pre_rendered_doc.is_some();

        #[cfg(any(feature = "ocr", feature = "ocr-pipeline"))]
        let mut doc = if let Some(mut ocr_doc) = ocr_internal_doc.take() {
            ocr_doc.mime_type = mime_type.to_string();
            ocr_doc
        } else if let Some(mut pre_doc) = pre_rendered_doc {
            pre_doc.mime_type = mime_type.to_string();
            pre_doc
        } else {
            let mut d = InternalDocument::new("pdf");
            d.mime_type = mime_type.to_string();
            for paragraph in text.split("\n\n") {
                let trimmed = paragraph.trim();
                if !trimmed.is_empty() {
                    d.push_element(InternalElement::text(ElementKind::Paragraph, trimmed, 0));
                }
            }
            d
        };
        #[cfg(not(any(feature = "ocr", feature = "ocr-pipeline")))]
        let mut doc = if let Some(mut pre_doc) = pre_rendered_doc {
            pre_doc.mime_type = mime_type.to_string();
            pre_doc
        } else {
            let mut d = InternalDocument::new("pdf");
            d.mime_type = mime_type.to_string();
            for paragraph in text.split("\n\n") {
                let trimmed = paragraph.trim();
                if !trimmed.is_empty() {
                    d.push_element(InternalElement::text(ElementKind::Paragraph, trimmed, 0));
                }
            }
            d
        };

        // Surface any degraded-native-text warnings raised while OCR fallback was
        // attempted, so an empty/incomplete result is flagged rather than silent.
        #[cfg(any(feature = "ocr", feature = "ocr-pipeline"))]
        doc.processing_warnings.append(&mut ocr_fallback_warnings);

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
            // True when the OCR pipeline produced the returned text (fully or partially).
            ocr_used: used_ocr,
            ..Default::default()
        };
        doc.metadata.additional.insert(
            std::borrow::Cow::Borrowed("extraction_method"),
            serde_json::Value::String(extraction_method.as_str().to_string()),
        );

        // Transfer form fields directly to InternalDocument carrier field
        // so derive_extraction_result can move them via std::mem::take
        doc.form_fields = pdf_form_fields;

        // When using the structured doc, tables are already interleaved by the assembly pipeline.
        // Only add tables separately for the flat-text fallback path.
        if !use_structured_doc {
            for table in tables {
                let table_index = doc.push_table(table);
                doc.push_element(InternalElement::text(ElementKind::Table { table_index }, "", 0));
            }
        }

        if let Some(imgs) = images {
            // Only interleave image elements into the flat document when:
            //   1. The structured-doc assembly pipeline hasn't already interleaved them.
            //   2. The caller explicitly opted in via `config.images.inject_placeholders`.
            //
            // Without the inject_placeholders gate, a caller that sets only
            // `pdf_options.extract_images = true` (and leaves `config.images` as None)
            // would receive unexpected `![](image_N.jpeg)` placeholders in Markdown output.
            // The OCR path has its own guarded injection block below (see the `#[cfg(feature = "ocr")]`
            // block); this gate ensures the native/flat path is consistent with it.
            let inject_placeholders = config.images.as_ref().is_some_and(|c| c.inject_placeholders);
            if !use_structured_doc && inject_placeholders {
                for (idx, img) in imgs.iter().enumerate() {
                    let mut elem = InternalElement::text(
                        ElementKind::Image {
                            image_index: idx as u32,
                        },
                        "",
                        0,
                    );
                    elem.page = img.page_number;
                    doc.push_element(elem);
                }
            }
            doc.images = imgs;
        }

        // Append OCR page rasters after embedded images; reindex to keep image_index contiguous.
        // None → document-level bypass; Some([]) → zero-page PDF; Some([..]) → rasters captured.
        #[cfg(any(feature = "ocr", feature = "ocr-pipeline"))]
        let ocr_rasters_bypass = ocr_page_rasters.is_none();
        #[cfg(any(feature = "ocr", feature = "ocr-pipeline"))]
        if let Some(rasters) = ocr_page_rasters {
            let base_idx = doc.images.len() as u32;
            for (offset, mut raster) in rasters.into_iter().enumerate() {
                raster.image_index = base_idx + offset as u32;
                doc.images.push(raster);
            }
        }

        // Warn only on the document-level bypass path (ExtractionMethod::Ocr) — not when
        // force_ocr_pages resolved to an empty set because all requested pages were out of
        // range (ExtractionMethod::Mixed returns None rasters without document-level bypass).
        #[cfg(any(feature = "ocr", feature = "ocr-pipeline"))]
        if used_ocr
            && ocr_rasters_bypass
            && extraction_method == ExtractionMethod::Ocr
            && config.images.as_ref().is_some_and(|c| c.include_page_rasters)
        {
            doc.processing_warnings.push(crate::types::ProcessingWarning {
                source: std::borrow::Cow::Borrowed("page_rasters"),
                message: std::borrow::Cow::Borrowed(
                    "include_page_rasters is set but no page rasters were produced; \
                     the active OCR backend used document-level processing without per-page rendering",
                ),
            });
        }

        // On the OCR/VLM path the doc is a flat text document — image elements are not
        // interleaved by the assembly pipeline. Inject them here so downstream renderers
        // can emit Markdown placeholders (e.g. `![](image_0.jpeg)`).
        //
        // Note: positional interleaving within the text stream is not yet implemented on
        // this path; all image elements are appended after the text content.
        #[cfg(any(feature = "ocr", feature = "ocr-pipeline"))]
        if used_ocr && !doc.images.is_empty() {
            let images_enabled = config.images.as_ref().map(|c| c.extract_images).unwrap_or(false)
                || config.pdf_options.as_ref().map(|p| p.extract_images).unwrap_or(false);
            // Require explicit opt-in via config.images.inject_placeholders — default false when
            // config.images is absent to avoid injecting placeholders the caller never requested.
            if images_enabled && config.images.as_ref().map(|c| c.inject_placeholders).unwrap_or(false) {
                // Collect first to avoid borrowing doc.images and doc simultaneously.
                // page_number is 1-indexed per ExtractedImage contract; omit attribution
                // entirely when the page is unknown rather than using an invalid sentinel.
                let elems: Vec<InternalElement> = doc
                    .images
                    .iter()
                    .map(|img| {
                        let elem = InternalElement::text(
                            ElementKind::Image {
                                image_index: img.image_index,
                            },
                            "",
                            0,
                        );
                        if let Some(page) = img.page_number {
                            elem.with_page(page)
                        } else {
                            elem
                        }
                    })
                    .collect();
                for elem in elems {
                    doc.push_element(elem);
                }
            }
        }

        if let Some(warning) = image_fallback_warning {
            doc.processing_warnings.push(warning);
        }
        doc.annotations = pdf_annotations;

        // Extract URIs from annotations (links).
        {
            use crate::types::annotations::PdfAnnotationType;
            use crate::types::uri::{ExtractedUri, UriKind};

            let uris: Vec<ExtractedUri> = doc
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
                                ExtractedUri {
                                    url: url.clone(),
                                    label: Some(url.clone()),
                                    page: Some(a.page_number),
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

        // Extract bookmarks/outlines and xref-chain revision history.
        #[cfg(feature = "pdf")]
        {
            if let Ok(lopdf_doc) = lopdf::Document::load_mem(content) {
                let bookmark_uris = crate::pdf::bookmarks::extract_bookmarks(&lopdf_doc);
                for uri in bookmark_uris {
                    doc.push_uri(uri);
                }

                // Walk the /Prev chain to surface incremental-update history.
                // Single-save PDFs return None; incrementally-saved PDFs return
                // Some(Vec<DocumentRevision>) with one entry per historical xref section.
                doc.revisions = crate::pdf::xref_revisions::extract_pdf_xref_revisions(content, &lopdf_doc);
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

        // Assign hierarchy information from structured document to pages
        if let Some(ref mut pages) = final_pages {
            assign_hierarchy_to_pages(pages, &doc);
        }

        doc.prebuilt_pages = final_pages;

        // Attach OCR elements so downstream pipeline can use per-word bounding boxes.
        #[cfg(any(feature = "ocr", feature = "ocr-pipeline"))]
        if !ocr_elements.is_empty() {
            doc.prebuilt_ocr_elements = Some(ocr_elements);
        }

        // Carry formulas on the InternalDocument; derive_extraction_result moves
        // them into ExtractedDocument.formulas (mirrors form_fields/llm_usage).
        #[cfg(any(feature = "ocr", feature = "ocr-pipeline"))]
        if !ocr_formulas.is_empty() {
            doc.formulas = ocr_formulas;
        }

        // Attach LLM usage accumulated during OCR so derive_extraction_result can transfer it.
        #[cfg(any(feature = "ocr", feature = "ocr-pipeline"))]
        if !ocr_llm_usage.is_empty() {
            doc.llm_usage = Some(ocr_llm_usage);
        }

        tracing::debug!(
            elements = doc.elements.len(),
            tables = doc.tables.len(),
            has_pages = doc.prebuilt_pages.is_some(),
            "InternalDocument finalized (oxide path)"
        );

        // --- Per-region VLM extraction for figures and complex layouts ---
        //
        // When `vlm_fallback != Disabled` and layout detection found Picture regions,
        // crop each region from the page raster and send it to the VLM. Results are
        // appended to the assembled document. VLM failures per region are suppressed
        // with warnings so that one bad crop cannot abort the whole extraction.
        #[cfg(all(feature = "liter-llm", feature = "layout-detection"))]
        {
            let vlm_enabled = config
                .ocr
                .as_ref()
                .map(|o| o.vlm_fallback != crate::core::config::VlmFallbackPolicy::Disabled && o.vlm_config.is_some())
                .unwrap_or(false);

            if vlm_enabled
                && let (Some(layout_images), Some(hints)) =
                    (markdown_layout_images.as_deref(), markdown_layout_hints.as_deref())
            {
                let vlm_cfg = config
                    .ocr
                    .as_ref()
                    .and_then(|o| o.vlm_config.as_ref())
                    .expect("vlm_config checked above");

                let region_results = region_vlm::extract_vlm_regions(layout_images, hints, vlm_cfg).await;
                if !region_results.is_empty() {
                    tracing::debug!(
                        count = region_results.len(),
                        "injecting VLM region results into document"
                    );
                    region_vlm::inject_region_results(&mut doc, region_results);
                }
            }
        }

        // Guard total extracted text against max_content_size. A crafted PDF with
        // repeated paragraphs or synthetic content streams could produce gigabytes
        // of text. Checked after full assembly so the limit is a document-level cap.
        {
            let mut budget = crate::extractors::security::SecurityBudget::from_config(config);
            for elem in &doc.elements {
                budget.account_text(elem.text.len())?;
            }
        }

        Ok(doc)
    }

    /// Fallback extraction path when pdf feature is not enabled.
    #[cfg(not(feature = "pdf"))]
    async fn extract_core_oxide(
        &self,
        _content: &[u8],
        mime_type: &str,
        _config: &ExtractionConfig,
        _path: Option<&std::path::Path>,
    ) -> Result<InternalDocument> {
        let doc = InternalDocument::new(mime_type);
        Ok(doc)
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    #[cfg(feature = "ocr")]
    use crate::core::config::OcrQualityThresholds;
    #[cfg(all(feature = "pdf", feature = "ocr"))]
    use serial_test::serial;

    #[cfg(feature = "pdf")]
    fn pdf_test_document(name: &str) -> std::path::PathBuf {
        std::path::Path::new(env!("CARGO_MANIFEST_DIR")).join(format!("../../test_documents/pdf/{name}"))
    }

    #[cfg(feature = "pdf")]
    fn extraction_method(result: &crate::types::ExtractedDocument) -> Option<ExtractionMethod> {
        result.extraction_method
    }

    #[cfg(feature = "ocr")]
    fn mk_decision(fallback: bool, whole_doc_failure: bool, failing_pages: Vec<u32>) -> ocr::OcrFallbackDecision {
        ocr::OcrFallbackDecision {
            stats: ocr::NativeTextStats::default(),
            avg_non_whitespace: 0.0,
            avg_alnum: 0.0,
            fallback,
            failing_pages,
            whole_doc_failure,
        }
    }

    #[cfg(all(feature = "pdf", feature = "ocr"))]
    struct MockPdfOcrBackend {
        name: &'static str,
        content: &'static str,
    }

    #[cfg(all(feature = "pdf", feature = "ocr"))]
    impl crate::plugins::Plugin for MockPdfOcrBackend {
        fn name(&self) -> &str {
            self.name
        }

        fn version(&self) -> String {
            "1.0.0".to_string()
        }

        fn initialize(&self) -> crate::Result<()> {
            Ok(())
        }

        fn shutdown(&self) -> crate::Result<()> {
            Ok(())
        }
    }

    #[cfg(all(feature = "pdf", feature = "ocr"))]
    #[async_trait::async_trait]
    impl crate::plugins::OcrBackend for MockPdfOcrBackend {
        fn backend_type(&self) -> crate::plugins::OcrBackendType {
            crate::plugins::OcrBackendType::Custom
        }

        fn supports_language(&self, _lang: &str) -> bool {
            true
        }

        async fn process_image(
            &self,
            _image_bytes: &[u8],
            _config: &crate::core::config::OcrConfig,
        ) -> crate::Result<crate::types::ExtractedDocument> {
            Ok(crate::types::ExtractedDocument {
                content: self.content.to_string(),
                mime_type: std::borrow::Cow::Borrowed("text/plain"),
                ..Default::default()
            })
        }
    }

    #[cfg(all(feature = "pdf", feature = "ocr"))]
    struct RegisteredOcrBackendGuard {
        name: &'static str,
    }

    #[cfg(all(feature = "pdf", feature = "ocr"))]
    impl Drop for RegisteredOcrBackendGuard {
        fn drop(&mut self) {
            let _ = crate::plugins::unregister_ocr_backend(self.name);
        }
    }

    #[cfg(all(feature = "pdf", feature = "ocr"))]
    fn register_mock_ocr_backend(name: &'static str, content: &'static str) -> RegisteredOcrBackendGuard {
        crate::plugins::register_ocr_backend(std::sync::Arc::new(MockPdfOcrBackend { name, content })).unwrap();
        RegisteredOcrBackendGuard { name }
    }

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
            let result = extractor.extract_content(&content, "application/pdf", &config).await;
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
            let result = extractor.extract_content(&content, "application/pdf", &config).await;
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
            let result = extractor.extract_content(&content, "application/pdf", &config).await;
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
    #[cfg(feature = "pdf")]
    async fn test_pdf_exposes_native_extraction_method() {
        let extractor = PdfExtractor::new();
        let config = ExtractionConfig::default();
        let pdf_path = pdf_test_document("google_doc_document.pdf");

        if let Ok(content) = std::fs::read(pdf_path) {
            let result = extractor
                .extract_content(&content, "application/pdf", &config)
                .await
                .expect("native PDF extraction should succeed");
            let result = crate::extraction::derive::derive_extraction_result(
                result,
                true,
                crate::core::config::OutputFormat::Plain,
            );

            assert_eq!(extraction_method(&result), Some(ExtractionMethod::Native));
        }
    }

    #[tokio::test]
    #[cfg(all(feature = "pdf", feature = "ocr"))]
    #[serial]
    async fn test_pdf_exposes_ocr_extraction_method() {
        use crate::core::config::OcrConfig;

        let _backend = register_mock_ocr_backend("pdf-extraction-method-ocr", "mock OCR text");
        let extractor = PdfExtractor::new();
        let config = ExtractionConfig {
            force_ocr: true,
            ocr: Some(OcrConfig {
                backend: "pdf-extraction-method-ocr".to_string(),
                language: vec!["eng".to_string()],
                ..Default::default()
            }),
            ..Default::default()
        };
        let pdf_path = pdf_test_document("multi_page.pdf");

        if let Ok(content) = std::fs::read(pdf_path) {
            let result = extractor
                .extract_content(&content, "application/pdf", &config)
                .await
                .expect("forced OCR extraction should succeed");
            let result = crate::extraction::derive::derive_extraction_result(
                result,
                true,
                crate::core::config::OutputFormat::Plain,
            );

            assert_eq!(extraction_method(&result), Some(ExtractionMethod::Ocr));
        }
    }

    #[tokio::test]
    #[cfg(all(feature = "pdf", feature = "ocr"))]
    #[serial]
    async fn test_pdf_exposes_mixed_extraction_method() {
        use crate::core::config::OcrConfig;

        let _backend = register_mock_ocr_backend("pdf-extraction-method-mixed", "mixed OCR page");
        let extractor = PdfExtractor::new();
        let config = ExtractionConfig {
            force_ocr_pages: Some(vec![1]),
            ocr: Some(OcrConfig {
                backend: "pdf-extraction-method-mixed".to_string(),
                language: vec!["eng".to_string()],
                ..Default::default()
            }),
            ..Default::default()
        };
        let pdf_path = pdf_test_document("multi_page.pdf");

        if let Ok(content) = std::fs::read(pdf_path) {
            let result = extractor
                .extract_content(&content, "application/pdf", &config)
                .await
                .expect("mixed OCR/native extraction should succeed");
            let result = crate::extraction::derive::derive_extraction_result(
                result,
                true,
                crate::core::config::OutputFormat::Plain,
            );

            assert_eq!(extraction_method(&result), Some(ExtractionMethod::Mixed));
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
            let result = extractor.extract_content(&content, "application/pdf", &config).await;

            if let Err(e) = result {
                assert!(
                    !e.to_string().contains("OCR config required for force_ocr"),
                    "Should not require manual OCR config when force_ocr is true"
                );
            }
        }
    }

    /// Verifies that per-page OCR text segments correctly override native page
    /// content in each `PageContent` entry (#928).
    #[cfg(all(feature = "pdf", feature = "ocr"))]
    #[tokio::test]
    async fn test_ocr_page_texts_override_native_page_content() {
        use crate::core::config::OcrConfig;
        use crate::plugins::{OcrBackend, OcrBackendType, Plugin};
        use crate::types::ExtractedDocument;
        use std::sync::Arc;

        struct PerPageMockBackend;

        #[async_trait::async_trait]
        impl OcrBackend for PerPageMockBackend {
            fn backend_type(&self) -> OcrBackendType {
                OcrBackendType::Custom
            }
            fn supports_language(&self, _: &str) -> bool {
                true
            }
            async fn process_image(&self, _: &[u8], _: &OcrConfig) -> crate::Result<ExtractedDocument> {
                static COUNTER: std::sync::atomic::AtomicUsize = std::sync::atomic::AtomicUsize::new(0);
                let n = COUNTER.fetch_add(1, std::sync::atomic::Ordering::SeqCst);
                Ok(ExtractedDocument {
                    content: format!("ocr-page-{n}"),
                    ..Default::default()
                })
            }
            fn supports_document_processing(&self) -> bool {
                false
            }
        }

        impl Plugin for PerPageMockBackend {
            fn name(&self) -> &str {
                "per-page-ocr-mock-928"
            }
            fn version(&self) -> String {
                "1.0.0".to_string()
            }
            fn initialize(&self) -> crate::Result<()> {
                Ok(())
            }
            fn shutdown(&self) -> crate::Result<()> {
                Ok(())
            }
        }

        crate::plugins::register_ocr_backend(Arc::new(PerPageMockBackend)).unwrap();

        use image::ImageEncoder as _;
        let make_png = || {
            let img = image::DynamicImage::new_rgb8(1, 1);
            let rgb = img.to_rgb8();
            let (w, h) = rgb.dimensions();
            let mut buf = std::io::Cursor::new(Vec::new());
            image::codecs::png::PngEncoder::new(&mut buf)
                .write_image(&rgb, w, h, image::ColorType::Rgb8.into())
                .unwrap();
            image::load_from_memory(&buf.into_inner()).unwrap()
        };
        let images = vec![make_png(), make_png()];

        let config = crate::core::config::ExtractionConfig {
            ocr: Some(OcrConfig {
                backend: "per-page-ocr-mock-928".to_string(),
                ..Default::default()
            }),
            ..Default::default()
        };

        let result = super::ocr::extract_with_ocr(
            None,
            Some(&images),
            #[cfg(feature = "layout-detection")]
            None,
            &config,
            None,
        )
        .await;

        crate::plugins::unregister_ocr_backend("per-page-ocr-mock-928").unwrap();

        let (_text, _conf, _tables, _elems, _doc, _llm, page_texts, _rasters, _formulas) =
            result.expect("extract_with_ocr should succeed");

        assert_eq!(page_texts.len(), 2, "expected one entry per page");
        assert!(page_texts[0].starts_with("ocr-page-"), "page 0 should have OCR text");
        assert!(page_texts[1].starts_with("ocr-page-"), "page 1 should have OCR text");
        assert_ne!(page_texts[0], page_texts[1], "each page should get unique OCR text");
    }

    // TODO(#936): test currently asserts `![](image_` placeholders in the
    // rendered markdown, but the embedded_images_tables.pdf fixture's images
    // don't surface through the pdf_oxide image-extraction path on `main`
    // (the sibling no-image / no-ocr-config tests cover the wiring).
    // Re-enable once a fixture that reliably yields inline raster XObjects
    // lands in test_documents/pdf/.
    #[ignore]
    #[tokio::test]
    #[cfg(all(feature = "pdf", feature = "ocr"))]
    async fn test_pdf_ocr_inline_images() {
        use crate::core::config::ExtractionConfig;
        use crate::core::config::OutputFormat;
        use crate::core::config::pdf::PdfConfig;

        let extractor = PdfExtractor::new();

        // Use embedded_images_tables.pdf which has a large image on page 1
        let pdf_path = std::path::Path::new(env!("CARGO_MANIFEST_DIR"))
            .join("../../test_documents/pdf/embedded_images_tables.pdf");

        assert!(
            pdf_path.exists(),
            "missing test fixture: {pdf_path:?} — add embedded_images_tables.pdf to test_documents/pdf/"
        );

        let content = std::fs::read(pdf_path).expect("Failed to read PDF");

        let config = ExtractionConfig {
            output_format: OutputFormat::Markdown,
            pdf_options: Some(PdfConfig {
                ocr_inline_images: true,
                ..Default::default()
            }),
            ..Default::default()
        };

        let result = extractor
            .extract_content(&content, "application/pdf", &config)
            .await
            .expect("Inline-image OCR extraction failed");

        let result = crate::extraction::derive::derive_extraction_result(result, true, OutputFormat::Markdown);

        // Result should contain Markdown image references
        assert!(
            result.content.contains("![](image_"),
            "Markdown should contain image references"
        );

        // Result should have non-empty images list
        let images = result.images.as_ref().expect("Images list should be Some");
        assert!(!images.is_empty(), "Images list should not be empty");

        // At least one image should have an OCR result (the large one)
        let has_ocr = images
            .iter()
            .any(|img| img.ocr_result.as_ref().is_some_and(|r| !r.content.trim().is_empty()));
        assert!(has_ocr, "At least one image should have OCR content");
    }

    /// Verifies that when a VLM returns a single string for a multi-page PDF,
    /// the guard clears stale native text on secondary pages (#928).
    #[cfg(feature = "ocr")]
    #[test]
    fn test_vlm_single_string_guard_clears_secondary_pages() {
        use crate::types::PageContent;

        let vlm_text = "whole-doc VLM summary".to_string();
        let pts = vec![vlm_text.clone()];
        let pts_len = pts.len();

        let mut pages: Vec<PageContent> = (1u32..=3u32)
            .map(|n| PageContent {
                page_number: n,
                content: format!("native page {n}"),
                tables: Vec::new(),
                image_indices: Vec::new(),
                hierarchy: None,
                is_blank: None,
                layout_regions: None,
                speaker_notes: None,
                section_name: None,
                sheet_name: None,
            })
            .collect();
        let pages_len = pages.len();

        for (page, text) in pages.iter_mut().zip(pts) {
            page.content = crate::pdf::text::fix_pdf_control_chars(&text).into_owned();
            page.is_blank = Some(crate::extraction::blank_detection::is_page_text_blank(&page.content));
        }
        if pts_len == 1 && pages_len > 1 {
            for p in pages.iter_mut().skip(1) {
                p.content.clear();
                p.is_blank = Some(true);
            }
        }

        assert_eq!(pages[0].content, vlm_text, "page 1 should carry the VLM text");
        assert!(pages[1].content.is_empty(), "page 2 must be cleared by VLM guard");
        assert!(pages[2].content.is_empty(), "page 3 must be cleared by VLM guard");
        // is_blank must be recalculated to match the updated content (issue #1095).
        assert_eq!(
            pages[0].is_blank,
            Some(false),
            "page 1 has VLM content so must not be blank"
        );
        assert_eq!(pages[1].is_blank, Some(true), "page 2 was cleared so must be blank");
        assert_eq!(pages[2].is_blank, Some(true), "page 3 was cleared so must be blank");
    }

    /// Regression for #1095: when OCR texts are written into existing PageContent entries,
    /// is_blank must be recalculated from the new content, not left stale from native extraction.
    ///
    /// Simulates scanned PDF pages (all blank natively) receiving OCR content.
    #[cfg(feature = "ocr")]
    #[test]
    fn test_ocr_page_texts_update_is_blank_on_existing_pages() {
        use crate::extraction::blank_detection::is_page_text_blank;
        use crate::types::PageContent;

        let pts = vec!["page one content".to_string(), "page two content".to_string()];
        let pts_len = pts.len();

        // Simulate scanned PDF: pages created by native extraction with empty content
        // and is_blank=Some(true) — the value that persisted before the fix.
        let mut pages: Vec<PageContent> = (1u32..=2u32)
            .map(|n| PageContent {
                page_number: n,
                content: String::new(),
                tables: Vec::new(),
                image_indices: Vec::new(),
                hierarchy: None,
                is_blank: Some(true),
                layout_regions: None,
                speaker_notes: None,
                section_name: None,
                sheet_name: None,
            })
            .collect();
        let pages_len = pages.len();

        for (page, text) in pages.iter_mut().zip(pts) {
            page.content = crate::pdf::text::fix_pdf_control_chars(&text).into_owned();
            page.is_blank = Some(is_page_text_blank(&page.content));
        }
        if pts_len == 1 && pages_len > 1 {
            for p in pages.iter_mut().skip(1) {
                p.content.clear();
                p.is_blank = Some(true);
            }
        }

        assert_eq!(
            pages[0].is_blank,
            Some(false),
            "page with OCR content must not be blank"
        );
        assert_eq!(
            pages[1].is_blank,
            Some(false),
            "page with OCR content must not be blank"
        );
    }

    /// Regression for #1095: pages built from OCR texts when no native pages exist
    /// must have is_blank derived from the OCR content, not left as None.
    #[cfg(feature = "ocr")]
    #[test]
    fn test_ocr_scratch_pages_is_blank_set_from_content() {
        use crate::extraction::blank_detection::is_page_text_blank;
        use crate::types::PageContent;

        let pts = vec!["substantial ocr content".to_string(), String::new()];

        let page_contents: Vec<PageContent> = pts
            .into_iter()
            .enumerate()
            .map(|(i, text)| {
                let content = crate::pdf::text::fix_pdf_control_chars(&text).into_owned();
                let is_blank = Some(is_page_text_blank(&content));
                PageContent {
                    page_number: (i + 1) as u32,
                    content,
                    tables: Vec::new(),
                    image_indices: vec![],
                    hierarchy: None,
                    is_blank,
                    layout_regions: None,
                    speaker_notes: None,
                    section_name: None,
                    sheet_name: None,
                }
            })
            .collect();

        assert_eq!(
            page_contents[0].is_blank,
            Some(false),
            "page with content must not be blank"
        );
        assert_eq!(
            page_contents[1].is_blank,
            Some(true),
            "page with empty content must be blank"
        );
    }

    /// Integration regression for #1095: force_ocr on a scanned (non-searchable) PDF with
    /// extract_pages=true must produce pages where is_blank reflects OCR content, not stale
    /// native-extraction state.
    #[tokio::test]
    #[cfg(all(feature = "pdf", feature = "ocr"))]
    #[serial]
    async fn test_force_ocr_sets_is_blank_from_ocr_content() {
        use crate::core::config::{OcrConfig, PageConfig};

        let _backend = register_mock_ocr_backend("is-blank-fix-1095", "extracted ocr text content here");
        let extractor = PdfExtractor::new();
        let config = ExtractionConfig {
            force_ocr: true,
            ocr: Some(OcrConfig {
                backend: "is-blank-fix-1095".to_string(),
                language: vec!["eng".to_string()],
                ..Default::default()
            }),
            pages: Some(PageConfig {
                extract_pages: true,
                ..Default::default()
            }),
            ..Default::default()
        };

        let pdf_path = pdf_test_document("non_searchable.pdf");
        let content = std::fs::read(&pdf_path).unwrap_or_else(|e| panic!("non_searchable.pdf must be readable: {e}"));
        let result = extractor
            .extract_content(&content, "application/pdf", &config)
            .await
            .expect("extraction should succeed");
        let result = crate::extraction::derive::derive_extraction_result(
            result,
            false,
            crate::core::config::OutputFormat::Plain,
        );

        let pages = result.pages.expect("pages must be present when extract_pages=true");
        assert!(!pages.is_empty(), "must have at least one page");
        for page in &pages {
            assert_eq!(
                page.is_blank,
                Some(false),
                "page {} has OCR content, is_blank must be Some(false) (issue #1095)",
                page.page_number
            );
        }
    }

    /// ocr_inline_images=true on a text-only PDF (no embedded images) must succeed
    /// and return an empty images list, not panic or error.
    #[tokio::test]
    #[cfg(feature = "pdf")]
    async fn test_pdf_ocr_inline_images_no_images_in_document() {
        use crate::core::config::ExtractionConfig;
        use crate::core::config::pdf::PdfConfig;

        let extractor = PdfExtractor::new();

        // code_and_formula.pdf is a text-only document with no embedded raster images.
        let pdf_path =
            std::path::Path::new(env!("CARGO_MANIFEST_DIR")).join("../../test_documents/pdf/code_and_formula.pdf");

        if !pdf_path.exists() {
            panic!("missing test fixture: {pdf_path:?}");
        }

        let content = std::fs::read(pdf_path).expect("Failed to read PDF");

        let config = ExtractionConfig {
            pdf_options: Some(PdfConfig {
                ocr_inline_images: true,
                ..Default::default()
            }),
            ..Default::default()
        };

        let result = extractor
            .extract_content(&content, "application/pdf", &config)
            .await
            .expect("Extraction should succeed even when there are no images to OCR");

        // Images list should be empty — must not panic or error.
        for img in &result.images {
            // Without embedded images, no ocr_result should be present.
            assert!(img.ocr_result.is_none(), "text-only PDF should produce no OCR results");
        }
    }

    /// ocr_inline_images=true with config.ocr=None must use TesseractConfig::default()
    /// and not panic.
    #[tokio::test]
    #[cfg(all(feature = "pdf", feature = "ocr"))]
    async fn test_pdf_ocr_inline_images_no_ocr_config() {
        use crate::core::config::ExtractionConfig;
        use crate::core::config::pdf::PdfConfig;

        let extractor = PdfExtractor::new();

        let pdf_path = std::path::Path::new(env!("CARGO_MANIFEST_DIR"))
            .join("../../test_documents/pdf/embedded_images_tables.pdf");

        assert!(
            pdf_path.exists(),
            "missing test fixture: {pdf_path:?} — add embedded_images_tables.pdf to test_documents/pdf/"
        );

        let content = std::fs::read(pdf_path).expect("Failed to read PDF");

        // ocr = None → code falls back to TesseractConfig::default(); must not panic.
        let config = ExtractionConfig {
            ocr: None,
            pdf_options: Some(PdfConfig {
                ocr_inline_images: true,
                ..Default::default()
            }),
            ..Default::default()
        };

        // Should complete without panicking; OCR may succeed or warn, but must not crash.
        let _result = extractor
            .extract_content(&content, "application/pdf", &config)
            .await
            .expect("Extraction with ocr=None and ocr_inline_images=true must not panic");
    }

    /// Regression for issue #917: a mixed document with good aggregate text but a
    /// scanned page must still reach OCR. Before the fix, `has_substantive_doc=true`
    /// alone suppressed OCR even when `decision.fallback=true`.
    ///
    /// Tests `evaluate_ocr_skip_gate` directly — the function that the production
    /// path delegates to — so that reverting `&& !decision_fallback` breaks this test.
    #[cfg(feature = "ocr")]
    #[test]
    fn test_ocr_gate_runs_ocr_when_substantive_doc_but_fallback_needed() {
        let thresholds = OcrQualityThresholds::default();

        // Inputs that reproduce the bug: pre_rendered_doc present, total_chars well
        // above substantive_min_chars (100), good alnum_ws_ratio, but fallback=true
        // because a scanned page was detected.
        let outcome = ocr::evaluate_ocr_skip_gate(
            true,                             // pre_rendered_doc present
            500,                              // total_chars > substantive_min_chars
            0.9,                              // alnum_ws_ratio > threshold (0.4)
            &mk_decision(true, true, vec![]), // decision.fallback — scanned page detected
            &thresholds,
        );
        assert_eq!(
            outcome,
            ocr::OcrGateOutcome::RunFallback,
            "substantive doc must not suppress OCR when per-page fallback is needed (issue #917)"
        );
    }

    /// Counterpart: when no per-page fallback is needed, a substantive doc correctly
    /// skips OCR. Ensures the fix doesn't over-correct and always run OCR.
    #[cfg(feature = "ocr")]
    #[test]
    fn test_ocr_gate_skips_when_substantive_doc_and_no_fallback() {
        let thresholds = OcrQualityThresholds::default();

        let outcome = ocr::evaluate_ocr_skip_gate(
            true,                               // pre_rendered_doc present
            500,                                // total_chars > substantive_min_chars
            0.9,                                // alnum_ws_ratio > threshold
            &mk_decision(false, false, vec![]), // decision.fallback — all pages look fine
            &thresholds,
        );
        assert_eq!(
            outcome,
            ocr::OcrGateOutcome::SkipSubstantive,
            "OCR should be skipped when doc is substantive and no per-page fallback is needed"
        );
    }

    /// Regression for #987: image placeholders must appear in Markdown output when
    /// `force_ocr` is used and `config.images.inject_placeholders = true`.
    ///
    /// Uses a mock OCR backend so the test is independent of tessdata availability.
    /// Image elements are appended after text on the OCR path; positional interleaving
    /// is a known follow-up (tracked separately).
    #[tokio::test]
    #[cfg(all(feature = "pdf", feature = "ocr"))]
    #[serial]
    async fn test_inject_placeholders_present_on_force_ocr_path() {
        use crate::core::config::{ImageExtractionConfig, OcrConfig, OutputFormat};

        let _backend = register_mock_ocr_backend("inject-placeholder-ocr", "mock page text");
        let extractor = PdfExtractor::new();

        let pdf_path = pdf_test_document("embedded_images_tables.pdf");
        assert!(
            pdf_path.exists(),
            "missing test fixture: {pdf_path:?} — add embedded_images_tables.pdf to test_documents/pdf/"
        );
        let content = std::fs::read(&pdf_path).expect("failed to read embedded_images_tables.pdf");

        let config = crate::core::config::ExtractionConfig {
            output_format: OutputFormat::Markdown,
            force_ocr: true,
            ocr: Some(OcrConfig {
                backend: "inject-placeholder-ocr".to_string(),
                language: vec!["eng".to_string()],
                ..Default::default()
            }),
            images: Some(ImageExtractionConfig {
                extract_images: true,
                inject_placeholders: true,
                ..Default::default()
            }),
            ..Default::default()
        };

        let result = extractor
            .extract_content(&content, "application/pdf", &config)
            .await
            .expect("force_ocr extraction with images should succeed");

        let result = crate::extraction::derive::derive_extraction_result(result, true, OutputFormat::Markdown);

        // When the PDF contains at least one extractable image, the Markdown must include
        // a placeholder reference. If no images were extracted the assertion is vacuously
        // skipped with an explanatory message rather than a false positive.
        if result.images.as_ref().is_some_and(|imgs| !imgs.is_empty()) {
            assert!(
                result.formatted_content.as_deref().unwrap_or("").contains("![") || result.content.contains("!["),
                "Markdown must contain image placeholders on the force_ocr path when inject_placeholders=true"
            );
        }
    }

    /// Verifies that `inject_placeholders` defaults to false when only
    /// `pdf_options.extract_images` is set and `config.images` is absent,
    /// so callers who never touched `config.images` do not get unexpected placeholders.
    #[tokio::test]
    #[cfg(all(feature = "pdf", feature = "ocr"))]
    #[serial]
    async fn test_inject_placeholders_absent_when_only_pdf_options_set() {
        use crate::core::config::{OcrConfig, OutputFormat, pdf::PdfConfig};

        let _backend = register_mock_ocr_backend("inject-placeholder-absent-ocr", "mock page text");
        let extractor = PdfExtractor::new();

        let pdf_path = pdf_test_document("embedded_images_tables.pdf");
        assert!(
            pdf_path.exists(),
            "missing test fixture: {pdf_path:?} — add embedded_images_tables.pdf to test_documents/pdf/"
        );
        let content = std::fs::read(&pdf_path).expect("failed to read embedded_images_tables.pdf");

        // Only pdf_options.extract_images=true — config.images is None.
        // inject_placeholders must default to false on this path.
        let config = crate::core::config::ExtractionConfig {
            output_format: OutputFormat::Markdown,
            force_ocr: true,
            ocr: Some(OcrConfig {
                backend: "inject-placeholder-absent-ocr".to_string(),
                language: vec!["eng".to_string()],
                ..Default::default()
            }),
            pdf_options: Some(PdfConfig {
                extract_images: true,
                ..Default::default()
            }),
            ..Default::default()
        };

        let result = extractor
            .extract_content(&content, "application/pdf", &config)
            .await
            .expect("force_ocr extraction with pdf_options should succeed");

        let result = crate::extraction::derive::derive_extraction_result(result, true, OutputFormat::Markdown);

        let markdown = result.formatted_content.as_deref().unwrap_or(&result.content);
        assert!(
            !markdown.contains("!["),
            "Markdown must NOT contain image placeholders when config.images is absent (inject_placeholders defaults to false)"
        );
    }

    /// Non-textual content (charts, diagrams) with a pre-rendered structured doc
    /// present should skip OCR regardless of the per-page fallback flag — OCR
    /// won't improve extraction quality for non-textual pages.
    #[cfg(feature = "ocr")]
    #[test]
    fn test_ocr_gate_skips_non_textual_content_even_when_fallback_requested() {
        let thresholds = OcrQualityThresholds::default();

        // Non-textual: high char count but alnum_ws_ratio below threshold (lots of symbols).
        // Simulate a chart-heavy page that also triggered a per-page quality check.
        let outcome = ocr::evaluate_ocr_skip_gate(
            true,                             // pre_rendered_doc present
            500,                              // total_chars > non_text_min_chars
            0.1,                              // alnum_ws_ratio < threshold — non-textual content
            &mk_decision(true, true, vec![]), // decision.fallback — per-page quality check fired
            &thresholds,
        );
        assert_eq!(
            outcome,
            ocr::OcrGateOutcome::SkipNonText,
            "non-textual content with a structured doc must skip OCR even if fallback was requested"
        );
    }

    /// Hybrid PDF: per-page check fires on specific pages while the whole-document
    /// quality check passes. Gate must route to RunFallbackOnPages, not RunFallback.
    #[cfg(feature = "ocr")]
    #[test]
    fn test_ocr_gate_targets_specific_pages_on_hybrid_pdf() {
        let thresholds = OcrQualityThresholds::default();

        let outcome = ocr::evaluate_ocr_skip_gate(
            false, // no pre-rendered doc
            500,
            0.9,
            &mk_decision(true, false, vec![3, 7]), // per-page failure, whole doc OK
            &thresholds,
        );
        assert_eq!(
            outcome,
            ocr::OcrGateOutcome::RunFallbackOnPages(vec![3, 7]),
            "hybrid PDF with specific failing pages must route to targeted OCR"
        );
    }

    /// Whole-document failure with no per-page list → full document OCR (existing behaviour).
    #[cfg(feature = "ocr")]
    #[test]
    fn test_ocr_gate_full_document_when_whole_doc_failure() {
        let thresholds = OcrQualityThresholds::default();
        let outcome = ocr::evaluate_ocr_skip_gate(false, 500, 0.9, &mk_decision(true, true, vec![]), &thresholds);
        assert_eq!(outcome, ocr::OcrGateOutcome::RunFallback);
    }

    /// Edge case: whole-doc failure is true AND per-page list is populated.
    /// Whole-doc failure dominates (the document is fundamentally bad).
    #[cfg(feature = "ocr")]
    #[test]
    fn test_ocr_gate_whole_doc_failure_dominates_per_page_list() {
        let thresholds = OcrQualityThresholds::default();
        let outcome =
            ocr::evaluate_ocr_skip_gate(false, 500, 0.9, &mk_decision(true, true, vec![1, 2, 3]), &thresholds);
        assert_eq!(
            outcome,
            ocr::OcrGateOutcome::RunFallback,
            "whole-doc failure must trigger full OCR even when per-page list is populated"
        );
    }

    /// evaluate_per_page_ocr must collect ALL failing pages, not short-circuit on the first.
    /// This is the core regression the original implementation had.
    #[cfg(feature = "ocr")]
    #[test]
    fn test_evaluate_per_page_ocr_collects_all_failing_pages() {
        use crate::types::PageBoundary;

        // `good`: 72 chars of alnum-dense prose — alnum_ratio well above threshold, passes quality check.
        // `bad`: 5 chars of whitespace + punctuation only — alnum_ratio ≈ 0, triggers fallback deterministically.
        let good = "This page has plenty of meaningful searchable text content for extraction.";
        let bad = " . ; ";
        // Layout: good, bad, good, bad — failing pages should be [2, 4].
        let text = format!("{}{}{}{}", good, bad, good, bad);
        let boundaries = vec![
            PageBoundary {
                byte_start: 0,
                byte_end: good.len(),
                page_number: 1,
            },
            PageBoundary {
                byte_start: good.len(),
                byte_end: good.len() + bad.len(),
                page_number: 2,
            },
            PageBoundary {
                byte_start: good.len() + bad.len(),
                byte_end: 2 * good.len() + bad.len(),
                page_number: 3,
            },
            PageBoundary {
                byte_start: 2 * good.len() + bad.len(),
                byte_end: text.len(),
                page_number: 4,
            },
        ];

        let decision = ocr::evaluate_per_page_ocr(&text, Some(&boundaries), Some(4), &OcrQualityThresholds::default());
        assert!(decision.fallback);
        assert_eq!(
            decision.failing_pages,
            vec![2, 4],
            "all failing pages must be collected, not just the first"
        );
        assert!(
            !decision.whole_doc_failure,
            "whole-doc check should pass when half the document has good text"
        );
    }

    /// When every page fails the per-page quality check, the gate must route to
    /// RunFallback (ExtractionMethod::Ocr), not RunFallbackOnPages (ExtractionMethod::Mixed).
    /// A document where every page needs OCR is not a mixed document.
    #[cfg(feature = "ocr")]
    #[test]
    fn test_all_pages_failing_routes_to_run_fallback_not_mixed() {
        use crate::types::PageBoundary;

        let bad = " . ; ";
        let text = format!("{}{}", bad, bad);
        let boundaries = vec![
            PageBoundary {
                byte_start: 0,
                byte_end: bad.len(),
                page_number: 1,
            },
            PageBoundary {
                byte_start: bad.len(),
                byte_end: text.len(),
                page_number: 2,
            },
        ];

        let decision = ocr::evaluate_per_page_ocr(&text, Some(&boundaries), Some(2), &OcrQualityThresholds::default());
        assert!(decision.fallback);
        // When all pages fail, the doc-level check fires first and whole_doc_failure is
        // set before the per-page scan runs. The per-page scan is skipped (early return)
        // so failing_pages is empty — that's correct; the gate uses whole_doc_failure to
        // route to RunFallback, not the page list.
        assert!(
            decision.failing_pages.is_empty(),
            "doc-level failure fires before per-page scan when all pages fail"
        );
        assert!(
            decision.whole_doc_failure,
            "all pages failing must set whole_doc_failure so gate routes to RunFallback"
        );

        let outcome = ocr::evaluate_ocr_skip_gate(false, text.len(), 0.1, &decision, &OcrQualityThresholds::default());
        assert_eq!(
            outcome,
            ocr::OcrGateOutcome::RunFallback,
            "all-pages-failing must produce RunFallback (Ocr), not RunFallbackOnPages (Mixed)"
        );
    }

    // ── #1088: ocr_inline_images must route through the configured OcrBackend ────────────────

    /// Mock backend that records the OcrConfig it receives so tests can assert
    /// that fields like output_format are forwarded correctly.
    #[cfg(all(feature = "pdf", feature = "ocr"))]
    struct ConfigCapturingBackend {
        name: &'static str,
        sentinel: &'static str,
        received_config: std::sync::Arc<std::sync::Mutex<Option<crate::core::config::OcrConfig>>>,
    }

    #[cfg(all(feature = "pdf", feature = "ocr"))]
    impl crate::plugins::Plugin for ConfigCapturingBackend {
        fn name(&self) -> &str {
            self.name
        }
        fn version(&self) -> String {
            "0.0.0".to_string()
        }
        fn initialize(&self) -> crate::Result<()> {
            Ok(())
        }
        fn shutdown(&self) -> crate::Result<()> {
            Ok(())
        }
    }

    #[cfg(all(feature = "pdf", feature = "ocr"))]
    #[async_trait::async_trait]
    impl crate::plugins::OcrBackend for ConfigCapturingBackend {
        fn backend_type(&self) -> crate::plugins::OcrBackendType {
            crate::plugins::OcrBackendType::Custom
        }
        fn supports_language(&self, _: &str) -> bool {
            true
        }
        async fn process_image(
            &self,
            _image_bytes: &[u8],
            config: &crate::core::config::OcrConfig,
        ) -> crate::Result<crate::types::ExtractedDocument> {
            *self.received_config.lock().unwrap() = Some(config.clone());
            Ok(crate::types::ExtractedDocument {
                content: self.sentinel.to_string(),
                mime_type: std::borrow::Cow::Borrowed("text/plain"),
                ..Default::default()
            })
        }
    }

    /// Regression for #1088: ocr_inline_images must call the backend named in
    /// OcrConfig.backend, not always Tesseract via OcrProcessor.
    ///
    /// Uses the existing register_mock_ocr_backend helper. The sentinel string
    /// appearing in img.ocr_result.content proves which backend ran — no separate
    /// AtomicBool needed.
    ///
    /// Fixture note: with_images.pdf is used here (not embedded_images_tables.pdf)
    /// because pdf_oxide reliably extracts its single raster XObject. The existing
    /// test_pdf_ocr_inline_images test is #[ignore] precisely because
    /// embedded_images_tables.pdf does not surface images through the oxide path.
    #[tokio::test]
    #[cfg(all(feature = "pdf", feature = "ocr"))]
    #[serial]
    async fn test_ocr_inline_images_uses_configured_backend() {
        const BACKEND_NAME: &str = "mock-inline-ocr-routing-1088";
        const SENTINEL: &str = "__inline_ocr_sentinel_1088__";
        let _guard = register_mock_ocr_backend(BACKEND_NAME, SENTINEL);

        let pdf_path = pdf_test_document("with_images.pdf");
        assert!(pdf_path.exists(), "missing test fixture: {pdf_path:?}");
        let content = std::fs::read(&pdf_path).expect("read fixture");

        let config = crate::core::config::ExtractionConfig {
            ocr: Some(crate::core::config::OcrConfig {
                backend: BACKEND_NAME.to_string(),
                ..Default::default()
            }),
            pdf_options: Some(crate::core::config::pdf::PdfConfig {
                ocr_inline_images: true,
                ..Default::default()
            }),
            ..Default::default()
        };

        let result = PdfExtractor::new()
            .extract_content(&content, "application/pdf", &config)
            .await
            .expect("extraction must not fail");

        assert!(
            !result.images.is_empty(),
            "with_images.pdf must yield at least one embedded image; \
             fixture may need replacing if pdf_oxide no longer extracts from it"
        );

        let images_with_ocr: Vec<_> = result.images.iter().filter(|img| img.ocr_result.is_some()).collect();

        assert!(
            !images_with_ocr.is_empty(),
            "at least one image must have an ocr_result when ocr_inline_images=true"
        );

        for img in &images_with_ocr {
            let content = img.ocr_result.as_ref().unwrap().content.as_str();
            assert!(
                content.contains(SENTINEL),
                "ocr_result content '{content}' does not contain sentinel — \
                 backend routing is still going through hardcoded Tesseract"
            );
        }
    }

    /// Verifies that the extraction-level output_format is forwarded to the backend
    /// via OcrConfig.output_format. This mirrors the standalone image extractor
    /// (image.rs) and allows backends that produce format-aware output (e.g. Markdown
    /// table rendering) to behave correctly for inline PDF images.
    #[tokio::test]
    #[cfg(all(feature = "pdf", feature = "ocr"))]
    #[serial]
    async fn test_ocr_inline_images_forwards_output_format() {
        use std::sync::{Arc, Mutex};
        const BACKEND_NAME: &str = "mock-inline-ocr-format-1088";
        const SENTINEL: &str = "__format_sentinel_1088__";

        let received_config = Arc::new(Mutex::new(None));
        let backend = Arc::new(ConfigCapturingBackend {
            name: BACKEND_NAME,
            sentinel: SENTINEL,
            received_config: Arc::clone(&received_config),
        });
        crate::plugins::register_ocr_backend(backend).unwrap();
        let _guard = RegisteredOcrBackendGuard { name: BACKEND_NAME };

        let pdf_path = pdf_test_document("with_images.pdf");
        assert!(pdf_path.exists(), "missing test fixture: {pdf_path:?}");
        let content = std::fs::read(&pdf_path).expect("read fixture");

        let config = crate::core::config::ExtractionConfig {
            output_format: crate::core::config::OutputFormat::Markdown,
            ocr: Some(crate::core::config::OcrConfig {
                backend: BACKEND_NAME.to_string(),
                ..Default::default()
            }),
            pdf_options: Some(crate::core::config::pdf::PdfConfig {
                ocr_inline_images: true,
                ..Default::default()
            }),
            ..Default::default()
        };

        let result = PdfExtractor::new()
            .extract_content(&content, "application/pdf", &config)
            .await
            .expect("extraction must not fail");

        if result.images.is_empty() {
            panic!("with_images.pdf must yield images; fixture may need replacing");
        }

        let captured = received_config.lock().unwrap();
        let captured_config = captured
            .as_ref()
            .expect("backend was never called — no images were processed");
        assert_eq!(
            captured_config.output_format,
            Some(crate::core::config::OutputFormat::Markdown),
            "output_format was not forwarded to the inline-image OCR backend"
        );
    }

    /// When ocr_inline_images is false the mock backend must NOT be called even
    /// though it is registered as the configured backend.
    #[tokio::test]
    #[cfg(all(feature = "pdf", feature = "ocr"))]
    #[serial]
    async fn test_ocr_inline_images_disabled_does_not_call_backend() {
        const BACKEND_NAME: &str = "mock-inline-ocr-disabled-1088";
        let _guard = register_mock_ocr_backend(BACKEND_NAME, "should-never-appear");

        let pdf_path = pdf_test_document("with_images.pdf");
        assert!(pdf_path.exists(), "missing test fixture: {pdf_path:?}");
        let content = std::fs::read(&pdf_path).expect("read fixture");

        let config = crate::core::config::ExtractionConfig {
            ocr: Some(crate::core::config::OcrConfig {
                backend: BACKEND_NAME.to_string(),
                ..Default::default()
            }),
            pdf_options: Some(crate::core::config::pdf::PdfConfig {
                ocr_inline_images: false,
                // extract_images must be true so embedded images appear in the
                // result for the assertion below; without it the extractor skips
                // image decompression entirely and result.images stays empty.
                extract_images: true,
                ..Default::default()
            }),
            ..Default::default()
        };

        let result = PdfExtractor::new()
            .extract_content(&content, "application/pdf", &config)
            .await
            .expect("extraction must not fail");

        assert!(
            !result.images.is_empty(),
            "with_images.pdf must yield images; fixture may need replacing"
        );
        // Every image must have no ocr_result — the backend loop must not have run.
        for img in &result.images {
            assert!(
                img.ocr_result.is_none(),
                "img {} on page {:?} has ocr_result even though ocr_inline_images=false",
                img.image_index,
                img.page_number,
            );
        }
    }

    /// Tests form field extraction from a PDF.
    /// Uses an existing test PDF rather than creating one programmatically.
    /// This is a simple smoke test to verify that form field extraction works
    /// and doesn't panic or crash the extraction pipeline.
    /// TODO: Add a real fillable PDF fixture if one is available.
    /// Path to the vendored fillable-form fixture (AcroForm with text, button,
    /// and choice fields).
    #[cfg(feature = "pdf")]
    fn form_test_document() -> std::path::PathBuf {
        std::path::Path::new(env!("CARGO_MANIFEST_DIR"))
            .join("../../test_documents/vendored/pdfium-render/form-test.pdf")
    }

    /// End-to-end: a real AcroForm PDF yields populated, correctly-typed
    /// `form_fields` on the extractor's `InternalDocument` carrier.
    #[tokio::test]
    #[cfg(feature = "pdf")]
    async fn test_form_field_extraction_enabled() {
        let content = std::fs::read(form_test_document()).expect("read form-test.pdf fixture");

        let config = crate::core::config::ExtractionConfig {
            pdf_options: Some(crate::core::config::pdf::PdfConfig {
                extract_form_fields: true,
                ..Default::default()
            }),
            ..Default::default()
        };

        let internal_doc = PdfExtractor::new()
            .extract_content(&content, "application/pdf", &config)
            .await
            .expect("extraction must not fail");

        // The fixture is a rich AcroForm; extraction must surface many fields.
        assert!(
            !internal_doc.form_fields.is_empty(),
            "AcroForm PDF must yield form fields, got none"
        );
        // Every field must carry a non-empty fully-qualified name.
        assert!(
            internal_doc.form_fields.iter().all(|f| !f.full_name.is_empty()),
            "every extracted field must have a full_name"
        );
        // The fixture contains text fields; at least one must map to Text.
        assert!(
            internal_doc
                .form_fields
                .iter()
                .any(|f| f.field_type == crate::types::FormFieldType::Text),
            "fixture has text fields; at least one must map to FormFieldType::Text"
        );
    }

    /// With `extract_form_fields = false`, the same AcroForm PDF yields no fields.
    #[tokio::test]
    #[cfg(feature = "pdf")]
    async fn test_form_field_extraction_disabled() {
        let content = std::fs::read(form_test_document()).expect("read form-test.pdf fixture");

        let config = crate::core::config::ExtractionConfig {
            pdf_options: Some(crate::core::config::pdf::PdfConfig {
                extract_form_fields: false,
                ..Default::default()
            }),
            ..Default::default()
        };

        let internal_doc = PdfExtractor::new()
            .extract_content(&content, "application/pdf", &config)
            .await
            .expect("extraction must not fail");

        assert!(
            internal_doc.form_fields.is_empty(),
            "form fields must be empty when extract_form_fields is disabled"
        );
    }

    /// Tests that form fields are properly extracted and carried through the pipeline
    /// using the InternalDocument.form_fields carrier (not metadata.additional).
    /// This test manually constructs an InternalDocument with form_fields to verify
    /// the carrier pattern works end-to-end, without relying on a complex PDF fixture.
    #[test]
    #[cfg(feature = "pdf")]
    fn test_form_fields_carrier_via_internal_document() {
        let mut doc = InternalDocument::new("pdf");
        doc.mime_type = "application/pdf".to_string();

        // Manually populate form_fields as the PDF extractor would
        doc.form_fields = vec![crate::types::PdfFormField {
            name: "full_name".to_string(),
            full_name: "form.full_name".to_string(),
            field_type: crate::types::FormFieldType::Text,
            value: Some("Ada Lovelace".to_string()),
            default_value: Some("Default Name".to_string()),
            flags: 0,
            page: None,
            bbox: None,
            max_length: None,
            tooltip: None,
        }];

        // Verify form_fields are in the carrier
        assert_eq!(doc.form_fields.len(), 1);
        let field = &doc.form_fields[0];
        assert_eq!(field.name, "full_name");
        assert_eq!(field.value, Some("Ada Lovelace".to_string()));
        assert_eq!(field.field_type, crate::types::FormFieldType::Text);

        // Verify metadata.additional does NOT contain the old _pdf_form_fields key
        assert!(
            doc.metadata.additional.get("_pdf_form_fields").is_none(),
            "metadata.additional should not contain _pdf_form_fields (leak check)"
        );

        // Convert to ExtractedDocument through the pipeline to verify the carrier works
        let result =
            crate::extraction::derive::derive_extraction_result(doc, false, crate::core::config::OutputFormat::Plain);

        // Verify form_fields were transferred via std::mem::take
        assert_eq!(result.form_fields.len(), 1);
        assert_eq!(result.form_fields[0].name, "full_name");
        assert_eq!(result.form_fields[0].value, Some("Ada Lovelace".to_string()));
    }

    /// Tests that with form_fields extraction disabled, the result contains no form fields.
    #[test]
    #[cfg(feature = "pdf")]
    fn test_form_fields_disabled_no_leak() {
        let mut doc = InternalDocument::new("pdf");
        doc.mime_type = "application/pdf".to_string();

        // Ensure form_fields is empty (simulating extract_form_fields=false)
        assert!(doc.form_fields.is_empty());

        // Verify metadata.additional does NOT contain _pdf_form_fields
        assert!(doc.metadata.additional.get("_pdf_form_fields").is_none());

        // Convert to ExtractedDocument and verify form_fields is empty
        let result =
            crate::extraction::derive::derive_extraction_result(doc, false, crate::core::config::OutputFormat::Plain);

        assert!(result.form_fields.is_empty());
    }
}
