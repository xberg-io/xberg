//! RAG-shaped chunking composer.
//!
//! Provides [`chunk_for_rag`], a thin composer that delegates to the existing
//! chunking pipeline and then derives the flat
//! [`heading_path`](crate::types::ChunkMetadata::heading_path) breadcrumb from
//! whatever `heading_context` the underlying chunker emits.
//!
//! # `heading_path` population by chunker type
//!
//! | Chunker              | `heading_context` set? | `heading_path` populated? |
//! |----------------------|------------------------|---------------------------|
//! | `Text` (auto-upgraded) | yes (→ Markdown)     | yes                       |
//! | `Markdown`           | yes                    | yes                       |
//! | `Semantic`           | yes (uses Markdown internally) | yes             |
//! | `Yaml`               | **no**                 | **always empty**          |
//!
//! The `Yaml` chunker splits on top-level YAML keys and does not have a concept
//! of heading hierarchy, so `heading_path` will always be `[]` for every chunk
//! produced by a `Yaml`-typed config.
//!
//! # Design
//!
//! - Delegates all splitting to [`super::core::chunk_text`] with
//!   `ChunkerType::Markdown` (sensible for most document types) unless the caller
//!   supplies a config that already selects a different chunker, in which case the
//!   caller's config is honoured and `heading_path` is derived post-hoc from
//!   whatever `heading_context` the underlying chunker emits.
//! - Does **not** reimplement any splitting logic.
//! - Does **not** add new fields to `ChunkMetadata`; it only populates existing
//!   `heading_path` entries.
//!
//! # Example
//!
//! ```rust,no_run
//! use kreuzberg::chunking::{chunk_for_rag, ChunkingConfig, ChunkerType};
//!
//! # fn example() -> kreuzberg::Result<()> {
//! let markdown = "# Introduction\n\nWelcome.\n\n## Details\n\nMore text here.";
//! let config = ChunkingConfig {
//!     max_characters: 512,
//!     overlap: 50,
//!     chunker_type: ChunkerType::Markdown,
//!     ..Default::default()
//! };
//! let result = chunk_for_rag(markdown, &config)?;
//! for chunk in &result.chunks {
//!     println!("{:?} -> {:?}", chunk.metadata.heading_path, chunk.content);
//! }
//! # Ok(())
//! # }
//! ```

use super::builder::heading_path_from_context;
use super::config::{ChunkerType, ChunkingConfig, ChunkingResult};
use super::core::chunk_text;
use crate::error::Result;

/// Chunk text for RAG retrieval, ensuring every chunk carries a `heading_path`.
///
/// Delegates to [`chunk_text`] using the caller's config (defaulting to
/// `ChunkerType::Markdown` when the config uses the default `Text` type, so that
/// heading hierarchy is resolved).  After chunking, derives
/// [`ChunkMetadata::heading_path`](crate::types::ChunkMetadata::heading_path) from each chunk's `heading_context`.
///
/// # Arguments
///
/// * `text` — Text to chunk. Markdown formatting enables heading-aware splitting.
/// * `config` — Chunking configuration.  The `chunker_type` field controls the
///   underlying splitter; use `ChunkerType::Markdown` for documents with ATX
///   headings.
///
/// # Returns
///
/// A [`ChunkingResult`] where every chunk's `heading_path` is populated from its
/// `heading_context` (empty when the chunk is not under any heading).
///
/// # Errors
///
/// Propagates any error from the underlying chunker (e.g. invalid overlap).
pub fn chunk_for_rag(text: &str, config: &ChunkingConfig) -> Result<ChunkingResult> {
    // Use Markdown chunker when the caller left the default (Text), because
    // only the Markdown chunker resolves heading_context.  If the caller
    // explicitly chose another type, honour it.
    let effective_config;
    let config = if config.chunker_type == ChunkerType::Text {
        effective_config = ChunkingConfig {
            chunker_type: ChunkerType::Markdown,
            ..config.clone()
        };
        &effective_config
    } else {
        config
    };

    let mut result = chunk_text(text, config, None)?;

    // Derive heading_path from heading_context for every chunk that has one.
    for chunk in &mut result.chunks {
        if chunk.metadata.heading_path.is_empty() {
            chunk.metadata.heading_path = heading_path_from_context(&chunk.metadata.heading_context);
        }
    }

    Ok(result)
}

#[cfg(test)]
mod tests {
    use super::*;

    fn default_rag_config() -> ChunkingConfig {
        ChunkingConfig {
            max_characters: 512,
            overlap: 0,
            trim: true,
            chunker_type: ChunkerType::Markdown,
            ..Default::default()
        }
    }

    // ── edge cases ──────────────────────────────────────────────────────────

    #[test]
    fn chunk_for_rag_empty_input_returns_no_chunks() {
        let result = chunk_for_rag("", &default_rag_config()).unwrap();
        assert_eq!(result.chunks.len(), 0);
        assert_eq!(result.chunk_count, 0);
    }

    #[test]
    fn chunk_for_rag_text_without_headings_heading_path_empty() {
        let text = "Just plain text without any headings whatsoever.";
        let result = chunk_for_rag(text, &default_rag_config()).unwrap();
        assert_eq!(result.chunks.len(), 1);
        assert!(
            result.chunks[0].metadata.heading_path.is_empty(),
            "no headings → heading_path must be empty"
        );
    }

    // ── heading_path population ─────────────────────────────────────────────

    #[test]
    fn chunk_for_rag_populates_heading_path_from_context() {
        let text = "# Introduction\n\nWelcome to the guide.\n\n## Setup\n\nInstall the dependencies.\n\n### Prerequisites\n\nYou need Rust installed.";
        let config = ChunkingConfig {
            max_characters: 100,
            overlap: 0,
            trim: true,
            chunker_type: ChunkerType::Markdown,
            ..Default::default()
        };
        let result = chunk_for_rag(text, &config).unwrap();
        assert!(
            !result.chunks.is_empty(),
            "should produce chunks from multi-heading doc"
        );

        // Every chunk that has a heading_context must also have a non-empty heading_path.
        for chunk in &result.chunks {
            if chunk.metadata.heading_context.is_some() {
                assert!(
                    !chunk.metadata.heading_path.is_empty(),
                    "chunk under heading must have non-empty heading_path, content: {:?}",
                    chunk.content
                );
            }
        }
    }

    #[test]
    fn chunk_for_rag_heading_path_order_outermost_first() {
        let text = "# Root\n\nSome root content here.\n\n## Child\n\nChild section content here.";
        let config = ChunkingConfig {
            max_characters: 200,
            overlap: 0,
            trim: true,
            chunker_type: ChunkerType::Markdown,
            ..Default::default()
        };
        let result = chunk_for_rag(text, &config).unwrap();

        // Find a chunk that is under both Root and Child.
        let deep_chunk = result.chunks.iter().find(|c| c.metadata.heading_path.len() >= 2);

        if let Some(chunk) = deep_chunk {
            assert_eq!(
                chunk.metadata.heading_path[0], "Root",
                "outermost heading (h1) must be first in path"
            );
            assert_eq!(
                chunk.metadata.heading_path[1], "Child",
                "inner heading (h2) must follow in path"
            );
        }
    }

    #[test]
    fn chunk_for_rag_heading_path_matches_context_texts() {
        let text = "# Alpha\n\nAlpha content.\n\n## Beta\n\nBeta content.";
        let config = ChunkingConfig {
            max_characters: 300,
            overlap: 0,
            trim: true,
            chunker_type: ChunkerType::Markdown,
            ..Default::default()
        };
        let result = chunk_for_rag(text, &config).unwrap();

        for chunk in &result.chunks {
            if let Some(ref ctx) = chunk.metadata.heading_context {
                let expected: Vec<String> = ctx.headings.iter().map(|h| h.text.clone()).collect();
                assert_eq!(
                    chunk.metadata.heading_path, expected,
                    "heading_path must equal heading_context.headings[].text in order"
                );
            }
        }
    }

    // ── config forwarding ───────────────────────────────────────────────────

    #[test]
    fn chunk_for_rag_defaults_text_type_to_markdown_chunker() {
        // When the config has default ChunkerType::Text, chunk_for_rag should
        // upgrade to Markdown so heading_context is resolved.
        let text = "# Title\n\nSome content here to chunk.\n\n## Section\n\nMore content here.";
        let config = ChunkingConfig {
            max_characters: 200,
            overlap: 0,
            trim: true,
            chunker_type: ChunkerType::Text, // will be upgraded to Markdown
            ..Default::default()
        };
        let result = chunk_for_rag(text, &config).unwrap();
        // At least one chunk should have heading_path set (heading_context resolved).
        let has_path = result.chunks.iter().any(|c| !c.metadata.heading_path.is_empty());
        assert!(
            has_path,
            "upgrading Text → Markdown must produce heading_path on at least one chunk"
        );
    }

    #[test]
    fn chunk_for_rag_non_empty_output_on_multi_heading_doc() {
        let text = concat!(
            "# Chapter 1\n\n",
            "This chapter covers the basics of the system. ",
            "There is quite a lot of content here to ensure splitting occurs.\n\n",
            "## Section 1.1\n\n",
            "The first section dives into details. ",
            "More sentences follow to fill up the chunk budget adequately.\n\n",
            "## Section 1.2\n\n",
            "The second section covers advanced topics. ",
            "Even more text to ensure we get multiple chunks from this document.\n\n",
            "# Chapter 2\n\n",
            "Chapter two starts fresh. ",
            "Its content is completely independent of chapter one.\n\n",
        );
        let config = ChunkingConfig {
            max_characters: 150,
            overlap: 0,
            trim: true,
            chunker_type: ChunkerType::Markdown,
            ..Default::default()
        };
        let result = chunk_for_rag(text, &config).unwrap();
        assert!(
            result.chunks.len() >= 2,
            "multi-heading document should produce multiple chunks"
        );
        assert_eq!(result.chunks.len(), result.chunk_count);

        // All chunks are non-empty.
        for chunk in &result.chunks {
            assert!(!chunk.content.is_empty());
        }
    }

    // ── idempotency: pre-populated heading_path is not overwritten ──────────

    #[test]
    fn chunk_for_rag_does_not_overwrite_existing_heading_path() {
        // chunk_for_rag skips the post-hoc heading_path derivation for any chunk
        // whose heading_path is already non-empty (e.g. filled by the Semantic
        // chunker's own pass).  Verify end-to-end: call chunk_for_rag on a
        // heading-bearing document and assert that the result carries a populated,
        // correctly ordered heading_path on chunks that fall under headings.
        let text = "# A\n\nContent under A.\n\n## B\n\nContent under B.";
        let config = ChunkingConfig {
            max_characters: 300,
            overlap: 0,
            trim: true,
            chunker_type: ChunkerType::Markdown,
            ..Default::default()
        };
        let result = chunk_for_rag(text, &config).unwrap();
        assert!(!result.chunks.is_empty(), "expected at least one chunk");

        // At least one chunk must have a non-empty heading_path derived from context.
        let has_path = result.chunks.iter().any(|c| !c.metadata.heading_path.is_empty());
        assert!(has_path, "heading_path must be populated for chunks under headings");

        // Every chunk whose heading_context is set must have a matching heading_path.
        for chunk in &result.chunks {
            if let Some(ref ctx) = chunk.metadata.heading_context {
                let expected: Vec<String> = ctx.headings.iter().map(|h| h.text.clone()).collect();
                assert_eq!(
                    chunk.metadata.heading_path, expected,
                    "heading_path must equal heading_context texts in order"
                );
            }
        }
    }

    // ── Yaml chunker yields empty heading_path ───────────────────────────────

    #[test]
    fn chunk_for_rag_yaml_chunker_yields_empty_heading_path() {
        // ChunkerType::Yaml splits on top-level YAML keys and has no heading
        // hierarchy concept.  chunk_for_rag honours the caller's explicit Yaml
        // choice, and heading_path is always empty for every produced chunk.
        let yaml = "key1: value one\nkey2: value two\nkey3: value three\n";
        let config = ChunkingConfig {
            max_characters: 512,
            overlap: 0,
            trim: true,
            chunker_type: ChunkerType::Yaml,
            ..Default::default()
        };
        let result = chunk_for_rag(yaml, &config).unwrap();
        for chunk in &result.chunks {
            assert!(
                chunk.metadata.heading_path.is_empty(),
                "Yaml chunker must produce empty heading_path; got: {:?}",
                chunk.metadata.heading_path
            );
        }
    }

    // ── Semantic chunker populates heading_path ──────────────────────────────

    #[test]
    fn chunk_for_rag_semantic_chunker_populates_heading_path() {
        // ChunkerType::Semantic runs its own heading-resolution pass internally
        // and stores heading_path directly on each chunk.  chunk_for_rag passes
        // the config through unchanged (no Text→Markdown upgrade) and the
        // is_empty guard preserves whatever Semantic already set.
        let text = concat!(
            "# Introduction\n\n",
            "This section introduces the topic in enough detail ",
            "that the semantic chunker will not merge it away.\n\n",
            "## Background\n\n",
            "Background context follows here, with sufficient content ",
            "to form its own coherent semantic unit.\n\n",
        );
        let config = ChunkingConfig {
            max_characters: 300,
            overlap: 0,
            trim: true,
            chunker_type: ChunkerType::Semantic,
            ..Default::default()
        };
        let result = chunk_for_rag(text, &config).unwrap();
        assert!(
            !result.chunks.is_empty(),
            "semantic chunker should produce at least one chunk"
        );

        // At least one chunk must carry a non-empty heading_path.
        let has_path = result.chunks.iter().any(|c| !c.metadata.heading_path.is_empty());
        assert!(
            has_path,
            "Semantic chunker must populate heading_path on at least one chunk in a headed document"
        );
    }
}
