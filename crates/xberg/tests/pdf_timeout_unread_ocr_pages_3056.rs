//! Regression test for issue #3056: pages a time-limit stop leaves unread on the OCR path came
//! back flagged blank, so a caller could not tell them from blank pages.
//!
//! The case needs the OCR loop to outlast the limit, which holds only for an unoptimized build, so
//! it is compiled out of an optimized one; `unread_page_blank_flag_tests` cover every profile.

#![allow(clippy::print_stdout, clippy::print_stderr, clippy::dbg_macro)] // ~keep: test/bench binaries print by design; org logging policy exempts tests
#![cfg(all(
    feature = "pdf",
    feature = "ocr",
    feature = "bundle-tessdata-eng",
    feature = "tokio-runtime",
    debug_assertions
))]

mod helpers;
use helpers::{extract_uri_document, get_test_documents_dir};
use xberg::XbergError;
use xberg::core::config::{ConcurrencyConfig, ExtractionConfig, PageConfig};

const LIMITS_SECS: [u64; 3] = [6, 12, 24];

#[tokio::test(flavor = "multi_thread")]
async fn test_pages_a_stop_left_unread_are_not_flagged_blank() {
    let path = get_test_documents_dir().join("pdf_scanned/nougat_008_scanned.pdf");
    assert!(
        path.exists(),
        "corpus fixture {path:?} is missing: this test needs the test_documents corpus"
    );

    for secs in LIMITS_SECS {
        let config = ExtractionConfig {
            use_cache: false,
            extraction_timeout_secs: Some(secs),
            concurrency: Some(ConcurrencyConfig {
                max_threads: Some(1),
                ..Default::default()
            }),
            pages: Some(PageConfig {
                extract_pages: true,
                ..Default::default()
            }),
            ..Default::default()
        };
        let result = match extract_uri_document(&path, None, &config).await {
            Err(XbergError::Timeout { .. }) => continue,
            other => other.expect("a limited scan returns the pages it read"),
        };
        if !result
            .processing_warnings
            .iter()
            .any(|warning| warning.source == "timeout")
        {
            continue;
        }

        let unread: Vec<u32> = result
            .ocr_page_failures
            .iter()
            .flatten()
            .filter(|failure| failure.error.contains("time limit"))
            .map(|failure| failure.page)
            .collect();
        assert!(
            !unread.is_empty(),
            "{secs}s: a cut-short scan lists the pages it did not read"
        );

        let pages = result.pages.as_deref().expect("page contents were requested");
        for page in pages.iter().filter(|page| unread.contains(&page.page_number)) {
            assert_eq!(
                page.is_blank, None,
                "{secs}s: unread page {} was flagged",
                page.page_number
            );
        }
        let structure_pages = result
            .metadata
            .pages
            .as_ref()
            .and_then(|structure| structure.pages.as_ref())
            .expect("page structure");
        for info in structure_pages.iter().filter(|info| unread.contains(&info.number)) {
            assert_eq!(
                info.is_blank, None,
                "{secs}s: unread page {} was flagged in the page structure",
                info.number
            );
        }
        return;
    }
    panic!("no limit in {LIMITS_SECS:?} produced a cut-short scan");
}
