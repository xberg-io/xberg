#![cfg(all(feature = "pdf", feature = "ocr", feature = "bundle-tessdata-eng"))]

mod helpers;

use xberg::core::config::{OcrStrategy, PageConfig};
use xberg::types::{ExtractedDocument, ExtractionMethod};
use xberg::{ExtractionConfig, OcrConfig, OutputFormat};

const COMPLETE: &[u8] = include_bytes!("fixtures/pdf/khmer-cluster-2536/cluster-complete-mapped.pdf");
const PARTIAL: &[u8] = include_bytes!("fixtures/pdf/khmer-cluster-2536/genuinely-partial-mapped.pdf");
const SCANNED_ENGLISH_KHMER_OVERLAY: &[u8] =
    include_bytes!("fixtures/pdf/khmer-cluster-2536/scanned-english-khmer-overlay.pdf");
const KHMER_SHORT_FULLY_MAPPED: &[u8] = include_bytes!("fixtures/pdf/khmer-cluster-2536/khmer-short-fully-mapped.pdf");
const KHMER_TEXT_WITH_FIGURE: &[u8] = include_bytes!("fixtures/pdf/khmer-cluster-2536/khmer-text-with-figure.pdf");
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
    assert!(
        automatic
            .processing_warnings
            .iter()
            .any(|w| w.source == "ocr" && w.message.contains("page 1") && w.message.contains("Khmer")),
        "{:?}",
        automatic.processing_warnings
    );
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

#[test]
fn automatic_english_ocr_still_reads_a_scanned_page_that_carries_a_khmer_text_overlay() {
    let models = english_model();
    let native = extract(SCANNED_ENGLISH_KHMER_OVERLAY, models.path(), |config| {
        config.disable_ocr = true
    });
    assert!(khmer_count(&native.content) > 0 && !native.content.contains("bottles"));
    let automatic = extract(SCANNED_ENGLISH_KHMER_OVERLAY, models.path(), |_| {});
    assert!(
        automatic.content.contains("20,000 bottles of water"),
        "the rasterised English body was not read: {:?}",
        automatic.content
    );
}

#[test]
fn automatic_english_ocr_keeps_a_text_heavy_khmer_page_that_carries_a_figure() {
    let models = english_model();
    let native = extract(KHMER_TEXT_WITH_FIGURE, models.path(), |config| {
        config.disable_ocr = true
    });
    let automatic = extract(KHMER_TEXT_WITH_FIGURE, models.path(), |_| {});
    assert!(khmer_count(&native.content) > 400);
    // The figure's own text is read by the embedded-image pass and follows the kept layer.
    assert!(automatic.content.starts_with(native.content.trim_end()));
    assert_eq!(khmer_count(&automatic.content), khmer_count(&native.content));
    assert_eq!(automatic.extraction_method, Some(ExtractionMethod::Native));
}

#[test]
fn automatic_english_ocr_keeps_a_short_fully_mapped_khmer_page() {
    let models = english_model();
    let native = extract(KHMER_SHORT_FULLY_MAPPED, models.path(), |config| {
        config.disable_ocr = true
    });
    assert!(!native.content.contains('\u{FFFD}'), "{:?}", native.content);
    let khmer = khmer_count(&native.content);
    assert!(khmer >= 8, "{:?}", native.content);
    let automatic = extract(KHMER_SHORT_FULLY_MAPPED, models.path(), |_| {});
    assert_eq!(khmer_count(&automatic.content), khmer, "{:?}", automatic.content);
    assert_eq!(automatic.extraction_method, Some(ExtractionMethod::Native));
}

#[test]
fn automatic_ocr_through_an_english_only_pipeline_keeps_the_khmer_page() {
    let models = english_model();
    let pipeline = serde_json::json!({"stages": [
        {"backend": "tesseract", "priority": 100},
        {"backend": "tesseract", "priority": 50, "language": ["eng"]},
    ]});
    let native = extract(COMPLETE, models.path(), |config| config.disable_ocr = true);
    let automatic = extract(COMPLETE, models.path(), |config| {
        config.ocr.as_mut().unwrap().pipeline = Some(serde_json::from_value(pipeline).unwrap());
    });
    assert_eq!(khmer_count(&automatic.content), khmer_count(&native.content));
    assert_eq!(automatic.extraction_method, Some(ExtractionMethod::Native));
}

#[test]
fn a_scanned_page_with_a_khmer_layer_read_with_english_warns_to_add_khm() {
    let models = english_model();
    let automatic = extract(SCANNED_ENGLISH_KHMER_OVERLAY, models.path(), |_| {});
    let warnings: Vec<_> = automatic
        .processing_warnings
        .iter()
        .filter(|w| w.source == "ocr" && w.message.contains("Khmer"))
        .collect();
    assert_eq!(warnings.len(), 1, "{:?}", automatic.processing_warnings);
    assert!(warnings[0].message.contains("khm") && warnings[0].message.contains("page 1"));
    assert!(automatic.content.contains("20,000 bottles of water"));
}

#[test]
fn an_english_stage_before_a_khmer_stage_cannot_replace_the_khmer_layer() {
    let models = english_model();
    let native = extract(COMPLETE, models.path(), |config| config.disable_ocr = true);
    let pipeline = serde_json::json!({"stages": [
        {"backend": "tesseract", "priority": 100, "language": ["eng"]},
        {"backend": "tesseract", "priority": 50, "language": ["khm"]},
    ]});
    let automatic = extract(COMPLETE, models.path(), |config| {
        config.ocr.as_mut().unwrap().pipeline = Some(serde_json::from_value(pipeline).unwrap());
    });
    assert_eq!(khmer_count(&native.content), 222);
    assert_eq!(automatic.content, native.content);
    assert_eq!(automatic.extraction_method, Some(ExtractionMethod::Native));
}
