#![cfg(all(feature = "pdf", feature = "ocr", feature = "bundle-tessdata-eng"))]

//! An English page whose Type0 `ToUnicode` map sends Latin letters to Arabic or Bengali code points
//! and has one lost glyph: the script rule must not keep that layer, because the page's ink is
//! Latin and the English read is the correct reader. A page whose ink is really Arabic stays native.

mod helpers;

use xberg::core::config::{OcrStrategy, PageConfig};
use xberg::types::{ExtractedDocument, ExtractionMethod};
use xberg::{ExtractionConfig, OcrConfig, OutputFormat};

const ARABIC: &[u8] = include_bytes!("fixtures/pdf/wrong-script-tounicode-2830/arabic.pdf");
const BENGALI: &[u8] = include_bytes!("fixtures/pdf/wrong-script-tounicode-2830/bengali.pdf");
const GENUINE_ARABIC: &[u8] = include_bytes!("fixtures/pdf/wrong-script-tounicode-2830/genuine.pdf");
const PHRASES: [&str; 3] = [
    "committee met on Tuesday",
    "parking area",
    "architect said that building work",
];

fn extract(source: &[u8], models: &std::path::Path) -> ExtractedDocument {
    let config = ExtractionConfig {
        use_cache: false,
        output_format: OutputFormat::Plain,
        pages: Some(PageConfig {
            extract_pages: true,
            ..Default::default()
        }),
        ocr: Some(OcrConfig {
            tessdata_path: Some(models.into()),
            ..Default::default()
        }),
        ocr_strategy: OcrStrategy::Auto,
        ..Default::default()
    };
    helpers::extract_bytes_document_blocking(source, "application/pdf", &config).unwrap()
}

fn skipped(result: &ExtractedDocument) -> bool {
    result
        .processing_warnings
        .iter()
        .any(|warning| warning.message.contains("OCR was skipped"))
}

fn arabic_letters(text: &str) -> usize {
    text.chars().filter(|c| ('\u{0600}'..='\u{06FF}').contains(c)).count()
}

fn models() -> tempfile::TempDir {
    let models = tempfile::tempdir().unwrap();
    std::fs::write(
        models.path().join("eng.traineddata"),
        xberg_tesseract::bundled_eng_traineddata().expect("real English OCR model"),
    )
    .unwrap();
    models
}

#[test]
fn english_page_with_wrong_script_tounicode_is_read_by_ocr() {
    let models = models();
    for (name, source) in [("arabic", ARABIC), ("bengali", BENGALI)] {
        let result = extract(source, models.path());
        for phrase in PHRASES {
            assert!(
                result.content.contains(phrase),
                "{name}: missing {phrase:?} in {:?}",
                result.content
            );
        }
        assert!(!skipped(&result), "{name}: {:?}", result.processing_warnings);
        assert_ne!(result.extraction_method, Some(ExtractionMethod::Native), "{name}");
        assert_eq!(arabic_letters(&result.content), 0, "{name}");
    }
}

#[test]
fn genuine_arabic_page_stays_native_when_ocr_cannot_read_it() {
    let models = models();
    let result = extract(GENUINE_ARABIC, models.path());
    assert!(skipped(&result), "{:?}", result.processing_warnings);
    assert_eq!(result.extraction_method, Some(ExtractionMethod::Native));
    assert!(arabic_letters(&result.content) > 100, "{:?}", result.content);
}
