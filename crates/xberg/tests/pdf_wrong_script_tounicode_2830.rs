#![cfg(all(feature = "pdf", feature = "ocr", feature = "bundle-tessdata-eng"))]

//! An English page whose Type0 `ToUnicode` map sends Latin letters to Arabic or Bengali code points
//! and has one lost glyph: the script rule must not keep that layer, because the page's ink is
//! Latin and the English read is the correct reader. A page whose ink is really Arabic stays native.

mod helpers;

use helpers::pdf_picture::{PicturePlacement, with_full_page_picture};

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
const GREEK_MINORITY: &[u8] = include_bytes!("fixtures/pdf/wrong-script-tounicode-2830/greek-minority.pdf");
const CYRILLIC_MINORITY: &[u8] = include_bytes!("fixtures/pdf/wrong-script-tounicode-2830/cyrillic-minority.pdf");
const ARABIC_MINORITY: &[u8] = include_bytes!("fixtures/pdf/wrong-script-tounicode-2830/arabic-minority.pdf");
const ARABIC_FOOTER: &[u8] = include_bytes!("fixtures/pdf/wrong-script-tounicode-2830/arabic-footer.pdf");
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

fn extract_without_ocr(source: &[u8]) -> ExtractedDocument {
    let config = ExtractionConfig {
        use_cache: false,
        output_format: OutputFormat::Plain,
        disable_ocr: true,
        ..Default::default()
    };
    helpers::extract_bytes_document_blocking(source, "application/pdf", &config).unwrap()
}

/// An English page whose other script is a correctly mapped minority of its words keeps every letter
/// of that script that the page holds with OCR off.
fn assert_keeps_minority(source: &[u8], range: std::ops::RangeInclusive<char>) {
    let native = letters_in(&extract_without_ocr(source).content, range.clone());
    assert!(native > 80, "the control layer holds {native} letters of the script");
    let models = models();
    let result = extract(source, models.path());
    assert_eq!(
        letters_in(&result.content, range),
        native,
        "{:?} {:?}",
        result.content,
        result.processing_warnings
    );
    assert!(result.content.contains("clerks filed the"), "{:?}", result.content);
}

#[test]
fn an_english_page_keeps_its_correctly_mapped_greek_minority() {
    assert_keeps_minority(GREEK_MINORITY, '\u{0370}'..='\u{03FF}');
}

#[test]
fn an_english_page_keeps_its_correctly_mapped_cyrillic_minority() {
    assert_keeps_minority(CYRILLIC_MINORITY, '\u{0400}'..='\u{04FF}');
}

#[test]
fn an_english_page_keeps_its_correctly_mapped_arabic_minority() {
    assert_keeps_minority(ARABIC_MINORITY, '\u{0600}'..='\u{06FF}');
}

/// The accepted trade-off of the letter-share rule: a wrong-mapped body with a correctly mapped
/// English footer (15.8% of the letters) is not probed, so the page stays native and OCR is skipped.
#[test]
fn a_wrong_mapped_page_with_a_correctly_mapped_english_footer_stays_native() {
    let models = models();
    let result = extract(ARABIC_FOOTER, models.path());
    assert!(skipped(&result), "{:?}", result.processing_warnings);
    assert_eq!(result.extraction_method, Some(ExtractionMethod::Native));
    assert!(arabic_letters(&result.content) > 500, "{:?}", result.content);
    assert!(!result.content.contains(PHRASES[0]), "{:?}", result.content);
    assert!(
        result.content.contains("Minutes approved by the board"),
        "{:?}",
        result.content
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

/// #2826: on a light picture and on a dark, blocky one that turns a read of the whole page into
/// noise, the layer's ink is read without the picture and contradicts the layer, so OCR replaces
/// it. Only the light picture leaves the page's own OCR read clean enough to hold the phrases.
#[test]
fn a_wrong_mapped_layer_drawn_over_a_full_page_picture_is_still_read_by_ocr() {
    let models = models();
    for placement in [PicturePlacement::Background, PicturePlacement::TexturedBackground] {
        let result = extract(&with_full_page_picture(ARABIC, placement), models.path());
        assert!(!skipped(&result), "{placement:?}: {:?}", result.processing_warnings);
        assert_ne!(
            result.extraction_method,
            Some(ExtractionMethod::Native),
            "{placement:?}"
        );
        assert_eq!(arabic_letters(&result.content), 0, "{placement:?}");
        if matches!(placement, PicturePlacement::Background) {
            for phrase in PHRASES {
                assert!(
                    result.content.contains(phrase),
                    "missing {phrase:?} in {:?}",
                    result.content
                );
            }
        }
    }
}

#[test]
fn a_genuine_arabic_layer_drawn_over_a_full_page_picture_stays_native() {
    let models = models();
    let result = extract(
        &with_full_page_picture(GENUINE_ARABIC, PicturePlacement::Background),
        models.path(),
    );
    assert!(skipped(&result), "{:?}", result.processing_warnings);
    assert_eq!(result.extraction_method, Some(ExtractionMethod::Native));
    assert!(arabic_letters(&result.content) > 100, "{:?}", result.content);
}

/// The English model and the installed Khmer model, or `None` when Khmer is not installed.
fn english_and_khmer_models() -> Option<tempfile::TempDir> {
    let khm = std::env::var_os("TESSDATA_PREFIX")
        .map(std::path::PathBuf::from)
        .into_iter()
        .chain(
            [
                "/opt/homebrew/share/tessdata",
                "/usr/share/tesseract-ocr/5/tessdata",
                "/usr/share/tessdata",
            ]
            .map(Into::into),
        )
        .map(|dir| dir.join("khm.traineddata"))
        .find(|path| path.is_file())?;
    let models = models();
    std::fs::copy(khm, models.path().join("khm.traineddata")).unwrap();
    Some(models)
}

/// #2826: with only Khmer configured, the layer's ink is still read with the English model, which
/// contradicts the wrong map, so the page over its picture goes to OCR as at base.
#[test]
#[ignore = "needs khm.traineddata in TESSDATA_PREFIX or a system tessdata directory"]
#[allow(clippy::print_stderr)]
fn a_wrong_mapped_layer_over_a_picture_goes_to_ocr_with_only_khmer_configured() {
    let Some(models) = english_and_khmer_models() else {
        eprintln!("skipped: khm.traineddata is not installed");
        return;
    };
    let config = ExtractionConfig {
        use_cache: false,
        output_format: OutputFormat::Plain,
        ocr: Some(OcrConfig {
            language: vec!["khm".to_string()],
            tessdata_path: Some(models.path().into()),
            ..Default::default()
        }),
        ocr_strategy: OcrStrategy::Auto,
        ..Default::default()
    };
    let source = with_full_page_picture(ARABIC, PicturePlacement::Background);
    let result = helpers::extract_bytes_document_blocking(&source, "application/pdf", &config).unwrap();
    assert!(!skipped(&result), "{:?}", result.processing_warnings);
    assert_ne!(result.extraction_method, Some(ExtractionMethod::Native));
    assert_eq!(arabic_letters(&result.content), 0, "{:?}", result.content);
}
