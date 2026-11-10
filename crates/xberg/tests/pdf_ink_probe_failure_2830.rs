#![cfg(all(feature = "pdf", feature = "ocr", feature = "bundle-tessdata-eng"))]

//! The ink probe of the script rule when the run is cancelled while it reads: the cancellation is
//! returned and is not taken for "the ink agrees with the layer". A registered backend replaces the built-in `tesseract`
//! entry of this process's registry, so the file holds one test.

mod helpers;

use std::sync::Arc;
use xberg::core::config::{OcrStrategy, PageConfig};
use xberg::plugins::{OcrBackend, OcrBackendType, Plugin, register_ocr_backend, unregister_ocr_backend};
use xberg::types::ExtractedDocument;
use xberg::{ExtractionConfig, OcrConfig, OutputFormat};

const WRONG_SCRIPT: &[u8] = include_bytes!("fixtures/pdf/wrong-script-tounicode-2830/arabic.pdf");

struct Cancelling(xberg::cancellation::CancellationToken);

impl Plugin for Cancelling {
    fn name(&self) -> &str {
        "tesseract"
    }
    fn version(&self) -> String {
        "1.0.0".to_owned()
    }
    fn initialize(&self) -> xberg::Result<()> {
        Ok(())
    }
    fn shutdown(&self) -> xberg::Result<()> {
        Ok(())
    }
}

#[async_trait::async_trait]
impl OcrBackend for Cancelling {
    fn backend_type(&self) -> OcrBackendType {
        OcrBackendType::Custom
    }
    fn supports_language(&self, _: &str) -> bool {
        true
    }
    async fn process_image(&self, _: &[u8], _: &OcrConfig) -> xberg::Result<ExtractedDocument> {
        self.0.cancel();
        Err(xberg::XbergError::Cancelled)
    }
}

struct Guard;
impl Drop for Guard {
    fn drop(&mut self) {
        let _ = unregister_ocr_backend("tesseract");
    }
}

#[test]
fn a_run_cancelled_during_the_ink_probe_is_returned_as_cancelled() {
    let _guard = Guard;
    let token = xberg::cancellation::CancellationToken::new();
    register_ocr_backend(Arc::new(Cancelling(token.clone()))).unwrap();
    let config = ExtractionConfig {
        use_cache: false,
        output_format: OutputFormat::Plain,
        pages: Some(PageConfig {
            extract_pages: true,
            ..Default::default()
        }),
        ocr: Some(OcrConfig::default()),
        ocr_strategy: OcrStrategy::Auto,
        cancel_token: Some(token),
        ..Default::default()
    };
    let result = helpers::extract_bytes_document_blocking(WRONG_SCRIPT, "application/pdf", &config);
    assert!(
        matches!(result, Err(xberg::XbergError::Cancelled)),
        "{:?}",
        result.map(|r| r.processing_warnings)
    );
}
