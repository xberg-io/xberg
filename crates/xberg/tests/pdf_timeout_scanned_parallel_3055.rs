//! Regression test for issue #3055: a scanned PDF read by several OCR workers must return the
//! pages it finished once `extraction_timeout_secs` passes, not wait for the batch in flight and
//! fail with `Timeout` after the grace period.
//!
//! The case needs the OCR of one batch to outlast the limit, which holds only for an unoptimized
//! build, so it is compiled out of an optimized one. `in_flight_page_tests` in the OCR pipeline
//! module cover the abandonment deterministically in every profile.

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
use std::time::{Duration, Instant};
use xberg::XbergError;
use xberg::core::config::{ConcurrencyConfig, ExtractionConfig, PageConfig};

const THREADS: usize = 4;
const LIMITS_SECS: [u64; 2] = [8, 16];
/// The grace is 10 s; a result needs the in-flight grace (3 s) plus assembly, so well inside it.
const MAX_OVERRUN: Duration = Duration::from_secs(8);

#[tokio::test(flavor = "multi_thread")]
async fn test_scanned_pdf_read_by_four_workers_returns_its_pages_inside_the_grace() {
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
                max_threads: Some(THREADS),
                ..Default::default()
            }),
            pages: Some(PageConfig {
                extract_pages: true,
                ..Default::default()
            }),
            ..Default::default()
        };
        let started = Instant::now();
        let outcome = extract_uri_document(&path, None, &config).await;
        let took = started.elapsed();
        eprintln!("limit={secs}s took={took:?} ok={}", outcome.is_ok());
        match outcome {
            Ok(result) => {
                assert!(took < Duration::from_secs(secs) + MAX_OVERRUN, "{secs}s: took {took:?}");
                let read = result
                    .pages
                    .as_deref()
                    .expect("page contents were requested")
                    .iter()
                    .filter(|page| !page.content.trim().is_empty())
                    .count();
                assert!(read >= 1, "{secs}s: the result holds no page");
                assert!(
                    result
                        .processing_warnings
                        .iter()
                        .any(|warning| warning.source == "timeout"),
                    "{secs}s: an 84-page scan cannot finish inside the limit, so the result must say it was cut short"
                );
                return;
            }
            Err(XbergError::Timeout { .. }) if took < Duration::from_secs(secs) + MAX_OVERRUN => continue,
            Err(error) => panic!("{secs}s: {error} after {took:?}: the batch in flight outlasted the grace"),
        }
    }
    panic!("no limit in {LIMITS_SECS:?} produced a cut-short result");
}
