//! Regression test for issue #3044: chunk embedding runs model batches on the extraction's own
//! task, so the time limit could not fire during it and a long document never returned.
//!
//! The case needs the embeddings of a 570-chunk book to outlast a limit that the text loop meets,
//! which holds only for an unoptimized build. The unit tests that request the stop on the token
//! cover every profile.

#![allow(clippy::print_stdout, clippy::print_stderr, clippy::dbg_macro)] // ~keep: test/bench binaries print by design; org logging policy exempts tests
#![cfg(all(feature = "pdf", feature = "tokio-runtime", feature = "embeddings", debug_assertions))]

mod helpers;
use helpers::{extract_uri_document, get_test_documents_dir};
use std::time::{Duration, Instant};
use xberg::core::config::{ChunkingConfig, ConcurrencyConfig, EmbeddingConfig, EmbeddingModelType, ExtractionConfig};

const FIXTURE: &str = "pdf/a_course_in_machine_learning_ciml_v0_9_all.pdf";
/// Time past the text loop that the limit leaves for embedding.
const EMBEDDING_BUDGET: Duration = Duration::from_secs(8);
/// What the limit may be overrun by: the batch of 32 chunks in flight when it passes, with room for a loaded runner.
const MAX_OVERRUN: Duration = Duration::from_secs(25);

fn embedding_config() -> EmbeddingConfig {
    EmbeddingConfig {
        model: EmbeddingModelType::Preset {
            name: "gte-modernbert-base".to_string(),
        },
        ..EmbeddingConfig::default()
    }
}

fn config(secs: u64, embed: bool) -> ExtractionConfig {
    ExtractionConfig {
        use_cache: false,
        extraction_timeout_secs: Some(secs),
        concurrency: Some(ConcurrencyConfig {
            max_threads: Some(1),
            ..Default::default()
        }),
        chunking: Some(ChunkingConfig {
            embedding: embed.then(embedding_config),
            ..ChunkingConfig::default()
        }),
        ..Default::default()
    }
}

/// Needs the `gte-modernbert-base` model in the local cache (`HF_HUB_OFFLINE=1` works).
#[tokio::test(flavor = "multi_thread")]
#[ignore = "wall-clock: needs the cached gte-modernbert-base model and embeddings that outlast the text loop plus 8 s; the unit tests are the guard"]
async fn test_chunk_embedding_that_outlasts_the_limit_returns_its_chunks_without_vectors() {
    let path = get_test_documents_dir().join(FIXTURE);
    assert!(
        path.exists(),
        "corpus fixture {path:?} is missing: this test needs the test_documents corpus"
    );

    // Loading the model session cannot be interrupted and takes tens of seconds on a loaded machine,
    // so the limit is measured against batches of a loaded model, which is what it bounds.
    xberg::embed_texts(vec!["warm up".to_string()], &embedding_config()).expect("the model must be cached locally");

    let started = Instant::now();
    extract_uri_document(&path, None, &config(600, false))
        .await
        .expect("extraction without embeddings");
    let text_time = started.elapsed();
    let limit = Duration::from_secs((text_time + EMBEDDING_BUDGET).as_secs() + 1);

    let started = Instant::now();
    let result = extract_uri_document(&path, None, &config(limit.as_secs(), true))
        .await
        .unwrap_or_else(|error| panic!("expected a result with the chunks, got {error}"));
    let took = started.elapsed();

    assert!(
        took < limit + MAX_OVERRUN,
        "the call took {took:?} for a limit of {limit:?}: the limit did not bound the embedding"
    );
    let chunks = result.chunks.as_deref().expect("the chunks are kept");
    assert!(!chunks.is_empty());
    assert!(
        chunks.iter().all(|chunk| chunk.embedding.is_none()),
        "a stopped embedding stage attaches no vector"
    );
    assert!(
        result.processing_warnings.iter().any(|warning| {
            (warning.source == "time_limit" || warning.source == "pdf_time_limit")
                && warning.message.contains("chunk embeddings")
        }),
        "the dropped stage must be reported: {:?}",
        result.processing_warnings
    );
}
