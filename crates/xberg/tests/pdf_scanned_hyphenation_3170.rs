#![cfg(all(feature = "pdf", feature = "ocr", feature = "tokio-runtime"))]

mod helpers;

use helpers::{extract_bytes_document_blocking, extract_uri_document_blocking, get_test_file_path};
use xberg::core::config::{ExtractionConfig, OcrConfig, OutputFormat, PageConfig};
use xberg::types::{ImagePreprocessingConfig, TesseractConfig};

#[test]
fn scanned_prose_rejoins_witnessed_line_end_hyphens() {
    for psm in [None, Some(6)] {
        assert_witnessed_wraps(psm);
    }
}

#[test]
fn scanned_lexical_wrap_keeps_hyphen_despite_closed_spelling() {
    // The open-/source wrap needs the independent page witness to preserve its hyphen. ~keep
    let pdf = include_bytes!("../../../fixtures/repros/scanned-dehyphenation/lexical-wrap.pdf");
    let result = extract_bytes_document_blocking(pdf, "application/pdf", &scan_config(Some(6)))
        .expect("OCR of the synthetic image-only PDF must succeed");
    assert!(result.metadata.ocr_used);
    let pages = result.pages.expect("page extraction requested");
    assert_eq!(pages.len(), 1);
    let text = pages[0].content.split_whitespace().collect::<Vec<_>>().join(" ");
    assert!(
        text.contains("The printed paragraph ends with an open-source term and gives a clear example."),
        "the wrapped token must join with its lexical hyphen: {text}"
    );
    assert!(text.contains("The witness spells it opensource here."), "{text}");
    assert!(text.contains("A separate open-source label is valid."), "{text}");
}

fn scan_config(psm: Option<i32>) -> ExtractionConfig {
    ExtractionConfig {
        force_ocr: true,
        use_cache: false,
        output_format: OutputFormat::Plain,
        pages: Some(PageConfig {
            extract_pages: true,
            ..Default::default()
        }),
        ocr: Some(OcrConfig {
            backend: "tesseract".to_string(),
            tesseract_config: Some(TesseractConfig {
                psm,
                enable_table_detection: false,
                use_cache: false,
                preprocessing: Some(ImagePreprocessingConfig {
                    target_dpi: 96,
                    deskew: false,
                    binarization_method: "none".to_string(),
                    ..Default::default()
                }),
                ..Default::default()
            }),
            ..Default::default()
        }),
        ..Default::default()
    }
}

fn assert_witnessed_wraps(psm: Option<i32>) {
    let path = get_test_file_path("pdf_scanned/docling_scanned.pdf");
    assert!(path.is_file(), "fetch the corpus-pinned scan at {}", path.display());
    let result = extract_uri_document_blocking(path, Some("application/pdf"), &scan_config(psm))
        .expect("OCR of the public image-only PDF must succeed");
    let pages = result.pages.expect("page extraction requested");
    if psm.is_none() {
        let second_page = pages.get(1).expect("the scan's second page");
        let page_text = second_page.content.split_whitespace().collect::<Vec<_>>().join(" ");
        assert!(
            page_text.contains("locates figures and recovers table structures"),
            "join the independently witnessed last word of the list item: {page_text}"
        );
        assert!(page_text.contains("such as layout and table structures"), "{page_text}");
    }
    let first_page = pages.first().expect("the scan must have a first page");
    let page_text = first_page.content.split_whitespace().collect::<Vec<_>>().join(" ");
    assert!(
        page_text.contains("recognition we developed"),
        "join the witnessed wrap in the introduction: {page_text}"
    );
    assert!(
        page_text.contains("commodity hardware. Its code"),
        "join the witnessed wrap before the sentence boundary: {page_text}"
    );
    // This is a hard-hyphen preservation control. The abstract has no independent lexical
    // witness, so the extraction may leave the words apart. PSM 6 may insert one token from
    // the margin stamp. ~keep
    let license = page_text
        .split_once("self-contained, MIT-")
        .and_then(|(_, tail)| tail.split_once("licensed open-source package"));
    assert!(
        license.is_some_and(|(between, _)| {
            between.is_empty()
                || between == " "
                || (psm == Some(6)
                    && between.starts_with(' ')
                    && between.ends_with(' ')
                    && between.split_whitespace().count() == 1)
        }),
        "PSM {psm:?} must preserve the hard hyphen in the abstract's MIT license phrase: {page_text}"
    );
}
