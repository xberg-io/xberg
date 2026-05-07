//! TDD tests for DOCX formatting, heading hierarchy, lists, and hyperlinks.
//!
//! These tests verify that DOCX extraction produces high-quality markdown output
//! with proper formatting preservation (bold, italic, underline, hyperlinks),
//! heading hierarchy, list rendering, and document structure.

#![cfg(feature = "office")]

mod helpers;

use helpers::{assert_non_empty_content, get_test_file_path};
use kreuzberg::{ExtractionConfig, OutputFormat};
use kreuzberg::extract_file;

// ---------------------------------------------------------------------------
// Formatting tests
// ---------------------------------------------------------------------------

#[tokio::test]
async fn test_docx_bold_rendered_as_markdown() {
    let path = get_test_file_path("docx/unit_test_formatting.docx");
    if !path.exists() {
        return;
    }

    let config = ExtractionConfig {
        output_format: OutputFormat::Markdown,
        ..Default::default()
    };
    let result = extract_file(&path, None, &config)
        .await
        .expect("Should extract DOCX");

    assert_non_empty_content(&result);
    assert!(
        result.content.contains("**bold**"),
        "Bold text should be wrapped in ** markers. Got:\n{}",
        result.content
    );
}

#[tokio::test]
async fn test_docx_italic_rendered_as_markdown() {
    let path = get_test_file_path("docx/unit_test_formatting.docx");
    if !path.exists() {
        return;
    }

    let config = ExtractionConfig {
        output_format: OutputFormat::Markdown,
        ..Default::default()
    };
    let result = extract_file(&path, None, &config)
        .await
        .expect("Should extract DOCX");

    assert_non_empty_content(&result);
    assert!(
        result.content.contains("*italic*") || result.content.contains("*Italic"),
        "Italic text should be wrapped in * markers. Got:\n{}",
        result.content
    );
}

#[tokio::test]
async fn test_docx_hyperlink_rendered_as_markdown() {
    let path = get_test_file_path("docx/unit_test_formatting.docx");
    if !path.exists() {
        return;
    }

    let config = ExtractionConfig {
        output_format: OutputFormat::Markdown,
        ..Default::default()
    };
    let result = extract_file(&path, None, &config)
        .await
        .expect("Should extract DOCX");

    assert_non_empty_content(&result);
    assert!(
        result.content.contains("[hyperlink]("),
        "Hyperlinks should be rendered as [text](url). Got:\n{}",
        result.content
    );
    assert!(
        result.content.contains("https://"),
        "Hyperlink URLs should be resolved. Got:\n{}",
        result.content
    );
}

#[tokio::test]
async fn test_docx_mixed_formatting_on_same_line() {
    let path = get_test_file_path("docx/unit_test_formatting.docx");
    if !path.exists() {
        return;
    }

    let config = ExtractionConfig {
        output_format: OutputFormat::Markdown,
        ..Default::default()
    };
    let result = extract_file(&path, None, &config)
        .await
        .expect("Should extract DOCX");

    assert_non_empty_content(&result);
    // The document has a line: "Normal italic bold underline and hyperlink on the same line"
    // Where "italic" is italic, "bold" is bold, "underline" is underlined, "hyperlink" is a link
    let content = &result.content;
    assert!(
        content.contains("Normal ") && content.contains("*italic*") && content.contains("**bold**"),
        "Mixed formatting should be preserved inline. Got:\n{}",
        content
    );
}

// ---------------------------------------------------------------------------
// Heading hierarchy tests
// ---------------------------------------------------------------------------

#[tokio::test]
async fn test_docx_title_rendered_as_h1() {
    let path = get_test_file_path("docx/unit_test_headers.docx");
    if !path.exists() {
        return;
    }

    let config = ExtractionConfig {
        output_format: OutputFormat::Markdown,
        ..Default::default()
    };
    let result = extract_file(&path, None, &config)
        .await
        .expect("Should extract DOCX");

    assert_non_empty_content(&result);
    assert!(
        result.content.contains("# Test Document"),
        "Title style should be rendered as # heading. Got:\n{}",
        result.content
    );
}

#[tokio::test]
async fn test_docx_heading_hierarchy() {
    let path = get_test_file_path("docx/unit_test_headers.docx");
    if !path.exists() {
        return;
    }

    let config = ExtractionConfig {
        output_format: OutputFormat::Markdown,
        ..Default::default()
    };
    let result = extract_file(&path, None, &config)
        .await
        .expect("Should extract DOCX");

    assert_non_empty_content(&result);
    let content = &result.content;

    // Heading1 → # (outline_level 0 maps to h1, same as standard converters)
    assert!(
        content.contains("# Section 1"),
        "Heading1 should be rendered as #. Got:\n{}",
        content
    );

    // Heading2 → ##
    assert!(
        content.contains("## Section 1.1"),
        "Heading2 should be rendered as ##. Got:\n{}",
        content
    );

    // Heading3 → ###
    assert!(
        content.contains("### Section 1.2.3"),
        "Heading3 should be rendered as ###. Got:\n{}",
        content
    );
}

#[tokio::test]
async fn test_docx_paragraphs_separated_by_blank_lines() {
    let path = get_test_file_path("docx/unit_test_headers.docx");
    if !path.exists() {
        return;
    }

    let result = extract_file(&path, None, &ExtractionConfig::default())
        .await
        .expect("Should extract DOCX");

    assert_non_empty_content(&result);
    // Paragraphs should be separated by blank lines
    assert!(
        result.content.contains("Paragraph 1.1\n\nParagraph 1.2"),
        "Paragraphs should be separated by blank lines. Got:\n{}",
        result.content
    );
}

// ---------------------------------------------------------------------------
// List tests
// ---------------------------------------------------------------------------

#[tokio::test]
async fn test_docx_bullet_list_rendered() {
    let path = get_test_file_path("docx/unit_test_lists.docx");
    if !path.exists() {
        return;
    }

    let config = ExtractionConfig {
        output_format: OutputFormat::Markdown,
        ..Default::default()
    };
    let result = extract_file(&path, None, &config)
        .await
        .expect("Should extract DOCX");

    assert_non_empty_content(&result);
    assert!(
        result.content.contains("- List item 1"),
        "Bullet lists should use '- ' prefix. Got:\n{}",
        result.content
    );
}

#[tokio::test]
async fn test_docx_numbered_list_rendered() {
    let path = get_test_file_path("docx/unit_test_lists.docx");
    if !path.exists() {
        return;
    }

    let config = ExtractionConfig {
        output_format: OutputFormat::Markdown,
        ..Default::default()
    };
    let result = extract_file(&path, None, &config)
        .await
        .expect("Should extract DOCX");

    assert_non_empty_content(&result);
    assert!(
        result.content.contains("1. List item a"),
        "Numbered lists should use 'N. ' prefix. Got:\n{}",
        result.content
    );
}

#[tokio::test]
async fn test_docx_nested_list_indentation() {
    let path = get_test_file_path("docx/unit_test_lists.docx");
    if !path.exists() {
        return;
    }

    let config = ExtractionConfig {
        output_format: OutputFormat::Markdown,
        ..Default::default()
    };
    let result = extract_file(&path, None, &config)
        .await
        .expect("Should extract DOCX");

    assert_non_empty_content(&result);
    assert!(
        result.content.contains("  - List item 1.1"),
        "Nested lists should be indented with 2 spaces. Got:\n{}",
        result.content
    );
}

// ---------------------------------------------------------------------------
// Document structure tests
// ---------------------------------------------------------------------------

#[tokio::test]
async fn test_docx_document_structure_populated() {
    let path = get_test_file_path("docx/unit_test_headers.docx");
    if !path.exists() {
        return;
    }

    let config = ExtractionConfig {
        include_document_structure: true,
        ..Default::default()
    };

    let result = extract_file(&path, None, &config).await.expect("Should extract DOCX");

    assert!(
        result.document.is_some(),
        "DocumentStructure should be populated when include_document_structure=true"
    );

    let doc = result.document.as_ref().unwrap();
    assert!(!doc.nodes.is_empty(), "DocumentStructure should have nodes");
}

// ---------------------------------------------------------------------------
// Table tests
// ---------------------------------------------------------------------------

#[tokio::test]
async fn test_docx_tables_in_markdown_output() {
    let path = get_test_file_path("docx/docx_tables.docx");
    if !path.exists() {
        return;
    }

    let config = ExtractionConfig {
        output_format: OutputFormat::Markdown,
        ..Default::default()
    };
    let result = extract_file(&path, None, &config)
        .await
        .expect("Should extract DOCX");

    assert_non_empty_content(&result);
    // Tables should be rendered as markdown tables with pipe separators
    assert!(
        result.content.contains('|'),
        "Tables should be rendered as markdown tables with | separators. Got:\n{}",
        result.content
    );
    // Should have header separator row
    assert!(
        result.content.contains("---"),
        "Tables should have header separator row with ---. Got:\n{}",
        result.content
    );
}

#[tokio::test]
async fn test_docx_table_cell_formatting_preserved() {
    let path = get_test_file_path("docx/tablecell.docx");
    if !path.exists() {
        return;
    }

    let result = extract_file(&path, None, &ExtractionConfig::default())
        .await
        .expect("Should extract DOCX");

    assert_non_empty_content(&result);
    // The tables field should have table data
    assert!(
        !result.tables.is_empty(),
        "DOCX with tables should have tables in result"
    );
}

// ---------------------------------------------------------------------------
// MIME type test
// ---------------------------------------------------------------------------

#[tokio::test]
async fn test_docx_produces_markdown_mime_type() {
    let path = get_test_file_path("docx/unit_test_formatting.docx");
    if !path.exists() {
        return;
    }

    let result = extract_file(&path, None, &ExtractionConfig::default())
        .await
        .expect("Should extract DOCX");

    assert_eq!(
        result.mime_type.as_ref() as &str,
        "application/vnd.openxmlformats-officedocument.wordprocessingml.document",
        "DOCX extractor should preserve input MIME type"
    );
}
