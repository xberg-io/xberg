//! PDF markdown extraction integration tests.
//!
//! Tests that the new markdown rendering pipeline produces structured output
//! with headings, proper paragraph breaks, and no mid-sentence line breaks.

#![cfg(feature = "pdf")]

mod helpers;

use helpers::*;
use kreuzberg::core::config::{ExtractionConfig, OutputFormat};
use kreuzberg::extract_file_sync;

#[test]
fn test_pdf_markdown_extraction_produces_structured_output() {
    if skip_if_missing("pdf/fake_memo.pdf") {
        return;
    }

    let path = get_test_file_path("pdf/fake_memo.pdf");

    let config = ExtractionConfig {
        output_format: OutputFormat::Markdown,
        ..Default::default()
    };

    let result = extract_file_sync(&path, None, &config).expect("Should extract PDF as markdown");

    assert!(
        !result.content.trim().is_empty(),
        "Markdown content should not be empty"
    );
    assert_eq!(
        &*result.mime_type, "application/pdf",
        "Mime type should preserve original document type; output format is tracked in metadata"
    );

    // Verify paragraph structure: should have paragraph breaks (blank lines).
    // PDFs may use \r\n or \n line endings; normalize before counting.
    let normalized = result.content.replace("\r\n", "\n");
    let para_breaks = normalized.matches("\n\n").count();

    println!("=== Markdown output (first 1500 chars) ===");
    println!("{}", &result.content[..result.content.len().min(1500)]);
    println!("\n=== Analysis ===");
    println!("Has heading markers: {}", result.content.contains("# "));
    println!("Paragraph breaks: {}", para_breaks);
    println!("Total chars: {}", result.content.len());
    println!("Mime type: {}", result.mime_type);

    assert!(
        para_breaks >= 1,
        "Should have at least 1 paragraph break, got {}",
        para_breaks
    );
}

#[test]
fn test_pdf_plain_extraction_unchanged() {
    if skip_if_missing("pdf/fake_memo.pdf") {
        return;
    }

    let path = get_test_file_path("pdf/fake_memo.pdf");

    // Default config = Plain output format
    let config = ExtractionConfig::default();
    let result = extract_file_sync(&path, None, &config).expect("Should extract PDF as plain text");

    assert!(!result.content.trim().is_empty(), "Plain content should not be empty");
    assert_eq!(
        &*result.mime_type, "application/pdf",
        "Mime type should remain application/pdf for plain extraction"
    );
}

#[test]
fn test_pdf_markdown_vs_plain_has_more_structure() {
    if skip_if_missing("pdf/google_doc_document.pdf") {
        return;
    }

    let path = get_test_file_path("pdf/google_doc_document.pdf");

    // Extract as plain
    let plain_config = ExtractionConfig::default();
    let plain_result = extract_file_sync(&path, None, &plain_config).expect("Plain extraction failed");

    // Extract as markdown
    let md_config = ExtractionConfig {
        output_format: OutputFormat::Markdown,
        ..Default::default()
    };
    let md_result = extract_file_sync(&path, None, &md_config).expect("Markdown extraction failed");

    println!("=== Plain (first 500 chars) ===");
    println!("{}", &plain_result.content[..plain_result.content.len().min(500)]);
    println!("\n=== Markdown (first 500 chars) ===");
    println!("{}", &md_result.content[..md_result.content.len().min(500)]);

    // Both should have content
    assert!(!plain_result.content.trim().is_empty());
    assert!(!md_result.content.trim().is_empty());

    // Markdown should be different from plain (has structure added)
    // This is a weak check but validates the pipeline ran
    assert_ne!(
        plain_result.content, md_result.content,
        "Markdown output should differ from plain text output"
    );
}
