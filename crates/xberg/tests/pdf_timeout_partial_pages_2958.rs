//! Regression tests for issue #2958: a PDF that reaches `extraction_timeout_secs` used to
//! return `XbergError::Timeout` and discard every page already read. It must return the
//! finished pages and a warning naming the first page that was not read.

#![allow(clippy::print_stdout, clippy::print_stderr, clippy::dbg_macro)] // ~keep: test/bench binaries print by design; org logging policy exempts tests
#![cfg(all(feature = "ocr", feature = "tokio-runtime"))]

mod helpers;
use helpers::{extract_uri_document, get_test_documents_dir};
use std::path::Path;
use xberg::cancellation::CancellationToken;
use xberg::core::config::{ConcurrencyConfig, ExtractionConfig, PageConfig};
use xberg::{ExtractedDocument, XbergError};

/// Limits tried in turn. A loaded runner can read no page within the first one, which is
/// `Timeout` by design; only that outcome moves on to the next limit.
const SCAN_LIMITS_SECS: [u64; 3] = [6, 12, 24];
/// The native text loop reads about 60 pages a second in a debug build and the stages after it
/// need several times that, which must fit the 10 s grace: a document of a few hundred pages
/// with a 1 s limit stops inside the loop and still finishes in time.
const NATIVE_LIMITS_SECS: [u64; 3] = [1, 2, 3];
/// Smallest first: each is large enough that its text loop outlasts a 1 s limit on a usual
/// machine, and small enough that the stages after the loop fit the 10 s grace.
const NATIVE_FIXTURES: [&str; 2] = [
    "pdf/a_course_in_machine_learning_ciml_v0_9_all.pdf",
    "pdf/a_brief_introduction_to_neural_networks_neuronalenetze_en_zeta2_2col_dkrieselcom.pdf",
];

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

fn has_timeout_warning(result: &ExtractedDocument) -> bool {
    result
        .processing_warnings
        .iter()
        .any(|warning| warning.source == "timeout")
}

fn pages_read(result: &ExtractedDocument) -> usize {
    let pages = result.pages.as_deref().expect("page contents were requested");
    pages.iter().filter(|page| !page.content.trim().is_empty()).count()
}

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
    assert_eq!(
        first_unread,
        pages_read + 1,
        "the warning must name the page after the last page read: {}",
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

/// Cut short in the native text loop. A machine that reads a whole fixture within the 1 s
/// minimum limit moves on to the next, larger one; if every fixture finishes in time there is
/// nothing to cut short and the test says so instead of passing without having checked anything.
#[tokio::test(flavor = "multi_thread")]
async fn test_native_pdf_stopped_in_the_text_loop_keeps_the_pages_it_read() {
    for name in NATIVE_FIXTURES {
        let path = fixture(name);
        let total = page_count(&path);
        let Some(result) = extract_with_limits(&path, &NATIVE_LIMITS_SECS).await else {
            eprintln!("{name}: every limit timed out before the extraction could return, trying the next fixture");
            continue;
        };
        if !has_timeout_warning(&result) {
            eprintln!("{name}: read all {total} pages within the limit, trying a larger fixture");
            continue;
        }
        assert_cut_short(&result, total);
        return;
    }
    eprintln!("SKIPPED: no native fixture was cut short by a 1 s limit on this machine, so the stop was not exercised");
}

/// A caller's token reused for a second extraction must not carry the first one's stop.
#[tokio::test(flavor = "multi_thread")]
async fn test_a_token_reused_after_a_partial_result_reads_the_whole_document() {
    let path = fixture(NATIVE_FIXTURES[0]);
    let total = page_count(&path);
    let token = CancellationToken::new();

    let mut cut_short = None;
    for secs in NATIVE_LIMITS_SECS {
        let config = ExtractionConfig {
            cancel_token: Some(token.clone()),
            ..timed_config(secs)
        };
        match extract_uri_document(&path, None, &config).await {
            Err(XbergError::Timeout { .. }) => continue,
            other => {
                cut_short = Some(other.expect("a limited extraction returns its pages"));
                break;
            }
        }
    }
    let Some(first) = cut_short.filter(has_timeout_warning) else {
        eprintln!("SKIPPED: the first extraction was not cut short on this machine, so there is no stop to carry over");
        return;
    };
    assert!(pages_read(&first) < total);
    assert!(
        !token.is_cancelled(),
        "a partial result must leave the caller's token usable"
    );

    let config = ExtractionConfig {
        cancel_token: Some(token.clone()),
        ..timed_config(600)
    };
    let second = extract_uri_document(&path, None, &config)
        .await
        .expect("the second extraction must complete");

    assert!(!has_timeout_warning(&second), "{:?}", second.processing_warnings);
    assert!(
        pages_read(&second) > pages_read(&first),
        "the second extraction must read past the first one's stop"
    );
}
