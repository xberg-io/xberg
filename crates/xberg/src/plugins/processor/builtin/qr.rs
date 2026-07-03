//! Built-in Middle-stage post-processor that decodes QR codes inside every
//! [`ExtractedImage`](crate::types::ExtractedImage).
//!
//! Activates when [`ExtractionConfig::qr_codes`](crate::core::config::ExtractionConfig::qr_codes)
//! is `Some(true)`. Walks `result.images.iter_mut()`, runs the pure-Rust
//! [`crate::extractors::qr::detect_qr_codes`] decoder on each image's bytes,
//! and writes the result into [`crate::types::ExtractedImage::qr_codes`]. A `Some(vec![])`
//! is written when detection ran but found nothing; `None` only when QR
//! detection was not enabled.

use std::sync::Arc;

use async_trait::async_trait;

use crate::Result;
use crate::core::config::ExtractionConfig;
use crate::extractors::qr::detect_qr_codes;
use crate::plugins::{Plugin, PostProcessor, ProcessingStage, register_post_processor};
use crate::types::ExtractedDocument;

/// Post-processor that runs `rqrr` over each image and writes the decoded QR
/// payloads into [`crate::types::ExtractedImage::qr_codes`].
#[cfg_attr(alef, alef(skip))]
#[derive(Debug, Clone, Copy, Default)]
pub struct QrCodeProcessor;

impl Plugin for QrCodeProcessor {
    fn name(&self) -> &str {
        "qr-codes"
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
impl PostProcessor for QrCodeProcessor {
    async fn process(&self, result: &mut ExtractedDocument, config: &ExtractionConfig) -> Result<()> {
        if config.qr_codes != Some(true) {
            return Ok(());
        }
        let Some(images) = result.images.as_mut() else {
            return Ok(());
        };
        if images.is_empty() {
            return Ok(());
        }

        tracing::info!(
            target: "xberg::qr_codes",
            images = images.len(),
            "running rqrr QR detection across extracted images"
        );

        for image in images.iter_mut() {
            let codes = detect_qr_codes(image.data.as_ref(), Some(image.format.as_ref()));
            image.qr_codes = Some(codes);
        }
        Ok(())
    }

    fn processing_stage(&self) -> ProcessingStage {
        ProcessingStage::Middle
    }

    fn should_process(&self, _result: &ExtractedDocument, config: &ExtractionConfig) -> bool {
        config.qr_codes == Some(true)
    }

    fn priority(&self) -> i32 {
        50
    }
}

/// Register the default QR post-processor with the global registry.
#[cfg_attr(alef, alef(skip))]
pub fn register() -> Result<()> {
    register_post_processor(Arc::new(QrCodeProcessor))
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::types::ExtractedImage;
    use bytes::Bytes;
    use std::borrow::Cow;

    fn empty_result_with_image() -> ExtractedDocument {
        ExtractedDocument {
            content: String::new(),
            mime_type: Cow::Borrowed("text/plain"),
            images: Some(vec![ExtractedImage {
                data: Bytes::from_static(&[]),
                format: Cow::Borrowed("png"),
                ..Default::default()
            }]),
            ..Default::default()
        }
    }

    #[test]
    fn processor_metadata_is_correct() {
        let p = QrCodeProcessor;
        assert_eq!(p.name(), "qr-codes");
        assert_eq!(p.processing_stage(), ProcessingStage::Middle);
    }

    #[test]
    fn should_process_only_when_enabled() {
        let p = QrCodeProcessor;
        let result = empty_result_with_image();

        assert!(!p.should_process(&result, &ExtractionConfig::default()));

        let cfg = ExtractionConfig {
            qr_codes: Some(false),
            ..Default::default()
        };
        assert!(!p.should_process(&result, &cfg));

        let cfg = ExtractionConfig {
            qr_codes: Some(true),
            ..Default::default()
        };
        assert!(p.should_process(&result, &cfg));
    }

    #[tokio::test]
    async fn empty_image_bytes_yield_empty_vec() {
        let p = QrCodeProcessor;
        let mut result = empty_result_with_image();
        let cfg = ExtractionConfig {
            qr_codes: Some(true),
            ..Default::default()
        };
        p.process(&mut result, &cfg).await.unwrap();
        let images = result.images.as_ref().expect("images should still be Some");
        assert_eq!(images[0].qr_codes.as_deref(), Some(&[][..]));
    }

    #[tokio::test]
    async fn disabled_config_leaves_field_none() {
        let p = QrCodeProcessor;
        let mut result = empty_result_with_image();
        let cfg = ExtractionConfig::default();
        p.process(&mut result, &cfg).await.unwrap();
        let images = result.images.as_ref().expect("images should still be Some");
        assert!(images[0].qr_codes.is_none());
    }
}
