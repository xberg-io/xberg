//! A Plain page range selects exactly its page's text in `content`, or it is omitted with a
//! `page_boundaries` warning (#2945, #2946).

mod helpers;

use helpers::{extract_uri_document_blocking, get_test_file_path};
use xberg::{ExtractedDocument, ExtractionConfig, OutputFormat, PageConfig};

fn extract(relative_path: &str, config: &ExtractionConfig) -> ExtractedDocument {
    let path = get_test_file_path(relative_path);
    assert!(
        path.exists(),
        "required public test document {}; run test_documents/scripts/fetch_corpus.py",
        path.display()
    );
    extract_uri_document_blocking(&path, None, config).expect("extraction succeeds")
}

fn plain_pages_config() -> ExtractionConfig {
    ExtractionConfig {
        use_cache: false,
        output_format: OutputFormat::Plain,
        pages: Some(PageConfig {
            extract_pages: true,
            ..Default::default()
        }),
        ..Default::default()
    }
}

fn warned(document: &ExtractedDocument) -> bool {
    document
        .processing_warnings
        .iter()
        .any(|w| w.source == "page_boundaries")
}

/// The pages whose range is kept. Every kept range selects its page's text exactly, and a page
/// without a range is allowed only with the `page_boundaries` warning.
fn pages_with_exact_ranges(document: &ExtractedDocument) -> Vec<u32> {
    let pages = document.pages.as_deref().expect("pages are extracted");
    assert!(!pages.is_empty(), "the document has pages");
    let boundaries = document
        .metadata
        .pages
        .as_ref()
        .and_then(|structure| structure.boundaries.as_deref())
        .unwrap_or(&[]);
    for boundary in boundaries {
        let page = pages
            .iter()
            .find(|page| page.page_number == boundary.page_number)
            .expect("a range belongs to an extracted page");
        assert_eq!(
            document.content.get(boundary.byte_start..boundary.byte_end),
            Some(page.content.as_str()),
            "the range of page {} selects a different text than the page text",
            boundary.page_number
        );
    }
    let ranged: Vec<u32> = boundaries.iter().map(|boundary| boundary.page_number).collect();
    if pages.iter().any(|page| !ranged.contains(&page.page_number)) {
        assert!(
            warned(document),
            "a page lost its range without a page_boundaries warning"
        );
    }
    ranged
}

#[cfg(all(feature = "pdf", feature = "ocr"))]
#[test]
fn every_native_page_keeps_a_range_that_selects_its_text() {
    let config = ExtractionConfig {
        ocr: Some(xberg::OcrConfig::default()),
        ..plain_pages_config()
    };
    let document = extract("pdf/regression_pages/irs-fw9--2024-p1.pdf", &config);
    let ranged = pages_with_exact_ranges(&document);
    let pages: Vec<u32> = document
        .pages
        .as_deref()
        .unwrap()
        .iter()
        .map(|p| p.page_number)
        .collect();
    assert_eq!(ranged, pages, "every native page keeps its range");
    assert!(!warned(&document), "no page_boundaries warning on a native page");
}

#[cfg(feature = "office")]
#[test]
fn a_docx_page_range_selects_its_page_text_or_is_omitted_with_a_warning() {
    let document = extract("docx/budget_fy2027_memo.docx", &plain_pages_config());
    pages_with_exact_ranges(&document);
}
