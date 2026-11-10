#![cfg(all(feature = "pdf", feature = "ocr", feature = "bundle-tessdata-eng"))]

//! The OCR gate's script rule end to end, with a registered backend standing in for Tesseract so a
//! `khm` first stage can be made to return English. The backend replaces the built-in `tesseract`
//! entry of this process's registry, so every test here is serial and the file holds no other tests.

mod helpers;

use std::sync::Arc;
use std::sync::atomic::{AtomicUsize, Ordering};
use xberg::core::config::{OcrStrategy, PageConfig};
use xberg::plugins::{OcrBackend, OcrBackendType, Plugin, register_ocr_backend, unregister_ocr_backend};
use xberg::types::{ExtractedDocument, ExtractionMethod};
use xberg::{ExtractionConfig, OcrConfig, OutputFormat};

const COMPLETE: &[u8] = include_bytes!("fixtures/pdf/khmer-cluster-2536/cluster-complete-mapped.pdf");
const FIGURE: &[u8] = include_bytes!("fixtures/pdf/khmer-cluster-2536/khmer-text-with-figure.pdf");
const THEN_SCANNED: &[u8] = include_bytes!("fixtures/pdf/khmer-cluster-2536/khmer-then-scanned-english.pdf");
const AMONG_ENGLISH: &[u8] = include_bytes!("fixtures/pdf/khmer-cluster-2536/khmer-among-english-pages.pdf");
const KHMER_PAGE_LETTERS: usize = 222;
const ENGLISH: &str = "The quarterly report contains useful readable text about the country and its people.";
const KHMER: &str = "ខ្ញុំស្រលាញ់ប្រទេសកម្ពុជា សួស្តីភ្នំពេញ ប្រទេសកម្ពុជាមានប្រជាជនច្រើន សៀវភៅសាលារៀន";

struct Fixed(&'static str);

/// Errors on its first `failing` calls and reads `text` afterwards, so a page raster fails and the
/// image the fallback then reads from the same page succeeds.
struct FailingFirst {
    text: &'static str,
    failing: usize,
    calls: AtomicUsize,
}

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
        if self.calls.fetch_add(1, Ordering::SeqCst) < self.failing {
            return Err(xberg::XbergError::ocr("page raster refused"));
        }
        let mut document = ExtractedDocument::default();
        document.content = self.text.to_owned();
        Ok(document)
    }
}

/// Reads `text` for its first `ok_calls` calls and errors afterwards, counting every call, so a test
/// can let the whole-document read succeed and fail only the per-page retry that follows it.
struct Counting {
    text: &'static str,
    ok_calls: usize,
    calls: AtomicUsize,
}

impl Plugin for Counting {
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
impl OcrBackend for Counting {
    fn backend_type(&self) -> OcrBackendType {
        OcrBackendType::Custom
    }
    fn supports_language(&self, _: &str) -> bool {
        true
    }
    async fn process_image(&self, _: &[u8], _: &OcrConfig) -> xberg::Result<ExtractedDocument> {
        if self.calls.fetch_add(1, Ordering::SeqCst) >= self.ok_calls {
            return Err(xberg::XbergError::ocr("per-page read refused"));
        }
        let mut document = ExtractedDocument::default();
        document.content = self.text.to_owned();
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
    read_with(COMPLETE, Arc::new(Fixed(backend_text)), disable_ocr)
}

fn read_with(source: &[u8], backend: Arc<dyn OcrBackend>, disable_ocr: bool) -> ExtractedDocument {
    let _guard = Guard;
    register_ocr_backend(backend).unwrap();
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
    helpers::extract_bytes_document_blocking(source, "application/pdf", &config).unwrap()
}

#[test]
#[serial_test::serial]
fn a_khm_stage_that_reads_english_leaves_the_native_khmer_page() {
    let native = read(ENGLISH, true);
    let automatic = read(ENGLISH, false);
    assert_eq!(khmer_count(&native.content), KHMER_PAGE_LETTERS);
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

#[test]
#[serial_test::serial]
fn a_failed_page_read_whose_image_was_read_is_not_reported_as_recovered_when_the_native_page_is_kept() {
    let backend = Arc::new(FailingFirst {
        text: ENGLISH,
        failing: 1,
        calls: AtomicUsize::new(0),
    });
    let native = read_with(FIGURE, Arc::new(Fixed(ENGLISH)), true);
    let automatic = read_with(FIGURE, backend.clone(), false);
    assert!(
        backend.calls.load(Ordering::SeqCst) >= 2,
        "the failure and the retry must both run"
    );
    assert_eq!(automatic.extraction_method, Some(ExtractionMethod::Native));
    assert_eq!(khmer_count(&automatic.content), khmer_count(&native.content));
    // The retry reads the page's embedded figure, and that text is appended after the native page as
    // image text. The failed page read itself must add nothing, so the native page is unchanged.
    assert_eq!(automatic.content, format!("{}\n\n{ENGLISH}", native.content));
    let failures: Vec<_> = automatic.ocr_page_failures.iter().flatten().collect();
    assert!(!failures.is_empty(), "{:?}", automatic.processing_warnings);
    assert!(failures.iter().all(|failure| !failure.recovered), "{failures:?}");
    assert!(
        !automatic
            .processing_warnings
            .iter()
            .any(|w| w.message.contains("was recovered from")),
        "{:?}",
        automatic.processing_warnings
    );
}

#[test]
#[serial_test::serial]
fn only_the_marked_page_of_a_document_with_readable_pages_keeps_its_native_text() {
    let native = read_with(AMONG_ENGLISH, Arc::new(Fixed(ENGLISH)), true);
    let automatic = read_with(AMONG_ENGLISH, Arc::new(Fixed(ENGLISH)), false);
    let native_pages = native.pages.as_ref().unwrap();
    let pages = automatic.pages.as_ref().unwrap();
    assert_eq!(pages.len(), 4);
    assert_eq!(khmer_count(&native_pages[1].content), KHMER_PAGE_LETTERS);
    assert_eq!(pages[1].content, native_pages[1].content);
    assert_eq!(pages[0].content, native_pages[0].content);
    assert_eq!(pages[2].content, native_pages[2].content);
    assert!(native_pages[3].content.trim().is_empty(), "page 4 has no text layer");
    assert_eq!(pages[3].content.trim(), ENGLISH);
    let kept: Vec<_> = automatic
        .processing_warnings
        .iter()
        .filter(|w| w.source == "ocr" && w.message.contains("Page 2 retained its native text"))
        .collect();
    assert_eq!(kept.len(), 1, "{:?}", automatic.processing_warnings);
}

fn counting(text: &'static str, ok_calls: usize) -> Arc<Counting> {
    Arc::new(Counting {
        text,
        ok_calls,
        calls: AtomicUsize::new(0),
    })
}

#[test]
#[serial_test::serial]
fn a_marked_page_beside_a_scan_keeps_its_native_text_when_the_whole_document_read_drops_its_script() {
    let native = read_with(THEN_SCANNED, Arc::new(Fixed(ENGLISH)), true);
    let backend = counting(ENGLISH, usize::MAX);
    let automatic = read_with(THEN_SCANNED, backend.clone(), false);
    let native_pages = native.pages.as_ref().unwrap();
    let pages = automatic.pages.as_ref().unwrap();
    assert_eq!(pages.len(), 2);
    assert_eq!(khmer_count(&native_pages[0].content), KHMER_PAGE_LETTERS);
    assert!(native_pages[1].content.trim().is_empty(), "page 2 is a scan");
    assert_eq!(pages[0].content, native_pages[0].content);
    assert_eq!(pages[1].content.trim(), ENGLISH);
    assert_eq!(
        khmer_count(&automatic.content),
        KHMER_PAGE_LETTERS,
        "{:?}",
        automatic.content
    );
    assert_eq!(automatic.extraction_method, Some(ExtractionMethod::Mixed));
    let kept: Vec<_> = automatic
        .processing_warnings
        .iter()
        .filter(|w| w.source == "ocr" && w.message.contains("Page 1 retained its native text"))
        .collect();
    assert_eq!(kept.len(), 1, "{:?}", automatic.processing_warnings);
    assert_eq!(
        backend.calls.load(Ordering::SeqCst),
        4,
        "the whole-document read and the per-page retry both run"
    );
}

#[test]
#[serial_test::serial]
fn a_retry_that_cannot_read_a_scan_the_whole_read_read_keeps_the_whole_read_and_warns() {
    let backend = counting(ENGLISH, 2);
    let automatic = read_with(THEN_SCANNED, backend.clone(), false);
    assert!(
        backend.calls.load(Ordering::SeqCst) > 2,
        "the whole-document read took the first two calls, so the retry ran and was refused"
    );
    let pages = automatic.pages.as_ref().unwrap();
    assert_eq!(pages[1].content.trim(), ENGLISH);
    let warned: Vec<_> = automatic
        .processing_warnings
        .iter()
        .filter(|w| w.source == "ocr" && w.message.contains("Pages [1]"))
        .collect();
    assert_eq!(warned.len(), 1, "{:?}", automatic.processing_warnings);
}

#[test]
#[serial_test::serial]
fn a_healthy_khmer_read_replaces_the_marked_page_beside_a_scan() {
    let backend = counting(KHMER, usize::MAX);
    let automatic = read_with(THEN_SCANNED, backend.clone(), false);
    let pages = automatic.pages.as_ref().unwrap();
    assert_eq!(pages.len(), 2);
    assert!(
        pages[0].content.contains("ប្រទេសកម្ពុជាមានប្រជាជនច្រើន"),
        "{:?}",
        pages[0].content
    );
    assert_eq!(pages[1].content.trim(), KHMER);
    assert_eq!(
        backend.calls.load(Ordering::SeqCst),
        2,
        "a read that keeps the script needs no retry"
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
