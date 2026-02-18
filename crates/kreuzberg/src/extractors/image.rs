//! Image extractors for various image formats.

use crate::Result;
use crate::core::config::ExtractionConfig;
use crate::extraction::image::extract_image_metadata;
use crate::plugins::{DocumentExtractor, Plugin};
use crate::types::{ExtractionResult, Metadata};
use async_trait::async_trait;

/// Image extractor for various image formats.
///
/// Supports: PNG, JPEG, WebP, BMP, TIFF, GIF.
/// Extracts dimensions, format, and EXIF metadata.
/// Optionally runs OCR when configured.
pub struct ImageExtractor;

impl ImageExtractor {
    /// Create a new image extractor.
    pub fn new() -> Self {
        Self
    }

    /// Extract text from image using OCR with optional page tracking for multi-frame TIFFs.
    #[cfg(any(feature = "ocr", feature = "ocr-wasm"))]
    async fn extract_with_ocr(
        &self,
        content: &[u8],
        mime_type: &str,
        config: &ExtractionConfig,
    ) -> Result<ExtractionResult> {
        use crate::plugins::registry::get_ocr_backend_registry;

        let ocr_config = config.ocr.as_ref().ok_or_else(|| crate::KreuzbergError::Parsing {
            message: "OCR config required for image OCR".to_string(),
            source: None,
        })?;

        let backend = {
            let registry = get_ocr_backend_registry();
            let registry = registry.read().map_err(|e| crate::KreuzbergError::Plugin {
                message: format!("Failed to acquire read lock on OCR backend registry: {}", e),
                plugin_name: "ocr-registry".to_string(),
            })?;
            let backend_name = crate::core::config::canonical_ocr_backend_name(&ocr_config.backend);
            registry.get(&backend_name)?
        };

        // Thread output_format from ExtractionConfig to OcrConfig
        let mut ocr_config_with_format = ocr_config.clone();
        ocr_config_with_format.output_format = Some(config.output_format);

        let ocr_result = backend.process_image(content, &ocr_config_with_format).await?;

        // Full OCR with TIFF multi-frame support (requires tiff crate)
        #[cfg(feature = "ocr")]
        {
            let ocr_text = ocr_result.content.clone();
            let ocr_extraction_result = crate::extraction::image::extract_text_from_image_with_ocr(
                content,
                mime_type,
                ocr_text,
                config.pages.as_ref(),
            )?;

            let mut result = ocr_result;
            result.content = ocr_extraction_result.content;
            result.pages = ocr_extraction_result.page_contents;

            Ok(result)
        }

        // Simplified OCR path for WASM (no TIFF multi-frame support)
        #[cfg(not(feature = "ocr"))]
        {
            let _ = mime_type;
            Ok(ocr_result)
        }
    }
}

impl Default for ImageExtractor {
    fn default() -> Self {
        Self::new()
    }
}

impl Plugin for ImageExtractor {
    fn name(&self) -> &str {
        "image-extractor"
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

    fn description(&self) -> &str {
        "Extracts dimensions, format, and EXIF data from images (PNG, JPEG, WebP, BMP, TIFF, GIF)"
    }

    fn author(&self) -> &str {
        "Kreuzberg Team"
    }
}

#[cfg_attr(not(target_arch = "wasm32"), async_trait)]
#[cfg_attr(target_arch = "wasm32", async_trait(?Send))]
impl DocumentExtractor for ImageExtractor {
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
        let extraction_metadata = extract_image_metadata(content)?;

        let image_metadata = crate::types::ImageMetadata {
            width: extraction_metadata.width,
            height: extraction_metadata.height,
            format: extraction_metadata.format.clone(),
            exif: extraction_metadata.exif_data,
        };

        if config.ocr.is_some() {
            #[cfg(any(feature = "ocr", feature = "ocr-wasm"))]
            {
                let mut ocr_result = self.extract_with_ocr(content, mime_type, config).await?;

                ocr_result.metadata.format = Some(crate::types::FormatMetadata::Image(image_metadata));
                ocr_result.mime_type = mime_type.to_string().into();

                return Ok(ocr_result);
            }
            #[cfg(not(any(feature = "ocr", feature = "ocr-wasm")))]
            {
                let content_text = format!(
                    "Image: {} {}x{}",
                    extraction_metadata.format, extraction_metadata.width, extraction_metadata.height
                );

                return Ok(ExtractionResult {
                    content: content_text,
                    mime_type: mime_type.to_string().into(),
                    metadata: Metadata {
                        format: Some(crate::types::FormatMetadata::Image(image_metadata)),
                        ..Default::default()
                    },
                    pages: None,
                    tables: vec![],
                    detected_languages: None,
                    chunks: None,
                    images: None,
                    djot_content: None,
                    elements: None,
                    ocr_elements: None,
                    document: None,
                    #[cfg(any(feature = "keywords-yake", feature = "keywords-rake"))]
                    extracted_keywords: None,
                    quality_score: None,
                    processing_warnings: Vec::new(),
                    annotations: None,
                });
            }
        }

        Ok(ExtractionResult {
            content: format!(
                "Image: {} {}x{}",
                extraction_metadata.format, extraction_metadata.width, extraction_metadata.height
            ),
            mime_type: mime_type.to_string().into(),
            metadata: Metadata {
                format: Some(crate::types::FormatMetadata::Image(image_metadata)),
                ..Default::default()
            },
            pages: None,
            tables: vec![],
            detected_languages: None,
            chunks: None,
            images: None,
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
            "image/png",
            "image/jpeg",
            "image/jpg",
            "image/pjpeg",
            "image/webp",
            "image/bmp",
            "image/x-bmp",
            "image/x-ms-bmp",
            "image/tiff",
            "image/x-tiff",
            "image/gif",
            "image/jp2",
            "image/jpx",
            "image/jpm",
            "image/mj2",
            "image/x-jbig2",
            "image/x-portable-anymap",
            "image/x-portable-bitmap",
            "image/x-portable-graymap",
            "image/x-portable-pixmap",
        ]
    }

    fn priority(&self) -> i32 {
        50
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[tokio::test]
    async fn test_image_extractor_invalid_image() {
        let extractor = ImageExtractor::new();
        let invalid_bytes = vec![0, 1, 2, 3, 4, 5];
        let config = ExtractionConfig::default();

        let result = extractor.extract_bytes(&invalid_bytes, "image/png", &config).await;
        assert!(result.is_err());
    }

    #[test]
    fn test_image_plugin_interface() {
        let extractor = ImageExtractor::new();
        assert_eq!(extractor.name(), "image-extractor");
        assert_eq!(extractor.version(), env!("CARGO_PKG_VERSION"));
        assert!(extractor.supported_mime_types().contains(&"image/png"));
        assert!(extractor.supported_mime_types().contains(&"image/jpeg"));
        assert!(extractor.supported_mime_types().contains(&"image/webp"));
        assert_eq!(extractor.priority(), 50);
    }

    #[test]
    fn test_image_extractor_default() {
        let extractor = ImageExtractor;
        assert_eq!(extractor.name(), "image-extractor");
    }

    #[test]
    fn test_image_extractor_supports_alias_mime_types() {
        let extractor = ImageExtractor::new();
        let supported = extractor.supported_mime_types();
        assert!(supported.contains(&"image/pjpeg"));
        assert!(supported.contains(&"image/x-bmp"));
        assert!(supported.contains(&"image/x-ms-bmp"));
        assert!(supported.contains(&"image/x-tiff"));
        assert!(supported.contains(&"image/x-portable-anymap"));
    }
}
