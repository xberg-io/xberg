//! Semantic text chunking.
//!
//! Splits text into fine-grained segments, detects topic boundaries (optionally
//! using embeddings), and merges segments into coherent chunks.

pub mod merge;

#[cfg(feature = "embeddings")]
pub mod topic;

use crate::chunking::boundaries::calculate_page_range;
use crate::chunking::boundary_detection::detect_plain_text_boundaries;
use crate::chunking::classifier::classify_chunk;
use crate::chunking::config::{ChunkingConfig, ChunkingResult};
use crate::chunking::headings::{build_heading_map, resolve_heading_context};
use crate::error::Result;
use crate::types::{Chunk, ChunkMetadata, PageBoundary};
use merge::Segment;
use text_splitter::{MarkdownSplitter, TextSplitter};

/// Default segment size (characters) for the initial fine-grained split.
const SEGMENT_SIZE: usize = 200;

/// Default cosine-similarity threshold for topic boundary detection.
#[cfg(feature = "embeddings")]
const DEFAULT_TOPIC_THRESHOLD: f32 = 0.75;

/// Safety ceiling for auto-budget when no embedding model is configured.
/// Prevents unbounded chunks in header-less documents.
const AUTO_BUDGET_CEILING: usize = 4000;

/// Split text into semantically coherent chunks.
///
/// Splits text into fine-grained segments, detects structural (and optionally
/// embedding-based) topic boundaries, then merges segments into chunks that
/// respect those boundaries and the configured size budget.
pub fn chunk_semantic(
    text: &str,
    config: &ChunkingConfig,
    page_boundaries: Option<&[PageBoundary]>,
) -> Result<ChunkingResult> {
    if text.is_empty() {
        return Ok(ChunkingResult {
            chunks: vec![],
            chunk_count: 0,
        });
    }

    let seg_size = SEGMENT_SIZE;
    let has_markdown_headers = text.lines().any(crate::utils::markdown_utils::is_markdown_header);
    let splitter_segments: Vec<&str> = if has_markdown_headers {
        let splitter = MarkdownSplitter::new(seg_size);
        splitter.chunks(text).collect()
    } else {
        let splitter = TextSplitter::new(seg_size);
        splitter.chunks(text).collect()
    };

    if splitter_segments.is_empty() {
        return Ok(ChunkingResult {
            chunks: vec![],
            chunk_count: 0,
        });
    }

    let source_start = text.as_ptr() as usize;
    let segments: Vec<Segment<'_>> = splitter_segments
        .iter()
        .map(|&s| {
            let byte_start = s.as_ptr() as usize - source_start;
            debug_assert!(
                byte_start + s.len() <= text.len(),
                "text_splitter segment is not a subslice of the input"
            );
            Segment { text: s, byte_start }
        })
        .collect();

    let detected = detect_plain_text_boundaries(text);
    let mut forced: Vec<bool> = vec![false; segments.len()];
    forced[0] = true;

    // Both detected boundaries and segments are sorted by byte offset.
    // Use a two-pointer merge for O(n+m) instead of O(n*m).
    let mut seg_idx = 0;
    for boundary in &detected {
        while seg_idx < segments.len()
            && segments[seg_idx].byte_start + segments[seg_idx].text.len() <= boundary.byte_offset
        {
            seg_idx += 1;
        }
        if seg_idx < segments.len() {
            forced[seg_idx] = true;
        }
    }

    // text_splitter returns subslices of the input text. Verify this invariant.
    for seg in &segments {
        debug_assert!(
            seg.byte_start + seg.text.len() <= text.len(),
            "segment byte range exceeds source text length"
        );
    }

    let boundaries = compute_boundaries(&segments, &forced, config)?;

    let ceiling = resolve_ceiling(config);
    let merged = merge::merge_segments(text, &segments, &boundaries, ceiling, config.overlap);

    let heading_map = build_heading_map(text);
    let total_chunks = merged.len();
    let mut chunks = Vec::with_capacity(total_chunks);

    for (index, mc) in merged.into_iter().enumerate() {
        let heading_ctx = resolve_heading_context(mc.byte_start, &heading_map);
        let chunk_type = classify_chunk(&mc.text, heading_ctx.as_ref());

        let (first_page, last_page) = if let Some(pb) = page_boundaries {
            calculate_page_range(mc.byte_start, mc.byte_end, pb).unwrap_or((None, None))
        } else {
            (None, None)
        };

        chunks.push(Chunk {
            content: mc.text,
            chunk_type,
            embedding: None,
            metadata: ChunkMetadata {
                byte_start: mc.byte_start,
                byte_end: mc.byte_end,
                token_count: None,
                chunk_index: index,
                total_chunks,
                first_page,
                last_page,
                heading_context: heading_ctx,
            },
        });
    }

    Ok(ChunkingResult {
        chunk_count: chunks.len(),
        chunks,
    })
}

/// Compute final boundary vector, incorporating embeddings when available.
#[cfg(feature = "embeddings")]
fn compute_boundaries(segments: &[Segment<'_>], forced: &[bool], config: &ChunkingConfig) -> Result<Vec<bool>> {
    if let Some(ref embedding_config) = config.embedding {
        let segment_texts: Vec<&str> = segments.iter().map(|s| s.text).collect();
        let threshold = config
            .topic_threshold
            .unwrap_or(DEFAULT_TOPIC_THRESHOLD)
            .clamp(0.0, 1.0);
        topic::detect_topic_boundaries(&segment_texts, forced, embedding_config, threshold)
    } else {
        Ok(forced.to_vec())
    }
}

/// Compute final boundary vector (no embeddings available).
#[cfg(not(feature = "embeddings"))]
fn compute_boundaries(_segments: &[Segment<'_>], forced: &[bool], _config: &ChunkingConfig) -> Result<Vec<bool>> {
    Ok(forced.to_vec())
}

/// Resolve the safety ceiling for chunk size.
///
/// When an embedding preset is configured, use its chunk_size as the ceiling
/// (chunks must fit in the model's context window). Otherwise use a generous
/// default that prevents unbounded chunks in header-less documents.
fn resolve_ceiling(config: &ChunkingConfig) -> usize {
    #[cfg(feature = "embeddings")]
    if let Some(ref emb) = config.embedding
        && let crate::EmbeddingModelType::Preset { ref name } = emb.model
        && let Some(size) = crate::embeddings::preset_chunk_size(name)
    {
        return size;
    }
    let _ = config;
    AUTO_BUDGET_CEILING
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::chunking::config::ChunkerType;

    #[test]
    fn chunk_semantic_empty_text() {
        let config = ChunkingConfig {
            max_characters: 500,
            overlap: 0,
            trim: true,
            chunker_type: ChunkerType::Semantic,
            ..Default::default()
        };
        let result = chunk_semantic("", &config, None).unwrap();
        assert_eq!(result.chunks.len(), 0);
        assert_eq!(result.chunk_count, 0);
    }

    #[test]
    fn chunk_semantic_short_text() {
        let config = ChunkingConfig {
            max_characters: 500,
            overlap: 0,
            trim: true,
            chunker_type: ChunkerType::Semantic,
            ..Default::default()
        };
        let result = chunk_semantic("Hello world", &config, None).unwrap();
        assert_eq!(result.chunks.len(), 1);
        assert_eq!(result.chunks[0].content, "Hello world");
    }

    #[test]
    fn chunk_semantic_multi_paragraph_merges() {
        // No headers, no embeddings → all segments share one group → 1 chunk
        let text = "First paragraph about cats.\n\nSecond paragraph about cats.\n\nThird paragraph about cats.";
        let config = ChunkingConfig {
            max_characters: 2000,
            overlap: 0,
            trim: true,
            chunker_type: ChunkerType::Semantic,
            ..Default::default()
        };
        let result = chunk_semantic(text, &config, None).unwrap();
        assert_eq!(result.chunks.len(), 1, "all segments should merge into one chunk");
    }

    #[test]
    fn chunk_semantic_all_caps_headers_force_boundaries() {
        // Each section must be long enough to produce separate segments (SEGMENT_SIZE = 200).
        let intro_body = "This is the introduction. ".repeat(12); // ~300 chars
        let method_body = "Here we describe the methodology used. ".repeat(10); // ~390 chars
        let results_body = "The results show improvements. ".repeat(10); // ~300 chars
        let text = format!("INTRODUCTION\n\n{intro_body}\n\nMETHODOLOGY\n\n{method_body}\n\nRESULTS\n\n{results_body}");
        let config = ChunkingConfig {
            max_characters: 2000,
            overlap: 0,
            trim: true,
            chunker_type: ChunkerType::Semantic,
            ..Default::default()
        };
        let result = chunk_semantic(&text, &config, None).unwrap();
        assert!(
            result.chunks.len() >= 2,
            "ALL CAPS headers should force boundaries, got {} chunks",
            result.chunks.len()
        );
    }

    #[test]
    fn chunk_semantic_markdown_uses_markdown_splitter() {
        // Content with ATX headings should trigger the MarkdownSplitter path.
        let text = "# Introduction\n\nThis is the intro paragraph.\n\n## Details\n\nMore detail here.";
        let config = ChunkingConfig {
            max_characters: 2000,
            overlap: 0,
            trim: true,
            chunker_type: ChunkerType::Semantic,
            ..Default::default()
        };
        let result = chunk_semantic(text, &config, None).unwrap();
        assert!(!result.chunks.is_empty(), "markdown content should produce chunks");
        // The combined content should cover the full text.
        let combined: String = result
            .chunks
            .iter()
            .map(|c| c.content.as_str())
            .collect::<Vec<_>>()
            .join("");
        assert!(combined.contains("Introduction"));
        assert!(combined.contains("Details"));
    }

    #[test]
    fn chunk_semantic_overlap_between_topic_groups() {
        // Build text with two clear ALL-CAPS sections so boundaries are forced.
        let body_a = "Alpha paragraph content. ".repeat(15); // ~375 chars
        let body_b = "Beta paragraph content. ".repeat(15);
        let text = format!("SECTION ONE\n\n{body_a}\n\nSECTION TWO\n\n{body_b}");
        let config = ChunkingConfig {
            max_characters: 2000,
            overlap: 10,
            trim: true,
            chunker_type: ChunkerType::Semantic,
            ..Default::default()
        };
        let result = chunk_semantic(&text, &config, None).unwrap();
        assert!(
            result.chunks.len() >= 2,
            "should produce at least 2 chunks from 2 sections, got {}",
            result.chunks.len()
        );
        // The second chunk should start with overlap characters from the previous group.
        // We cannot predict exact overlap text, but the second chunk should contain
        // content from SECTION TWO.
        let second = &result.chunks[1].content;
        assert!(
            second.contains("SECTION TWO") || second.contains("Beta"),
            "second chunk should contain second section content"
        );
    }

    #[test]
    fn ceiling_caps_oversized_headerless_text() {
        // A large block of text with no headers should be split at the ceiling,
        // not produce one unbounded chunk.
        let text = "word ".repeat(1500); // ~7500 chars, exceeds AUTO_BUDGET_CEILING
        let config = ChunkingConfig {
            max_characters: 1000, // ignored by semantic chunker
            overlap: 0,
            trim: true,
            chunker_type: ChunkerType::Semantic,
            ..Default::default()
        };
        let result = chunk_semantic(&text, &config, None).unwrap();
        assert!(result.chunks.len() >= 2, "should split at ceiling, got 1 chunk");
        for (i, chunk) in result.chunks.iter().enumerate() {
            assert!(
                chunk.content.chars().count() <= super::AUTO_BUDGET_CEILING + 100,
                "chunk {} exceeds ceiling: {} > {}",
                i,
                chunk.content.chars().count(),
                super::AUTO_BUDGET_CEILING
            );
        }
    }

    #[test]
    fn sections_with_headers_produce_separate_chunks() {
        // Each section has enough content that the segments span multiple paragraphs.
        // Headers force boundaries, so each section should be its own chunk.
        let body = "Content paragraph with sufficient text. ".repeat(8); // ~320 chars
        let text = format!("SECTION A\n\n{body}\n\nSECTION B\n\n{body}\n\nSECTION C\n\n{body}");
        let config = ChunkingConfig {
            overlap: 0,
            trim: true,
            chunker_type: ChunkerType::Semantic,
            ..Default::default()
        };
        let result = chunk_semantic(&text, &config, None).unwrap();
        assert!(
            result.chunks.len() >= 3,
            "3 sections with headers should produce >= 3 chunks, got {}",
            result.chunks.len()
        );
    }

    #[test]
    fn single_short_paragraph_one_chunk() {
        let text = "A short paragraph.";
        let config = ChunkingConfig {
            chunker_type: ChunkerType::Semantic,
            ..Default::default()
        };
        let result = chunk_semantic(text, &config, None).unwrap();
        assert_eq!(result.chunks.len(), 1);
    }

    #[test]
    fn topic_boundaries_keep_sections_separate() {
        // Multi-section document with substantial content per section.
        let energy_body = "Solar panels have improved significantly over the past decade. ".repeat(6);
        let health_body = "AI diagnostics advanced rapidly during clinical trials. ".repeat(6);
        let quantum_body = "Qubits crossed the thousand mark in recent experiments. ".repeat(6);
        let text = format!(
            "RENEWABLE ENERGY\n\n{energy_body}\n\nHEALTHCARE\n\n{health_body}\n\nQUANTUM COMPUTING\n\n{quantum_body}"
        );

        let config = ChunkingConfig {
            chunker_type: ChunkerType::Semantic,
            ..Default::default()
        };
        let result = chunk_semantic(&text, &config, None).unwrap();

        assert!(
            result.chunks.len() >= 3,
            "3 sections should produce >= 3 chunks, got {}",
            result.chunks.len()
        );

        // Energy chunk shouldn't contain healthcare content.
        let energy = result.chunks.iter().find(|c| c.content.contains("Solar")).unwrap();
        assert!(
            !energy.content.contains("diagnostics"),
            "energy chunk contains healthcare content"
        );
    }
}
