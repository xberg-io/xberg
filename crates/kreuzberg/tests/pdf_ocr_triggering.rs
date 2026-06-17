//! TODO: Restored from 245539484 alef-migration cleanup. Currently exercises
//! pub(crate) APIs that the migration deliberately narrowed; gated until
//! either (a) these APIs are re-exposed publicly, or (b) the test is
//! rewritten against the public extraction surface.

#![cfg(any())]

// Original content preserved below; recompiled once gating cfg drops.
// Disabled by the file-level cfg(any()) above.

/*
//! Tests for smart OCR triggering based on text coverage.
//!
//! This module tests the logic for determining when OCR should be triggered
//! based on text block coverage of a PDF page. The tests use TDD approach
//! with edge cases for threshold boundary conditions.
//!
//! Test philosophy:
//! - Verify OCR is triggered when text coverage is below threshold
//! - Verify OCR is not triggered when text coverage is above threshold
//! - Test exact threshold edge cases (50% boundary)
//! - Use real PDF files for reliable page dimensions
//!
//! NOTE: Tests use real PDF files from test_documents directory to avoid
//! pdfium initialization issues that occur with dynamically created PDFs.

#![cfg(feature = "pdf")]

use kreuzberg::core::config::{ExtractionConfig, HierarchyConfig, PdfBackend, PdfConfig};
use kreuzberg::pdf::hierarchy::{BoundingBox, TextBlock, should_trigger_ocr};
use pdfium_render::prelude::*;
use std::path::Path;

/// Helper function to get test PDF path
fn get_test_pdf_path() -> String {
    "test_documents/pdf/tiny.pdf".to_string()
}

/// Helper function to skip test if PDF is unavailable
fn skip_if_no_pdf() -> bool {
    let pdf_path = get_test_pdf_path();
    if !Path::new(&pdf_path).exists() {
        eprintln!("PDF file not found at {}, skipping test", pdf_path);
        true
    } else {
        false
    }
}

/// Test OCR triggering with low text coverage (20% coverage → should trigger).
///
/// Creates mock text blocks that cover only ~20% of the page.
/// Expected: OCR should trigger since 20% < 50%
#[test]
fn test_ocr_trigger_low_coverage() {
    if skip_if_no_pdf() {
        return;
    }

    let pdfium = Pdfium;
    let document = pdfium
        .load_pdf_from_file(get_test_pdf_path().as_str(), None)
        .expect("Should load PDF document");

    let page = document.pages().get(0).expect("Should get first page");

    // Create text blocks covering only ~20% of the page
    let blocks = vec![
        TextBlock {
            text: "Small text block 1".to_string(),
            bbox: BoundingBox {
                left: 0.0,
                top: 0.0,
                right: 150.0,
                bottom: 200.0,
            },
            font_size: 12.0,
        },
        TextBlock {
            text: "Small text block 2".to_string(),
            bbox: BoundingBox {
                left: 200.0,
                top: 200.0,
                right: 300.0,
                bottom: 350.0,
            },
            font_size: 12.0,
        },
    ];

    let config = ExtractionConfig::default();

    // Should trigger OCR because coverage is below 50%
    assert!(
        should_trigger_ocr(&page, &blocks, &config),
        "OCR should trigger when text coverage is below 50%"
    );
}

/// Test OCR not triggering with high text coverage (60% coverage → don't trigger).
///
/// Creates mock text blocks that cover approximately 60% of the page.
/// Expected: OCR should not trigger since 60% > 50%
#[test]
fn test_ocr_no_trigger_high_coverage() {
    if skip_if_no_pdf() {
        return;
    }

    let pdfium = Pdfium;
    let document = pdfium
        .load_pdf_from_file(get_test_pdf_path().as_str(), None)
        .expect("Should load PDF document");

    let page = document.pages().get(0).expect("Should get first page");

    // Create text blocks covering approximately 60% of the page
    // For a typical page (~612 x 792 points), this is a large block
    let blocks = vec![TextBlock {
        text: "Large text block covering most of the page".to_string(),
        bbox: BoundingBox {
            left: 0.0,
            top: 0.0,
            right: 400.0,
            bottom: 600.0,
        },
        font_size: 12.0,
    }];

    let config = ExtractionConfig::default();

    // Should NOT trigger OCR because coverage is above 50%
    assert!(
        !should_trigger_ocr(&page, &blocks, &config),
        "OCR should not trigger when text coverage is above 50%"
    );
}

/// Test OCR triggering at exact threshold boundary (50% edge case).
///
/// Creates mock text blocks that cover exactly ~50% of the page.
/// Expected: OCR should trigger at boundary (uses < not <=)
#[test]
fn test_ocr_trigger_exact_threshold() {
    if skip_if_no_pdf() {
        return;
    }

    let pdfium = Pdfium;
    let document = pdfium
        .load_pdf_from_file(get_test_pdf_path().as_str(), None)
        .expect("Should load PDF document");

    let page = document.pages().get(0).expect("Should get first page");
    let page_width = page.width().value;
    let page_height = page.height().value;

    // Create text block covering exactly 50% of the page
    let coverage_height = page_height * 0.5;
    let blocks = vec![TextBlock {
        text: "Text block at 50% threshold".to_string(),
        bbox: BoundingBox {
            left: 0.0,
            top: 0.0,
            right: page_width,
            bottom: coverage_height,
        },
        font_size: 12.0,
    }];

    let config = ExtractionConfig::default();

    // At exactly 50%, OCR should trigger (using < not <=)
    // Due to floating point precision, we allow a small margin
    let result = should_trigger_ocr(&page, &blocks, &config);
    assert!(
        result,
        "OCR should trigger at or very near 50% threshold boundary (using < not <=)"
    );
}

/// Test OCR with overlapping text blocks.
///
/// Verifies that overlapping blocks are handled correctly.
/// Both blocks contribute to coverage calculation.
#[test]
fn test_ocr_overlapping_blocks() {
    if skip_if_no_pdf() {
        return;
    }

    let pdfium = Pdfium;
    let document = pdfium
        .load_pdf_from_file(get_test_pdf_path().as_str(), None)
        .expect("Should load PDF document");

    let page = document.pages().get(0).expect("Should get first page");

    // Create overlapping text blocks
    // Block 1: covers 150x200 area
    // Block 2: covers 200x200 area, overlaps with Block 1
    // Total area contribution: both areas count (not union)
    let blocks = vec![
        TextBlock {
            text: "Block 1".to_string(),
            bbox: BoundingBox {
                left: 0.0,
                top: 0.0,
                right: 150.0,
                bottom: 200.0,
            },
            font_size: 12.0,
        },
        TextBlock {
            text: "Block 2".to_string(),
            bbox: BoundingBox {
                left: 100.0,
                top: 150.0,
                right: 300.0,
                bottom: 350.0,
            },
            font_size: 12.0,
        },
    ];

    let config = ExtractionConfig::default();

    // Should trigger OCR because combined areas are still below 50%
    assert!(
        should_trigger_ocr(&page, &blocks, &config),
        "OCR should trigger with overlapping blocks below threshold"
    );
}

/// Test OCR with empty blocks list (no text → should trigger).
///
/// When there are no text blocks, coverage is 0%, so OCR should trigger.
#[test]
fn test_ocr_empty_blocks() {
    if skip_if_no_pdf() {
        return;
    }

    let pdfium = Pdfium;
    let document = pdfium
        .load_pdf_from_file(get_test_pdf_path().as_str(), None)
        .expect("Should load PDF document");

    let page = document.pages().get(0).expect("Should get first page");

    let blocks = vec![];
    let config = ExtractionConfig::default();

    // Should trigger OCR because there are no text blocks (0% coverage)
    assert!(
        should_trigger_ocr(&page, &blocks, &config),
        "OCR should trigger with empty blocks (0% coverage)"
    );
}

/// Test OCR triggering with custom threshold from config.
///
/// If HierarchyConfig specifies a custom ocr_coverage_threshold, respect it.
/// With 30% coverage and custom 25% threshold, OCR should NOT trigger.
#[test]
fn test_ocr_custom_threshold() {
    if skip_if_no_pdf() {
        return;
    }

    let pdfium = Pdfium;
    let document = pdfium
        .load_pdf_from_file(get_test_pdf_path().as_str(), None)
        .expect("Should load PDF document");

    let page = document.pages().get(0).expect("Should get first page");

    // Create text blocks covering ~30% of the page
    let blocks = vec![TextBlock {
        text: "Text block 30%".to_string(),
        bbox: BoundingBox {
            left: 0.0,
            top: 0.0,
            right: 250.0,
            bottom: 600.0,
        },
        font_size: 12.0,
    }];

    // Set custom threshold to 25% instead of default 50%
    let config = ExtractionConfig {
        pdf_options: Some(PdfConfig {
            extract_images: false,
            passwords: None,
            extract_metadata: true,
            hierarchy: Some(HierarchyConfig {
                enabled: true,
                k_clusters: 6,
                include_bbox: true,
                ocr_coverage_threshold: Some(0.25),
            }),
            extract_annotations: false,
            top_margin_fraction: None,
            bottom_margin_fraction: None,
            allow_single_column_tables: false,
            backend: PdfBackend::default(),
        }),
        ..Default::default()
    };

    // With 30% coverage and custom 25% threshold, should NOT trigger OCR
    // (30% > 25% threshold)
    assert!(
        !should_trigger_ocr(&page, &blocks, &config),
        "OCR should respect custom threshold from config"
    );
}

*/
