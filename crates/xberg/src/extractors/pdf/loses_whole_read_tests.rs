//! Unit tests for `loses_whole_read`, the pure function behind the whole-document-read fallback
//! of the marked-page per-page retry (#2833). Each conjunct is decided here; the public-API side
//! lives in crates/xberg/tests/pdf_marked_page_retry_fallback_2833.rs.

use super::loses_whole_read;
use crate::types::OcrPageFailure;

fn failure(page: u32, recovered: bool) -> OcrPageFailure {
    OcrPageFailure {
        page,
        error: "the page raster was refused".to_owned(),
        recovered,
    }
}

/// One-based page numbers: page 2 is the second entry of the whole read's page texts.
fn pages() -> Vec<String> {
    vec!["first page text".to_owned(), "second page text".to_owned()]
}

#[test]
fn should_lose_the_whole_read_when_an_unrecovered_failure_lands_on_a_page_it_read_with_text() {
    let whole_read_pages = pages();
    assert!(loses_whole_read(Some(&whole_read_pages), &[failure(2, false)], &[]));
}

#[test]
fn should_keep_the_whole_read_when_the_whole_read_itself_failed() {
    assert!(!loses_whole_read(None, &[failure(2, false)], &[]));
}

#[test]
fn should_keep_the_whole_read_when_every_failure_was_recovered() {
    let whole_read_pages = pages();
    assert!(!loses_whole_read(Some(&whole_read_pages), &[failure(2, true)], &[]));
}

/// The `!dropped_pages.contains(&failure.page)` conjunct: redundant through the public API — a
/// page in `dropped_pages` always has native text, which recovers its failure first — but it
/// keeps the fallback away from the pages the targeted run exists to serve.
#[test]
fn should_keep_the_whole_read_when_the_failing_page_is_one_whose_script_the_whole_read_dropped() {
    let whole_read_pages = pages();
    assert!(!loses_whole_read(Some(&whole_read_pages), &[failure(2, false)], &[2]));
}

#[test]
fn should_keep_the_whole_read_when_it_read_the_failing_page_empty() {
    let whole_read_pages = vec!["   ".to_owned(), "second page text".to_owned()];
    assert!(!loses_whole_read(Some(&whole_read_pages), &[failure(1, false)], &[]));
}

#[test]
fn should_keep_the_whole_read_when_the_failing_page_is_beyond_the_whole_reads_pages() {
    let whole_read_pages = pages();
    assert!(!loses_whole_read(Some(&whole_read_pages), &[failure(3, false)], &[]));
}

#[test]
fn should_keep_the_whole_read_for_a_page_number_below_one() {
    let whole_read_pages = pages();
    assert!(!loses_whole_read(Some(&whole_read_pages), &[failure(0, false)], &[]));
}

/// Several failures at once: a single unrecovered failure on a page the whole read read with
/// text decides the fallback even when every other failure was recovered — `any` over the
/// failures decides, `all` (every failure unrecovered) does not, so this case fails if the
/// `any` is replaced with an `all`.
#[test]
fn should_lose_the_whole_read_when_one_of_several_failures_is_unrecovered_on_a_page_it_read() {
    let whole_read_pages = pages();
    assert!(
        loses_whole_read(Some(&whole_read_pages), &[failure(1, true), failure(2, false)], &[]),
        "page 2's unrecovered failure loses the whole read even though page 1's was recovered"
    );
}
