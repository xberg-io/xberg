#![cfg(all(feature = "pdf", feature = "ocr", feature = "bundle-tessdata-eng"))]

//! The OCR gate's script rule end to end, with a registered backend standing in for Tesseract so a
//! `khm` first stage can be made to return English. The backend replaces the built-in `tesseract`
//! entry of this process's registry, so every test here is serial and the file holds no other tests.

mod helpers;

use std::sync::Arc;
use xberg::core::config::{OcrStrategy, PageConfig};
use xberg::plugins::{OcrBackend, OcrBackendType, Plugin, register_ocr_backend, unregister_ocr_backend};
use xberg::types::{ExtractedDocument, ExtractionMethod};
use xberg::{ExtractionConfig, OcrConfig, OutputFormat};

const COMPLETE: &[u8] = include_bytes!("fixtures/pdf/khmer-cluster-2536/cluster-complete-mapped.pdf");
const ENGLISH: &str = "The quarterly report contains useful readable text about the country and its people.";
const KHMER: &str = "ខ្ញុំស្រលាញ់ប្រទេសកម្ពុជា សួស្តីភ្នំពេញ ប្រទេសកម្ពុជាមានប្រជាជនច្រើន សៀវភៅសាលារៀន";

struct Fixed(&'static str);

impl Plugin for Fixed {
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
impl OcrBackend for Fixed {
    fn backend_type(&self) -> OcrBackendType {
        OcrBackendType::Custom
    }
    fn supports_language(&self, _: &str) -> bool {
        true
    }
    async fn process_image(&self, _: &[u8], _: &OcrConfig) -> xberg::Result<ExtractedDocument> {
        let mut document = ExtractedDocument::default();
        document.content = self.0.to_owned();
        Ok(document)
    }
}

struct Guard;
impl Drop for Guard {
    fn drop(&mut self) {
        let _ = unregister_ocr_backend("tesseract");
    }
}

fn khmer_count(text: &str) -> usize {
    text.chars().filter(|c| ('\u{1780}'..='\u{17ff}').contains(c)).count()
}

fn read(backend_text: &'static str, disable_ocr: bool) -> ExtractedDocument {
    let _guard = Guard;
    register_ocr_backend(Arc::new(Fixed(backend_text))).unwrap();
    let pipeline = serde_json::json!({"stages": [
        {"backend": "tesseract", "priority": 100, "language": ["khm"]},
    ]});
    let config = ExtractionConfig {
        use_cache: false,
        output_format: OutputFormat::Plain,
        pages: Some(PageConfig {
            extract_pages: true,
            ..Default::default()
        }),
        ocr: Some(OcrConfig {
            pipeline: Some(serde_json::from_value(pipeline).unwrap()),
            ..Default::default()
        }),
        ocr_strategy: OcrStrategy::Auto,
        disable_ocr,
        ..Default::default()
    };
    helpers::extract_bytes_document_blocking(COMPLETE, "application/pdf", &config).unwrap()
}

#[test]
#[serial_test::serial]
fn a_khm_stage_that_reads_english_leaves_the_native_khmer_page() {
    let native = read(ENGLISH, true);
    let automatic = read(ENGLISH, false);
    assert_eq!(khmer_count(&native.content), 222);
    assert_eq!(automatic.extraction_method, Some(ExtractionMethod::Native));
    assert_eq!(automatic.content, native.content);
    let kept: Vec<_> = automatic
        .processing_warnings
        .iter()
        .filter(|w| w.source == "ocr" && w.message.contains("retained its native text"))
        .collect();
    assert_eq!(kept.len(), 1, "{:?}", automatic.processing_warnings);
    assert!(
        automatic
            .ocr_page_failures
            .iter()
            .flatten()
            .all(|failure| !failure.recovered),
        "{:?}",
        automatic.ocr_page_failures
    );
}

#[test]
#[serial_test::serial]
fn a_khm_stage_that_reads_khmer_replaces_the_native_page() {
    let automatic = read(KHMER, false);
    assert_eq!(
        automatic.extraction_method,
        Some(ExtractionMethod::Ocr),
        "{:?}",
        automatic.content
    );
    assert!(
        automatic.content.contains("ប្រទេសកម្ពុជាមានប្រជាជនច្រើន"),
        "{:?}",
        automatic.content
    );
    assert!(
        !automatic
            .processing_warnings
            .iter()
            .any(|w| w.message.contains("retained its native text")),
        "{:?}",
        automatic.processing_warnings
    );
}
