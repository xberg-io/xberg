//! Core text chunking logic and public API.
//!
//! This module implements the main chunking algorithms and provides the primary
//! public API functions for splitting text into chunks.

use std::fmt::Write;

#[cfg(feature = "chunking-tokenizers")]
use crate::chunking::text_splitter::ChunkCapacity;
use crate::chunking::text_splitter::{ChunkConfig, ChunkSizer, MarkdownSplitter, TextSplitter};
use crate::error::Result;
use crate::types::PageBoundary;

use super::builder::{build_chunk_config, build_chunks};
use super::config::{ChunkerType, ChunkingConfig, ChunkingResult, TableChunkingMode};
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
/// use xberg::chunking::{chunk_text, ChunkingConfig, ChunkerType};
///
/// # fn example() -> xberg::Result<()> {
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
#[cfg_attr(alef, alef(skip))]
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
pub(crate) fn chunk_text_with_heading_source(
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
        return super::yaml_section::chunk_yaml_by_sections(text, config, page_boundaries);
    }

    // Semantic chunker has its own pipeline (segment → topic detect → merge).
    if config.chunker_type == ChunkerType::Semantic {
        return super::semantic::chunk_semantic(text, config, page_boundaries);
    }

    let text_chunks: Vec<&str> = match &config.sizing {
        #[cfg(feature = "chunking-tokenizers")]
        crate::core::config::ChunkSizing::Tokenizer { model, .. } => {
            // A tokenizer backend registered under this name takes precedence
            // over a HuggingFace model id, so callers can size chunks with the
            // exact tokenizer their embedder uses.
            let registered = crate::plugins::registry::get_tokenizer_backend_registry()
                .read()
                .lookup(model);
            if let Some(backend) = registered {
                let chunk_config = ChunkConfig::new(ChunkCapacity::new(config.max_characters))
                    .with_sizer(TokenizerBackendSizer(backend))
                    .with_overlap(config.overlap)
                    .map(|c| c.with_trim(config.trim))
                    .map_err(|e| crate::XbergError::validation(format!("Invalid chunking configuration: {}", e)))?;
                split_with_config(text, &config.chunker_type, chunk_config)
            } else {
                // `load_tokenizer` already names the tokenizer and the load failure;
                // append the registration hint to validation failures and pass any
                // other error (e.g. a poisoned cache lock) through untouched.
                let tokenizer = super::tokenizer_cache::get_or_init_tokenizer(model).map_err(|e| match e {
                    crate::XbergError::Validation { message, source } => crate::XbergError::Validation {
                        message: format!(
                            "{message}. No tokenizer backend is registered under '{model}' either; \
                             use a HuggingFace model id, or register your own tokenizer with \
                             register_tokenizer_backend."
                        ),
                        source,
                    },
                    other => other,
                })?;
                let chunk_config = ChunkConfig::new(ChunkCapacity::new(config.max_characters))
                    .with_sizer((*tokenizer).clone())
                    .with_overlap(config.overlap)
                    .map(|c| c.with_trim(config.trim))
                    .map_err(|e| crate::XbergError::validation(format!("Invalid chunking configuration: {}", e)))?;
                split_with_config(text, &config.chunker_type, chunk_config)
            }
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

    // For Markdown chunker with RepeatHeader mode, prepend the table header to
    // every continuation chunk so downstream consumers retain column context.
    if config.chunker_type == ChunkerType::Markdown && config.table_chunking == TableChunkingMode::RepeatHeader {
        inject_table_headers(&mut chunks);
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

/// Adapts a registered [`crate::plugins::TokenizerBackend`] to the splitter's
/// [`ChunkSizer`] interface: chunk size is the backend's token count.
///
/// A backend reporting zero tokens for non-empty text is not trusted: a zero
/// count would make every span appear to fit any budget and silently produce
/// oversized chunks. Host-language bridges surface backend exceptions as a
/// zero count, so this is also the failure mode of a backend that starts
/// erroring mid-run. The sizer falls back to the character count — tokens
/// don't exceed characters for practical tokenizers, so the budget degrades
/// to the conservative `max_characters` semantics instead of an unbounded
/// chunk — and logs the substitution.
#[cfg(feature = "chunking-tokenizers")]
struct TokenizerBackendSizer(std::sync::Arc<dyn crate::plugins::TokenizerBackend>);

#[cfg(feature = "chunking-tokenizers")]
impl ChunkSizer for TokenizerBackendSizer {
    fn size(&self, chunk: &str) -> usize {
        let count = self.0.count_tokens(chunk);
        if count == 0 && !chunk.is_empty() {
            tracing::warn!(
                backend = self.0.name(),
                chunk_len = chunk.len(),
                "Tokenizer backend reported zero tokens for non-empty text; using character count instead"
            );
            return chunk.chars().count();
        }
        count
    }
}

/// Split text using the appropriate splitter type with a generic sizer.
fn split_with_config<'a, S: ChunkSizer>(
    text: &'a str,
    chunker_type: &ChunkerType,
    config: ChunkConfig<S>,
) -> Vec<&'a str> {
    match chunker_type {
        ChunkerType::Text | ChunkerType::Yaml | ChunkerType::Semantic => {
            TextSplitter::new(config).chunks(text).collect()
        }
        ChunkerType::Markdown => MarkdownSplitter::new(config).chunks(text).collect(),
    }
}

/// Prepend the table header to every chunk that is a continuation of a split table.
///
/// A "table header" is two consecutive lines where the first starts with `|` and
/// the second is a separator (`|---…` or `|:--…`). A "continuation chunk" is one
/// that starts with a `|`-prefixed line but does NOT already have a separator line
/// within its first two lines — i.e. it is mid-table rows without a header.
///
/// Only called when `table_chunking == RepeatHeader` and `chunker_type == Markdown`.
fn inject_table_headers(chunks: &mut [crate::types::Chunk]) {
    // Extract the table header from a chunk: header row + separator row as a
    // standalone string. Returns None if the chunk contains no complete table header.
    //
    // The injected header uses the same line terminator as the chunk body so that
    // CRLF input produces a CRLF-consistent header.
    fn extract_table_header(content: &str) -> Option<String> {
        // Detect the body's line terminator: prefer \r\n if present, fall back to \n.
        let line_ending = if content.contains("\r\n") { "\r\n" } else { "\n" };

        let mut lines = content.lines();
        // `.lines()` strips the terminator; strip_line_terminator keeps interior
        // whitespace intact while removing only the trailing \r\n or \n.
        let first_raw = lines.next()?;
        let first = strip_line_terminator(first_raw);
        if !first.starts_with('|') {
            return None;
        }
        let second_raw = lines.next()?;
        let second = strip_line_terminator(second_raw);
        if !is_table_separator(second) {
            return None;
        }
        Some(format!("{first}{line_ending}{second}{line_ending}"))
    }

    // Strip only the trailing line terminator (\r\n or \n) from a line, leaving
    // interior whitespace (cell padding, alignment) untouched.
    fn strip_line_terminator(line: &str) -> &str {
        line.strip_suffix("\r\n")
            .or_else(|| line.strip_suffix('\n'))
            .unwrap_or(line)
    }

    // Return true if `line` is a GFM table separator row.
    //
    // A valid separator row looks like `|:---|:---:|---:|---|`. It must:
    //   - start with `|`
    //   - when split on `|`, yield at least one non-empty cell
    //   - have EVERY non-empty cell match `^\s*:?-+:?\s*$` (optional leading/trailing
    //     colon, one-or-more dashes, optional surrounding whitespace)
    //
    // This rejects data rows whose cells happen to contain a single dash (e.g. `| - |`
    // or `| :- |`) because those cells have fewer than one dash between the colons, or
    // more precisely because `:` must be adjacent to dashes in the separator pattern.
    //
    // Implemented with char scanning to avoid a regex dependency.
    fn is_table_separator(line: &str) -> bool {
        if !line.starts_with('|') {
            return false;
        }
        // Split on `|`; the leading `|` produces a leading empty segment and the
        // trailing `|` (if present) produces a trailing empty segment — skip both.
        let cells: Vec<&str> = line.split('|').filter(|c| !c.is_empty()).collect();
        if cells.is_empty() {
            return false;
        }
        cells.iter().all(|cell| is_separator_cell(cell))
    }

    // Return true if `cell` matches the GFM separator-cell pattern:
    //   optional leading whitespace, optional ':', two or more '-', optional ':', optional trailing whitespace.
    //
    // We require at least two dashes (not one) because a single dash (`-`) is
    // indistinguishable from a data cell in common Markdown tables. Real GFM
    // separator rows use `---` or longer. This prevents `| - |` from being
    // misidentified as a separator row.
    fn is_separator_cell(cell: &str) -> bool {
        // Strip surrounding whitespace only; preserve colons and dashes.
        let s = cell.trim_matches(|c: char| c == ' ' || c == '\t');
        // Strip optional leading colon.
        let s = s.strip_prefix(':').unwrap_or(s);
        // Must start with at least one '-'.
        if !s.starts_with('-') {
            return false;
        }
        // Strip ALL leading dashes; count how many there were.
        let after_dashes = s.trim_start_matches('-');
        let dash_count = s.len() - after_dashes.len();
        // Require at least two dashes to reject ambiguous single-dash data cells.
        if dash_count < 2 {
            return false;
        }
        // After the dashes, only an optional trailing colon (and nothing else) is allowed.
        matches!(after_dashes, "" | ":")
    }

    // Return true if a chunk starts with table rows but has no header separator
    // within the first two lines (i.e. it is a continuation, not a table start).
    fn is_table_continuation(content: &str) -> bool {
        let trimmed = content.trim_start();
        if !trimmed.starts_with('|') {
            return false;
        }
        let mut lines = trimmed.lines();
        let first = lines.next().unwrap_or("").trim();
        if !first.starts_with('|') {
            return false;
        }
        let second = lines.next().unwrap_or("").trim();
        // If the second line is a separator this chunk already has a header.
        !is_table_separator(second)
    }

    let mut last_header: Option<String> = None;

    for chunk in chunks.iter_mut() {
        let content = &chunk.content;

        if let Some(header) = extract_table_header(content) {
            last_header = Some(header);
        } else if is_table_continuation(content) {
            if let Some(ref header) = last_header {
                chunk.content = format!("{header}{}", chunk.content);
            }
        } else {
            // Non-table chunk resets the header context.
            last_header = None;
        }
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
/// use xberg::chunking::{chunk_text_with_type, ChunkerType};
///
/// # fn example() -> xberg::Result<()> {
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
/// use xberg::chunking::{chunk_texts_batch, ChunkingConfig};
///
/// # fn example() -> xberg::Result<()> {
/// let config = ChunkingConfig::default();
/// let texts: Vec<String> = vec!["First text".to_string(), "Second text".to_string()];
/// let results = chunk_texts_batch(&texts, &config)?;
/// assert_eq!(results.len(), 2);
/// # Ok(())
/// # }
/// ```
#[cfg(test)]
pub(crate) fn chunk_texts_batch(texts: &[String], config: &ChunkingConfig) -> Result<Vec<ChunkingResult>> {
    texts.iter().map(|text| chunk_text(text, config, None)).collect()
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::XbergError;

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
        assert!(matches!(err, XbergError::Validation { .. }));
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

    // -----------------------------------------------------------------------
    // Issue #1100: table chunks must retain header in RepeatHeader mode
    // -----------------------------------------------------------------------

    fn make_large_table(rows: usize) -> String {
        let mut s = "| Name | Value | Description |\n|------|-------|-------------|\n".to_string();
        for i in 0..rows {
            s.push_str(&format!(
                "| item{i} | {i} | Description of item {i} with some extra text |\n"
            ));
        }
        s
    }

    #[test]
    fn table_repeat_header_prepends_to_continuation_chunks() {
        let markdown = make_large_table(40);
        let config = ChunkingConfig {
            max_characters: 300,
            overlap: 0,
            trim: true,
            chunker_type: ChunkerType::Markdown,
            table_chunking: TableChunkingMode::RepeatHeader,
            ..Default::default()
        };
        let result = chunk_text(&markdown, &config, None).unwrap();
        assert!(result.chunks.len() > 1, "table must split into multiple chunks");

        for chunk in &result.chunks {
            let trimmed = chunk.content.trim_start();
            if trimmed.starts_with('|') {
                assert!(
                    chunk.content.contains("|------|"),
                    "chunk missing separator row (header must be prepended):\n{:?}",
                    chunk.content,
                );
                assert!(
                    chunk.content.contains("| Name | Value | Description |"),
                    "chunk missing header row:\n{:?}",
                    chunk.content,
                );
            }
        }
    }

    #[test]
    fn table_split_mode_default_leaves_continuation_chunks_without_header() {
        let markdown = make_large_table(40);
        let config = ChunkingConfig {
            max_characters: 300,
            overlap: 0,
            trim: true,
            chunker_type: ChunkerType::Markdown,
            // Default: Split — no header injection
            ..Default::default()
        };
        let result = chunk_text(&markdown, &config, None).unwrap();
        assert!(result.chunks.len() > 1, "table must split into multiple chunks");

        // At least one continuation chunk must lack the header (proving default unchanged)
        let continuation_without_header = result.chunks.iter().skip(1).any(|c| {
            let t = c.content.trim_start();
            t.starts_with('|') && !c.content.contains("|------|")
        });
        assert!(
            continuation_without_header,
            "default Split mode must not inject headers into continuation chunks"
        );
    }

    #[test]
    fn table_repeat_header_two_split_tables_each_get_own_header() {
        // Two tables both large enough to split. Each table's continuation chunks
        // must carry THAT table's header, not the other table's header.
        let table1 = "| Name | Value | Description |\n|------|-------|-------------|\n".to_string()
            + &"| item | val | some description text here |\n".repeat(30);
        let separator = "\n\n---\n\n";
        let table2 = "| Alpha | Beta | Gamma |\n|-------|------|-------|\n".to_string()
            + &"| alpha | beta | gamma value here |\n".repeat(30);
        let markdown = format!("{table1}{separator}{table2}");

        let config = ChunkingConfig {
            max_characters: 300,
            overlap: 0,
            trim: true,
            chunker_type: ChunkerType::Markdown,
            table_chunking: TableChunkingMode::RepeatHeader,
            ..Default::default()
        };
        let result = chunk_text(&markdown, &config, None).unwrap();
        assert!(result.chunks.len() > 2, "both tables must produce multiple chunks");

        // Every chunk that starts with a `|` row must have a header + separator
        // within its first two lines (via injection or natural start).
        for chunk in &result.chunks {
            let trimmed = chunk.content.trim_start();
            if !trimmed.starts_with('|') {
                continue;
            }
            let mut lines = trimmed.lines();
            lines.next(); // header row
            let second = lines.next().unwrap_or("").trim();
            assert!(
                second.starts_with('|') && second.contains('-'),
                "table chunk missing separator on second line:\n{:?}",
                chunk.content,
            );
        }

        // Chunks that contain table2 content must NOT have table1's header.
        for chunk in &result.chunks {
            if chunk.content.contains("alpha") || chunk.content.contains("Alpha") {
                assert!(
                    !chunk.content.contains("Name") || chunk.content.contains("Alpha"),
                    "table2 chunk must not be contaminated with table1 header:\n{:?}",
                    chunk.content,
                );
            }
        }
    }

    #[test]
    fn table_repeat_header_text_chunker_is_unaffected() {
        // RepeatHeader is a no-op when chunker_type is Text.
        let markdown = make_large_table(40);
        let config = ChunkingConfig {
            max_characters: 300,
            overlap: 0,
            trim: true,
            chunker_type: ChunkerType::Text,
            table_chunking: TableChunkingMode::RepeatHeader,
            ..Default::default()
        };
        let result = chunk_text(&markdown, &config, None).unwrap();
        // Text chunker should not crash and default split behaviour applies.
        assert!(!result.chunks.is_empty());
    }

    #[test]
    fn table_repeat_header_single_chunk_table_unchanged() {
        // A table that fits in one chunk must not be duplicated.
        let markdown = "| Col1 | Col2 |\n|------|------|\n| A    | B    |\n| C    | D    |\n";
        let config = ChunkingConfig {
            max_characters: 5000,
            overlap: 0,
            trim: true,
            chunker_type: ChunkerType::Markdown,
            table_chunking: TableChunkingMode::RepeatHeader,
            ..Default::default()
        };
        let result = chunk_text(markdown, &config, None).unwrap();
        assert_eq!(result.chunks.len(), 1, "small table must stay in one chunk");
        assert_eq!(
            result.chunks[0].content.matches("|------|").count(),
            1,
            "header must appear exactly once, not be duplicated"
        );
    }

    #[test]
    fn table_repeat_header_with_overlap_prepends_header_to_continuation_chunks() {
        // With overlap > 0, continuation chunks start with rows from the tail of the
        // previous chunk. The is_table_continuation check still fires correctly because
        // those rows start with `|` and the second line is also a data row (no separator).
        let markdown = make_large_table(40);
        let config = ChunkingConfig {
            max_characters: 300,
            overlap: 50,
            trim: true,
            chunker_type: ChunkerType::Markdown,
            table_chunking: TableChunkingMode::RepeatHeader,
            ..Default::default()
        };
        let result = chunk_text(&markdown, &config, None).unwrap();
        assert!(
            result.chunks.len() > 2,
            "table must split into multiple chunks with overlap"
        );
        // Every chunk that starts with `|` (table content) must have a separator on its
        // second non-empty line — either it's the header chunk itself, or the header was
        // injected into a continuation chunk.
        for chunk in &result.chunks {
            let lines: Vec<&str> = chunk.content.lines().filter(|l| !l.is_empty()).collect();
            if lines.first().is_some_and(|l| l.starts_with('|')) {
                let second_line_is_separator = lines
                    .get(1)
                    .is_some_and(|l| l.chars().all(|c| matches!(c, '|' | '-' | ' ')));
                assert!(
                    second_line_is_separator,
                    "table chunk must have separator on 2nd line; got:\n{}",
                    chunk.content
                );
            }
        }
    }

    #[test]
    fn table_repeat_header_oversized_header_does_not_panic() {
        // When the header row itself exceeds max_characters it becomes its own chunk.
        // inject_table_headers will prepend it to continuation chunks, producing chunks
        // larger than max_characters. This is documented accepted behaviour — the
        // invariant is no panic and no data loss, not size capping.
        let wide_cols: String = (0..20).map(|i| format!("| Column {i:02} ")).collect::<String>() + "|";
        let separator: String = (0..20).map(|_| "|----------").collect::<String>() + "|";
        let row: String = (0..20).map(|i| format!("| value  {i:02} ")).collect::<String>() + "|";
        let rows: String = (0..20).map(|_| format!("{row}\n")).collect::<String>();
        let markdown = format!("{wide_cols}\n{separator}\n{rows}");
        let config = ChunkingConfig {
            max_characters: 50,
            overlap: 0,
            trim: true,
            chunker_type: ChunkerType::Markdown,
            table_chunking: TableChunkingMode::RepeatHeader,
            ..Default::default()
        };
        let result = chunk_text(&markdown, &config, None).unwrap();
        assert!(
            !result.chunks.is_empty(),
            "must produce chunks even with oversized header"
        );
    }

    // -----------------------------------------------------------------------
    // is_table_separator false-positive fixes
    // -----------------------------------------------------------------------

    /// A data row whose cell contains a lone dash (`| - |`) must NOT be treated
    /// as a separator row and must NOT trigger spurious header injection.
    #[test]
    fn table_repeat_header_single_dash_data_cell_not_misidentified_as_separator() {
        // "| - |" in a data row is a literal dash value, not a GFM separator.
        // The table below has a valid header + separator on lines 1-2, then data rows
        // where one cell is literally " - ". None of the data rows should be
        // mistaken for a separator, so no spurious header injection should occur.
        let markdown = "| Item | Status |\n|------|--------|\n| foo  | - |\n| bar  | done |\n| baz  | :- |\n";
        let config = ChunkingConfig {
            max_characters: 5000,
            overlap: 0,
            trim: true,
            chunker_type: ChunkerType::Markdown,
            table_chunking: TableChunkingMode::RepeatHeader,
            ..Default::default()
        };
        let result = chunk_text(markdown, &config, None).unwrap();
        // The whole table fits in one chunk — header must appear exactly once.
        assert_eq!(result.chunks.len(), 1, "small table must fit in one chunk");
        assert_eq!(
            result.chunks[0].content.matches("|------|").count(),
            1,
            "separator must appear exactly once — no spurious injection from data rows with dashes:\n{:?}",
            result.chunks[0].content,
        );
        // Data rows must be present and unmodified.
        assert!(
            result.chunks[0].content.contains("| foo  | - |"),
            "data row with lone-dash cell must be preserved:\n{:?}",
            result.chunks[0].content,
        );
        assert!(
            result.chunks[0].content.contains("| baz  | :- |"),
            "data row with colon-dash cell must be preserved:\n{:?}",
            result.chunks[0].content,
        );
    }

    /// A CRLF-input table whose rows are split across chunks must produce continuation
    /// chunks where the injected header uses `\r\n` consistently with the body.
    #[test]
    fn table_repeat_header_crlf_input_header_uses_crlf_line_endings() {
        // Build a CRLF table large enough to force a split.
        let header = "| Name | Value |\r\n|------|-------|\r\n";
        let data_rows: String = (0..40).map(|i| format!("| item{i:02} | {i:03} |\r\n")).collect();
        let markdown = format!("{header}{data_rows}");

        let config = ChunkingConfig {
            max_characters: 200,
            overlap: 0,
            trim: true,
            chunker_type: ChunkerType::Markdown,
            table_chunking: TableChunkingMode::RepeatHeader,
            ..Default::default()
        };
        let result = chunk_text(&markdown, &config, None).unwrap();
        assert!(result.chunks.len() > 1, "CRLF table must split into multiple chunks");

        // Every continuation chunk that starts with `|` must have its injected header
        // terminated with \r\n, not \n, so that line endings are consistent with the body.
        for (i, chunk) in result.chunks.iter().enumerate().skip(1) {
            let trimmed = chunk.content.trim_start();
            if !trimmed.starts_with('|') {
                continue;
            }
            // The first line of the chunk (the injected header row) must end with \r\n.
            // Find the position of the first line break in the content.
            assert!(
                chunk.content.contains("\r\n"),
                "continuation chunk {} must contain \\r\\n line endings from injected header, but got:\n{:?}",
                i,
                chunk.content,
            );
            // The injected header's first line (`| Name | Value |`) must be \r\n
            // terminated, proving the injected header uses CRLF consistently with
            // the body rather than a bare \n.
            assert!(
                chunk.content.contains("| Name | Value |\r\n"),
                "continuation chunk {} injected header row must end with \\r\\n, not \\n:\n{:?}",
                i,
                chunk.content,
            );
            // The injected separator row (`|------|-------|`) must also end with \r\n.
            assert!(
                chunk.content.contains("|-------|\r\n"),
                "continuation chunk {} injected separator must end with \\r\\n, not \\n:\n{:?}",
                i,
                chunk.content,
            );
        }
    }

    /// Tables using GFM alignment separators (`:---`, `---:`, `:---:`) must still be
    /// correctly detected and their headers injected into continuation chunks.
    #[test]
    fn table_repeat_header_alignment_separators_are_detected_correctly() {
        // Use left-align, right-align, and center-align separator cells.
        let header = "| Left | Right | Center |\n|:-----|------:|:------:|\n";
        let data_rows: String = (0..40).map(|i| format!("| l{i:02} | r{i:02} | c{i:02} |\n")).collect();
        let markdown = format!("{header}{data_rows}");

        let config = ChunkingConfig {
            max_characters: 200,
            overlap: 0,
            trim: true,
            chunker_type: ChunkerType::Markdown,
            table_chunking: TableChunkingMode::RepeatHeader,
            ..Default::default()
        };
        let result = chunk_text(&markdown, &config, None).unwrap();
        assert!(
            result.chunks.len() > 1,
            "alignment-separator table must split into multiple chunks"
        );

        // Every chunk starting with `|` must carry the header row.
        for chunk in &result.chunks {
            let trimmed = chunk.content.trim_start();
            if !trimmed.starts_with('|') {
                continue;
            }
            assert!(
                chunk.content.contains("| Left | Right | Center |"),
                "alignment-separator table chunk missing header row:\n{:?}",
                chunk.content,
            );
            // The separator row with alignment markers must appear exactly once per chunk.
            assert!(
                chunk.content.contains("|:-----|------:|:------:|"),
                "alignment-separator table chunk missing alignment separator row:\n{:?}",
                chunk.content,
            );
        }
    }
}
