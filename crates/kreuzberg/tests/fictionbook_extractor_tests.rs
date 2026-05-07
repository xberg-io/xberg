#![cfg(feature = "office")]

use kreuzberg::core::config::{ExtractionConfig, OutputFormat};
use std::path::PathBuf;

/// Helper to get absolute path to test documents
fn test_file_path(filename: &str) -> PathBuf {
    let manifest_dir = env!("CARGO_MANIFEST_DIR");
    PathBuf::from(manifest_dir)
        .parent()
        .expect("Operation failed")
        .parent()
        .expect("Operation failed")
        .join("test_documents")
        .join("fictionbook")
        .join(filename)
}

#[tokio::test]
async fn test_fictionbook_extract_metadata_title() {
    let path = test_file_path("meta.fb2");
    let result = kreuzberg::extract_file(&path, None, &ExtractionConfig::default())
        .await
        .expect("Failed to extract FB2 file");

    assert!(
        result.content.contains("Book title"),
        "Book title should be extracted from FB2 content"
    );
}

#[tokio::test]
async fn test_fictionbook_extract_metadata_genre() {
    let path = test_file_path("meta.fb2");
    let result = kreuzberg::extract_file(&path, None, &ExtractionConfig::default())
        .await
        .expect("Failed to extract FB2 file");

    assert!(result.metadata.subject.is_none());
}

#[tokio::test]
async fn test_fictionbook_extract_content_sections() {
    let path = test_file_path("titles.fb2");
    let result = kreuzberg::extract_file(&path, None, &ExtractionConfig::default())
        .await
        .expect("Failed to extract FB2 file");

    assert!(
        result.content.contains("Simple title"),
        "Section titles should be extracted"
    );
    assert!(
        result.content.contains("Emphasized"),
        "Section with emphasis should be extracted"
    );
}

#[tokio::test]
async fn test_fictionbook_extract_section_hierarchy() {
    let path = test_file_path("basic.fb2");
    let result = kreuzberg::extract_file(&path, None, &ExtractionConfig::default())
        .await
        .expect("Failed to extract FB2 file");

    assert!(
        result.content.contains("Top-level title"),
        "Top-level section should be extracted"
    );
    assert!(result.content.contains("Section"), "Nested section should be extracted");
    assert!(
        result.content.contains("Subsection"),
        "Nested subsection should be extracted"
    );
}

#[tokio::test]
async fn test_fictionbook_extract_inline_markup() {
    let path = test_file_path("emphasis.fb2");
    let result = kreuzberg::extract_file(&path, None, &ExtractionConfig::default())
        .await
        .expect("Failed to extract FB2 file");

    let content = result.content.to_lowercase();
    assert!(content.contains("plain"), "Plain text should be extracted");
    assert!(content.contains("strong"), "Strong emphasis should be extracted");
    assert!(content.contains("emphasis"), "Emphasis should be extracted");
    assert!(content.contains("strikethrough"), "Strikethrough should be extracted");
}

#[tokio::test]
async fn test_fictionbook_extract_emphasis() {
    let path = test_file_path("basic.fb2");
    let result = kreuzberg::extract_file(&path, None, &ExtractionConfig::default())
        .await
        .expect("Failed to extract FB2 file");

    assert!(
        result.content.contains("emphasized"),
        "Emphasized text should be extracted"
    );
}

#[tokio::test]
async fn test_fictionbook_extract_strong() {
    let path = test_file_path("basic.fb2");
    let result = kreuzberg::extract_file(&path, None, &ExtractionConfig::default())
        .await
        .expect("Failed to extract FB2 file");

    assert!(result.content.contains("strong"), "Strong text should be extracted");
}

#[tokio::test]
async fn test_fictionbook_extract_code() {
    let path = test_file_path("basic.fb2");
    let result = kreuzberg::extract_file(&path, None, &ExtractionConfig::default())
        .await
        .expect("Failed to extract FB2 file");

    assert!(result.content.contains("verbatim"), "Code content should be extracted");
}

#[tokio::test]
async fn test_fictionbook_extract_blockquote() {
    let path = test_file_path("basic.fb2");
    let result = kreuzberg::extract_file(&path, None, &ExtractionConfig::default())
        .await
        .expect("Failed to extract FB2 file");

    assert!(result.content.contains("Blockquote"), "Blockquote should be extracted");
}

#[tokio::test]
async fn test_fictionbook_extract_tables() {
    let path = test_file_path("tables.fb2");
    let result = kreuzberg::extract_file(&path, None, &ExtractionConfig::default())
        .await
        .expect("Failed to extract FB2 file");

    assert!(
        !result.content.is_empty(),
        "Content should be extracted from file with tables"
    );
}

#[tokio::test]
async fn test_fictionbook_markdown_formatting_preservation() {
    let path = test_file_path("emphasis.fb2");
    let config = ExtractionConfig {
        output_format: OutputFormat::Markdown,
        ..Default::default()
    };
    let result = kreuzberg::extract_file(&path, None, &config)
        .await
        .expect("Failed to extract FB2 file");

    let md = &result.content;
    assert!(
        md.contains("**strong**"),
        "Strong text should be formatted as **bold** in markdown"
    );
    assert!(
        md.contains("*emphasis*"),
        "Emphasis text should be formatted as *italic* in markdown"
    );
    assert!(
        md.contains("~~deleted~~"),
        "Strikethrough text should be formatted as ~~strikethrough~~ in markdown"
    );
    assert!(
        md.contains("`code`"),
        "Code text should be wrapped in backticks in markdown"
    );
}

#[tokio::test]
async fn test_fictionbook_formatting_in_body_paragraphs() {
    let path = test_file_path("basic.fb2");
    let config = ExtractionConfig {
        output_format: OutputFormat::Markdown,
        ..Default::default()
    };
    let result = kreuzberg::extract_file(&path, None, &config)
        .await
        .expect("Failed to extract FB2 file");

    let md = &result.content;
    assert!(
        md.contains("*emphasized*"),
        "Emphasis formatting should be preserved in body content"
    );
    assert!(
        md.contains("**strong**"),
        "Strong formatting should be preserved in body content"
    );
    assert!(
        md.contains("`verbatim`"),
        "Code formatting should be preserved in body content"
    );
}
