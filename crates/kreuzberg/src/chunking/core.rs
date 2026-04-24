//! Core text chunking logic and public API.
//!
//! This module implements the main chunking algorithms and provides the primary
//! public API functions for splitting text into chunks.

use std::fmt::Write;

use crate::error::Result;
use crate::types::PageBoundary;
use text_splitter::{ChunkSizer, MarkdownSplitter, TextSplitter};

use super::builder::{build_chunk_config, build_chunks};
use super::config::{ChunkerType, ChunkingConfig, ChunkingResult};
use super::headings::{build_heading_map, resolve_heading_context};
use super::validation::validate_utf8_boundaries;

/// Split text into chunks with optional page boundary tracking.
///
/// This is the primary API function for chunking text. It supports both plain text
/// and Markdown with configurable chunk size, overlap, and page boundary mapping.
///
/// # Arguments
///
/// * `text` - The text to split into chunks
/// * `config` - Chunking configuration (max size, overlap, type)
/// * `page_boundaries` - Optional page boundary markers for mapping chunks to pages
///
/// # Returns
///
/// A ChunkingResult containing all chunks and their metadata.
///
/// # Examples
///
/// ```rust
/// use kreuzberg::chunking::{chunk_text, ChunkingConfig, ChunkerType};
///
/// # fn example() -> kreuzberg::Result<()> {
/// let config = ChunkingConfig {
///     max_characters: 500,
///     overlap: 50,
///     trim: true,
///     chunker_type: ChunkerType::Text,
///     ..Default::default()
/// };
/// let result = chunk_text("Long text...", &config, None)?;
/// assert!(!result.chunks.is_empty());
/// # Ok(())
/// # }
/// ```
pub fn chunk_text(
    text: &str,
    config: &ChunkingConfig,
    page_boundaries: Option<&[PageBoundary]>,
) -> Result<ChunkingResult> {
    chunk_text_with_heading_source(text, config, page_boundaries, None)
}

/// Chunk text with an optional separate markdown source for heading context resolution.
///
/// When `heading_source` is provided, it is used instead of `text` for building the
/// heading map. This is needed when `text` is plain text (no markdown headings) but
/// the original document had headings that were stripped during rendering.
pub fn chunk_text_with_heading_source(
    text: &str,
    config: &ChunkingConfig,
    page_boundaries: Option<&[PageBoundary]>,
    heading_source: Option<&str>,
) -> Result<ChunkingResult> {
    if text.is_empty() {
        return Ok(ChunkingResult {
            chunks: vec![],
            chunk_count: 0,
        });
    }

    if let Some(boundaries) = page_boundaries {
        validate_utf8_boundaries(text, boundaries)?;
    }

    // Yaml creates new content per chunk (key prefix), can't use generic &str splitter.
    if config.chunker_type == ChunkerType::Yaml {
        return super::yaml_section::chunk_yaml_by_sections(text, config);
    }

    // Semantic chunker has its own pipeline (segment → topic detect → merge).
    if config.chunker_type == ChunkerType::Semantic {
        return super::semantic::chunk_semantic(text, config, page_boundaries);
    }

    let text_chunks: Vec<&str> = match &config.sizing {
        #[cfg(feature = "chunking-tokenizers")]
        crate::core::config::ChunkSizing::Tokenizer { model, .. } => {
            let tokenizer = super::tokenizer_cache::get_or_init_tokenizer(model)?;
            let chunk_config =
                text_splitter::ChunkConfig::new(text_splitter::ChunkCapacity::new(config.max_characters))
                    .with_sizer((*tokenizer).clone())
                    .with_overlap(config.overlap)
                    .map(|c| c.with_trim(config.trim))
                    .map_err(|e| crate::KreuzbergError::validation(format!("Invalid chunking configuration: {}", e)))?;
            split_with_config(text, &config.chunker_type, chunk_config)
        }
        // Characters sizing (default) — also matches when no token features are enabled
        _ => {
            let chunk_config = build_chunk_config(config.max_characters, config.overlap, config.trim)?;
            split_with_config(text, &config.chunker_type, chunk_config)
        }
    };

    let mut chunks = build_chunks(text, text_chunks, page_boundaries)?;

    // For Markdown chunker, resolve heading context for each chunk.
    // Use the heading_source (markdown-formatted content) if provided, otherwise fall back to text.
    if config.chunker_type == ChunkerType::Markdown {
        let heading_map = build_heading_map(heading_source.unwrap_or(text));
        if !heading_map.is_empty() {
            for chunk in &mut chunks {
                chunk.metadata.heading_context = resolve_heading_context(chunk.metadata.byte_start, &heading_map);
            }

            // Optionally prepend heading hierarchy path to chunk content.
            if config.prepend_heading_context {
                for chunk in &mut chunks {
                    let Some(ref ctx) = chunk.metadata.heading_context else {
                        continue;
                    };

                    // Build breadcrumb prefix directly into the output buffer.
                    let mut new_content = String::with_capacity(chunk.content.len() + 64);
                    for (i, h) in ctx.headings.iter().enumerate() {
                        if i > 0 {
                            new_content.push_str(" > ");
                        }
                        for _ in 0..h.level {
                            new_content.push('#');
                        }
                        // Writing to String is infallible.
                        let _ = write!(new_content, " {}", h.text);
                    }
                    new_content.push_str("\n\n");

                    // If the markdown splitter included the deepest heading at the
                    // start of the chunk, skip it to avoid duplication.
                    let body = match ctx.headings.last() {
                        Some(h) => strip_leading_heading(&chunk.content, h.level, &h.text),
                        None => &chunk.content,
                    };
                    new_content.push_str(body);
                    chunk.content = new_content;
                }
            }
        }
    }

    let chunk_count = chunks.len();

    Ok(ChunkingResult { chunks, chunk_count })
}

/// If `text` starts with a markdown ATX heading matching `level` and `title`,
/// return the remainder after that heading line with leading newlines trimmed.
/// Otherwise return the input unchanged.
///
/// Handles optional closing ATX hashes (e.g. `## Heading ##`).
fn strip_leading_heading<'a>(text: &'a str, level: u8, title: &str) -> &'a str {
    debug_assert!(level > 0, "heading level must be 1..=6");
    let n = level as usize;
    let bytes = text.as_bytes();
    // Must start with exactly `n` '#' characters followed by a space.
    if bytes.len() <= n || bytes[..n].iter().any(|&b| b != b'#') || bytes[n] != b' ' {
        return text;
    }
    let after_prefix = &text[n + 1..];
    if !after_prefix.starts_with(title) {
        return text;
    }
    // Consume only through the end of the heading line, then trim leading newlines.
    // This avoids greedily eating into body content that follows on the same line.
    let rest = &after_prefix[title.len()..];
    let line_end = rest.find('\n').unwrap_or(rest.len());
    rest[line_end..].trim_start_matches('\n')
}

/// Split text using the appropriate splitter type with a generic sizer.
fn split_with_config<'a, S: ChunkSizer>(
    text: &'a str,
    chunker_type: &ChunkerType,
    config: text_splitter::ChunkConfig<S>,
) -> Vec<&'a str> {
    match chunker_type {
        ChunkerType::Text | ChunkerType::Yaml | ChunkerType::Semantic => {
            TextSplitter::new(config).chunks(text).collect()
        }
        ChunkerType::Markdown => MarkdownSplitter::new(config).chunks(text).collect(),
    }
}

/// Chunk text with explicit type specification.
///
/// This is a convenience function that constructs a ChunkingConfig from individual
/// parameters and calls `chunk_text`.
///
/// # Arguments
///
/// * `text` - The text to split into chunks
/// * `max_characters` - Maximum characters per chunk
/// * `overlap` - Character overlap between consecutive chunks
/// * `trim` - Whether to trim whitespace from boundaries
/// * `chunker_type` - Type of chunker to use (Text or Markdown)
///
/// # Returns
///
/// A ChunkingResult containing all chunks and their metadata.
///
/// # Examples
///
/// ```rust
/// use kreuzberg::chunking::{chunk_text_with_type, ChunkerType};
///
/// # fn example() -> kreuzberg::Result<()> {
/// let result = chunk_text_with_type("Some text", 500, 50, true, ChunkerType::Text)?;
/// assert!(!result.chunks.is_empty());
/// # Ok(())
/// # }
/// ```
#[cfg(test)]
pub(crate) fn chunk_text_with_type(
    text: &str,
    max_characters: usize,
    overlap: usize,
    trim: bool,
    chunker_type: ChunkerType,
) -> Result<ChunkingResult> {
    let config = ChunkingConfig {
        max_characters,
        overlap,
        trim,
        chunker_type,
        ..Default::default()
    };
    chunk_text(text, &config, None)
}

/// Batch process multiple texts with the same configuration.
///
/// This convenience function applies the same chunking configuration to multiple
/// texts in sequence.
///
/// # Arguments
///
/// * `texts` - Slice of text strings to chunk
/// * `config` - Chunking configuration to apply to all texts
///
/// # Returns
///
/// A vector of ChunkingResult objects, one per input text.
///
/// # Errors
///
/// Returns an error if chunking any individual text fails.
///
/// # Examples
///
/// ```rust
/// use kreuzberg::chunking::{chunk_texts_batch, ChunkingConfig};
///
/// # fn example() -> kreuzberg::Result<()> {
/// let config = ChunkingConfig::default();
/// let texts: Vec<String> = vec!["First text".to_string(), "Second text".to_string()];
/// let results = chunk_texts_batch(&texts, &config)?;
/// assert_eq!(results.len(), 2);
/// # Ok(())
/// # }
/// ```
pub fn chunk_texts_batch(texts: &[String], config: &ChunkingConfig) -> Result<Vec<ChunkingResult>> {
    texts.iter().map(|text| chunk_text(text, config, None)).collect()
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::KreuzbergError;

    #[test]
    fn test_chunk_empty_text() {
        let config = ChunkingConfig::default();
        let result = chunk_text("", &config, None).unwrap();
        assert_eq!(result.chunks.len(), 0);
        assert_eq!(result.chunk_count, 0);
    }

    #[test]
    fn test_chunk_short_text_single_chunk() {
        let config = ChunkingConfig {
            max_characters: 100,
            overlap: 10,
            trim: true,
            chunker_type: ChunkerType::Text,
            ..Default::default()
        };
        let text = "This is a short text.";
        let result = chunk_text(text, &config, None).unwrap();
        assert_eq!(result.chunks.len(), 1);
        assert_eq!(result.chunk_count, 1);
        assert_eq!(result.chunks[0].content, text);
    }

    #[test]
    fn test_chunk_long_text_multiple_chunks() {
        let config = ChunkingConfig {
            max_characters: 20,
            overlap: 5,
            trim: true,
            chunker_type: ChunkerType::Text,
            ..Default::default()
        };
        let text = "0123456789ABCDEFGHIJKLMNOPQRSTUVWXYZ";
        let result = chunk_text(text, &config, None).unwrap();
        assert!(result.chunk_count >= 2);
        assert_eq!(result.chunks.len(), result.chunk_count);
        assert!(result.chunks.iter().all(|chunk| chunk.content.len() <= 20));
    }

    #[test]
    fn test_chunk_text_with_overlap() {
        let config = ChunkingConfig {
            max_characters: 20,
            overlap: 5,
            trim: true,
            chunker_type: ChunkerType::Text,
            ..Default::default()
        };
        let text = "abcdefghijklmnopqrstuvwxyz0123456789";
        let result = chunk_text(text, &config, None).unwrap();
        assert!(result.chunk_count >= 2);

        if result.chunks.len() >= 2 {
            let first_chunk_end = &result.chunks[0].content[result.chunks[0].content.len().saturating_sub(5)..];
            assert!(
                result.chunks[1].content.starts_with(first_chunk_end),
                "Expected overlap '{}' at start of second chunk '{}'",
                first_chunk_end,
                result.chunks[1].content
            );
        }
    }

    #[test]
    fn test_chunk_markdown_preserves_structure() {
        let config = ChunkingConfig {
            max_characters: 50,
            overlap: 10,
            trim: true,
            chunker_type: ChunkerType::Markdown,
            ..Default::default()
        };
        let markdown = "# Title\n\nParagraph one.\n\n## Section\n\nParagraph two.";
        let result = chunk_text(markdown, &config, None).unwrap();
        assert!(result.chunk_count >= 1);
        assert!(result.chunks.iter().any(|chunk| chunk.content.contains("# Title")));
    }

    #[test]
    fn test_chunk_markdown_with_code_blocks() {
        let config = ChunkingConfig {
            max_characters: 100,
            overlap: 10,
            trim: true,
            chunker_type: ChunkerType::Markdown,
            ..Default::default()
        };
        let markdown = "# Code Example\n\n```python\nprint('hello')\n```\n\nSome text after code.";
        let result = chunk_text(markdown, &config, None).unwrap();
        assert!(result.chunk_count >= 1);
        assert!(result.chunks.iter().any(|chunk| chunk.content.contains("```")));
    }

    #[test]
    fn test_chunk_markdown_with_links() {
        let config = ChunkingConfig {
            max_characters: 80,
            overlap: 10,
            trim: true,
            chunker_type: ChunkerType::Markdown,
            ..Default::default()
        };
        let markdown = "Check out [this link](https://example.com) for more info.";
        let result = chunk_text(markdown, &config, None).unwrap();
        assert_eq!(result.chunk_count, 1);
        assert!(result.chunks[0].content.contains("[this link]"));
    }

    #[test]
    fn test_chunk_text_with_trim() {
        let config = ChunkingConfig {
            max_characters: 30,
            overlap: 5,
            trim: true,
            chunker_type: ChunkerType::Text,
            ..Default::default()
        };
        let text = "  Leading and trailing spaces  should be trimmed  ";
        let result = chunk_text(text, &config, None).unwrap();
        assert!(result.chunk_count >= 1);
        assert!(result.chunks.iter().all(|chunk| !chunk.content.starts_with(' ')));
    }

    #[test]
    fn test_chunk_text_without_trim() {
        let config = ChunkingConfig {
            max_characters: 30,
            overlap: 5,
            trim: false,
            chunker_type: ChunkerType::Text,
            ..Default::default()
        };
        let text = "  Text with spaces  ";
        let result = chunk_text(text, &config, None).unwrap();
        assert_eq!(result.chunk_count, 1);
        assert!(result.chunks[0].content.starts_with(' ') || result.chunks[0].content.len() < text.len());
    }

    #[test]
    fn test_chunk_with_invalid_overlap() {
        let config = ChunkingConfig {
            max_characters: 10,
            overlap: 20,
            trim: true,
            chunker_type: ChunkerType::Text,
            ..Default::default()
        };
        let result = chunk_text("Some text", &config, None);
        assert!(result.is_err());
        let err = result.unwrap_err();
        assert!(matches!(err, KreuzbergError::Validation { .. }));
    }

    #[test]
    fn test_chunk_text_with_type_text() {
        let result = chunk_text_with_type("Simple text", 50, 10, true, ChunkerType::Text).unwrap();
        assert_eq!(result.chunk_count, 1);
        assert_eq!(result.chunks[0].content, "Simple text");
    }

    #[test]
    fn test_chunk_text_with_type_markdown() {
        let markdown = "# Header\n\nContent here.";
        let result = chunk_text_with_type(markdown, 50, 10, true, ChunkerType::Markdown).unwrap();
        assert_eq!(result.chunk_count, 1);
        assert!(result.chunks[0].content.contains("# Header"));
    }

    #[test]
    fn test_chunk_texts_batch_empty() {
        let config = ChunkingConfig::default();
        let texts: Vec<String> = vec![];
        let results = chunk_texts_batch(&texts, &config).unwrap();
        assert_eq!(results.len(), 0);
    }

    #[test]
    fn test_chunk_texts_batch_multiple() {
        let config = ChunkingConfig {
            max_characters: 30,
            overlap: 5,
            trim: true,
            chunker_type: ChunkerType::Text,
            ..Default::default()
        };
        let texts: Vec<String> = vec![
            "First text".to_string(),
            "Second text".to_string(),
            "Third text".to_string(),
        ];
        let results = chunk_texts_batch(&texts, &config).unwrap();
        assert_eq!(results.len(), 3);
        assert!(results.iter().all(|r| r.chunk_count >= 1));
    }

    #[test]
    fn test_chunk_texts_batch_mixed_lengths() {
        let config = ChunkingConfig {
            max_characters: 20,
            overlap: 5,
            trim: true,
            chunker_type: ChunkerType::Text,
            ..Default::default()
        };
        let texts: Vec<String> = vec![
            "Short".to_string(),
            "This is a longer text that should be split into multiple chunks".to_string(),
            String::new(),
        ];
        let results = chunk_texts_batch(&texts, &config).unwrap();
        assert_eq!(results.len(), 3);
        assert_eq!(results[0].chunk_count, 1);
        assert!(results[1].chunk_count > 1);
        assert_eq!(results[2].chunk_count, 0);
    }

    #[test]
    fn test_chunk_texts_batch_error_propagation() {
        let config = ChunkingConfig {
            max_characters: 10,
            overlap: 20,
            trim: true,
            chunker_type: ChunkerType::Text,
            ..Default::default()
        };
        let texts: Vec<String> = vec!["Text one".to_string(), "Text two".to_string()];
        let result = chunk_texts_batch(&texts, &config);
        assert!(result.is_err());
    }

    #[test]
    fn test_chunking_config_default() {
        let config = ChunkingConfig::default();
        assert_eq!(config.max_characters, 1000);
        assert_eq!(config.overlap, 200);
        assert!(config.trim);
        assert_eq!(config.chunker_type, ChunkerType::Text);
    }

    #[test]
    fn test_chunk_very_long_text() {
        let config = ChunkingConfig {
            max_characters: 100,
            overlap: 20,
            trim: true,
            chunker_type: ChunkerType::Text,
            ..Default::default()
        };
        let text = "a".repeat(1000);
        let result = chunk_text(&text, &config, None).unwrap();
        assert!(result.chunk_count >= 10);
        assert!(result.chunks.iter().all(|chunk| chunk.content.len() <= 100));
    }

    #[test]
    fn test_chunk_text_with_newlines() {
        let config = ChunkingConfig {
            max_characters: 30,
            overlap: 5,
            trim: true,
            chunker_type: ChunkerType::Text,
            ..Default::default()
        };
        let text = "Line one\nLine two\nLine three\nLine four\nLine five";
        let result = chunk_text(text, &config, None).unwrap();
        assert!(result.chunk_count >= 1);
    }

    #[test]
    fn test_chunk_markdown_with_lists() {
        let config = ChunkingConfig {
            max_characters: 100,
            overlap: 10,
            trim: true,
            chunker_type: ChunkerType::Markdown,
            ..Default::default()
        };
        let markdown = "# List Example\n\n- Item 1\n- Item 2\n- Item 3\n\nMore text.";
        let result = chunk_text(markdown, &config, None).unwrap();
        assert!(result.chunk_count >= 1);
        assert!(result.chunks.iter().any(|chunk| chunk.content.contains("- Item")));
    }

    #[test]
    fn test_chunk_markdown_with_tables() {
        let config = ChunkingConfig {
            max_characters: 150,
            overlap: 10,
            trim: true,
            chunker_type: ChunkerType::Markdown,
            ..Default::default()
        };
        let markdown = "# Table\n\n| Col1 | Col2 |\n|------|------|\n| A    | B    |\n| C    | D    |";
        let result = chunk_text(markdown, &config, None).unwrap();
        assert!(result.chunk_count >= 1);
        assert!(result.chunks.iter().any(|chunk| chunk.content.contains("|")));
    }

    #[test]
    fn test_chunk_special_characters() {
        let config = ChunkingConfig {
            max_characters: 50,
            overlap: 5,
            trim: true,
            chunker_type: ChunkerType::Text,
            ..Default::default()
        };
        let text = "Special chars: @#$%^&*()[]{}|\\<>?/~`";
        let result = chunk_text(text, &config, None).unwrap();
        assert_eq!(result.chunk_count, 1);
        assert!(result.chunks[0].content.contains("@#$%"));
    }

    #[test]
    fn test_chunk_unicode_characters() {
        let config = ChunkingConfig {
            max_characters: 50,
            overlap: 5,
            trim: true,
            chunker_type: ChunkerType::Text,
            ..Default::default()
        };
        let text = "Unicode: 你好世界 🌍 café résumé";
        let result = chunk_text(text, &config, None).unwrap();
        assert_eq!(result.chunk_count, 1);
        assert!(result.chunks[0].content.contains("你好"));
        assert!(result.chunks[0].content.contains("🌍"));
    }

    #[test]
    fn test_chunk_cjk_text() {
        let config = ChunkingConfig {
            max_characters: 30,
            overlap: 5,
            trim: true,
            chunker_type: ChunkerType::Text,
            ..Default::default()
        };
        let text = "日本語のテキストです。これは長い文章で、複数のチャンクに分割されるべきです。";
        let result = chunk_text(text, &config, None).unwrap();
        assert!(result.chunk_count >= 1);
    }

    #[test]
    fn test_prepend_heading_context() {
        let config = ChunkingConfig {
            max_characters: 50,
            overlap: 0,
            trim: true,
            chunker_type: ChunkerType::Markdown,
            prepend_heading_context: true,
            ..Default::default()
        };
        let markdown = "# Title\n\nSome text\n\n## Section\n\nMore text";
        let result = chunk_text(markdown, &config, None).unwrap();
        assert!(result.chunk_count >= 1);
        // Each chunk with heading context should have its content prefixed with
        // a heading breadcrumb path like "# Title" or "# Title > ## Section".
        for chunk in &result.chunks {
            if chunk.metadata.heading_context.is_some() {
                assert!(
                    chunk.content.starts_with('#'),
                    "Expected chunk content to start with heading path, got: {:?}",
                    &chunk.content
                );
            }
        }
        // At least one chunk should contain the section breadcrumb
        let has_section = result
            .chunks
            .iter()
            .any(|c| c.content.contains("# Title") || c.content.contains("## Section"));
        assert!(
            has_section,
            "Expected at least one chunk with heading breadcrumb in content"
        );
        // No heading should appear more than once per chunk (breadcrumb + body duplication).
        for chunk in &result.chunks {
            if let Some(ref ctx) = chunk.metadata.heading_context
                && let Some(deepest) = ctx.headings.last()
            {
                let heading_line = format!("{} {}", "#".repeat(deepest.level as usize), deepest.text);
                let occurrences = chunk.content.matches(&heading_line).count();
                assert!(
                    occurrences <= 1,
                    "Heading '{}' appears {} times in chunk (expected at most 1): {:?}",
                    heading_line,
                    occurrences,
                    &chunk.content
                );
            }
        }
    }

    #[test]
    fn test_strip_leading_heading_basic() {
        assert_eq!(strip_leading_heading("## Section\n\nBody", 2, "Section"), "Body");
    }

    #[test]
    fn test_strip_leading_heading_closing_atx() {
        assert_eq!(strip_leading_heading("## Section ##\n\nBody", 2, "Section"), "Body");
    }

    #[test]
    fn test_strip_leading_heading_no_match() {
        let text = "Some paragraph text";
        assert_eq!(strip_leading_heading(text, 2, "Section"), text);
    }

    #[test]
    fn test_strip_leading_heading_wrong_level() {
        let text = "### Section\n\nBody";
        assert_eq!(strip_leading_heading(text, 2, "Section"), text);
    }

    #[test]
    fn test_strip_leading_heading_single_newline() {
        assert_eq!(strip_leading_heading("# Title\nBody", 1, "Title"), "Body");
    }

    #[test]
    fn test_strip_leading_heading_no_body() {
        assert_eq!(strip_leading_heading("## Section", 2, "Section"), "");
    }

    #[test]
    fn test_strip_leading_heading_empty_input() {
        assert_eq!(strip_leading_heading("", 2, "Section"), "");
    }

    #[test]
    fn test_strip_leading_heading_unicode() {
        assert_eq!(
            strip_leading_heading("## Übersicht\n\nInhalt", 2, "Übersicht"),
            "Inhalt"
        );
        assert_eq!(strip_leading_heading("# 概要\n\n本文", 1, "概要"), "本文");
    }

    #[test]
    fn test_chunk_mixed_languages() {
        let config = ChunkingConfig {
            max_characters: 40,
            overlap: 5,
            trim: true,
            chunker_type: ChunkerType::Text,
            ..Default::default()
        };
        let text = "English text mixed with 中文文本 and some français";
        let result = chunk_text(text, &config, None).unwrap();
        assert!(result.chunk_count >= 1);
    }

    #[test]
    fn test_chunk_offset_calculation_with_overlap() {
        let config = ChunkingConfig {
            max_characters: 20,
            overlap: 5,
            trim: false,
            chunker_type: ChunkerType::Text,
            ..Default::default()
        };
        let text = "AAAAA BBBBB CCCCC DDDDD EEEEE FFFFF";
        let result = chunk_text(text, &config, None).unwrap();

        assert!(result.chunks.len() >= 2, "Expected at least 2 chunks");

        for i in 0..result.chunks.len() {
            let chunk = &result.chunks[i];
            let metadata = &chunk.metadata;

            assert_eq!(
                metadata.byte_end - metadata.byte_start,
                chunk.content.len(),
                "Chunk {} offset range doesn't match content length",
                i
            );

            assert_eq!(metadata.chunk_index, i);
            assert_eq!(metadata.total_chunks, result.chunks.len());
        }

        for i in 0..result.chunks.len() - 1 {
            let current_chunk = &result.chunks[i];
            let next_chunk = &result.chunks[i + 1];

            assert!(
                next_chunk.metadata.byte_start < current_chunk.metadata.byte_end,
                "Chunk {} and {} don't overlap: next starts at {} but current ends at {}",
                i,
                i + 1,
                next_chunk.metadata.byte_start,
                current_chunk.metadata.byte_end
            );

            let overlap_size = current_chunk.metadata.byte_end - next_chunk.metadata.byte_start;
            assert!(
                overlap_size <= config.overlap + 10,
                "Overlap between chunks {} and {} is too large: {}",
                i,
                i + 1,
                overlap_size
            );
        }
    }

    #[test]
    fn test_chunk_offset_calculation_without_overlap() {
        let config = ChunkingConfig {
            max_characters: 20,
            overlap: 0,
            trim: false,
            chunker_type: ChunkerType::Text,
            ..Default::default()
        };
        let text = "AAAAA BBBBB CCCCC DDDDD EEEEE FFFFF";
        let result = chunk_text(text, &config, None).unwrap();

        for i in 0..result.chunks.len() - 1 {
            let current_chunk = &result.chunks[i];
            let next_chunk = &result.chunks[i + 1];

            assert!(
                next_chunk.metadata.byte_start >= current_chunk.metadata.byte_end,
                "Chunk {} and {} overlap when they shouldn't: next starts at {} but current ends at {}",
                i,
                i + 1,
                next_chunk.metadata.byte_start,
                current_chunk.metadata.byte_end
            );
        }
    }

    #[test]
    fn test_chunk_offset_covers_full_text() {
        let config = ChunkingConfig {
            max_characters: 15,
            overlap: 3,
            trim: false,
            chunker_type: ChunkerType::Text,
            ..Default::default()
        };
        let text = "0123456789 ABCDEFGHIJ KLMNOPQRST UVWXYZ";
        let result = chunk_text(text, &config, None).unwrap();

        assert!(result.chunks.len() >= 2, "Expected multiple chunks");

        assert_eq!(
            result.chunks[0].metadata.byte_start, 0,
            "First chunk should start at position 0"
        );

        for i in 0..result.chunks.len() - 1 {
            let current_chunk = &result.chunks[i];
            let next_chunk = &result.chunks[i + 1];

            assert!(
                next_chunk.metadata.byte_start <= current_chunk.metadata.byte_end,
                "Gap detected between chunk {} (ends at {}) and chunk {} (starts at {})",
                i,
                current_chunk.metadata.byte_end,
                i + 1,
                next_chunk.metadata.byte_start
            );
        }
    }

    #[test]
    fn test_chunk_offset_with_various_overlap_sizes() {
        for overlap in [0, 5, 10, 20] {
            let config = ChunkingConfig {
                max_characters: 30,
                overlap,
                trim: false,
                chunker_type: ChunkerType::Text,
                ..Default::default()
            };
            let text = "Word ".repeat(30);
            let result = chunk_text(&text, &config, None).unwrap();

            for chunk in &result.chunks {
                assert!(
                    chunk.metadata.byte_end > chunk.metadata.byte_start,
                    "Invalid offset range for overlap {}: start={}, end={}",
                    overlap,
                    chunk.metadata.byte_start,
                    chunk.metadata.byte_end
                );
            }

            for chunk in &result.chunks {
                assert!(
                    chunk.metadata.byte_start < text.len(),
                    "char_start with overlap {} is out of bounds: {}",
                    overlap,
                    chunk.metadata.byte_start
                );
            }
        }
    }

    #[test]
    fn test_chunk_last_chunk_offset() {
        let config = ChunkingConfig {
            max_characters: 20,
            overlap: 5,
            trim: false,
            chunker_type: ChunkerType::Text,
            ..Default::default()
        };
        let text = "AAAAA BBBBB CCCCC DDDDD EEEEE";
        let result = chunk_text(text, &config, None).unwrap();

        assert!(result.chunks.len() >= 2, "Need multiple chunks for this test");

        let last_chunk = result.chunks.last().unwrap();
        let second_to_last = &result.chunks[result.chunks.len() - 2];

        assert!(
            last_chunk.metadata.byte_start < second_to_last.metadata.byte_end,
            "Last chunk should overlap with previous chunk"
        );

        let expected_end = text.len();
        let last_chunk_covers_end =
            last_chunk.content.trim_end() == text.trim_end() || last_chunk.metadata.byte_end >= expected_end - 5;
        assert!(last_chunk_covers_end, "Last chunk should cover the end of the text");
    }

    #[test]
    fn test_chunk_with_page_boundaries() {
        let config = ChunkingConfig {
            max_characters: 30,
            overlap: 5,
            trim: true,
            chunker_type: ChunkerType::Text,
            ..Default::default()
        };
        let text = "Page one content here. Page two starts here and continues.";

        let boundaries = vec![
            PageBoundary {
                byte_start: 0,
                byte_end: 21,
                page_number: 1,
            },
            PageBoundary {
                byte_start: 22,
                byte_end: 58,
                page_number: 2,
            },
        ];

        let result = chunk_text(text, &config, Some(&boundaries)).unwrap();
        assert!(result.chunks.len() >= 2);

        assert_eq!(result.chunks[0].metadata.first_page, Some(1));

        let last_chunk = result.chunks.last().unwrap();
        assert_eq!(last_chunk.metadata.last_page, Some(2));
    }

    #[test]
    fn test_chunk_without_page_boundaries() {
        let config = ChunkingConfig {
            max_characters: 30,
            overlap: 5,
            trim: true,
            chunker_type: ChunkerType::Text,
            ..Default::default()
        };
        let text = "This is some test content that should be split into multiple chunks.";

        let result = chunk_text(text, &config, None).unwrap();
        assert!(result.chunks.len() >= 2);

        for chunk in &result.chunks {
            assert_eq!(chunk.metadata.first_page, None);
            assert_eq!(chunk.metadata.last_page, None);
        }
    }

    #[test]
    fn test_chunk_empty_boundaries() {
        let config = ChunkingConfig {
            max_characters: 30,
            overlap: 5,
            trim: true,
            chunker_type: ChunkerType::Text,
            ..Default::default()
        };
        let text = "Some text content here.";
        let boundaries: Vec<PageBoundary> = vec![];

        let result = chunk_text(text, &config, Some(&boundaries)).unwrap();
        assert_eq!(result.chunks.len(), 1);

        assert_eq!(result.chunks[0].metadata.first_page, None);
        assert_eq!(result.chunks[0].metadata.last_page, None);
    }

    #[test]
    fn test_chunk_spanning_multiple_pages() {
        let config = ChunkingConfig {
            max_characters: 50,
            overlap: 5,
            trim: false,
            chunker_type: ChunkerType::Text,
            ..Default::default()
        };
        let text = "0123456789 AAAAAAAAAA 1111111111 BBBBBBBBBB 2222222222";

        let boundaries = vec![
            PageBoundary {
                byte_start: 0,
                byte_end: 20,
                page_number: 1,
            },
            PageBoundary {
                byte_start: 20,
                byte_end: 40,
                page_number: 2,
            },
            PageBoundary {
                byte_start: 40,
                byte_end: 54,
                page_number: 3,
            },
        ];

        let result = chunk_text(text, &config, Some(&boundaries)).unwrap();
        assert!(result.chunks.len() >= 2);

        for chunk in &result.chunks {
            assert!(chunk.metadata.first_page.is_some() || chunk.metadata.last_page.is_some());
        }
    }

    #[test]
    fn test_chunk_text_with_invalid_boundary_range() {
        let config = ChunkingConfig {
            max_characters: 30,
            overlap: 5,
            trim: true,
            chunker_type: ChunkerType::Text,
            ..Default::default()
        };
        let text = "Page one content here. Page two content.";

        let boundaries = vec![PageBoundary {
            byte_start: 10,
            byte_end: 5,
            page_number: 1,
        }];

        let result = chunk_text(text, &config, Some(&boundaries));
        assert!(result.is_err());
        let err = result.unwrap_err();
        assert!(err.to_string().contains("Invalid boundary range"));
        assert!(err.to_string().contains("byte_start"));
    }

    #[test]
    fn test_chunk_text_with_unsorted_boundaries() {
        let config = ChunkingConfig {
            max_characters: 30,
            overlap: 5,
            trim: true,
            chunker_type: ChunkerType::Text,
            ..Default::default()
        };
        let text = "Page one content here. Page two content.";

        let boundaries = vec![
            PageBoundary {
                byte_start: 22,
                byte_end: 40,
                page_number: 2,
            },
            PageBoundary {
                byte_start: 0,
                byte_end: 21,
                page_number: 1,
            },
        ];

        let result = chunk_text(text, &config, Some(&boundaries));
        assert!(result.is_err());
        let err = result.unwrap_err();
        assert!(err.to_string().contains("not sorted"));
        assert!(err.to_string().contains("boundaries"));
    }

    #[test]
    fn test_chunk_text_with_overlapping_boundaries() {
        let config = ChunkingConfig {
            max_characters: 30,
            overlap: 5,
            trim: true,
            chunker_type: ChunkerType::Text,
            ..Default::default()
        };
        let text = "Page one content here. Page two content.";

        let boundaries = vec![
            PageBoundary {
                byte_start: 0,
                byte_end: 25,
                page_number: 1,
            },
            PageBoundary {
                byte_start: 20,
                byte_end: 40,
                page_number: 2,
            },
        ];

        let result = chunk_text(text, &config, Some(&boundaries));
        assert!(result.is_err());
        let err = result.unwrap_err();
        assert!(err.to_string().contains("Overlapping"));
        assert!(err.to_string().contains("boundaries"));
    }

    #[test]
    fn test_chunk_with_pages_basic() {
        let config = ChunkingConfig {
            max_characters: 25,
            overlap: 5,
            trim: true,
            chunker_type: ChunkerType::Text,
            ..Default::default()
        };
        let text = "First page content here.Second page content here.Third page.";

        let boundaries = vec![
            PageBoundary {
                byte_start: 0,
                byte_end: 24,
                page_number: 1,
            },
            PageBoundary {
                byte_start: 24,
                byte_end: 50,
                page_number: 2,
            },
            PageBoundary {
                byte_start: 50,
                byte_end: 60,
                page_number: 3,
            },
        ];

        let result = chunk_text(text, &config, Some(&boundaries)).unwrap();

        if !result.chunks.is_empty() {
            assert!(result.chunks[0].metadata.first_page.is_some());
        }
    }

    #[test]
    fn test_chunk_with_pages_single_page_chunk() {
        let config = ChunkingConfig {
            max_characters: 100,
            overlap: 10,
            trim: true,
            chunker_type: ChunkerType::Text,
            ..Default::default()
        };
        let text = "All content on single page fits in one chunk.";

        let boundaries = vec![PageBoundary {
            byte_start: 0,
            byte_end: 45,
            page_number: 1,
        }];

        let result = chunk_text(text, &config, Some(&boundaries)).unwrap();
        assert_eq!(result.chunks.len(), 1);
        assert_eq!(result.chunks[0].metadata.first_page, Some(1));
        assert_eq!(result.chunks[0].metadata.last_page, Some(1));
    }

    #[test]
    fn test_chunk_with_pages_no_overlap() {
        let config = ChunkingConfig {
            max_characters: 20,
            overlap: 0,
            trim: false,
            chunker_type: ChunkerType::Text,
            ..Default::default()
        };
        let text = "AAAAA BBBBB CCCCC DDDDD";

        let boundaries = vec![
            PageBoundary {
                byte_start: 0,
                byte_end: 11,
                page_number: 1,
            },
            PageBoundary {
                byte_start: 11,
                byte_end: 23,
                page_number: 2,
            },
        ];

        let result = chunk_text(text, &config, Some(&boundaries)).unwrap();
        assert!(!result.chunks.is_empty());

        for chunk in &result.chunks {
            if let (Some(first), Some(last)) = (chunk.metadata.first_page, chunk.metadata.last_page) {
                assert!(first <= last);
            }
        }
    }

    #[test]
    fn test_chunk_metadata_page_range_accuracy() {
        let config = ChunkingConfig {
            max_characters: 30,
            overlap: 5,
            trim: true,
            chunker_type: ChunkerType::Text,
            ..Default::default()
        };
        let text = "Page One Content Here.Page Two.";

        let boundaries = vec![
            PageBoundary {
                byte_start: 0,
                byte_end: 21,
                page_number: 1,
            },
            PageBoundary {
                byte_start: 21,
                byte_end: 31,
                page_number: 2,
            },
        ];

        let result = chunk_text(text, &config, Some(&boundaries)).unwrap();

        for chunk in &result.chunks {
            assert_eq!(chunk.metadata.byte_end - chunk.metadata.byte_start, chunk.content.len());
        }
    }

    /// Regression test for GitHub issue #439:
    /// Chunk metadata reports wrong page numbers for documents with many pages.
    /// The byte offset drift causes chunks near the end of the document to
    /// reference pages far earlier than where their content actually resides.
    #[test]
    fn test_issue_439_chunk_page_metadata_many_pages() {
        let num_pages = 50;
        let mut full_text = String::new();
        let mut boundaries = Vec::new();

        for p in 1..=num_pages {
            let page_content = format!(
                "Page {} content. This is the text on page number {}. It has some words to fill space here. ",
                p, p
            );
            let start = full_text.len();
            full_text.push_str(&page_content);
            let end = full_text.len();
            boundaries.push(PageBoundary {
                byte_start: start,
                byte_end: end,
                page_number: p,
            });
        }

        let config = ChunkingConfig {
            max_characters: 200,
            overlap: 50,
            trim: true,
            chunker_type: ChunkerType::Text,
            ..Default::default()
        };

        let result = chunk_text(&full_text, &config, Some(&boundaries)).unwrap();

        // The last chunk must reference pages near the end of the document
        let last_chunk = result.chunks.last().unwrap();
        assert!(
            last_chunk.metadata.last_page.unwrap() >= num_pages - 2,
            "Last chunk should reference near the last page ({}), but got {:?}",
            num_pages,
            last_chunk.metadata.last_page
        );

        // Every chunk's byte range must correspond to where its content
        // actually lives in the original text
        for (i, chunk) in result.chunks.iter().enumerate() {
            let actual_pos = full_text
                .find(&chunk.content)
                .expect("Chunk content must be a substring of the original text");
            let actual_page = boundaries
                .iter()
                .find(|b| actual_pos >= b.byte_start && actual_pos < b.byte_end)
                .map(|b| b.page_number);

            if let (Some(reported), Some(actual)) = (chunk.metadata.first_page, actual_page) {
                assert_eq!(
                    reported, actual,
                    "Chunk {} reports first_page={} but content starts on page {} \
                     (byte_start={}, actual_pos={})",
                    i, reported, actual, chunk.metadata.byte_start, actual_pos
                );
            }
        }
    }

    /// Verify that chunk byte_start/byte_end match the actual position of the
    /// chunk content within the original text.
    #[test]
    fn test_issue_439_chunk_byte_offsets_match_text_position() {
        let text = "Alpha bravo charlie delta echo foxtrot golf hotel india juliet kilo lima mike november oscar papa quebec romeo sierra tango uniform victor whiskey xray yankee zulu. ";
        let repeated = text.repeat(5);

        let boundaries = vec![
            PageBoundary {
                byte_start: 0,
                byte_end: text.len(),
                page_number: 1,
            },
            PageBoundary {
                byte_start: text.len(),
                byte_end: text.len() * 2,
                page_number: 2,
            },
            PageBoundary {
                byte_start: text.len() * 2,
                byte_end: text.len() * 3,
                page_number: 3,
            },
            PageBoundary {
                byte_start: text.len() * 3,
                byte_end: text.len() * 4,
                page_number: 4,
            },
            PageBoundary {
                byte_start: text.len() * 4,
                byte_end: text.len() * 5,
                page_number: 5,
            },
        ];

        let config = ChunkingConfig {
            max_characters: 80,
            overlap: 20,
            trim: true,
            chunker_type: ChunkerType::Text,
            ..Default::default()
        };

        let result = chunk_text(&repeated, &config, Some(&boundaries)).unwrap();

        for (i, chunk) in result.chunks.iter().enumerate() {
            // The chunk content at byte_start..byte_end must match the actual content
            let byte_start = chunk.metadata.byte_start;
            let byte_end = chunk.metadata.byte_end;
            assert!(
                byte_end <= repeated.len(),
                "Chunk {} byte_end ({}) exceeds text length ({})",
                i,
                byte_end,
                repeated.len()
            );
            assert_eq!(
                &repeated[byte_start..byte_end],
                chunk.content,
                "Chunk {} content doesn't match text at byte_start={}..byte_end={}",
                i,
                byte_start,
                byte_end
            );
        }
    }

    #[test]
    fn test_chunk_page_range_boundary_edge_cases() {
        let config = ChunkingConfig {
            max_characters: 10,
            overlap: 2,
            trim: false,
            chunker_type: ChunkerType::Text,
            ..Default::default()
        };
        let text = "0123456789ABCDEFGHIJ";

        let boundaries = vec![
            PageBoundary {
                byte_start: 0,
                byte_end: 10,
                page_number: 1,
            },
            PageBoundary {
                byte_start: 10,
                byte_end: 20,
                page_number: 2,
            },
        ];

        let result = chunk_text(text, &config, Some(&boundaries)).unwrap();

        for chunk in &result.chunks {
            let on_page1 = chunk.metadata.byte_start < 10;
            let on_page2 = chunk.metadata.byte_end > 10;

            if on_page1 && on_page2 {
                assert_eq!(chunk.metadata.first_page, Some(1));
                assert_eq!(chunk.metadata.last_page, Some(2));
            } else if on_page1 {
                assert_eq!(chunk.metadata.first_page, Some(1));
            } else if on_page2 {
                assert_eq!(chunk.metadata.first_page, Some(2));
            }
        }
    }
}
