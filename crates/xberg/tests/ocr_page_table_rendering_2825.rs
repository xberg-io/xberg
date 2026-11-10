#![cfg(all(feature = "pdf", feature = "ocr", feature = "tokio-runtime"))]

mod helpers;

use xberg::{ExtractedDocument, ExtractionConfig, OcrConfig, OutputFormat, PageConfig};

fn plain_pages_config(force_ocr: bool) -> ExtractionConfig {
    ExtractionConfig {
        use_cache: false,
        force_ocr,
        output_format: OutputFormat::Plain,
        ocr: Some(OcrConfig::default()),
        pages: Some(PageConfig {
            extract_pages: true,
            ..Default::default()
        }),
        ..Default::default()
    }
}

fn extract(name: &str, force_ocr: bool) -> ExtractedDocument {
    let path = helpers::get_test_file_path(name);
    helpers::extract_uri_document_blocking(path, None, &plain_pages_config(force_ocr)).unwrap()
}

fn assert_pages_match_content(name: &str, result: &ExtractedDocument) {
    assert!(
        !result.processing_warnings.iter().any(|w| w.source == "page_boundaries"),
        "{name}: page_boundaries warning: {:?}",
        result.processing_warnings
    );
    let pages = result.pages.as_deref().expect("requested page output");
    let boundaries = result
        .metadata
        .pages
        .as_ref()
        .and_then(|structure| structure.boundaries.as_ref())
        .unwrap_or_else(|| panic!("{name}: page boundaries were dropped"));
    assert_eq!(pages.len(), boundaries.len(), "{name}");
    for (page, boundary) in pages.iter().zip(boundaries) {
        assert!(
            !page.content.trim().is_empty(),
            "{name}: page {} is empty",
            page.page_number
        );
        assert!(
            !page.content.contains("| ---") && !page.content.contains("|---"),
            "{name}: page {} still carries a markdown table",
            page.page_number
        );
        assert_eq!(
            result.content.get(boundary.byte_start..boundary.byte_end),
            Some(page.content.as_str()),
            "{name}: page {} text differs from its content range",
            page.page_number
        );
    }
}

#[test]
fn plain_ocr_page_text_matches_content_when_a_table_page_is_scanned() {
    let result = extract("pdf_scanned/nougat_002_scanned.pdf", false);
    assert!(!result.tables.is_empty(), "fixture must produce a table");
    assert_pages_match_content("nougat_002_scanned", &result);
}

#[test]
fn plain_ocr_page_text_matches_content_when_the_document_has_several_tables() {
    let result = extract("pdf_scanned/nougat_009_scanned.pdf", false);
    assert!(result.tables.len() > 1, "fixture must produce several tables");
    assert_pages_match_content("nougat_009_scanned", &result);
}

#[test]
fn plain_ocr_page_text_matches_content_when_ocr_is_forced_over_a_text_pdf() {
    let result = extract("pdf/nougat_005.pdf", true);
    assert!(!result.tables.is_empty(), "fixture must produce a table");
    assert_pages_match_content("nougat_005", &result);
}
