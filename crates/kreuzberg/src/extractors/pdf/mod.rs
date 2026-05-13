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

use crate::Result;
use crate::core::config::ExtractionConfig;
use crate::plugins::{DocumentExtractor, Plugin};
use crate::types::internal::{ElementKind, InternalDocument, InternalElement};
use crate::types::{ExtractionMethod, Metadata};
use async_trait::async_trait;
#[cfg(feature = "tokio-runtime")]
use std::path::Path;

use extraction::extract_all_from_oxide_document;
#[cfg(feature = "ocr")]
use ocr::extract_with_ocr;
use pages::{assign_hierarchy_to_pages, assign_tables_and_images_to_pages};

/// Run OCR with optional layout detection on PDF bytes.
///
/// Returns `None` when layout detection is not configured or fails.
/// Failures are logged as warnings but do not propagate — extraction
/// continues without layout hints (graceful degradation).
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
    Vec<crate::types::LlmUsage>,
    Vec<String>,
)> {
    let default_ocr_config = crate::core::config::OcrConfig::default();
    let ocr_config = config.ocr.as_ref().unwrap_or(&default_ocr_config);

    #[cfg(feature = "layout-detection")]
    let layout_detections: Option<Vec<crate::layout::DetectionResult>> = None;

    // Check for pipeline configuration
    if let Some(pipeline) = ocr_config.effective_pipeline() {
        let (text, _ocr_tables, ocr_elements, pipeline_doc, llm_usage, ocr_pts) = ocr::run_ocr_pipeline(
            Some(content),
            None,
            #[cfg(feature = "layout-detection")]
            layout_detections.as_deref(),
            config,
            &pipeline,
            path,
        )
        .await?;
        return Ok((text, Vec::new(), ocr_elements, pipeline_doc, llm_usage, ocr_pts));
    }

    let (text, _mean_conf, ocr_tables, ocr_elements, ocr_doc, llm_usage, ocr_pts) = extract_with_ocr(
        Some(content),
        None,
        #[cfg(feature = "layout-detection")]
        layout_detections.as_deref(),
        config,
        path,
    )
    .await?;
    Ok((text, ocr_tables, ocr_elements, ocr_doc, llm_usage, ocr_pts))
}

/// PDF document extractor using pdf_oxide.
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
            pdf_metadata,
            native_text,
            mut tables,
            page_contents,
            boundaries,
            pre_rendered_doc,
            _has_font_encoding_issues,
            pdf_annotations,
            extracted_images,
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

        // --- OCR evaluation ---
        #[cfg(feature = "ocr")]
        let mut ocr_tables: Vec<crate::types::Table> = Vec::new();
        #[cfg(feature = "ocr")]
        let mut ocr_elements: Vec<crate::types::OcrElement> = Vec::new();
        #[cfg(feature = "ocr")]
        let mut ocr_internal_doc: Option<InternalDocument> = None;
        #[cfg(feature = "ocr")]
        let mut ocr_llm_usage: Vec<crate::types::LlmUsage> = Vec::new();
        #[cfg(feature = "ocr")]
        let mut ocr_page_texts: Option<Vec<String>> = None;
        #[cfg(feature = "ocr")]
        let mut ocr_results_map: Option<ahash::AHashMap<usize, String>> = None;

        #[cfg(feature = "ocr")]
        let (text, extraction_method) = if config.effective_disable_ocr() {
            (native_text, ExtractionMethod::Native)
        } else if config.force_ocr {
            let (ocr_text, ocr_tbls, ocr_elems, ocr_doc, llm_usage, ocr_pts) =
                run_ocr_with_layout(content, config, path).await?;
            ocr_tables = ocr_tbls;
            ocr_elements = ocr_elems;
            ocr_internal_doc = ocr_doc;
            ocr_llm_usage = llm_usage;
            ocr_page_texts = Some(ocr_pts);
            (ocr_text, ExtractionMethod::Ocr)
        } else if let Some(ref ocr_pages) = config.force_ocr_pages {
            if !ocr_pages.is_empty() {
                if let Some(ref bounds) = boundaries {
                    if !bounds.is_empty() {
                        let (mixed, results_map, mixed_llm_usage) =
                            ocr::extract_mixed_ocr_native(&native_text, bounds, ocr_pages, content, config, path)
                                .await?;
                        ocr_llm_usage = mixed_llm_usage;
                        ocr_results_map = Some(results_map);
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

            if std::env::var("KREUZBERG_DEBUG_OCR").is_ok() {
                eprintln!(
                    "[kreuzberg::pdf::ocr] fallback={} non_whitespace={} alnum={} meaningful_words={} \
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
                    "Skipping OCR: content is non-textual and pre-rendered structured doc available"
                );
                (native_text, ExtractionMethod::Native)
            } else if has_substantive_doc {
                tracing::debug!(
                    total_chars,
                    alnum_ws_ratio,
                    ocr_fallback = decision.fallback,
                    "Skipping OCR: pre-rendered structured doc available with substantive native text"
                );
                (native_text, ExtractionMethod::Native)
            } else if decision.fallback {
                match run_ocr_with_layout(content, config, path).await {
                    Ok((ocr_text, ocr_tbls, ocr_elems, ocr_doc, llm_usage, ocr_pts)) => {
                        ocr_tables = ocr_tbls;
                        ocr_elements = ocr_elems;
                        ocr_internal_doc = ocr_doc;
                        ocr_llm_usage = llm_usage;
                        ocr_page_texts = Some(ocr_pts);
                        (ocr_text, ExtractionMethod::Ocr)
                    }
                    Err(e) => {
                        tracing::warn!(
                            error = %e,
                            "OCR fallback failed; using native text extraction result"
                        );
                        (native_text, ExtractionMethod::Native)
                    }
                }
            } else {
                (native_text, ExtractionMethod::Native)
            }
        } else {
            (native_text, ExtractionMethod::Native)
        };

        #[cfg(not(feature = "ocr"))]
        let (text, extraction_method) = (native_text, ExtractionMethod::Native);

        #[cfg(feature = "ocr")]
        if !ocr_tables.is_empty() {
            tables.extend(ocr_tables);
            tables.sort_by_key(|t| t.page_number);
        }

        // --- Image extraction ---
        let (images, image_fallback_warning): (
            Option<Vec<crate::types::ExtractedImage>>,
            Option<crate::types::ProcessingWarning>,
        ) = (extracted_images, None);

        #[cfg(feature = "ocr")]
        let mut page_contents = page_contents;

        #[cfg(feature = "ocr")]
        {
            if let Some(pts) = ocr_page_texts {
                if let Some(ref mut pages) = page_contents {
                    let pts_len = pts.len();
                    let pages_len = pages.len();

                    for (page, text) in pages.iter_mut().zip(pts) {
                        page.content = text;
                    }

                    // Special case for VLM models that return a single string for a multi-page PDF:
                    // we clear the content of the remaining pages to avoid stale native text fallback.
                    if pts_len == 1 && pages_len > 1 {
                        for p in pages.iter_mut().skip(1) {
                            p.content.clear();
                        }
                    }
                } else {
                    // No native pages, but we have OCR page texts - build the page array.
                    page_contents = Some(
                        pts.into_iter()
                            .enumerate()
                            .map(|(i, text)| crate::types::PageContent {
                                page_number: i + 1,
                                content: text,
                                tables: Vec::new(),
                                images: Vec::new(),
                                hierarchy: None,
                                is_blank: None,
                                layout_regions: None,
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
                        page.content = ocr_text.clone();
                    }
                }
            }
        }

        // --- Page assembly ---
        let mut final_pages =
            assign_tables_and_images_to_pages(page_contents, &tables, images.as_deref().unwrap_or(&[]));

        // --- Build InternalDocument ---
        let pre_formatted_output: Option<String> = None;

        let used_ocr = extraction_method.used_ocr();
        let use_structured_doc = !used_ocr && pre_rendered_doc.is_some();

        #[cfg(feature = "ocr")]
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
        #[cfg(not(feature = "ocr"))]
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

        // Assign hierarchy information from structured document to pages
        if let Some(ref mut pages) = final_pages {
            assign_hierarchy_to_pages(pages, &doc);
        }

        doc.prebuilt_pages = final_pages;

        // Attach OCR elements so downstream pipeline can use per-word bounding boxes.
        #[cfg(feature = "ocr")]
        if !ocr_elements.is_empty() {
            doc.prebuilt_ocr_elements = Some(ocr_elements);
        }

        // Attach LLM usage accumulated during OCR so derive_extraction_result can transfer it.
        #[cfg(feature = "ocr")]
        if !ocr_llm_usage.is_empty() {
            doc.llm_usage = Some(ocr_llm_usage);
        }

        tracing::debug!(
            elements = doc.elements.len(),
            tables = doc.tables.len(),
            has_pages = doc.prebuilt_pages.is_some(),
            "InternalDocument finalized (oxide path)"
        );

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
    fn extraction_method(result: &crate::types::ExtractionResult) -> Option<ExtractionMethod> {
        result.extraction_method
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
        ) -> crate::Result<crate::types::ExtractionResult> {
            Ok(crate::types::ExtractionResult {
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
    #[cfg(feature = "pdf")]
    async fn test_pdf_exposes_native_extraction_method() {
        let extractor = PdfExtractor::new();
        let config = ExtractionConfig::default();
        let pdf_path = pdf_test_document("google_doc_document.pdf");

        if let Ok(content) = std::fs::read(pdf_path) {
            let result = extractor
                .extract_bytes(&content, "application/pdf", &config)
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
                language: "eng".to_string(),
                ..Default::default()
            }),
            ..Default::default()
        };
        let pdf_path = pdf_test_document("multi_page.pdf");

        if let Ok(content) = std::fs::read(pdf_path) {
            let result = extractor
                .extract_bytes(&content, "application/pdf", &config)
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
                language: "eng".to_string(),
                ..Default::default()
            }),
            ..Default::default()
        };
        let pdf_path = pdf_test_document("multi_page.pdf");

        if let Ok(content) = std::fs::read(pdf_path) {
            let result = extractor
                .extract_bytes(&content, "application/pdf", &config)
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
            let result = extractor.extract_bytes(&content, "application/pdf", &config).await;

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
        use crate::types::ExtractionResult;
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
            async fn process_image(&self, _: &[u8], _: &OcrConfig) -> crate::Result<ExtractionResult> {
                static COUNTER: std::sync::atomic::AtomicUsize = std::sync::atomic::AtomicUsize::new(0);
                let n = COUNTER.fetch_add(1, std::sync::atomic::Ordering::SeqCst);
                Ok(ExtractionResult {
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

        let (_text, _conf, _tables, _elems, _doc, _llm, page_texts) = result.expect("extract_with_ocr should succeed");

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
            .extract_bytes(&content, "application/pdf", &config)
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

        let mut pages: Vec<PageContent> = (1..=3)
            .map(|n| PageContent {
                page_number: n,
                content: format!("native page {n}"),
                tables: Vec::new(),
                images: Vec::new(),
                hierarchy: None,
                is_blank: None,
                layout_regions: None,
            })
            .collect();
        let pages_len = pages.len();

        for (page, text) in pages.iter_mut().zip(pts) {
            page.content = text;
        }
        if pts_len == 1 && pages_len > 1 {
            for p in pages.iter_mut().skip(1) {
                p.content.clear();
            }
        }

        assert_eq!(pages[0].content, vlm_text, "page 1 should carry the VLM text");
        assert!(pages[1].content.is_empty(), "page 2 must be cleared by VLM guard");
        assert!(pages[2].content.is_empty(), "page 3 must be cleared by VLM guard");
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
            .extract_bytes(&content, "application/pdf", &config)
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
            .extract_bytes(&content, "application/pdf", &config)
            .await
            .expect("Extraction with ocr=None and ocr_inline_images=true must not panic");
    }
}
