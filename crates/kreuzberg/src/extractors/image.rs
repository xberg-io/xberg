//! Image extractors for various image formats.

use crate::Result;
use crate::core::config::ExtractionConfig;
use crate::extraction::image::extract_image_metadata;
use crate::plugins::{DocumentExtractor, Plugin};
use crate::types::internal::InternalDocument;
use crate::types::internal_builder::InternalDocumentBuilder;
use crate::types::metadata::Metadata;
use async_trait::async_trait;

/// Image extractor for various image formats.
///
/// Supports: PNG, JPEG, WebP, BMP, TIFF, GIF.
/// Extracts dimensions, format, and EXIF metadata.
/// Optionally runs OCR when configured.
/// When layout detection is also enabled, uses per-region OCR with
/// markdown formatting based on detected layout classes.
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
    ) -> Result<InternalDocument> {
        use crate::plugins::registry::get_ocr_backend_registry;

        let default_ocr_config;
        let ocr_config = match config.ocr.as_ref() {
            Some(c) => c,
            None => {
                default_ocr_config = crate::core::config::OcrConfig::default();
                &default_ocr_config
            }
        };

        let backend = {
            let registry = get_ocr_backend_registry();
            let registry = registry.read();
            registry.get(&ocr_config.backend)?
        };

        // Thread output_format from ExtractionConfig to OcrConfig
        let mut ocr_config_with_format = ocr_config.clone();
        ocr_config_with_format.output_format = Some(config.output_format.clone());

        // Always request OCR elements so that build_pages can populate pages[].
        // Backends that gate element output behind include_elements (e.g. paddle-ocr)
        // would otherwise return None, leaving pages[] empty while content is correct.
        // This mirrors the ensure_elements_enabled pattern used by the PDF extractor.
        match ocr_config_with_format.element_config.as_mut() {
            Some(ec) => ec.include_elements = true,
            None => {
                ocr_config_with_format.element_config = Some(crate::types::OcrElementConfig {
                    include_elements: true,
                    ..Default::default()
                });
            }
        }

        let ocr_result = backend.process_image(content, &ocr_config_with_format).await?;

        // Destructure to avoid partial-move issues when propagating OCR elements.
        let ocr_content = ocr_result.content;
        let ocr_metadata = ocr_result.metadata;
        let ocr_elements = ocr_result.ocr_elements;

        // Full OCR with TIFF multi-frame support (requires tiff crate)
        #[cfg(feature = "ocr")]
        {
            let ocr_extraction_result = crate::extraction::image::extract_text_from_image_with_ocr(
                content,
                mime_type,
                ocr_content,
                config.pages.as_ref(),
            )?;

            // Build InternalDocument from OCR text
            let mut doc = build_image_internal_document(Some(&ocr_extraction_result.content), None);
            doc.metadata = ocr_metadata;

            // Store OCR elements directly to avoid injecting raw word tokens into the
            // rendering pipeline, which would double the top-level content (#706).
            doc.prebuilt_ocr_elements = ocr_elements;

            // Use the coherent HOCR string for pages[*].content. Multi-frame TIFFs
            // already have per-frame page_contents; single images get a page-1 wrapper.
            if let Some(pages) = ocr_extraction_result.page_contents {
                doc.prebuilt_pages = Some(pages);
            } else {
                let text = ocr_extraction_result.content.trim().to_string();
                if !text.is_empty() {
                    doc.prebuilt_pages = Some(vec![crate::types::PageContent {
                        page_number: 1,
                        content: text,
                        tables: vec![],
                        images: vec![],
                        hierarchy: None,
                        is_blank: None,
                        layout_regions: None,
                    }]);
                }
            }

            Ok(doc)
        }

        // Simplified OCR path for WASM (no TIFF multi-frame support)
        #[cfg(not(feature = "ocr"))]
        {
            let _ = mime_type;
            let mut doc = build_image_internal_document(Some(&ocr_content), None);
            doc.metadata = ocr_metadata;
            doc.prebuilt_ocr_elements = ocr_elements;
            let text = ocr_content.trim().to_string();
            if !text.is_empty() {
                doc.prebuilt_pages = Some(vec![crate::types::PageContent {
                    page_number: 1,
                    content: text,
                    tables: vec![],
                    images: vec![],
                    hierarchy: None,
                    is_blank: None,
                    layout_regions: None,
                }]);
            }
            Ok(doc)
        }
    }

    /// Extract text from image using layout detection + per-region OCR.
    ///
    /// Runs layout detection to identify document regions (headings, text,
    /// code, formulas, etc.), then OCRs each region individually and
    /// assembles the results into structured markdown.
    #[cfg(all(feature = "layout-detection", any(feature = "ocr", feature = "ocr-wasm")))]
    async fn extract_with_layout_ocr(&self, content: &[u8], config: &ExtractionConfig) -> Result<InternalDocument> {
        use crate::layout::LayoutClass;
        use crate::plugins::registry::get_ocr_backend_registry;
        use crate::types::internal::{ElementKind, InternalElement};
        use image::ImageEncoder;
        use std::io::Cursor;

        let layout_config = config.layout.as_ref().ok_or_else(|| crate::KreuzbergError::Parsing {
            message: "Layout config required for layout-enhanced OCR".to_string(),
            source: None,
        })?;

        let ocr_config = config.ocr.as_ref().ok_or_else(|| crate::KreuzbergError::Parsing {
            message: "OCR config required for layout-enhanced OCR".to_string(),
            source: None,
        })?;

        // 1. Decode image
        let img = image::load_from_memory(content).map_err(|e| crate::KreuzbergError::Parsing {
            message: format!("Failed to decode image for layout detection: {e}"),
            source: None,
        })?;
        let rgb = img.to_rgb8();

        // 2. Run layout detection (reuse cached engine when available)
        let mut engine = crate::layout::take_or_create_engine(layout_config)
            .map_err(|e| crate::KreuzbergError::Other(format!("Layout engine init failed: {e}")))?;

        let detection = engine
            .detect(&rgb)
            .map_err(|e| crate::KreuzbergError::Other(format!("Layout detection failed: {e}")))?;

        // Return engine to cache immediately — we're done with inference
        crate::layout::return_engine(engine);

        tracing::info!(
            detections = detection.detections.len(),
            img_width = rgb.width(),
            img_height = rgb.height(),
            "Layout detection completed for image"
        );

        if detection.detections.is_empty() {
            tracing::debug!("No layout regions detected, falling back to whole-image OCR");
            return self.extract_with_ocr(content, "image/png", config).await;
        }

        // 3. Sort detections by reading order (top-to-bottom, left-to-right)
        let mut detections = detection.detections;
        // Quantize y-centers into discrete rows to ensure transitive ordering.
        let row_threshold = (rgb.height() as f32 * 0.05).max(1.0);
        detections.sort_by(|a, b| {
            let ay = (a.bbox.y1 + a.bbox.y2) / 2.0;
            let by = (b.bbox.y1 + b.bbox.y2) / 2.0;
            let a_row = (ay / row_threshold) as i64;
            let b_row = (by / row_threshold) as i64;
            a_row.cmp(&b_row).then_with(|| {
                let ax = (a.bbox.x1 + a.bbox.x2) / 2.0;
                let bx = (b.bbox.x1 + b.bbox.x2) / 2.0;
                ax.total_cmp(&bx)
            })
        });

        // 4. Get OCR backend
        let backend = {
            let registry = get_ocr_backend_registry();
            let registry = registry.read();
            registry.get(&ocr_config.backend)?
        };

        // Use plain text for per-region OCR (we build markdown structure ourselves)
        let mut region_ocr_config = ocr_config.clone();
        region_ocr_config.output_format = Some(crate::core::config::OutputFormat::Plain);

        // 5. Per-region OCR + formatting into InternalDocument
        let mut builder = InternalDocumentBuilder::new("image");
        let img_width = rgb.width();
        let img_height = rgb.height();

        for det in &detections {
            // Skip picture regions (OCR on an embedded image is not useful)
            if det.class == LayoutClass::Picture {
                continue;
            }

            // Crop region (clamp to image bounds)
            let x1 = (det.bbox.x1.max(0.0) as u32).min(img_width.saturating_sub(1));
            let y1 = (det.bbox.y1.max(0.0) as u32).min(img_height.saturating_sub(1));
            let x2 = (det.bbox.x2.max(0.0).ceil() as u32).min(img_width);
            let y2 = (det.bbox.y2.max(0.0).ceil() as u32).min(img_height);

            let crop_w = x2.saturating_sub(x1);
            let crop_h = y2.saturating_sub(y1);
            if crop_w < 4 || crop_h < 4 {
                continue; // Too small to OCR meaningfully
            }

            let crop = image::imageops::crop_imm(&rgb, x1, y1, crop_w, crop_h).to_image();

            // Encode crop as PNG for OCR backend
            let mut png_buf = Cursor::new(Vec::new());
            image::codecs::png::PngEncoder::new(&mut png_buf)
                .write_image(
                    crop.as_raw(),
                    crop.width(),
                    crop.height(),
                    image::ExtendedColorType::Rgb8,
                )
                .map_err(|e| crate::KreuzbergError::Other(format!("Failed to encode crop as PNG: {e}")))?;
            let crop_bytes = png_buf.into_inner();

            // OCR the cropped region
            let ocr_result = backend.process_image(&crop_bytes, &region_ocr_config).await?;
            let text = ocr_result.content.trim().to_string();
            if text.is_empty() {
                continue;
            }

            tracing::trace!(
                class = ?det.class,
                confidence = det.confidence,
                text_len = text.len(),
                "OCR result for layout region"
            );

            // Map layout class to InternalElement
            match det.class {
                LayoutClass::Title => {
                    builder.push_heading(1, &text, None, None);
                }
                LayoutClass::SectionHeader => {
                    builder.push_heading(2, &text, None, None);
                }
                LayoutClass::Code => {
                    builder.push_code(&text, None, None, None);
                }
                LayoutClass::Formula => {
                    let elem = InternalElement::text(ElementKind::Formula, &text, 0);
                    builder.push_element(elem);
                }
                LayoutClass::ListItem | LayoutClass::CheckboxSelected | LayoutClass::CheckboxUnselected => {
                    builder.push_list_item(&text, false, vec![], None, None);
                }
                LayoutClass::Caption | LayoutClass::Footnote => {
                    builder.push_paragraph(&text, vec![], None, None);
                }
                LayoutClass::Table => {
                    builder.push_paragraph(&text, vec![], None, None);
                }
                LayoutClass::PageHeader | LayoutClass::PageFooter => continue,
                _ => {
                    builder.push_paragraph(&text, vec![], None, None);
                }
            };
        }

        let mut doc = builder.build();
        doc.metadata = Metadata {
            output_format: Some("markdown".to_string()),
            ..Default::default()
        };

        Ok(doc)
    }
}

/// Build a simple `InternalDocument` for an image extraction result.
///
/// If OCR text is available, pushes it as a paragraph. Always pushes
/// the image itself as an `Image` node. When `image_data` is provided,
/// the binary data is stored in `InternalDocument::images` and the
/// element references it by index.
fn build_image_internal_document(
    ocr_text: Option<&str>,
    image_data: Option<crate::types::ExtractedImage>,
) -> InternalDocument {
    let mut builder = InternalDocumentBuilder::new("image");
    if let Some(text) = ocr_text
        && !text.trim().is_empty()
    {
        builder.push_paragraph(text.trim(), vec![], None, None);
    }
    // Push image element — if we have actual image data, use push_image so
    // it is stored in InternalDocument::images and referenced by index.
    if let Some(img) = image_data {
        builder.push_image(None, img, None, None);
    } else {
        use crate::types::document_structure::ContentLayer;
        use crate::types::internal::{ElementKind, InternalElement, InternalElementId};

        let kind = ElementKind::Image { image_index: 0 };
        let id = InternalElementId::generate(kind.discriminant(), "", None, 0);
        builder.push_element(InternalElement {
            id,
            kind,
            text: String::new(),
            depth: 0,
            page: None,
            bbox: None,
            layer: ContentLayer::Body,
            annotations: Vec::new(),
            attributes: None,
            anchor: None,
            ocr_geometry: None,
            ocr_confidence: None,
            ocr_rotation: None,
        });
    }
    builder.build()
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
    async fn extract_bytes(
        &self,
        content: &[u8],
        mime_type: &str,
        config: &ExtractionConfig,
    ) -> Result<InternalDocument> {
        tracing::debug!(format = "image", size_bytes = content.len(), "extraction starting");
        let extraction_metadata = extract_image_metadata(content)?;

        let format_str = extraction_metadata.format;
        let image_metadata = crate::types::ImageMetadata {
            width: extraction_metadata.width,
            height: extraction_metadata.height,
            format: format_str.clone(),
            exif: extraction_metadata.exif_data,
        };

        // Build an ExtractedImage from the raw content so it is stored in doc.images
        let extracted_image = crate::types::ExtractedImage {
            data: bytes::Bytes::copy_from_slice(content),
            format: std::borrow::Cow::Owned(format_str),
            image_index: 0,
            page_number: None,
            width: Some(extraction_metadata.width),
            height: Some(extraction_metadata.height),
            colorspace: None,
            bits_per_component: None,
            is_mask: false,
            description: None,
            ocr_result: None,
            bounding_box: None,
            source_path: None,
        };

        // When disable_ocr is set (or ocr.enabled = false), skip OCR and return metadata only
        if config.effective_disable_ocr() {
            let mut doc = build_image_internal_document(None, Some(extracted_image));
            doc.metadata = Metadata {
                format: Some(crate::types::FormatMetadata::Image(image_metadata)),
                ..Default::default()
            };
            doc.mime_type = std::borrow::Cow::Owned(mime_type.to_string());
            tracing::debug!(
                format = "image",
                "OCR disabled via disable_ocr, returning metadata only"
            );
            return Ok(doc);
        }

        // Images are OCR'd by default when an OCR backend is available.
        // OCR is skipped only when the feature is not compiled in.
        {
            // Layout-enhanced OCR: when both OCR and layout detection are configured,
            // run layout detection first, then OCR each detected region individually
            // and assemble into structured markdown.
            #[cfg(all(feature = "layout-detection", any(feature = "ocr", feature = "ocr-wasm")))]
            if config.layout.is_some() {
                match self.extract_with_layout_ocr(content, config).await {
                    Ok(mut doc) => {
                        doc.metadata.format = Some(crate::types::FormatMetadata::Image(image_metadata));
                        doc.mime_type = std::borrow::Cow::Owned(mime_type.to_string());
                        return Ok(doc);
                    }
                    Err(e) => {
                        tracing::warn!("Layout-enhanced OCR failed, falling back to regular OCR: {e}");
                        // Fall through to regular OCR below
                    }
                }
            }

            #[cfg(any(feature = "ocr", feature = "ocr-wasm"))]
            {
                let mut doc = self.extract_with_ocr(content, mime_type, config).await?;
                doc.metadata.format = Some(crate::types::FormatMetadata::Image(image_metadata));
                doc.mime_type = std::borrow::Cow::Owned(mime_type.to_string());
                return Ok(doc);
            }
        }

        #[cfg(not(any(feature = "ocr", feature = "ocr-wasm")))]
        {
            let mut doc = build_image_internal_document(None, Some(extracted_image));
            doc.metadata = Metadata {
                format: Some(crate::types::FormatMetadata::Image(image_metadata)),
                ..Default::default()
            };
            doc.mime_type = std::borrow::Cow::Owned(mime_type.to_string());

            tracing::debug!(
                element_count = doc.elements.len(),
                format = "image",
                "extraction complete"
            );
            Ok(doc)
        }
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

    /// Regression test for #705: a backend that gates ocr_elements on include_elements
    /// (e.g. paddle-ocr) must still produce non-empty pages[].
    ///
    /// extract_with_ocr forces include_elements=true before calling the backend so that
    /// elements are available; prebuilt_pages is then set from the HOCR content string,
    /// ensuring pages[] is populated regardless of the original config.
    #[cfg(feature = "ocr")]
    #[tokio::test]
    async fn test_extract_with_ocr_populates_pages_for_elements_gated_backend() {
        use crate::core::config::OcrConfig;
        use crate::plugins::{OcrBackend, OcrBackendType, Plugin, register_ocr_backend, unregister_ocr_backend};
        use crate::types::{ExtractionResult, OcrBoundingGeometry, OcrConfidence, OcrElement, OcrElementLevel};

        // 1×1 white PNG generated via the `image` crate so CRCs are valid.
        let mut png_buf = std::io::Cursor::new(Vec::new());
        image::ImageBuffer::<image::Rgb<u8>, _>::from_pixel(1, 1, image::Rgb([255u8, 255, 255]))
            .write_to(&mut png_buf, image::ImageFormat::Png)
            .expect("failed to encode test PNG");
        let png_1x1 = png_buf.into_inner();

        /// A mock backend that behaves like paddle-ocr: returns ocr_elements only when
        /// include_elements is true. This is the exact contract that caused issue #705.
        struct GatedElementsBackend;

        #[async_trait::async_trait]
        impl OcrBackend for GatedElementsBackend {
            fn backend_type(&self) -> OcrBackendType {
                OcrBackendType::Custom
            }
            fn supports_language(&self, _: &str) -> bool {
                true
            }
            async fn process_image(&self, _: &[u8], config: &OcrConfig) -> crate::Result<ExtractionResult> {
                let include_elements = config.element_config.as_ref().is_some_and(|ec| ec.include_elements);

                let elements = if include_elements {
                    let geo = OcrBoundingGeometry::Rectangle {
                        left: 0,
                        top: 0,
                        width: 100,
                        height: 20,
                    };
                    let elem = OcrElement::new("hello world".to_string(), geo, OcrConfidence::from_tesseract(99.0))
                        .with_level(OcrElementLevel::Line)
                        .with_page_number(1);
                    Some(vec![elem])
                } else {
                    None
                };

                Ok(ExtractionResult {
                    content: "hello world".to_string(),
                    ocr_elements: elements,
                    ..Default::default()
                })
            }
        }

        impl Plugin for GatedElementsBackend {
            fn name(&self) -> &str {
                "gated-elements-test"
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

        register_ocr_backend(std::sync::Arc::new(GatedElementsBackend)).unwrap();

        let config = ExtractionConfig {
            ocr: Some(OcrConfig {
                backend: "gated-elements-test".to_string(),
                ..Default::default()
            }),
            ..Default::default()
        };

        let extractor = ImageExtractor::new();
        let internal_doc = extractor.extract_bytes(&png_1x1, "image/png", &config).await.unwrap();

        // Run the full derivation pipeline. pages[] is now populated via prebuilt_pages
        // (set from the HOCR content string), not from element page numbers.
        let result = crate::extraction::derive::derive_extraction_result(
            internal_doc,
            false,
            crate::core::config::OutputFormat::Plain,
        );

        let pages = result
            .pages
            .as_ref()
            .expect("pages must be populated (regression of #705)");
        assert!(!pages.is_empty(), "pages[] must not be empty (regression of #705)");
        assert_eq!(pages[0].content.trim(), "hello world");

        unregister_ocr_backend("gated-elements-test").unwrap();
    }

    /// Regression test for #706: pages[0].content must be the coherent HOCR-rendered
    /// text, not a word-by-word dump assembled from raw OcrText elements.
    #[cfg(feature = "ocr")]
    #[tokio::test]
    async fn test_extract_with_ocr_page_content_matches_top_level_content() {
        use crate::core::config::OcrConfig;
        use crate::plugins::{OcrBackend, OcrBackendType, Plugin, register_ocr_backend, unregister_ocr_backend};
        use crate::types::{ExtractionResult, OcrBoundingGeometry, OcrConfidence, OcrElement, OcrElementLevel};

        let mut png_buf = std::io::Cursor::new(Vec::new());
        image::ImageBuffer::<image::Rgb<u8>, _>::from_pixel(1, 1, image::Rgb([255u8, 255, 255]))
            .write_to(&mut png_buf, image::ImageFormat::Png)
            .expect("failed to encode test PNG");
        let png_1x1 = png_buf.into_inner();

        const COHERENT: &str = "Sales Report 2024\n\nThis report contains quarterly sales data.";

        struct TesseractLikeBackend;

        #[async_trait::async_trait]
        impl OcrBackend for TesseractLikeBackend {
            fn backend_type(&self) -> OcrBackendType {
                OcrBackendType::Custom
            }
            fn supports_language(&self, _: &str) -> bool {
                true
            }
            async fn process_image(&self, _: &[u8], _: &OcrConfig) -> crate::Result<ExtractionResult> {
                let content = COHERENT.to_string();
                let words = [
                    "Sales",
                    "Report",
                    "2024",
                    "This",
                    "report",
                    "contains",
                    "quarterly",
                    "sales",
                    "data.",
                ];
                let mut elements = Vec::new();
                for (i, word) in words.iter().enumerate() {
                    let geo = OcrBoundingGeometry::Rectangle {
                        left: i as u32 * 60,
                        top: 0,
                        width: 50,
                        height: 20,
                    };
                    let elem = OcrElement::new(word.to_string(), geo, OcrConfidence::from_tesseract(99.0))
                        .with_level(OcrElementLevel::Word)
                        .with_page_number(1);
                    elements.push(elem);
                }
                Ok(ExtractionResult {
                    content,
                    ocr_elements: Some(elements),
                    ..Default::default()
                })
            }
        }

        impl Plugin for TesseractLikeBackend {
            fn name(&self) -> &str {
                "tesseract-like-706"
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

        register_ocr_backend(std::sync::Arc::new(TesseractLikeBackend)).unwrap();

        let config = ExtractionConfig {
            ocr: Some(OcrConfig {
                backend: "tesseract-like-706".to_string(),
                ..Default::default()
            }),
            ..Default::default()
        };

        let extractor = ImageExtractor::new();
        let internal_doc = extractor.extract_bytes(&png_1x1, "image/png", &config).await.unwrap();

        let result = crate::extraction::derive::derive_extraction_result(
            internal_doc,
            false,
            crate::core::config::OutputFormat::Plain,
        );

        assert_eq!(result.content.trim(), COHERENT, "top-level content mismatch");

        let pages = result
            .pages
            .as_ref()
            .expect("pages must be populated (regression of #706)");
        assert!(!pages.is_empty(), "pages must not be empty");
        assert_eq!(
            pages[0].content.trim(),
            COHERENT,
            "pages[0].content is a word-by-word dump instead of coherent text (regression of #706)"
        );

        unregister_ocr_backend("tesseract-like-706").unwrap();
    }

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
