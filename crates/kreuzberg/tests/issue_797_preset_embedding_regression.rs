//! Regression tests for issue #797: chunking preset must not auto-inject embeddings.
//!
//! When a `ChunkingConfig` carries a `preset` but no explicit `embedding`, the
//! extraction pipeline must leave every `chunk.embedding` as `None`.  Before
//! the fix, `resolve_preset()` silently injected an `EmbeddingConfig`, causing
//! `generate_embeddings_for_chunks()` to run and populate embeddings that the
//! caller never requested.

#[cfg(feature = "chunking")]
mod preset_no_embedding {
    use kreuzberg::core::config::{ChunkingConfig, ExtractionConfig};
    use kreuzberg::core::extractor::extract_bytes;

    /// A preset with no explicit `EmbeddingConfig` must not produce chunk embeddings.
    #[tokio::test]
    async fn test_preset_without_embedding_config_produces_no_embeddings() {
        let config = ExtractionConfig {
            chunking: Some(ChunkingConfig {
                preset: Some("multilingual".to_string()),
                embedding: None,
                ..Default::default()
            }),
            ..Default::default()
        };

        // Small plain-text document — forces at least one chunk even with large defaults.
        let text = b"Hello world. This is a short document used to verify that preset-based \
                     chunking does not unexpectedly generate embeddings.";

        let result = extract_bytes(text, "text/plain", &config)
            .await
            .expect("extraction should succeed");

        let chunks = result
            .chunks
            .expect("chunks should be produced when chunking is configured");

        for (i, chunk) in chunks.iter().enumerate() {
            assert!(
                chunk.embedding.is_none(),
                "chunk[{i}] should have no embedding when no EmbeddingConfig was supplied (#797)"
            );
        }
    }

    /// Regression guard: no preset, no embedding — chunks must still have no embeddings.
    #[tokio::test]
    async fn test_no_preset_no_embedding_produces_no_embeddings() {
        let config = ExtractionConfig {
            chunking: Some(ChunkingConfig {
                max_characters: 50,
                overlap: 10,
                preset: None,
                embedding: None,
                ..Default::default()
            }),
            ..Default::default()
        };

        let text = b"Short text that will be chunked without any embedding configuration.";

        let result = extract_bytes(text, "text/plain", &config)
            .await
            .expect("extraction should succeed");

        let chunks = result.chunks.expect("chunks should be produced");

        for (i, chunk) in chunks.iter().enumerate() {
            assert!(
                chunk.embedding.is_none(),
                "chunk[{i}] must have no embedding when embedding is not configured"
            );
        }
    }
}
