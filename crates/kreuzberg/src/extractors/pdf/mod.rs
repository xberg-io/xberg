//! PDF document extractor.
//!
//! Provides extraction of text, metadata, tables, and images from PDF documents
//! using pypdfium2 and playa-pdf. Supports both native text extraction and OCR fallback.

mod extraction;
mod ocr;
mod pages;

use crate::Result;
use crate::core::config::ExtractionConfig;
use crate::plugins::{DocumentExtractor, Plugin};
use crate::types::{ExtractionResult, Metadata};
use async_trait::async_trait;
#[cfg(feature = "tokio-runtime")]
use std::path::Path;

#[cfg(feature = "pdf")]
use crate::pdf::error::PdfError;

// Re-export for backward compatibility
#[cfg(feature = "ocr")]
pub use ocr::{evaluate_native_text_for_ocr, NativeTextStats, OcrFallbackDecision};

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

#[async_trait]
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
        #[cfg(feature = "pdf")]
        let (pdf_metadata, native_text, tables, page_contents) = {
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

                let document = pdfium.load_pdf_from_byte_slice(content, None).map_err(|e| {
                    let err_msg = crate::pdf::error::format_pdfium_error(e);
                    if err_msg.contains("password") || err_msg.contains("Password") {
                        PdfError::PasswordRequired
                    } else {
                        PdfError::InvalidPdf(err_msg)
                    }
                })?;

                extract_all_from_document(&document, config)?
            }
            #[cfg(all(not(target_arch = "wasm32"), feature = "tokio-runtime"))]
            {
                if crate::core::batch_mode::is_batch_mode() {
                    let content_owned = content.to_vec();
                    let span = tracing::Span::current();
                    let config_owned = config.clone();
                    tokio::task::spawn_blocking(move || {
                        let _guard = span.entered();

                        let pdfium =
                            crate::pdf::bindings::bind_pdfium(PdfError::MetadataExtractionFailed, "initialize Pdfium")?;

                        let document = pdfium.load_pdf_from_byte_slice(&content_owned, None).map_err(|e| {
                            let err_msg = crate::pdf::error::format_pdfium_error(e);
                            if err_msg.contains("password") || err_msg.contains("Password") {
                                PdfError::PasswordRequired
                            } else {
                                PdfError::InvalidPdf(err_msg)
                            }
                        })?;

                        let (pdf_metadata, native_text, tables, page_contents) =
                            extract_all_from_document(&document, &config_owned)?;

                        if let Some(page_cfg) = config_owned.pages.as_ref()
                            && page_cfg.extract_pages
                            && page_contents.is_none()
                        {
                            return Err(PdfError::ExtractionFailed(
                                "Page extraction was configured but no page data was extracted in batch mode"
                                    .to_string(),
                            )
                            .into());
                        }

                        Ok::<_, crate::error::KreuzbergError>((pdf_metadata, native_text, tables, page_contents))
                    })
                    .await
                    .map_err(|e| crate::error::KreuzbergError::Other(format!("PDF extraction task failed: {}", e)))??
                } else {
                    let pdfium =
                        crate::pdf::bindings::bind_pdfium(PdfError::MetadataExtractionFailed, "initialize Pdfium")?;

                    let document = pdfium.load_pdf_from_byte_slice(content, None).map_err(|e| {
                        let err_msg = crate::pdf::error::format_pdfium_error(e);
                        if err_msg.contains("password") || err_msg.contains("Password") {
                            PdfError::PasswordRequired
                        } else {
                            PdfError::InvalidPdf(err_msg)
                        }
                    })?;

                    extract_all_from_document(&document, config)?
                }
            }
            #[cfg(all(not(target_arch = "wasm32"), not(feature = "tokio-runtime")))]
            {
                let pdfium =
                    crate::pdf::bindings::bind_pdfium(PdfError::MetadataExtractionFailed, "initialize Pdfium")?;

                let document = pdfium.load_pdf_from_byte_slice(content, None).map_err(|e| {
                    let err_msg = crate::pdf::error::format_pdfium_error(e);
                    if err_msg.contains("password") || err_msg.contains("Password") {
                        PdfError::PasswordRequired
                    } else {
                        PdfError::InvalidPdf(err_msg)
                    }
                })?;

                extract_all_from_document(&document, config)?
            }
        };

        #[cfg(feature = "ocr")]
        let text = if config.force_ocr {
            if config.ocr.is_some() {
                extract_with_ocr(content, config).await?
            } else {
                native_text
            }
        } else if config.ocr.is_some() {
            let decision = ocr::evaluate_native_text_for_ocr(&native_text, None);

            if std::env::var("KREUZBERG_DEBUG_OCR").is_ok() {
                eprintln!(
                    "[kreuzberg::pdf::ocr] fallback={} non_whitespace={} alnum={} meaningful_words={} \
                     avg_non_whitespace={:.2} avg_alnum={:.2} alnum_ratio={:.3}",
                    decision.fallback,
                    decision.stats.non_whitespace,
                    decision.stats.alnum,
                    decision.stats.meaningful_words,
                    decision.avg_non_whitespace,
                    decision.avg_alnum,
                    decision.stats.alnum_ratio
                );
            }

            if decision.fallback {
                extract_with_ocr(content, config).await?
            } else {
                native_text
            }
        } else {
            native_text
        };

        #[cfg(not(feature = "ocr"))]
        let text = native_text;

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
                            let format = img.filters.first().cloned().unwrap_or_else(|| "unknown".to_string());
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

        let final_pages = assign_tables_and_images_to_pages(page_contents, &tables, images.as_deref().unwrap_or(&[]));

        Ok(ExtractionResult {
            content: text,
            mime_type: mime_type.to_string(),
            metadata: Metadata {
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
            std::path::Path::new(env!("CARGO_MANIFEST_DIR")).join("../../test_documents/pdfs/google_doc_document.pdf");
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
            std::path::Path::new(env!("CARGO_MANIFEST_DIR")).join("../../test_documents/pdfs/google_doc_document.pdf");
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

        let pdf_path =
            std::path::Path::new(env!("CARGO_MANIFEST_DIR")).join("../../test_documents/pdfs/multi_page.pdf");
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
