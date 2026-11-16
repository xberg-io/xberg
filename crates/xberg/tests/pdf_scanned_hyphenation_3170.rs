#![cfg(all(feature = "pdf", feature = "ocr", feature = "tokio-runtime"))]

mod helpers;

use helpers::{extract_uri_document_blocking, get_test_file_path};
use xberg::core::config::{ExtractionConfig, OcrConfig, OutputFormat, PageConfig};
use xberg::types::{ImagePreprocessingConfig, TesseractConfig};

#[test]
fn scanned_prose_rejoins_witnessed_line_end_hyphens() {
    for psm in [None, Some(6)] {
        assert_witnessed_wraps(psm);
    }
}

fn assert_witnessed_wraps(psm: Option<i32>) {
    let path = get_test_file_path("pdf_scanned/docling_scanned.pdf");
    assert!(path.is_file(), "fetch the corpus-pinned scan at {}", path.display());
    let config = ExtractionConfig {
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
    };
    let result = extract_uri_document_blocking(path, Some("application/pdf"), &config)
        .expect("OCR of the public image-only PDF must succeed");
    let pages = result.pages.expect("page extraction requested");
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
    assert!(page_text.contains("commercial soft- ware, cloud"));
    assert!(page_text.contains("printing-optimized"));
}
