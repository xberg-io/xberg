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
    #[cfg_attr(feature = "otel", tracing::instrument(
        skip(self, content, config),
        fields(
            extractor.name = self.name(),
            content.size_bytes = content.len(),
        )
    ))]
    async fn extract_bytes(
        &self,
        content: &[u8],
        mime_type: &str,
        config: &ExtractionConfig,
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

                let document = load_pdf_from_byte_slice(&pdfium, &content, &config)?;

                extract_all_from_document(&document, config)?
            }
            #[cfg(all(not(target_arch = "wasm32"), feature = "tokio-runtime"))]
            {
                if crate::core::batch_mode::is_batch_mode() {
                    let content_owned = content.to_vec();
                    let span = tracing::Span::current();
                    let config_owned = config.clone();
                    let result = tokio::task::spawn_blocking(move || {
                        let _guard = span.entered();

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
                        ) = extract_all_from_document(&document, &config_owned)
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

                    extract_all_from_document(&document, config)?
                }
            }
            #[cfg(all(not(target_arch = "wasm32"), not(feature = "tokio-runtime")))]
            {
                let pdfium =
                    crate::pdf::bindings::bind_pdfium(PdfError::MetadataExtractionFailed, "initialize Pdfium")?;

                let document = load_pdf_from_byte_slice(&pdfium, content, config)?;

                extract_all_from_document(&document, config)?
            }
        };

        #[cfg(feature = "ocr")]
        let (text, used_ocr) = if config.force_ocr {
            if config.ocr.is_some() {
                (extract_with_ocr(content, config).await?, true)
            } else {
                (native_text, false)
            }
        } else if config.ocr.is_some() {
            let decision = ocr::evaluate_per_page_ocr(
                &native_text,
                boundaries.as_deref(),
                pdf_metadata.pdf_specific.page_count,
            );

            if std::env::var("KREUZBERG_DEBUG_OCR").is_ok() {
                eprintln!(
                    "[kreuzberg::pdf::ocr] fallback={} font_encoding_issues={} non_whitespace={} alnum={} meaningful_words={} \
                     avg_non_whitespace={:.2} avg_alnum={:.2} alnum_ratio={:.3}",
                    decision.fallback,
                    has_font_encoding_issues,
                    decision.stats.non_whitespace,
                    decision.stats.alnum,
                    decision.stats.meaningful_words,
                    decision.avg_non_whitespace,
                    decision.avg_alnum,
                    decision.stats.alnum_ratio
                );
            }

            if decision.fallback || has_font_encoding_issues {
                (extract_with_ocr(content, config).await?, true)
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
                            let format = img
                                .filters
                                .first()
                                .cloned()
                                .map(std::borrow::Cow::Owned)
                                .unwrap_or(std::borrow::Cow::Borrowed("unknown"));
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
        // Images (including OCR) are now fully resolved, so we can inject placeholders.
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

    #[cfg(feature = "tokio-runtime")]
    async fn extract_file(&self, path: &Path, mime_type: &str, config: &ExtractionConfig) -> Result<ExtractionResult> {
        let bytes = tokio::fs::read(path).await?;
        self.extract_bytes(&bytes, mime_type, config).await
    }

    fn supported_mime_types(&self) -> &[&str] {
        &["application/pdf"]
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
        assert!(ocr::evaluate_native_text_for_ocr("", Some(1)).fallback);
    }

    #[cfg(feature = "ocr")]
    #[test]
    fn test_should_not_fallback_for_meaningful_text() {
        let sample = "This page has searchable vector text and should avoid OCR.";
        assert!(!ocr::evaluate_native_text_for_ocr(sample, Some(1)).fallback);
    }

    #[cfg(feature = "ocr")]
    #[test]
    fn test_should_fallback_for_punctuation_only_text() {
        let sample = " . , ; : -- -- ";
        assert!(ocr::evaluate_native_text_for_ocr(sample, Some(2)).fallback);
    }

    #[cfg(feature = "ocr")]
    #[test]
    fn test_per_page_ocr_no_boundaries_falls_back_to_whole_doc() {
        let text = "This document has enough meaningful words for evaluation purposes here.";
        let decision = ocr::evaluate_per_page_ocr(text, None, Some(1));
        assert!(!decision.fallback);
    }

    #[cfg(feature = "ocr")]
    #[test]
    fn test_per_page_ocr_empty_boundaries_falls_back_to_whole_doc() {
        let text = "This document has enough meaningful words for evaluation purposes here.";
        let decision = ocr::evaluate_per_page_ocr(text, Some(&[]), Some(1));
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

        let decision = ocr::evaluate_per_page_ocr(&text, Some(&boundaries), Some(2));
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

        let decision = ocr::evaluate_per_page_ocr(&text, Some(&boundaries), Some(2));
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

        let decision = ocr::evaluate_per_page_ocr(&text, Some(&boundaries), Some(2));
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

        let decision = ocr::evaluate_per_page_ocr(&text, Some(&boundaries), Some(2));
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

        let decision = ocr::evaluate_per_page_ocr(text, Some(&boundaries), Some(1));
        assert!(!decision.fallback);
    }

    #[cfg(feature = "ocr")]
    #[test]
    fn test_per_page_ocr_multi_page_correct_page_count() {
        let text = "ab cd ef";
        let decision_wrong = ocr::evaluate_native_text_for_ocr(text, None);
        let decision_correct = ocr::evaluate_native_text_for_ocr(text, Some(20));
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

    #[test]
    #[cfg(feature = "pdf")]
    fn test_pdf_extractor_without_feature_pdf() {
        let extractor = PdfExtractor::new();
        assert_eq!(extractor.name(), "pdf-extractor");
    }
}
