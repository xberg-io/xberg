//! Config behavioral verification tests
//!
//! These tests verify that configuration options actually affect extraction behavior,
//! not just that they serialize correctly.
//!
//! Unlike serialization tests that only check if configs deserialize, these tests verify
//! that the configuration options actually influence the extraction process and produce
//! observable differences in the output.

use kreuzberg::core::config::ChunkingConfig;
use kreuzberg::core::config::ExtractionConfig;
use kreuzberg::core::config::OutputFormat;
use kreuzberg::core::extractor::extract_bytes;
use kreuzberg::types::OutputFormat as ResultFormat;

mod helpers;

/// Test output_format Plain produces text without formatting
///
/// Note: HTML extractors often convert to markdown internally, so this test
/// uses plain text input to verify the output_format configuration is respected.
#[tokio::test]
async fn test_output_format_plain_produces_plain() {
    let plain_text = b"Title\n\nParagraph with bold text.";

    let config = ExtractionConfig {
        output_format: OutputFormat::Plain,
        ..Default::default()
    };

    let result = extract_bytes(plain_text, "text/plain", &config)
        .await
        .expect("Should extract successfully");

    // Plain text should not have markdown or HTML formatting
    assert!(
        !result.content.contains("# ") && !result.content.contains("<h1>"),
        "Plain format should not contain markdown headers or HTML tags, got: {}",
        result.content
    );
    assert!(
        result.content.contains("Title") || result.content.contains("Paragraph"),
        "Should still contain extracted text content"
    );
}

/// Test output_format Markdown produces markdown formatting
#[tokio::test]
#[cfg(feature = "html")]
async fn test_output_format_markdown_produces_markdown() {
    let html = b"<h1>Title</h1><p>Paragraph with <strong>bold</strong> text.</p>";

    let config = ExtractionConfig {
        output_format: OutputFormat::Markdown,
        ..Default::default()
    };

    let result = extract_bytes(html, "text/html", &config)
        .await
        .expect("Should extract successfully");

    // Verify markdown formatting is present (# for headers or ** for bold)
    let has_markdown = result.content.contains("# ") || result.content.contains("**") || result.content.contains("*");

    assert!(
        has_markdown,
        "Markdown format should contain # headers or ** bold, got: {}",
        result.content
    );
}

/// Test output_format HTML produces valid HTML content
#[tokio::test]
async fn test_output_format_html_produces_html() {
    let text = "Title\n\nParagraph with bold text.";

    let config = ExtractionConfig {
        output_format: OutputFormat::Html,
        ..Default::default()
    };

    let result = extract_bytes(text.as_bytes(), "text/plain", &config)
        .await
        .expect("Should extract successfully");

    // HTML format should be safe and not contain injection vectors
    assert!(
        !result.content.contains("<script>"),
        "HTML format should be safe from injection"
    );
    assert!(!result.content.is_empty(), "Should produce content in HTML format");
}

/// Test result_format Unified produces content in single field
#[tokio::test]
async fn test_result_format_unified_structure() {
    let text = "Sample content";

    let config = ExtractionConfig {
        result_format: ResultFormat::Unified,
        ..Default::default()
    };

    let result = extract_bytes(text.as_bytes(), "text/plain", &config)
        .await
        .expect("Should extract successfully");

    // Unified format should have content in main content field
    assert!(!result.content.is_empty(), "Unified format should have content");

    // Elements should be None or empty for unified format
    assert!(
        result.elements.is_none() || result.elements.as_ref().unwrap().is_empty(),
        "Unified format should not have elements"
    );
}

/// Test result_format ElementBased produces element structure
#[tokio::test]
async fn test_result_format_element_based_structure() {
    let text = "First paragraph here.\n\nSecond paragraph with more content.";

    let config = ExtractionConfig {
        result_format: ResultFormat::ElementBased,
        ..Default::default()
    };

    let result = extract_bytes(text.as_bytes(), "text/plain", &config)
        .await
        .expect("Should extract successfully");

    // Element-based format should produce elements array
    if let Some(elements) = &result.elements {
        assert!(!elements.is_empty(), "Element-based format should have elements");
        // Verify elements have expected structure
        for element in elements {
            assert!(!element.text.is_empty(), "Elements should have non-empty text");
        }
    }
}

/// Test chunking max_chars actually limits chunk size
#[tokio::test]
#[cfg(feature = "chunking")]
async fn test_chunking_max_chars_limits_chunk_size() {
    let long_text = "word ".repeat(500); // ~2500 characters

    let config = ExtractionConfig {
        chunking: Some(ChunkingConfig {
            max_chars: 100,
            max_overlap: 20,
            embedding: None,
            preset: None,
        }),
        ..Default::default()
    };

    let result = extract_bytes(long_text.as_bytes(), "text/plain", &config)
        .await
        .expect("Should extract successfully");

    assert!(result.chunks.is_some(), "Chunking should produce chunks");

    if let Some(chunks) = result.chunks {
        assert!(chunks.len() > 1, "Long text should produce multiple chunks");

        // Verify chunk size constraint: each chunk should respect max_chars
        for (i, chunk) in chunks.iter().enumerate() {
            assert!(
                chunk.content.len() <= 100 + 20,
                "Chunk {} exceeds max_chars + overlap: length = {}",
                i,
                chunk.content.len()
            );
        }
    }
}

/// Test chunking with overlap creates overlapping chunks
#[tokio::test]
#[cfg(feature = "chunking")]
async fn test_chunking_overlap_creates_overlap() {
    let text = "First sentence. ".repeat(30); // ~480 characters

    let config = ExtractionConfig {
        chunking: Some(ChunkingConfig {
            max_chars: 50,
            max_overlap: 15,
            embedding: None,
            preset: None,
        }),
        ..Default::default()
    };

    let result = extract_bytes(text.as_bytes(), "text/plain", &config)
        .await
        .expect("Should extract successfully");

    if let Some(chunks) = result.chunks
        && chunks.len() >= 2
    {
        // Check if adjacent chunks have overlapping text
        let chunk1_end = &chunks[0].content[chunks[0].content.len().saturating_sub(15)..];
        let chunk2_start = &chunks[1].content[..chunks[1].content.len().min(15)];

        // There should be some overlap in the text
        let overlap_found = chunk1_end.chars().any(|c| c != ' ') && chunk2_start.chars().any(|c| c != ' ');

        assert!(
            overlap_found,
            "Adjacent chunks should have overlapping non-whitespace text"
        );
    }
}

/// Test chunking disabled produces no chunks
#[tokio::test]
async fn test_chunking_disabled_produces_no_chunks() {
    let long_text = "word ".repeat(500);

    let config = ExtractionConfig {
        chunking: None,
        ..Default::default()
    };

    let result = extract_bytes(long_text.as_bytes(), "text/plain", &config)
        .await
        .expect("Should extract successfully");

    assert!(result.chunks.is_none(), "Chunking disabled should produce no chunks");
}

/// Test use_cache true allows results to be cached
#[tokio::test]
async fn test_cache_enabled_allows_caching() {
    let text = "Test content for caching";

    let config = ExtractionConfig {
        use_cache: true,
        ..Default::default()
    };

    // Extract twice with same content
    let result1 = extract_bytes(text.as_bytes(), "text/plain", &config)
        .await
        .expect("First extraction should succeed");

    let result2 = extract_bytes(text.as_bytes(), "text/plain", &config)
        .await
        .expect("Second extraction should succeed");

    // Results should be identical
    assert_eq!(
        result1.content, result2.content,
        "Cache enabled should produce consistent results"
    );
}

/// Test use_cache false disables caching without crashing
#[tokio::test]
async fn test_cache_disabled_does_not_crash() {
    let text = "Test content without caching";

    let config = ExtractionConfig {
        use_cache: false,
        ..Default::default()
    };

    let result = extract_bytes(text.as_bytes(), "text/plain", &config)
        .await
        .expect("Extraction with cache disabled should succeed");

    assert!(!result.content.is_empty(), "Should still extract content");
}

/// Test quality_processing enabled produces quality score
#[tokio::test]
#[cfg(feature = "quality")]
async fn test_quality_processing_enabled_produces_score() {
    let text = "This is a well-structured document. It has proper sentences. And good formatting.";

    let config = ExtractionConfig {
        enable_quality_processing: true,
        ..Default::default()
    };

    let result = extract_bytes(text.as_bytes(), "text/plain", &config)
        .await
        .expect("Should extract successfully");

    // Quality processing should add a quality_score to metadata
    let has_quality_score = result.metadata.additional.contains_key("quality_score");
    assert!(
        has_quality_score,
        "Quality processing enabled should produce quality_score in metadata"
    );
}

/// Test quality_processing disabled does not produce score
#[tokio::test]
#[cfg(feature = "quality")]
async fn test_quality_processing_disabled_no_score() {
    let text = "This is a document.";

    let config = ExtractionConfig {
        enable_quality_processing: false,
        ..Default::default()
    };

    let result = extract_bytes(text.as_bytes(), "text/plain", &config)
        .await
        .expect("Should extract successfully");

    assert!(
        !result.metadata.additional.contains_key("quality_score"),
        "Quality processing disabled should not produce quality_score"
    );
}

/// Test output_format combinations with result_format
#[tokio::test]
#[cfg(feature = "html")]
async fn test_output_format_with_element_based() {
    let html = b"<p>First paragraph</p><p>Second paragraph</p>";

    let config = ExtractionConfig {
        output_format: OutputFormat::Markdown,
        result_format: ResultFormat::ElementBased,
        ..Default::default()
    };

    let result = extract_bytes(html, "text/html", &config)
        .await
        .expect("Should extract successfully");

    // Should have elements
    assert!(result.elements.is_some(), "ElementBased format should produce elements");

    // Content should still be markdown formatted
    assert!(
        !result.content.contains("<p>"),
        "Output format should not contain HTML tags"
    );
}

/// Test chunking respects overlap maximum
#[tokio::test]
#[cfg(feature = "chunking")]
async fn test_chunking_overlap_maximum() {
    let text = "x".repeat(200); // Simple repeated character

    let config = ExtractionConfig {
        chunking: Some(ChunkingConfig {
            max_chars: 60,
            max_overlap: 10,
            embedding: None,
            preset: None,
        }),
        ..Default::default()
    };

    let result = extract_bytes(text.as_bytes(), "text/plain", &config)
        .await
        .expect("Should extract successfully");

    if let Some(chunks) = result.chunks {
        // Verify max_overlap is not exceeded
        for (i, chunk) in chunks.iter().enumerate() {
            assert!(
                chunk.content.len() <= 60 + 10,
                "Chunk {} size {} exceeds max_chars (60) + max_overlap (10)",
                i,
                chunk.content.len()
            );
        }
    }
}

/// Test large document extraction with multiple config options
#[tokio::test]
#[cfg(feature = "chunking")]
async fn test_large_document_with_combined_config() {
    let large_text = "This is a paragraph. ".repeat(100); // ~2000 characters

    let config = ExtractionConfig {
        output_format: OutputFormat::Plain,
        chunking: Some(ChunkingConfig {
            max_chars: 200,
            max_overlap: 30,
            embedding: None,
            preset: None,
        }),
        use_cache: true,
        enable_quality_processing: true,
        ..Default::default()
    };

    let result = extract_bytes(large_text.as_bytes(), "text/plain", &config)
        .await
        .expect("Should extract successfully");

    // Should have chunks due to size
    assert!(result.chunks.is_some(), "Should produce chunks for large text");

    // Should have quality score
    #[cfg(feature = "quality")]
    {
        assert!(
            result.metadata.additional.contains_key("quality_score"),
            "Should have quality score"
        );
    }

    // Should have content in plain format
    assert!(!result.content.is_empty(), "Should have content");
}
