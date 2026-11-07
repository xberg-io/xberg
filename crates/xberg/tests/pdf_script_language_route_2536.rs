#![cfg(all(feature = "pdf", feature = "ocr", feature = "bundle-tessdata-eng"))]

mod helpers;

use xberg::core::config::{OcrStrategy, PageConfig};
use xberg::types::{ExtractedDocument, ExtractionMethod};
use xberg::{ExtractionConfig, OcrConfig, OutputFormat};

const COMPLETE: &[u8] = include_bytes!("fixtures/pdf/khmer-cluster-2536/cluster-complete-mapped.pdf");
const PARTIAL: &[u8] = include_bytes!("fixtures/pdf/khmer-cluster-2536/genuinely-partial-mapped.pdf");
const KHMER_THEN_SCANNED_ENGLISH: &[u8] =
    include_bytes!("fixtures/pdf/khmer-cluster-2536/khmer-then-scanned-english.pdf");

fn khmer_count(text: &str) -> usize {
    text.chars().filter(|c| ('\u{1780}'..='\u{17ff}').contains(c)).count()
}

fn extract(source: &[u8], tessdata: &std::path::Path, tweak: impl FnOnce(&mut ExtractionConfig)) -> ExtractedDocument {
    let mut config = ExtractionConfig {
        use_cache: false,
        output_format: OutputFormat::Plain,
        pages: Some(PageConfig {
            extract_pages: true,
            ..Default::default()
        }),
        ocr: Some(OcrConfig {
            tessdata_path: Some(tessdata.into()),
            ..Default::default()
        }),
        ocr_strategy: OcrStrategy::Auto,
        ..Default::default()
    };
    tweak(&mut config);
    helpers::extract_bytes_document_blocking(source, "application/pdf", &config).unwrap()
}

fn english_model() -> tempfile::TempDir {
    let models = tempfile::tempdir().unwrap();
    let eng = xberg_tesseract::bundled_eng_traineddata().expect("real English OCR model");
    std::fs::write(models.path().join("eng.traineddata"), eng).unwrap();
    models
}

#[test]
fn automatic_english_ocr_keeps_a_native_page_in_a_script_it_cannot_read() {
    let models = english_model();
    for (source, mapped) in [(COMPLETE, 222), (PARTIAL, 28)] {
        let native = extract(source, models.path(), |config| config.disable_ocr = true);
        let automatic = extract(source, models.path(), |_| {});
        assert_eq!(khmer_count(&native.content), mapped);
        assert_eq!(khmer_count(&automatic.content), mapped);
        assert_eq!(automatic.extraction_method, Some(ExtractionMethod::Native));
        assert_eq!(automatic.content, native.content);
    }
}

#[test]
fn automatic_english_ocr_reads_the_scanned_page_and_keeps_the_khmer_page() {
    let models = english_model();
    let native = extract(KHMER_THEN_SCANNED_ENGLISH, models.path(), |config| {
        config.disable_ocr = true
    });
    let automatic = extract(KHMER_THEN_SCANNED_ENGLISH, models.path(), |_| {});
    let native_pages = native.pages.as_ref().unwrap();
    let pages = automatic.pages.as_ref().unwrap();
    assert_eq!(pages.len(), 2);
    assert_eq!(khmer_count(&native_pages[0].content), 222);
    assert_eq!(pages[0].content, native_pages[0].content);
    assert!(native_pages[1].content.trim().is_empty(), "page 2 has no text layer");
    assert!(
        pages[1].content.contains("20,000 bottles of water"),
        "OCR read page 2: {:?}",
        pages[1].content
    );
    assert_eq!(automatic.extraction_method, Some(ExtractionMethod::Mixed));
}

#[test]
fn forced_ocr_still_replaces_the_page_so_the_automatic_result_is_the_routing() {
    let models = english_model();
    let forced = extract(KHMER_THEN_SCANNED_ENGLISH, models.path(), |config| {
        config.force_ocr = true
    });
    assert_eq!(forced.extraction_method, Some(ExtractionMethod::Ocr));
    assert_eq!(khmer_count(&forced.content), 0);
}
