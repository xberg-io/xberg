use super::*;
use crate::core::config::OcrConfig;
use crate::plugins::{OcrBackend, OcrBackendType, Plugin, register_ocr_backend, unregister_ocr_backend};
use std::sync::Arc;

const BACKEND: &str = "native-script-kept-fixed";
const KHMER: &str = "ខ្ញុំ\u{FFFD}\u{FFFD} ស្រលាញ់\u{FFFD} ប្រទេស\u{FFFD}\u{FFFD} កម្ពុជា\u{FFFD} សួស្តី\u{FFFD} ភ្នំពេញ";
const KHMER_READ: &str = "ខ្ញុំស្រលាញ់ប្រទេសកម្ពុជា សួស្តីភ្នំពេញ ប្រទេសកម្ពុជាមានប្រជាជនច្រើន";
const LATIN_READ: &str = "The quarterly report contains useful readable text about the country.";

struct FixedBackend(&'static str);

impl Plugin for FixedBackend {
    fn name(&self) -> &str {
        BACKEND
    }
    fn version(&self) -> String {
        "1.0.0".to_owned()
    }
    fn initialize(&self) -> crate::Result<()> {
        Ok(())
    }
    fn shutdown(&self) -> crate::Result<()> {
        Ok(())
    }
}

#[async_trait::async_trait]
impl OcrBackend for FixedBackend {
    fn backend_type(&self) -> OcrBackendType {
        OcrBackendType::Custom
    }
    fn supports_language(&self, _: &str) -> bool {
        true
    }
    async fn process_image(&self, _: &[u8], _: &OcrConfig) -> crate::Result<crate::ExtractedDocument> {
        Ok(crate::ExtractedDocument {
            content: self.0.to_owned(),
            ..Default::default()
        })
    }
}

struct BackendGuard;
impl Drop for BackendGuard {
    fn drop(&mut self) {
        let _ = unregister_ocr_backend(BACKEND);
    }
}

/// Reads the one-page document `KHMER` with a backend that returns `read`, the page being listed in
/// `native_script` or not. Returns the text and the warnings.
async fn read_page(read: &'static str, native_script: &[u32]) -> (String, Vec<crate::types::ProcessingWarning>) {
    let _guard = BackendGuard;
    register_ocr_backend(Arc::new(FixedBackend(read))).unwrap();
    let config = ExtractionConfig {
        ocr: Some(OcrConfig {
            backend: BACKEND.to_owned(),
            ..Default::default()
        }),
        ocr_embedded_images: Some(false),
        use_cache: false,
        ..Default::default()
    };
    let boundaries = [crate::types::PageBoundary {
        byte_start: 0,
        byte_end: KHMER.len(),
        page_number: 1,
    }];
    let pdf = crate::pdf::render::build_minimal_pdf_with_mediabox(720.0, 720.0);
    let run = extract_mixed_ocr_native_with_layout_pages(
        KHMER,
        &boundaries,
        (&[1], &[]),
        &pdf,
        &config,
        None,
        (&[], native_script),
    )
    .await
    .unwrap();
    (run.output.0, run.output.8)
}

#[tokio::test]
#[serial_test::serial]
async fn a_read_without_the_native_script_keeps_the_native_page() {
    let (text, warnings) = read_page(LATIN_READ, &[1]).await;
    assert_eq!(text, KHMER);
    assert_eq!(warnings.len(), 1, "{warnings:?}");
    assert!(
        warnings[0].message.contains("Page 1 retained its native text"),
        "{warnings:?}"
    );
    assert!(warnings[0].message.contains("Khmer"), "{warnings:?}");
}

#[tokio::test]
#[serial_test::serial]
async fn a_read_in_the_native_script_replaces_the_native_page() {
    let (text, warnings) = read_page(KHMER_READ, &[1]).await;
    assert_eq!(text.trim(), KHMER_READ);
    assert!(warnings.is_empty(), "{warnings:?}");
}

#[tokio::test]
#[serial_test::serial]
async fn a_page_the_script_rule_did_not_mark_is_replaced_by_any_read() {
    let (text, warnings) = read_page(LATIN_READ, &[]).await;
    assert_eq!(text.trim(), LATIN_READ);
    assert!(warnings.is_empty(), "{warnings:?}");
}
