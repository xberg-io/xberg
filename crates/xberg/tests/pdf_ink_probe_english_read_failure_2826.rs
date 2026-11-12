#![cfg(all(feature = "pdf", feature = "ocr", feature = "bundle-tessdata-eng"))]

//! The English read that judges a text layer drawn over a full-page picture, when that read fails:
//! the page has no verdict and is read as a scan, never kept for its layer. A registered backend
//! replaces the built-in `tesseract` entry of this process's registry, so the file holds one test.

mod helpers;

use helpers::pdf_picture::{PicturePlacement, with_full_page_picture};
use std::sync::Arc;
use std::sync::atomic::{AtomicUsize, Ordering};
use xberg::core::config::{OcrStrategy, PageConfig};
use xberg::plugins::{OcrBackend, OcrBackendType, Plugin, register_ocr_backend, unregister_ocr_backend};
use xberg::types::ExtractedDocument;
use xberg::{ExtractionConfig, OcrConfig, OutputFormat};

const COMPLETE: &[u8] = include_bytes!("fixtures/pdf/khmer-cluster-2536/cluster-complete-mapped.pdf");

/// Fails its first read, as the English read does when the model is missing offline, and reads a
/// line of English afterwards.
struct FailingFirst(AtomicUsize);

impl Plugin for FailingFirst {
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
impl OcrBackend for FailingFirst {
    fn backend_type(&self) -> OcrBackendType {
        OcrBackendType::Custom
    }
    fn supports_language(&self, _: &str) -> bool {
        true
    }
    async fn process_image(&self, _: &[u8], _: &OcrConfig) -> xberg::Result<ExtractedDocument> {
        if self.0.fetch_add(1, Ordering::SeqCst) == 0 {
            return Err(xberg::XbergError::ocr("the eng model could not be loaded"));
        }
        let mut document = ExtractedDocument::default();
        document.content = "The picture reads as a page of plain English words.".to_owned();
        Ok(document)
    }
}

struct Guard;
impl Drop for Guard {
    fn drop(&mut self) {
        let _ = unregister_ocr_backend("tesseract");
    }
}

#[test]
fn a_layer_over_a_picture_the_english_read_fails_on_is_read_as_a_scan() {
    let _guard = Guard;
    let backend = Arc::new(FailingFirst(AtomicUsize::new(0)));
    register_ocr_backend(backend.clone()).unwrap();
    let config = ExtractionConfig {
        use_cache: false,
        output_format: OutputFormat::Plain,
        pages: Some(PageConfig {
            extract_pages: true,
            ..Default::default()
        }),
        ocr: Some(OcrConfig::default()),
        ocr_strategy: OcrStrategy::Auto,
        ..Default::default()
    };
    let flyer = with_full_page_picture(COMPLETE, PicturePlacement::Background);
    let result = helpers::extract_bytes_document_blocking(&flyer, "application/pdf", &config).unwrap();
    let warned = |needle: &str| {
        result
            .processing_warnings
            .iter()
            .any(|w| w.source == "ocr" && w.message.contains(needle))
    };
    assert!(
        warned("could not be read with the English OCR model"),
        "{:?}",
        result.processing_warnings
    );
    // The page went to OCR as the scan its picture made it: the rule names the picture it read. ~keep
    assert!(warned("carries a picture"), "{:?}", result.processing_warnings);
    assert!(
        backend.0.load(Ordering::SeqCst) >= 2,
        "the page was read again after the failed English read"
    );
}
