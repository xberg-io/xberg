//! Integration tests verifying that chunk `first_page`/`last_page` are populated
//! for all chunks when extracting single-page and multi-page PDFs with chunking enabled.
//!
//! Fixtures required (relative to `test_documents/`):
//! - `pdf/multi_page.pdf`  — any native-text PDF with ≥ 2 pages
//! - `pdf/searchable.pdf` — a single-page PDF with native text

#![allow(clippy::print_stdout, clippy::print_stderr, clippy::dbg_macro)] // ~keep: test/bench binaries print by design; org logging policy exempts tests
#![cfg(all(feature = "pdf", feature = "chunking"))]

mod helpers;
use helpers::extract_uri_document_blocking;

use helpers::*;
use xberg::core::config::{ChunkingConfig, ExtractionConfig};

/// All chunks produced from a multi-page PDF must have non-null page metadata.
///
/// Verifies that `recompute_boundaries_from_pages` successfully locates every
/// page's content inside `result.content` so the chunker receives valid byte
/// offsets and can populate `first_page`/`last_page` on every chunk.
#[test]
fn chunks_from_multi_page_pdf_all_have_page_metadata() {
    if skip_if_missing("pdf/multi_page.pdf") {
        eprintln!("skipping: fixture pdf/multi_page.pdf not found");
        return;
    }

    let config = ExtractionConfig {
        chunking: Some(ChunkingConfig {
            max_characters: 500,
            overlap: 50,
            ..Default::default()
        }),
        ..Default::default()
    };

    let result = extract_uri_document_blocking(get_test_file_path("pdf/multi_page.pdf"), None, &config)
        .expect("multi_page.pdf extraction should succeed");

    let chunks = result.chunks.expect("chunking was configured — chunks must be present");

    assert!(!chunks.is_empty(), "multi-page PDF should produce at least one chunk");

    let null_page_chunks: Vec<_> = chunks
        .iter()
        .filter(|c| c.metadata.first_page.is_none() || c.metadata.last_page.is_none())
        .collect();

    assert!(
        null_page_chunks.is_empty(),
        "{} of {} chunks have null page metadata (first_page or last_page is None). \
         Chunk indices with null metadata: {:?}",
        null_page_chunks.len(),
        chunks.len(),
        null_page_chunks
            .iter()
            .map(|c| c.metadata.chunk_index)
            .collect::<Vec<_>>()
    );
}

/// Chunks from a multi-page PDF must have monotonically non-decreasing page numbers.
///
/// Verifies that page boundaries are contiguous and in order — a secondary property
/// that would be violated if `recompute_boundaries_from_pages` miscalculated
/// `search_offset` for any page.
#[test]
fn chunks_from_multi_page_pdf_have_monotonic_page_numbers() {
    if skip_if_missing("pdf/multi_page.pdf") {
        eprintln!("skipping: fixture pdf/multi_page.pdf not found");
        return;
    }

    let config = ExtractionConfig {
        chunking: Some(ChunkingConfig {
            max_characters: 500,
            overlap: 50,
            ..Default::default()
        }),
        ..Default::default()
    };

    let result = extract_uri_document_blocking(get_test_file_path("pdf/multi_page.pdf"), None, &config)
        .expect("multi_page.pdf extraction should succeed");

    let chunks = result.chunks.expect("chunking was configured — chunks must be present");

    assert!(
        chunks.iter().all(|c| c.metadata.first_page.is_some()),
        "all chunks must have first_page before checking order"
    );

    let mut prev_first_page = 0u32;
    for chunk in &chunks {
        if let Some(first) = chunk.metadata.first_page {
            assert!(
                first >= prev_first_page,
                "chunk {} first_page ({}) must be >= previous first_page ({})",
                chunk.metadata.chunk_index,
                first,
                prev_first_page
            );
            prev_first_page = first;
        }
    }
}

/// Single-page PDFs must produce `chunks: Some([...])`, never `chunks: null`.
///
/// Regression test for issue #1105 (Problem 2): a one-page PDF with extracted text
/// returned `chunks: null` instead of a populated chunk list when chunking was
/// configured. An empty recomputed boundary list could shadow the metadata fallback.
#[test]
fn chunks_from_single_page_pdf_are_not_null() {
    const FIXTURE: &str = "pdf/searchable.pdf";
    if skip_if_missing(FIXTURE) {
        return;
    }

    let config = ExtractionConfig {
        chunking: Some(ChunkingConfig {
            max_characters: 500,
            overlap: 50,
            ..Default::default()
        }),
        ..Default::default()
    };

    let result = extract_uri_document_blocking(get_test_file_path(FIXTURE), None, &config)
        .expect("single-page fixture extraction should succeed");

    let chunks = result
        .chunks
        .expect("chunks must be Some([...]) for a single-page PDF with chunking configured, not null");

    assert!(
        !chunks.is_empty(),
        "single-page PDF with extracted text must produce at least one chunk"
    );

    for chunk in &chunks {
        assert_eq!(
            chunk.metadata.first_page,
            Some(1),
            "all chunks from a single-page PDF must have first_page = Some(1)"
        );
        assert_eq!(
            chunk.metadata.last_page,
            Some(1),
            "all chunks from a single-page PDF must have last_page = Some(1)"
        );
    }
}
