//! Regression tests for issue 1004: nested list content duplication.
//!
//! Two bugs tracked together:
//!
//! 1. html-to-markdown-rs emitted malformed/duplicated Markdown for nested
//!    `ul > li > ul > li > ol` HTML structures.
//!    Fixed in html-to-markdown-rs 3.5.0 (xberg-io/html-to-markdown#385).
//!
//! 2. The Markdown chunker panicked on the malformed Markdown from (1).
//!    The underflow originated in `sep.start - offset` in
//!    `text_splitter/splitter.rs`.
//!
//! Fix contract:
//!   - `chunk_text` with `ChunkerType::Markdown` MUST NOT PANIC on any input.
//!   - HTML extraction of deeply nested mixed lists MUST NOT duplicate content.

#![cfg(feature = "chunking")]

mod helpers;

use xberg::chunking::{ChunkerType, ChunkingConfig, chunk_text};

/// Malformed Markdown produced by html-to-markdown on a `ul > li > ul > li > ol`
/// HTML snippet before the fix.  Passing this to the chunker triggered an
/// integer underflow → panic.
const MALFORMED_NESTED_LIST_MD: &str = "\
Title
  *     1. Item 1
    2. Item 2
    3. Item 3
    4. Item 4
    5. Item 5
    6. Item 6
    7. Item 7
    8. Item 8
    9. Item 9
    10. Item 10
    11. Item 11
1. Item 1
    2. Item 2
    3. Item 3
Item 1
Item 2
Item 3
";

fn markdown_chunk_config() -> ChunkingConfig {
    ChunkingConfig {
        max_characters: 200,
        overlap: 0,
        chunker_type: ChunkerType::Markdown,
        ..Default::default()
    }
}

/// The chunker must return Ok or Err — never panic — on any input string.
#[test]
fn markdown_chunker_never_panics_on_malformed_nested_list() {
    let result = chunk_text(MALFORMED_NESTED_LIST_MD, &markdown_chunk_config(), None);
    // We don't assert the content — just that no panic occurred.
    // Both Ok and Err are acceptable; a panic is not.
    let _ = result;
}

/// Chunker must not panic on completely empty input.
#[test]
fn markdown_chunker_empty_input_returns_empty_chunks() {
    let result = chunk_text("", &markdown_chunk_config(), None);
    assert!(result.is_ok());
    assert!(result.unwrap().chunks.is_empty());
}

/// Chunker must not panic on single-character input; Ok or Err is acceptable.
#[test]
fn markdown_chunker_single_char_input() {
    let result = chunk_text("x", &markdown_chunk_config(), None);
    let _ = result;
}

/// Chunker must not panic on input consisting only of newlines; Ok or Err is acceptable.
#[test]
fn markdown_chunker_only_newlines() {
    let result = chunk_text("\n\n\n\n", &markdown_chunk_config(), None);
    let _ = result;
}

/// Chunker must not panic on a valid deeply-nested Markdown list (the
/// well-formed counterpart of the malformed input above).
#[test]
fn markdown_chunker_valid_nested_list() {
    let valid_nested = "\
- outer 1
  - mid 1
    1. inner 1
    2. inner 2
  - mid 2
- outer 2
";
    let result = chunk_text(valid_nested, &markdown_chunk_config(), None);
    assert!(result.is_ok(), "valid nested list must chunk without error");
}

#[cfg(feature = "html")]
mod html_extraction {
    use crate::helpers::extract_bytes_document_blocking;
    use xberg::chunking::{ChunkerType, ChunkingConfig};
    use xberg::core::config::{ExtractionConfig, OutputFormat};

    fn config_with_markdown_chunking() -> ExtractionConfig {
        ExtractionConfig {
            output_format: OutputFormat::Markdown,
            chunking: Some(ChunkingConfig {
                max_characters: 300,
                chunker_type: ChunkerType::Markdown,
                ..Default::default()
            }),
            ..Default::default()
        }
    }

    /// Extracting the nested-list HTML must not panic and must not duplicate
    /// list item content in the extraction result.
    #[test]
    fn html_nested_list_extraction_no_panic() {
        let html = b"<ul><li>outer<ul><li>mid<ol><li>inner1</li><li>inner2</li></ol></li></ul></li></ul>";
        let result = extract_bytes_document_blocking(html, "text/html", &ExtractionConfig::default());
        assert!(result.is_ok(), "extraction must not error: {:?}", result.err());
    }

    /// After fixing html-to-markdown: every list item must appear exactly once.
    #[test]
    fn html_nested_list_no_content_duplication() {
        let html = b"<ul><li>outer<ul><li>mid<ol><li>inner1</li><li>inner2</li></ol></li></ul></li></ul>";
        let result = extract_bytes_document_blocking(html, "text/html", &ExtractionConfig::default())
            .expect("extraction must not error");
        let content = &result.content;
        for word in ["outer", "mid", "inner1", "inner2"] {
            assert_eq!(
                content.matches(word).count(),
                1,
                "{word} must appear exactly once, got content:\n{content}"
            );
        }
    }

    /// Passing the malformed markdown from step 1 into a second extraction
    /// with Markdown chunking must not panic (original bug report scenario).
    #[test]
    fn second_pass_markdown_chunking_no_panic() {
        let first = extract_bytes_document_blocking(
            b"<ul><li>outer<ul><li>mid<ol><li>inner1</li><li>inner2</li></ol></li></ul></li></ul>",
            "text/html",
            &ExtractionConfig::default(),
        )
        .expect("first extraction must not error");

        // Pass result.content back as Markdown bytes — this is the exact
        // scenario from the issue that triggered the panic.
        let second =
            extract_bytes_document_blocking(first.content.as_bytes(), "text/plain", &config_with_markdown_chunking());
        // Must not panic; Ok or Err are both acceptable.
        let _ = second;
    }
}
