//! Regression tests for issue #3029: a native PDF that reaches `extraction_timeout_secs` inside
//! its text loop must return the pages the loop read within the grace period, not wait the grace
//! out and fail with `Timeout` because the stages after the loop ignore the stop.

#![allow(clippy::print_stdout, clippy::print_stderr, clippy::dbg_macro)] // ~keep: test/bench binaries print by design; org logging policy exempts tests
#![cfg(all(feature = "pdf", feature = "tokio-runtime"))]

mod helpers;
use helpers::{extract_uri_document, get_test_documents_dir};
use std::time::{Duration, Instant};
use xberg::core::config::{ConcurrencyConfig, ExtractionConfig, PageConfig};
use xberg::{ExtractedDocument, XbergError};

const LIMITS_SECS: [u64; 3] = [1, 2, 3];
/// Long enough that no machine reads them within the smallest limit.
const FIXTURES: [&str; 2] = [
    "pdf/a_course_in_machine_learning_ciml_v0_9_all.pdf",
    "pdf/fundamentals_of_deep_learning_2014.pdf",
];
/// What a limit may overrun by: the grace is 10 s, and the stages after the text loop are skipped
/// on a stop, so a result arrives well inside it. A `Timeout` after the whole grace is the bug.
const MAX_OVERRUN: Duration = Duration::from_secs(5);

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

fn has_timeout_warning(result: &ExtractedDocument) -> bool {
    result
        .processing_warnings
        .iter()
        .any(|warning| warning.source == "timeout")
}

fn pages_read(result: &ExtractedDocument) -> usize {
    result
        .pages
        .as_deref()
        .expect("page contents were requested")
        .iter()
        .filter(|page| !page.content.trim().is_empty())
        .count()
}

#[tokio::test(flavor = "multi_thread")]
async fn test_native_pdf_cut_short_in_the_text_loop_returns_its_pages_without_waiting_out_the_grace() {
    let mut cut_short = 0;
    for name in FIXTURES {
        let path = get_test_documents_dir().join(name);
        assert!(
            path.exists(),
            "corpus fixture {path:?} is missing: this test needs the test_documents corpus"
        );
        for secs in LIMITS_SECS {
            let limit = Duration::from_secs(secs);
            let started = Instant::now();
            let outcome = extract_uri_document(&path, None, &timed_config(secs)).await;
            let took = started.elapsed();
            eprintln!("{name} limit={secs}s took={took:?} ok={}", outcome.is_ok());
            match outcome {
                Ok(result) if has_timeout_warning(&result) => {
                    assert!(
                        took < limit + MAX_OVERRUN,
                        "{name} at {secs}s: the cut-short result took {took:?}, past the limit plus {MAX_OVERRUN:?}"
                    );
                    assert!(pages_read(&result) >= 1, "{name} at {secs}s: the result holds no page");
                    assert!(!result.content.trim().is_empty());
                    cut_short += 1;
                }
                Ok(_) => {}
                Err(XbergError::Timeout { .. }) => assert!(
                    took < limit + MAX_OVERRUN,
                    "{name} at {secs}s: Timeout after {took:?}, past the limit plus {MAX_OVERRUN:?}: the grace was waited out and no pages came back"
                ),
                Err(error) => panic!("{name} at {secs}s: unexpected error {error}"),
            }
        }
    }
    assert!(
        cut_short > 0,
        "no extraction returned pages cut short by the limit, so the stop was not exercised"
    );
}
