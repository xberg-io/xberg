//! #3029: a native PDF that reaches `extraction_timeout_secs` inside its text loop must return the
//! pages the loop read within the grace period, not wait the grace out and fail with `Timeout`
//! because the stages after the loop ignore the stop.
//!
//! The first page carries [`HOLD_PAGE_LOOP_MARKER`], so the loop is still running when the limit
//! passes however fast the build reads pages; every later page is left unread.

use super::HOLD_PAGE_LOOP_MARKER;
use super::tests::build_paged_text_pdf;
use crate::XbergError;
use crate::core::config::{ChunkingConfig, ConcurrencyConfig, ExtractionConfig, PageConfig};
use crate::core::extractor::extract_bytes;
use crate::types::ExtractedDocument;
use std::time::{Duration, Instant};

const LIMIT_SECS: u64 = 2;
/// Unread pages stay in the document as empty pages, so a stage after the loop that ignores the
/// stop and reworks every page has enough of them to outlast the grace.
const PAGE_COUNT: usize = 200;
/// What the limit may overrun by: the grace is 10 s, and the stages after the text loop are skipped
/// on a stop, so a result arrives well inside it. A `Timeout` after the whole grace is the bug.
const MAX_OVERRUN: Duration = Duration::from_secs(7);

fn held_pdf() -> Vec<u8> {
    let pdf = build_paged_text_pdf(PAGE_COUNT, 10);
    let first_page_token = b"PAGEMARK0001";
    assert_eq!(
        first_page_token.len(),
        HOLD_PAGE_LOOP_MARKER.len(),
        "an equal-length swap keeps the stream lengths and xref offsets valid"
    );
    let at = pdf
        .windows(first_page_token.len())
        .position(|window| window == first_page_token)
        .expect("the first page opens with its page mark");
    let mut held = pdf;
    held[at..at + first_page_token.len()].copy_from_slice(HOLD_PAGE_LOOP_MARKER.as_bytes());
    held
}

fn timed_config() -> ExtractionConfig {
    ExtractionConfig {
        use_cache: false,
        extraction_timeout_secs: Some(LIMIT_SECS),
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

async fn extract_held(config: &ExtractionConfig) -> (ExtractedDocument, Duration) {
    let started = Instant::now();
    let outcome = extract_bytes(&held_pdf(), "application/pdf", config).await;
    let took = started.elapsed();
    match outcome {
        Ok(result) => (result, took),
        Err(XbergError::Timeout { .. }) => {
            panic!("Timeout after {took:?}: the pages read before the limit were not returned")
        }
        Err(error) => panic!("unexpected error after {took:?}: {error}"),
    }
}

fn assert_cut_short_after_the_first_page(result: &ExtractedDocument, took: Duration) {
    assert!(
        took < Duration::from_secs(LIMIT_SECS) + MAX_OVERRUN,
        "the cut-short result took {took:?}, past the limit plus {MAX_OVERRUN:?}"
    );
    assert!(
        result
            .processing_warnings
            .iter()
            .any(|warning| warning.source == "timeout"
                && warning.message.contains("page 2 is the first page that was not read")),
        "the result must name page 2 as the first unread page: {:?}",
        result.processing_warnings
    );
    let pages = result.pages.as_deref().expect("page contents were requested");
    assert_eq!(pages.len(), PAGE_COUNT, "unread pages stay in place");
    assert!(pages[0].content.contains(HOLD_PAGE_LOOP_MARKER));
    assert!(
        pages[1..].iter().all(|page| page.content.trim().is_empty()),
        "only the first page was read"
    );
    assert!(result.content.contains(HOLD_PAGE_LOOP_MARKER));
}

#[tokio::test(flavor = "multi_thread")]
async fn native_pdf_cut_short_in_the_text_loop_returns_its_pages_without_waiting_out_the_grace() {
    let (result, took) = extract_held(&timed_config()).await;

    assert_cut_short_after_the_first_page(&result, took);
}

/// The post-processing pipeline runs inside the grace too, so a processor that reworks every page
/// must not carry a cut-short result past it.
#[tokio::test(flavor = "multi_thread")]
async fn native_pdf_cut_short_with_chunking_still_returns_inside_the_grace() {
    let config = ExtractionConfig {
        chunking: Some(ChunkingConfig::default()),
        ..timed_config()
    };

    let (result, took) = extract_held(&config).await;

    assert_cut_short_after_the_first_page(&result, took);
}

/// Embedding the chunks of a cut-short document runs on the extraction's own task, so it cannot
/// be timed out once started: it must be skipped. The model is a preset that does not exist, so a
/// run that did reach the embedding stage fails at once with an `embedding` warning instead of
/// downloading a model.
#[cfg(feature = "embeddings")]
#[tokio::test(flavor = "multi_thread")]
async fn native_pdf_cut_short_skips_chunk_embeddings() {
    use crate::core::config::{EmbeddingConfig, EmbeddingModelType};

    let config = ExtractionConfig {
        chunking: Some(ChunkingConfig {
            embedding: Some(EmbeddingConfig {
                model: EmbeddingModelType::Preset {
                    name: "no-such-embedding-preset".to_string(),
                },
                ..EmbeddingConfig::default()
            }),
            ..ChunkingConfig::default()
        }),
        ..timed_config()
    };

    let (result, took) = extract_held(&config).await;

    assert_cut_short_after_the_first_page(&result, took);
    assert!(
        result
            .processing_warnings
            .iter()
            .all(|warning| warning.source != "embedding"),
        "the embedding stage ran on a cut-short result: {:?}",
        result.processing_warnings
    );
    assert!(
        result
            .processing_warnings
            .iter()
            .any(|warning| warning.source == "pdf_time_limit" && warning.message.contains("chunk embeddings")),
        "the skipped embeddings must be reported: {:?}",
        result.processing_warnings
    );
}
