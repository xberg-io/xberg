//! Regression test for issue #3040: one page of `bayesian_data_analysis` took over 12 s in a
//! debug build, so the extraction time limit's stop never reached a page boundary inside the
//! grace. The page's cost was a content-stream tokenizer that compared the whole remaining stream
//! once per token; the page must read in a small fraction of a second.

// The page cost is only visible unoptimized: a release build read these pages in 0.05 s before the fix.
#![cfg(all(feature = "pdf", debug_assertions))]
#![allow(clippy::print_stdout, clippy::print_stderr, clippy::dbg_macro)] // ~keep: test/bench binaries print by design; org logging policy exempts tests

mod helpers;
use helpers::get_test_documents_dir;
use std::time::{Duration, Instant};
use xberg_native_pdf::PdfDocument;
use xberg_native_pdf::document::ReadingOrder;

const FIXTURE: &str = "pdf/bayesian_data_analysis_third_edition_13th_feb_2020.pdf";
/// Pages that took 8 to 13 s before the fix (0-based).
const SLOW_PAGES: [usize; 2] = [27, 28];
/// Over 300 times the cost after the fix in a debug build, and well under the 12 s it cost before.
const PAGE_BUDGET: Duration = Duration::from_secs(4);

// The page cost is only visible unoptimized: a release build read these pages in 0.05 s before the fix.
#[test]
fn test_pages_with_a_large_content_stream_read_in_linear_time() {
    let path = get_test_documents_dir().join(FIXTURE);
    assert!(
        path.exists(),
        "corpus fixture {path:?} is missing: this test needs the test_documents corpus"
    );
    let document = PdfDocument::open(&path).expect("open the fixture");
    for page in SLOW_PAGES {
        let started = Instant::now();
        let spans = document
            .extract_spans_with_reading_order(page, ReadingOrder::ColumnAware)
            .expect("read the page");
        let took = started.elapsed();
        assert!(!spans.is_empty(), "page {} read no text", page + 1);
        assert!(
            took < PAGE_BUDGET,
            "page {} took {took:?}, over {PAGE_BUDGET:?}",
            page + 1
        );
    }
}
