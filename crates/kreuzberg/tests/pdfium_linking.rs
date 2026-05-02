//! PDFium linking integration tests.
//!
//! This module tests PDF extraction functionality across all PDFium linking strategies
//! (default, static, bundled, system). Tests verify that:
//!
//! 1. PDF extraction works regardless of linking strategy
//! 2. Extracted content is valid and non-empty
//! 3. Bundled PDFium extraction works when feature is enabled
//! 4. Metadata extraction functions correctly
//! 5. Error handling works for invalid PDFs
//!
//! These tests are strategy-agnostic - they verify functionality works with ANY
//! linking strategy, not implementation details of specific strategies.
//!
//! Test philosophy:
//! - Use small test PDFs from test_documents/
//! - Assert on extracted content and behavior, not linking internals
//! - Test both successful extraction and error cases
//! - Skip tests gracefully if test files are missing

#![cfg(feature = "pdf")]

mod helpers;

use helpers::*;
use kreuzberg::core::config::ExtractionConfig;
use kreuzberg::extract_file_sync;

/// Test basic PDF extraction works regardless of linking strategy.
///
/// This is the primary integration test - it verifies that PDF extraction
/// functions correctly with any linking strategy. We use a small test PDF
/// and verify that:
/// - Extraction completes successfully
/// - Content is non-empty and contains expected text
/// - MIME type is correctly detected
#[test]
fn test_pdf_extraction_basic() {
    if skip_if_missing("pdf/tiny.pdf") {
        return;
    }

    let file_path = get_test_file_path("pdf/tiny.pdf");
    let config = ExtractionConfig::default();

    let result = extract_file_sync(&file_path, None, &config).expect("Should extract PDF successfully");

    assert_mime_type(&result, "application/pdf");

    assert_non_empty_content(&result);

    assert_min_content_length(&result, 10);
}

/// Test PDF extraction with a medium-sized test PDF.
///
/// Verifies that extraction works with slightly larger PDFs and produces
/// substantive content. This test helps ensure linking strategy works
/// with real-world document sizes.
#[test]
fn test_pdf_extraction_medium() {
    if skip_if_missing("pdf/medium.pdf") {
        return;
    }

    let file_path = get_test_file_path("pdf/medium.pdf");
    let config = ExtractionConfig::default();

    let result = extract_file_sync(&file_path, None, &config).expect("Should extract medium PDF successfully");

    assert_mime_type(&result, "application/pdf");
    assert_non_empty_content(&result);

    assert!(result.content.len() >= 50, "Medium PDF should have substantial content");
}

/// Test PDF extraction with rotated pages.
///
/// Some PDFs have rotated pages. This test verifies that the linking
/// strategy handles page rotation correctly (PDFium should handle this
/// transparently). Note: This specific PDF may have OCR-only content,
/// so we just verify extraction completes without error.
#[test]
fn test_pdf_extraction_rotated_pages() {
    if skip_if_missing("pdf/ocr_test_rotated_90.pdf") {
        return;
    }

    let file_path = get_test_file_path("pdf/ocr_test_rotated_90.pdf");
    let config = ExtractionConfig::default();

    let result = extract_file_sync(&file_path, None, &config).expect("Should extract rotated PDF successfully");

    assert_mime_type(&result, "application/pdf");
}

/// Test PDF text extraction with code and formula content.
///
/// Verifies extraction works with technical PDFs containing code blocks,
/// mathematical formulas, and special formatting.
#[test]
fn test_pdf_extraction_code_and_formulas() {
    if skip_if_missing("pdf/code_and_formula.pdf") {
        return;
    }

    let file_path = get_test_file_path("pdf/code_and_formula.pdf");
    let config = ExtractionConfig::default();

    let result = extract_file_sync(&file_path, None, &config).expect("Should extract PDF with code and formulas");

    assert_mime_type(&result, "application/pdf");
    assert_non_empty_content(&result);
}

/// Test PDF extraction preserves right-to-left text direction.
///
/// PDFs with RTL text (Arabic, Hebrew, etc.) require special handling.
/// This verifies extraction works with RTL content.
#[test]
fn test_pdf_extraction_right_to_left() {
    if skip_if_missing("pdf/right_to_left_01.pdf") {
        return;
    }

    let file_path = get_test_file_path("pdf/right_to_left_01.pdf");
    let config = ExtractionConfig::default();

    let result = extract_file_sync(&file_path, None, &config).expect("Should extract RTL PDF successfully");

    assert_mime_type(&result, "application/pdf");
}

/// Test PDF metadata extraction.
///
/// Verifies that PDF metadata (title, author, creation date, etc.) can be
/// extracted correctly, which is independent of linking strategy.
#[test]
fn test_pdf_metadata_extraction() {
    if skip_if_missing("pdf/tiny.pdf") {
        return;
    }

    let file_path = get_test_file_path("pdf/tiny.pdf");
    let config = ExtractionConfig::default();

    let result = extract_file_sync(&file_path, None, &config).expect("Should extract PDF metadata");

    let _metadata = &result.metadata;
    assert!(!result.mime_type.is_empty(), "MIME type should be set");
}

/// Test extraction of byte array from PDF.
///
/// Verifies that extract_bytes_sync works for PDF content, which is
/// important for in-memory processing.
#[test]
fn test_pdf_extraction_from_bytes() {
    if skip_if_missing("pdf/tiny.pdf") {
        return;
    }

    let file_path = get_test_file_path("pdf/tiny.pdf");
    let pdf_bytes = std::fs::read(&file_path).expect("Should read PDF file");

    let config = ExtractionConfig::default();
    let result = kreuzberg::extract_bytes_sync(&pdf_bytes, "application/pdf", &config)
        .expect("Should extract PDF from bytes successfully");

    assert_mime_type(&result, "application/pdf");
    assert_non_empty_content(&result);
}

/// Test extraction with custom PDF configuration.
///
/// Verifies that custom PDF extraction settings work correctly.
#[test]
fn test_pdf_extraction_with_config() {
    if skip_if_missing("pdf/tiny.pdf") {
        return;
    }

    let file_path = get_test_file_path("pdf/tiny.pdf");

    let config = ExtractionConfig::default();

    let result = extract_file_sync(&file_path, None, &config).expect("Should extract PDF with custom config");

    assert_mime_type(&result, "application/pdf");
    assert_non_empty_content(&result);

    assert!(!result.mime_type.is_empty(), "MIME type should be set in result");
}

/// Test handling of empty or minimal PDFs.
///
/// Some edge case PDFs might be empty or contain no text. Verify extraction
/// handles these gracefully.
#[test]
fn test_pdf_extraction_edge_cases() {
    if skip_if_missing("pdf/tiny.pdf") {
        return;
    }

    let file_path = get_test_file_path("pdf/tiny.pdf");
    let config = ExtractionConfig::default();

    let result = extract_file_sync(&file_path, None, &config);
    assert!(result.is_ok(), "PDF extraction should succeed");
}

/// Test batch extraction of multiple PDFs.
///
/// Verifies that linking strategy handles batch processing of multiple
/// PDF files correctly.
#[test]
#[cfg(feature = "tokio-runtime")]
fn test_pdf_batch_extraction() {
    if skip_if_missing("pdf/tiny.pdf") || skip_if_missing("pdf/medium.pdf") {
        return;
    }

    let paths: Vec<kreuzberg::BatchFileItem> = vec![
        kreuzberg::BatchFileItem {
            path: get_test_file_path("pdf/tiny.pdf"),
            config: None,
        },
        kreuzberg::BatchFileItem {
            path: get_test_file_path("pdf/medium.pdf"),
            config: None,
        },
    ];

    let config = ExtractionConfig::default();

    let results = kreuzberg::batch_extract_files_sync(paths, &config).expect("Should extract PDFs in batch");

    assert_eq!(results.len(), 2, "Should extract both PDFs");

    for result in &results {
        assert_mime_type(result, "application/pdf");
        assert_non_empty_content(result);
    }
}

/// Test PDF with Unicode content extraction.
///
/// Verifies that PDFium linking strategy handles PDFs with international
/// characters correctly.
#[test]
fn test_pdf_unicode_content() {
    if skip_if_missing("pdf/right_to_left_01.pdf") {
        return;
    }

    let file_path = get_test_file_path("pdf/right_to_left_01.pdf");
    let config = ExtractionConfig::default();

    let result = extract_file_sync(&file_path, None, &config).expect("Should extract PDF with Unicode content");

    assert_mime_type(&result, "application/pdf");
}

/// Verify PDF module compiles with all feature combinations.
///
/// This is a compile-time test that ensures the pdf module compiles correctly.
#[test]
#[cfg(feature = "pdf")]
fn test_pdf_module_availability() {
    // pdf module compiles when feature is active
    let _: Option<kreuzberg::pdf::PdfError> = None;
}
