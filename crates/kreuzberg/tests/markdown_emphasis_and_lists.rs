//! Tests for W2.A, W2.B, W2.C: inline emphasis, indentation-based lists, and monospace code blocks
//!
//! Integration tests using the public API (extract_file_sync) to verify markdown output quality.

#![cfg(feature = "pdf")]

mod helpers;

use helpers::*;
use kreuzberg::core::config::{ExtractionConfig, OutputFormat};
use kreuzberg::extract_file_sync;

// ============================================================================
// W2.A: Inline emphasis emission (bold/italic)
// ============================================================================

#[test]
#[ignore = "W2.A: Requires test PDF with mixed bold/italic spans"]
fn test_w2a_inline_bold_emphasis_in_markdown() {
    // This test verifies that when a PDF contains bold text in a paragraph,
    // the extracted markdown includes **bold** markup.
    // Test fixture should be: PDF with paragraph containing "Normal **bold** text"
    if !test_documents_available() {
    }
    // TODO: Once we have a suitable test PDF, extract it and assert:
    // let content = extract_markdown("pdf/mixed_emphasis.pdf");
    // assert!(content.contains("**"), "Markdown should contain bold (**) markers");
}

#[test]
#[ignore = "W2.A: Requires test PDF with mixed bold/italic spans"]
fn test_w2a_inline_italic_emphasis_in_markdown() {
    // This test verifies that when a PDF contains italic text in a paragraph,
    // the extracted markdown includes *italic* markup.
    if !test_documents_available() {
    }
    // TODO: Once we have a suitable test PDF, extract it and assert:
    // let content = extract_markdown("pdf/mixed_emphasis.pdf");
    // assert!(content.contains("*") && !content.contains("**"), "Markdown should contain italic (*) markers");
}

// ============================================================================
// W2.B: Indentation-based list detection
// ============================================================================

#[test]
#[ignore = "W2.B: Requires test PDF with indentation-based lists"]
fn test_w2b_indentation_list_detected_in_markdown() {
    // This test verifies that paragraphs shifted right due to indentation are detected
    // as list items and rendered with markdown list markers.
    if !test_documents_available() {
    }
    // TODO: Once we have a suitable test PDF with indented paragraphs:
    // let content = extract_markdown("pdf/indented_lists.pdf");
    // Count list markers in output (-, *, •, etc.)
    // assert!(content.contains("- ") || content.contains("* "), "Markdown should contain list markers");
}

// ============================================================================
// W2.C: Monospace code-block heuristic
// ============================================================================

#[test]
#[ignore = "W2.C: Requires test PDF with monospace code blocks"]
fn test_w2c_monospace_code_block_detection() {
    // This test verifies that consecutive monospace paragraphs are detected as code blocks
    // and rendered with fenced markdown code block syntax (```).
    if !test_documents_available() {
    }
    // TODO: Once we have a suitable test PDF with monospace text:
    // let content = extract_markdown("pdf/code_blocks.pdf");
    // assert!(content.contains("```"), "Markdown should contain code block fences (```)");
}

fn extract_markdown(relative_path: &str) -> String {
    let pdf_path = get_test_file_path(relative_path);
    if !pdf_path.exists() {
        panic!("Test document not found: {}", relative_path);
    }
    let config = ExtractionConfig {
        output_format: OutputFormat::Markdown,
        ..Default::default()
    };
    extract_file_sync(&pdf_path, None, &config)
        .expect("extraction should succeed")
        .content
}
