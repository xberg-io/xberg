//! Built-in Middle-stage post-processor that captions every
//! [`ExtractedImage`](crate::types::ExtractedImage) using a VLM.
//!
//! Activates when [`ExtractionConfig::captioning`](crate::core::config::ExtractionConfig::captioning)
//! is `Some`. The processor walks `result.images`, and for each image whose
//! pixel area (`width * height`) is at least `min_image_area` it invokes
//! [`crate::llm::region_extractor::extract_region_with_vlm_usage`] in
//! [`RegionKind::Caption`](crate::llm::region_extractor::RegionKind::Caption)
//! mode. The caption is stored on [`ExtractedImage::caption`].
//!
//! Every VLM call's [`LlmUsage`](crate::types::LlmUsage) is appended to
//! [`ExtractionResult::llm_usage`] so token / cost accounting carries over
//! into downstream telemetry.

use std::sync::Arc;

use async_trait::async_trait;

use crate::Result;
use crate::core::config::ExtractionConfig;
use crate::llm::region_extractor::{RegionKind, extract_region_with_vlm_usage};
use crate::plugins::{Plugin, PostProcessor, ProcessingStage, register_post_processor};
use crate::types::{ExtractedImage, ExtractionResult};

/// Post-processor that captions every extracted image via a VLM.
#[cfg_attr(alef, alef(skip))]
#[derive(Debug, Clone, Copy, Default)]
pub struct CaptioningProcessor;

impl Plugin for CaptioningProcessor {
    fn name(&self) -> &str {
        "captioning"
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
impl PostProcessor for CaptioningProcessor {
    async fn process(&self, result: &mut ExtractionResult, config: &ExtractionConfig) -> Result<()> {
        let Some(caption_config) = config.captioning.as_ref() else {
            return Ok(());
        };
        let Some(images) = result.images.as_mut() else {
            result.processing_warnings.push(crate::types::ProcessingWarning {
                source: std::borrow::Cow::Borrowed("captioning"),
                message: std::borrow::Cow::Borrowed(
                    "captioning configured but no images were extracted; \
                     set config.images to enable image extraction from documents",
                ),
            });
            return Ok(());
        };
        if images.is_empty() {
            return Ok(());
        }

        tracing::info!(
            target: "kreuzberg::captioning",
            images = images.len(),
            model = %caption_config.llm.model,
            min_image_area = caption_config.min_image_area,
            "running per-image VLM captioning"
        );

        let prompt = caption_config.prompt.as_deref();
        let min_area = u64::from(caption_config.min_image_area);

        let mut captured_usage: Vec<crate::types::LlmUsage> = Vec::new();

        for image in images.iter_mut() {
            if !image_is_caption_candidate(image, min_area) {
                continue;
            }

            let mime = mime_for_format(image.format.as_ref());
            match extract_region_with_vlm_usage(
                image.data.as_ref(),
                mime,
                RegionKind::Caption,
                &caption_config.llm,
                prompt,
            )
            .await
            {
                Ok((text, usage)) => {
                    let trimmed = text.trim().to_string();
                    if !trimmed.is_empty() {
                        image.caption = Some(trimmed);
                    }
                    if let Some(mut usage) = usage {
                        if usage.source.is_empty() || usage.source == "vlm_ocr" {
                            usage.source = "captioning".to_string();
                        }
                        captured_usage.push(usage);
                    }
                }
                Err(error) => {
                    tracing::warn!(
                        target: "kreuzberg::captioning",
                        index = image.image_index,
                        format = %image.format,
                        error = %error,
                        "VLM caption call failed; image left without caption"
                    );
                }
            }
        }

        if !captured_usage.is_empty() {
            match result.llm_usage.as_mut() {
                Some(existing) => existing.extend(captured_usage),
                None => result.llm_usage = Some(captured_usage),
            }
        }
        Ok(())
    }

    fn processing_stage(&self) -> ProcessingStage {
        ProcessingStage::Middle
    }

    fn should_process(&self, _result: &ExtractionResult, config: &ExtractionConfig) -> bool {
        config.captioning.is_some()
    }

    fn priority(&self) -> i32 {
        50
    }
}

/// Pixel-area gate: skip masks and images smaller than the configured threshold.
///
/// Images with unknown dimensions are treated as candidates so format hints alone do
/// not silently suppress captioning when the extractor failed to populate width/height.
fn image_is_caption_candidate(image: &ExtractedImage, min_area: u64) -> bool {
    if image.is_mask {
        return false;
    }
    match (image.width, image.height) {
        (Some(w), Some(h)) => u64::from(w) * u64::from(h) >= min_area,
        _ => true,
    }
}

/// Map an [`ExtractedImage::format`] string into a MIME type the VLM call accepts.
fn mime_for_format(format: &str) -> &'static str {
    match format.to_ascii_lowercase().as_str() {
        "jpeg" | "jpg" => "image/jpeg",
        "png" => "image/png",
        "webp" => "image/webp",
        "gif" => "image/gif",
        "bmp" => "image/bmp",
        "tiff" | "tif" => "image/tiff",
        _ => "image/png",
    }
}

/// Register the default captioning post-processor with the global registry.
#[cfg_attr(alef, alef(skip))]
pub fn register() -> Result<()> {
    register_post_processor(Arc::new(CaptioningProcessor))
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::core::config::{CaptioningConfig, LlmConfig};
    use bytes::Bytes;
    use std::borrow::Cow;

    fn caption_config(min_image_area: u32) -> CaptioningConfig {
        CaptioningConfig {
            llm: LlmConfig {
                model: "openai/gpt-4o-mini".to_string(),
                ..Default::default()
            },
            prompt: None,
            min_image_area,
        }
    }

    fn image_with(width: Option<u32>, height: Option<u32>, is_mask: bool) -> ExtractedImage {
        ExtractedImage {
            data: Bytes::from_static(&[]),
            format: Cow::Borrowed("png"),
            width,
            height,
            is_mask,
            ..Default::default()
        }
    }

    #[test]
    fn processor_metadata_is_correct() {
        let p = CaptioningProcessor;
        assert_eq!(p.name(), "captioning");
        assert_eq!(p.processing_stage(), ProcessingStage::Middle);
    }

    #[test]
    fn should_process_only_when_config_present() {
        let p = CaptioningProcessor;
        let result = ExtractionResult {
            content: "x".to_string(),
            mime_type: Cow::Borrowed("text/plain"),
            ..Default::default()
        };
        assert!(!p.should_process(&result, &ExtractionConfig::default()));

        let cfg = ExtractionConfig {
            captioning: Some(caption_config(1000)),
            ..Default::default()
        };
        assert!(p.should_process(&result, &cfg));
    }

    #[test]
    fn masks_are_never_candidates() {
        let image = image_with(Some(2000), Some(2000), true);
        assert!(!image_is_caption_candidate(&image, 1_000));
    }

    #[test]
    fn small_images_are_skipped() {
        let image = image_with(Some(10), Some(10), false);
        assert!(!image_is_caption_candidate(&image, 1_000));
    }

    #[test]
    fn large_images_pass_threshold() {
        let image = image_with(Some(64), Some(64), false);
        assert!(image_is_caption_candidate(&image, 1_000));
    }

    #[test]
    fn unknown_dimensions_pass_through() {
        let image = image_with(None, None, false);
        assert!(image_is_caption_candidate(&image, 1_000));
    }

    #[tokio::test]
    async fn warns_when_no_images_extracted() {
        let p = CaptioningProcessor;
        let cfg = ExtractionConfig {
            captioning: Some(caption_config(1000)),
            ..Default::default()
        };
        let mut result = ExtractionResult {
            content: "x".to_string(),
            mime_type: std::borrow::Cow::Borrowed("text/plain"),
            ..Default::default()
        };
        // result.images is None — simulates forgetting config.images
        p.process(&mut result, &cfg).await.unwrap();
        let warnings = result.processing_warnings;
        assert!(
            warnings.iter().any(|w| w.source == "captioning"),
            "expected captioning warning when images is None; got: {warnings:?}"
        );
    }

    #[test]
    fn mime_mapping_covers_common_formats() {
        assert_eq!(mime_for_format("png"), "image/png");
        assert_eq!(mime_for_format("JPG"), "image/jpeg");
        assert_eq!(mime_for_format("jpeg"), "image/jpeg");
        assert_eq!(mime_for_format("webp"), "image/webp");
        assert_eq!(mime_for_format("tiff"), "image/tiff");
        assert_eq!(mime_for_format("bmp"), "image/bmp");
        assert_eq!(mime_for_format("gif"), "image/gif");
        assert_eq!(mime_for_format("unknown"), "image/png");
    }
}
