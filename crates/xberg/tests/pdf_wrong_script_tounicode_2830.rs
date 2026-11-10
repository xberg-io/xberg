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
const HEBREW: &[u8] = include_bytes!("fixtures/pdf/wrong-script-tounicode-2830/hebrew.pdf");
const BILINGUAL: &[u8] = include_bytes!("fixtures/pdf/wrong-script-tounicode-2830/bilingual.pdf");
const THEN_SCANNED: &[u8] = include_bytes!("fixtures/pdf/wrong-script-tounicode-2830/arabic-then-scanned-english.pdf");
const TWO_WRONG_SCRIPTS: &[u8] = include_bytes!("fixtures/pdf/wrong-script-tounicode-2830/arabic-and-bengali.pdf");
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

fn letters_in(text: &str, range: std::ops::RangeInclusive<char>) -> usize {
    text.chars().filter(|c| range.contains(c)).count()
}

#[test]
fn a_hebrew_page_stays_native_when_ocr_cannot_read_it() {
    let models = models();
    let result = extract(HEBREW, models.path());
    assert_eq!(result.extraction_method, Some(ExtractionMethod::Native));
    assert!(
        letters_in(&result.content, '\u{05D0}'..='\u{05EA}') > 100,
        "{:?}",
        result.content
    );
}

#[test]
fn a_mostly_arabic_page_with_an_english_paragraph_keeps_its_arabic() {
    let models = models();
    let result = extract(BILINGUAL, models.path());
    assert!(
        letters_in(&result.content, '\u{0621}'..='\u{064A}') > 100,
        "{:?} {:?}",
        result.content,
        result.processing_warnings
    );
}

#[test]
fn a_contradicted_page_is_read_next_to_a_scan_page() {
    let models = models();
    let result = extract(THEN_SCANNED, models.path());
    let pages = result.pages.as_ref().unwrap();
    assert_eq!(pages.len(), 2);
    for phrase in PHRASES {
        assert!(
            pages[0].content.contains(phrase),
            "{phrase:?} in {:?}",
            pages[0].content
        );
    }
    assert!(
        pages[1].content.contains("20,000 bottles of water"),
        "{:?}",
        pages[1].content
    );
    assert_eq!(arabic_letters(&result.content), 0);
    assert!(!skipped(&result), "{:?}", result.processing_warnings);
}

#[test]
fn a_document_of_wrong_script_pages_is_read_whole() {
    let models = models();
    let result = extract(TWO_WRONG_SCRIPTS, models.path());
    let pages = result.pages.as_ref().unwrap();
    assert_eq!(pages.len(), 2);
    for page in pages {
        for phrase in PHRASES {
            assert!(page.content.contains(phrase), "{phrase:?} in {:?}", page.content);
        }
    }
    assert!(!skipped(&result), "{:?}", result.processing_warnings);
}
