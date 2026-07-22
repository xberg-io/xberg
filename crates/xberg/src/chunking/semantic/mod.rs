//! Semantic text chunking.
//!
//! Splits text into fine-grained segments, detects topic boundaries (optionally
//! using embeddings), and merges segments into coherent chunks.

pub mod merge;

#[cfg(feature = "embeddings")]
pub mod topic;

use crate::chunking::boundaries::calculate_page_range;
use crate::chunking::boundary_detection::detect_plain_text_boundaries;
use crate::chunking::builder::heading_path_from_context;
use crate::chunking::classifier::classify_chunk;
use crate::chunking::config::{ChunkingConfig, ChunkingResult};
use crate::chunking::headings::{build_heading_map, resolve_heading_context};
use crate::chunking::text_splitter::{MarkdownSplitter, TextSplitter};
use crate::error::Result;
use crate::types::{Chunk, ChunkMetadata, PageBoundary};
use merge::Segment;

/// Default segment size (characters) for the initial fine-grained split.
const SEGMENT_SIZE: usize = 200;

/// Default cosine-similarity threshold for topic boundary detection.
#[cfg(feature = "embeddings")]
const DEFAULT_TOPIC_THRESHOLD: f32 = 0.75;

/// Split text into semantically coherent chunks.
///
/// Splits text into fine-grained segments, detects structural (and optionally
/// embedding-based) topic boundaries, then merges segments into chunks that
/// respect those boundaries and the configured size budget.
pub(crate) fn chunk_semantic(
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

    warn_if_fallback_path(config);

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

        let heading_path = heading_path_from_context(&heading_ctx);
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
                heading_path,
                image_indices: Vec::new(),
                node_ids: Vec::new(),
                page_spans: Vec::new(),
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

/// Warn when the semantic chunker is invoked without an embedding model.
///
/// Without an embedding, `chunk_semantic` falls back to a structural-boundary
/// heuristic (ALL-CAPS headers, numbered sections, blank-line paragraphs).
/// Topic-similarity chunking requires an embedding model. This warning makes
/// the fallback mode discoverable to callers who think they're getting
/// embedding-driven topic detection.
#[cfg(feature = "embeddings")]
fn warn_if_fallback_path(config: &ChunkingConfig) {
    if config.embedding.is_none() {
        tracing::warn!(
            "chunker_type='semantic' without an EmbeddingConfig falls back to a \
             structural-boundary heuristic; topic-similarity chunking requires an \
             embedding model. Either configure `embedding` or switch to \
             chunker_type='text'/'markdown' to silence this warning."
        );
    }
}

#[cfg(not(feature = "embeddings"))]
fn warn_if_fallback_path(_config: &ChunkingConfig) {}

/// Resolve the size ceiling for merged chunks.
///
/// When an embedding preset is configured, use its `chunk_size` so chunks fit
/// in the model's context window. Otherwise honor the caller's configured
/// `max_characters`.
fn resolve_ceiling(config: &ChunkingConfig) -> usize {
    #[cfg(feature = "embeddings")]
    if let Some(ref emb) = config.embedding
        && let crate::EmbeddingModelType::Preset { ref name } = emb.model
        && let Some(size) = crate::embeddings::preset_chunk_size(name)
    {
        return size;
    }
    config.max_characters
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
        let intro_body = "This is the introduction. ".repeat(12);
        let method_body = "Here we describe the methodology used. ".repeat(10);
        let results_body = "The results show improvements. ".repeat(10);
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
        let body_a = "Alpha paragraph content. ".repeat(15);
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
        let second = &result.chunks[1].content;
        assert!(
            second.contains("SECTION TWO") || second.contains("Beta"),
            "second chunk should contain second section content"
        );
    }

    #[test]
    fn max_characters_caps_oversized_headerless_text() {
        let text = "word ".repeat(1500);
        let max = 1000;
        let config = ChunkingConfig {
            max_characters: max,
            overlap: 0,
            trim: true,
            chunker_type: ChunkerType::Semantic,
            ..Default::default()
        };
        let result = chunk_semantic(&text, &config, None).unwrap();
        assert!(result.chunks.len() >= 2, "should split at max_characters, got 1 chunk");
        for (i, chunk) in result.chunks.iter().enumerate() {
            assert!(
                chunk.content.chars().count() <= max,
                "chunk {} exceeds max_characters: {} > {}",
                i,
                chunk.content.chars().count(),
                max
            );
        }
    }

    #[test]
    fn max_characters_controls_fallback_chunk_size() {
        let sample = format!(
            "{}{}{}",
            "Solar panel efficiency improves. ".repeat(200),
            "\n\nFDA clinical trials require double-blind. ".repeat(200),
            "\n\nQuantum entanglement needs cooling. ".repeat(200),
        );

        let run = |max: usize| {
            let config = ChunkingConfig {
                max_characters: max,
                overlap: 0,
                trim: true,
                chunker_type: ChunkerType::Semantic,
                ..Default::default()
            };
            chunk_semantic(&sample, &config, None).unwrap()
        };

        let small = run(500);
        let large = run(1500);

        assert!(
            small.chunks.len() > large.chunks.len(),
            "smaller max_characters must yield more chunks: small={}, large={}",
            small.chunks.len(),
            large.chunks.len()
        );
        for chunk in &small.chunks {
            assert!(
                chunk.content.chars().count() <= 500,
                "small chunk exceeds cap: {}",
                chunk.content.chars().count()
            );
        }
        for chunk in &large.chunks {
            assert!(
                chunk.content.chars().count() <= 1500,
                "large chunk exceeds cap: {}",
                chunk.content.chars().count()
            );
        }
    }

    #[cfg(feature = "embeddings")]
    #[test]
    fn semantic_without_embedding_warns() {
        use std::io::Write;
        use std::sync::{Arc, Mutex};

        #[derive(Clone, Default)]
        struct Buf(Arc<Mutex<Vec<u8>>>);
        impl Write for Buf {
            fn write(&mut self, buf: &[u8]) -> std::io::Result<usize> {
                self.0.lock().unwrap().extend_from_slice(buf);
                Ok(buf.len())
            }
            fn flush(&mut self) -> std::io::Result<()> {
                Ok(())
            }
        }
        impl<'a> tracing_subscriber::fmt::MakeWriter<'a> for Buf {
            type Writer = Buf;
            fn make_writer(&'a self) -> Self::Writer {
                self.clone()
            }
        }

        let buffer = Buf::default();
        let subscriber = tracing_subscriber::fmt()
            .with_writer(buffer.clone())
            .with_max_level(tracing::Level::WARN)
            .with_ansi(false)
            .finish();

        tracing::subscriber::with_default(subscriber, || {
            let config = ChunkingConfig {
                chunker_type: ChunkerType::Semantic,
                ..Default::default()
            };
            let _ = chunk_semantic("hello world", &config, None).unwrap();
        });

        let captured = String::from_utf8(buffer.0.lock().unwrap().clone()).unwrap();
        assert!(
            captured.contains("without an EmbeddingConfig"),
            "expected fallback warning in captured logs, got: {captured:?}"
        );
    }

    #[test]
    fn sections_with_headers_produce_separate_chunks() {
        let body = "Content paragraph with sufficient text. ".repeat(8);
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

        let energy = result.chunks.iter().find(|c| c.content.contains("Solar")).unwrap();
        assert!(
            !energy.content.contains("diagnostics"),
            "energy chunk contains healthcare content"
        );
    }
}
