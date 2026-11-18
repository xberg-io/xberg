//! Regression tests for issue #2958: a PDF that reaches `extraction_timeout_secs` used to
//! return `XbergError::Timeout` and discard every page already read. It must return the
//! finished pages and a warning naming the first page that was not read.
//!
//! The case here cuts a scanned PDF short in its OCR page loop. It needs that loop to outlast a
//! 6-24 s limit, which holds only for an unoptimized build: an optimized build finishes the fixture
//! first and would fail it, so it is compiled out of such a build.
//!
//! The native text loop is covered deterministically in every profile, not here: the
//! `pdf::native::text::time_limit_tests` unit tests hold the loop on a marked page until the time
//! limit stops it, then check through `extract_bytes` that the pages read are kept and the first
//! unread page is named. A corpus fixture cut short by a 1 s limit depended on the runner's speed.

#![cfg(all(
    feature = "pdf",
    feature = "ocr",
    feature = "bundle-tessdata-eng",
    feature = "tokio-runtime",
    debug_assertions
))]
#![allow(clippy::print_stdout, clippy::print_stderr, clippy::dbg_macro)] // ~keep: test/bench binaries print by design; org logging policy exempts tests

mod helpers;
use helpers::{extract_uri_document, get_test_documents_dir};
use std::path::Path;
use xberg::core::config::{ConcurrencyConfig, ExtractionConfig, PageConfig};
use xberg::{ExtractedDocument, XbergError};

/// Limits tried in turn. A loaded runner can read no page within the first one, which is
/// `Timeout` by design; only that outcome moves on to the next limit.
const SCAN_LIMITS_SECS: [u64; 3] = [6, 12, 24];

fn fixture(relative: &str) -> std::path::PathBuf {
    let path = get_test_documents_dir().join(relative);
    assert!(
        path.exists(),
        "corpus fixture {path:?} is missing: this test needs the test_documents corpus"
    );
    path
}

fn page_count(path: &Path) -> usize {
    let bytes = std::fs::read(path).expect("fixture must be readable");
    xberg_native_pdf::PdfDocument::from_bytes(bytes)
        .expect("fixture must open")
        .page_count()
        .expect("fixture must report its page count")
}

fn first_unread_page(message: &str) -> Option<usize> {
    let rest = message.split("page ").nth(1)?;
    rest.split(|c: char| !c.is_ascii_digit()).next()?.parse().ok()
}

fn timed_config(secs: u64) -> ExtractionConfig {
    ExtractionConfig {
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
    }
}

/// The first result obtained under `limits`, or `None` when every limit timed out.
async fn extract_with_limits(path: &Path, limits: &[u64]) -> Option<ExtractedDocument> {
    for &secs in limits {
        match extract_uri_document(path, None, &timed_config(secs)).await {
            Err(XbergError::Timeout { .. }) => continue,
            other => {
                return Some(
                    other.expect(
                        "a document that reaches the time limit must return the pages it finished, not an error",
                    ),
                );
            }
        }
    }
    None
}

/// Every page of the fixture carries text. Pages are read concurrently, so a page after the named
/// one may already be finished; the warning only promises that every page before it was read and
/// that it was not.
fn assert_cut_short(result: &ExtractedDocument, total: usize) {
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
    let pages_read = pages.iter().filter(|page| !page.content.trim().is_empty()).count();
    assert!(
        pages_read >= 1,
        "at least one page must have been read before the limit"
    );
    assert!(
        pages_read < total,
        "the document must not have finished: the limit exists to cut it short"
    );
    assert!(
        (2..=total).contains(&first_unread),
        "the named page must follow a read page: {}",
        warning.message
    );
    let read = &pages[..first_unread - 1];
    assert!(
        pages[first_unread - 1].content.trim().is_empty(),
        "the named page must not have been read: {}",
        warning.message
    );
    assert!(
        read.iter().all(|page| !page.content.trim().is_empty()),
        "every page before the named page {first_unread} must have been read: {}",
        warning.message
    );
    assert!(!result.content.trim().is_empty());
}

#[tokio::test(flavor = "multi_thread")]
async fn test_scanned_pdf_at_the_time_limit_returns_finished_pages_and_names_the_first_unread() {
    let path = fixture("pdf_scanned/nougat_008_scanned.pdf");
    let total = page_count(&path);

    let result = extract_with_limits(&path, &SCAN_LIMITS_SECS)
        .await
        .unwrap_or_else(|| panic!("no page was read within any of the limits tried: {SCAN_LIMITS_SECS:?}"));

    assert_cut_short(&result, total);
}
