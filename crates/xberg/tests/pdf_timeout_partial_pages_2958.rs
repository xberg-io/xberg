//! Regression test for issue #2958: a scanned document that reaches
//! `extraction_timeout_secs` used to return `XbergError::Timeout` and discard every page
//! already read. It must return the finished pages and a warning naming the first page
//! that was not read.

#![allow(clippy::print_stdout, clippy::print_stderr, clippy::dbg_macro)] // ~keep: test/bench binaries print by design; org logging policy exempts tests
#![cfg(all(feature = "ocr", feature = "tokio-runtime"))]

mod helpers;
use helpers::{extract_uri_document, get_test_documents_dir, test_documents_available};
use xberg::XbergError;
use xberg::core::config::{ConcurrencyConfig, ExtractionConfig, PageConfig};

/// Limits tried in turn. A loaded runner can read no page within the first one, which is
/// `Timeout` by design; only that outcome moves on to the next limit.
const TIMEOUT_SECS: [u64; 3] = [6, 12, 24];

fn first_unread_page(message: &str) -> Option<u32> {
    let rest = message.split("page ").nth(1)?;
    rest.split(|c: char| !c.is_ascii_digit()).next()?.parse().ok()
}

#[tokio::test(flavor = "multi_thread")]
async fn test_scanned_pdf_at_the_time_limit_returns_finished_pages_and_names_the_first_unread() {
    if !test_documents_available() {
        eprintln!("test_documents not available, skipping");
        return;
    }
    let path = get_test_documents_dir().join("pdf_scanned/nougat_008_scanned.pdf");
    if !path.exists() {
        eprintln!("fixture {path:?} not found (submodule not updated?), skipping");
        return;
    }

    let mut outcome = None;
    for secs in TIMEOUT_SECS {
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
        match extract_uri_document(&path, None, &config).await {
            Err(XbergError::Timeout { .. }) => continue,
            other => {
                outcome = Some(other);
                break;
            }
        }
    }
    let result = outcome
        .expect("no page was read within any of the limits tried")
        .expect("a document that reaches the time limit must return the pages it finished, not an error");

    let warning = result
        .processing_warnings
        .iter()
        .find(|warning| warning.source == "timeout")
        .unwrap_or_else(|| {
            panic!(
                "a truncated result must carry a timeout warning: {:?}",
                result.processing_warnings
            )
        });
    let first_unread = first_unread_page(&warning.message)
        .unwrap_or_else(|| panic!("the warning must name the first unread page: {}", warning.message));

    let pages = result.pages.as_deref().expect("page contents were requested");
    let pages_read = pages.iter().filter(|page| !page.content.trim().is_empty()).count() as u32;
    assert!(
        pages_read >= 1,
        "at least one page must have been read before the limit"
    );
    assert!(
        pages_read < 84,
        "the document must not have finished: the limit exists to cut it short"
    );
    assert_eq!(
        first_unread,
        pages_read + 1,
        "the warning must name the page after the last page read: {}",
        warning.message
    );
    assert!(!result.content.trim().is_empty());
}
