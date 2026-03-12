//! PowerPoint presentation extractor.

use crate::Result;
use crate::core::config::ExtractionConfig;
use crate::plugins::{DocumentExtractor, Plugin};
use crate::types::{ExtractionResult, Metadata};
use ahash::AHashMap;
use async_trait::async_trait;
use std::borrow::Cow;
use std::path::Path;

/// PowerPoint presentation extractor.
///
/// Supports: .pptx, .pptm, .ppsx
pub struct PptxExtractor;

impl Default for PptxExtractor {
    fn default() -> Self {
        Self::new()
    }
}

impl PptxExtractor {
    pub fn new() -> Self {
        Self
    }
}

impl Plugin for PptxExtractor {
    fn name(&self) -> &str {
        "pptx-extractor"
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
impl DocumentExtractor for PptxExtractor {
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
        let extract_images = config.images.as_ref().is_some_and(|img| img.extract_images);
        let plain = matches!(
            config.output_format,
            crate::core::config::OutputFormat::Plain | crate::core::config::OutputFormat::Structured
        );

        let pptx_result = {
            #[cfg(feature = "tokio-runtime")]
            {
                let pages_config = config.pages.clone();
                if crate::core::batch_mode::is_batch_mode() {
                    let content_owned = content.to_vec();
                    let span = tracing::Span::current();
                    tokio::task::spawn_blocking(move || {
                        let _guard = span.entered();
                        crate::extraction::pptx::extract_pptx_from_bytes(
                            &content_owned,
                            extract_images,
                            pages_config.as_ref(),
                            plain,
                        )
                    })
                    .await
                    .map_err(|e| {
                        crate::error::KreuzbergError::parsing(format!("PPTX extraction task failed: {}", e))
                    })??
                } else {
                    crate::extraction::pptx::extract_pptx_from_bytes(
                        content,
                        extract_images,
                        config.pages.as_ref(),
                        plain,
                    )?
                }
            }

            #[cfg(not(feature = "tokio-runtime"))]
            {
                crate::extraction::pptx::extract_pptx_from_bytes(content, extract_images, config.pages.as_ref(), plain)?
            }
        };

        let mut additional: AHashMap<Cow<'static, str>, serde_json::Value> = AHashMap::new();
        additional.insert(Cow::Borrowed("slide_count"), serde_json::json!(pptx_result.slide_count));
        additional.insert(Cow::Borrowed("image_count"), serde_json::json!(pptx_result.image_count));
        additional.insert(Cow::Borrowed("table_count"), serde_json::json!(pptx_result.table_count));

        let images = if extract_images {
            // Image extraction is enabled, return images or empty vector
            if !pptx_result.images.is_empty() {
                #[cfg(all(feature = "ocr", feature = "tokio-runtime"))]
                {
                    let processed_images =
                        crate::extraction::image_ocr::process_images_with_ocr(pptx_result.images, config).await?;
                    Some(processed_images)
                }
                #[cfg(not(all(feature = "ocr", feature = "tokio-runtime")))]
                {
                    Some(pptx_result.images)
                }
            } else {
                Some(vec![])
            }
        } else {
            // Image extraction is disabled
            None
        };

        let mut metadata = Metadata {
            format: Some(crate::types::FormatMetadata::Pptx(pptx_result.metadata)),
            additional,
            ..Default::default()
        };

        if let Some(page_structure) = pptx_result.page_structure {
            metadata.pages = Some(page_structure);
        }

        Ok(ExtractionResult {
            content: pptx_result.content,
            mime_type: mime_type.to_string().into(),
            metadata,
            pages: pptx_result.page_contents,
            tables: vec![],
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
            annotations: None,
        })
    }

    #[cfg_attr(feature = "otel", tracing::instrument(
        skip(self, path, config),
        fields(
            extractor.name = self.name(),
        )
    ))]
    async fn extract_file(&self, path: &Path, mime_type: &str, config: &ExtractionConfig) -> Result<ExtractionResult> {
        let path_str = path
            .to_str()
            .ok_or_else(|| crate::KreuzbergError::validation("Invalid file path".to_string()))?;

        let extract_images = config.images.as_ref().is_some_and(|img| img.extract_images);

        let plain = matches!(
            config.output_format,
            crate::core::config::OutputFormat::Plain | crate::core::config::OutputFormat::Structured
        );
        let pptx_result =
            crate::extraction::pptx::extract_pptx_from_path(path_str, extract_images, config.pages.as_ref(), plain)?;

        let mut additional: AHashMap<Cow<'static, str>, serde_json::Value> = AHashMap::new();
        additional.insert(Cow::Borrowed("slide_count"), serde_json::json!(pptx_result.slide_count));
        additional.insert(Cow::Borrowed("image_count"), serde_json::json!(pptx_result.image_count));
        additional.insert(Cow::Borrowed("table_count"), serde_json::json!(pptx_result.table_count));

        let images = if extract_images {
            // Image extraction is enabled, return images or empty vector
            if !pptx_result.images.is_empty() {
                #[cfg(all(feature = "ocr", feature = "tokio-runtime"))]
                {
                    let processed_images =
                        crate::extraction::image_ocr::process_images_with_ocr(pptx_result.images, config).await?;
                    Some(processed_images)
                }
                #[cfg(not(all(feature = "ocr", feature = "tokio-runtime")))]
                {
                    Some(pptx_result.images)
                }
            } else {
                Some(vec![])
            }
        } else {
            // Image extraction is disabled
            None
        };

        let mut metadata = Metadata {
            format: Some(crate::types::FormatMetadata::Pptx(pptx_result.metadata)),
            additional,
            ..Default::default()
        };

        if let Some(page_structure) = pptx_result.page_structure {
            metadata.pages = Some(page_structure);
        }

        Ok(ExtractionResult {
            content: pptx_result.content,
            mime_type: mime_type.to_string().into(),
            metadata,
            pages: pptx_result.page_contents,
            tables: vec![],
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
            annotations: None,
        })
    }

    fn supported_mime_types(&self) -> &[&str] {
        &[
            "application/vnd.openxmlformats-officedocument.presentationml.presentation",
            "application/vnd.ms-powerpoint.presentation.macroEnabled.12",
            "application/vnd.openxmlformats-officedocument.presentationml.slideshow",
        ]
    }

    fn priority(&self) -> i32 {
        50
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_pptx_extractor_plugin_interface() {
        let extractor = PptxExtractor::new();
        assert_eq!(extractor.name(), "pptx-extractor");
        assert!(extractor.initialize().is_ok());
        assert!(extractor.shutdown().is_ok());
    }

    #[test]
    fn test_pptx_extractor_supported_mime_types() {
        let extractor = PptxExtractor::new();
        let mime_types = extractor.supported_mime_types();
        assert_eq!(mime_types.len(), 3);
        assert!(mime_types.contains(&"application/vnd.openxmlformats-officedocument.presentationml.presentation"));
    }
}
