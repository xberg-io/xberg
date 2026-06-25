//! Reading-order reconstruction tests for multi-column PDFs.
//!
//! Tests verify that:
//! - Plain text output respects reading-order reordering when enabled
//! - Markdown output respects reading-order reordering when enabled
//!
//! Run plain-text test (no model download):
//! ```
//! cargo test -p xberg --test reading_order plain_text
//! ```
//!
//! Run with layout detection (downloads layout ONNX model, ~300MB):
//! ```
//! XBERG_RUN_LAYOUT_TESTS=1 cargo test -p xberg --features full --test reading_order -- --nocapture
//! ```

#![cfg(all(feature = "pdf", feature = "layout-detection", not(target_arch = "wasm32")))]

use std::path::PathBuf;
use xberg::core::config::{ExtractionConfig, OutputFormat, PdfConfig};
use xberg::extract_bytes_sync;

/// Helper: check if layout-detection tests are enabled via env var
fn should_run_layout_tests() -> bool {
    std::env::var("XBERG_RUN_LAYOUT_TESTS").is_ok()
}

/// Helper: load the test document
fn load_test_pdf() -> Vec<u8> {
    let path =
        PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("../../test_documents/vendored/docling/pdf/2206.01062.pdf");
    std::fs::read(path).expect("Failed to load test PDF 2206.01062.pdf")
}

/// Test: plain-text reading-order differs with vs without reordering
///
/// CONFIRMED: this should PASS, proving the text path works.
#[test]
#[cfg_attr(not(feature = "layout-detection"), ignore)]
fn text_reading_order_changes_output() {
    if !should_run_layout_tests() {
        eprintln!("Skipping layout-detection test (set XBERG_RUN_LAYOUT_TESTS=1 to enable)");
        return;
    }

    let content = load_test_pdf();

    // Extract with reading_order = false (baseline)
    let mut config_no_ro = ExtractionConfig {
        output_format: OutputFormat::Plain,
        pdf_options: Some(PdfConfig {
            reading_order: false,
            ..Default::default()
        }),
        ..Default::default()
    };
    config_no_ro.layout = Some(Default::default()); // Enable layout detection
    config_no_ro.use_layout_for_markdown = true; // Needed for layout hints to be computed

    eprintln!(
        "Config no_ro: reading_order={}, layout={}, use_layout_for_markdown={}",
        config_no_ro
            .pdf_options
            .as_ref()
            .map(|p| p.reading_order)
            .unwrap_or(false),
        config_no_ro.layout.is_some(),
        config_no_ro.use_layout_for_markdown
    );

    let result_no_ro = extract_bytes_sync(&content, "application/pdf", &config_no_ro)
        .expect("Failed to extract with reading_order=false");

    // Extract with reading_order = true
    let mut config_with_ro = ExtractionConfig {
        output_format: OutputFormat::Plain,
        pdf_options: Some(PdfConfig {
            reading_order: true,
            ..Default::default()
        }),
        ..Default::default()
    };
    config_with_ro.layout = Some(Default::default()); // Enable layout detection
    config_with_ro.use_layout_for_markdown = true; // Needed for layout hints to be computed

    eprintln!(
        "Config with_ro: reading_order={}, layout={}, use_layout_for_markdown={}",
        config_with_ro
            .pdf_options
            .as_ref()
            .map(|p| p.reading_order)
            .unwrap_or(false),
        config_with_ro.layout.is_some(),
        config_with_ro.use_layout_for_markdown
    );

    let result_with_ro = extract_bytes_sync(&content, "application/pdf", &config_with_ro)
        .expect("Failed to extract with reading_order=true");

    // Both should produce text
    assert!(!result_no_ro.content.is_empty(), "No-RO extraction produced empty text");
    assert!(
        !result_with_ro.content.is_empty(),
        "With-RO extraction produced empty text"
    );

    // Text outputs should differ (if they're identical, the feature is broken)
    if result_no_ro.content == result_with_ro.content {
        eprintln!("WARNING: text outputs are IDENTICAL with vs without reading_order");
        eprintln!("This suggests reading_order is not being applied to the plain-text path.");
        eprintln!("No-RO length: {}", result_no_ro.content.len());
        eprintln!("With-RO length: {}", result_with_ro.content.len());
        panic!("Text reading_order did not change output (feature may be broken)");
    } else {
        println!(
            "✓ Text outputs differ ({} vs {} bytes)",
            result_no_ro.content.len(),
            result_with_ro.content.len()
        );
    }
}

/// Test: markdown reading-order differs with vs without reordering
///
/// This test will FAIL before the fix, PASS after.
#[test]
#[cfg_attr(not(feature = "layout-detection"), ignore)]
fn markdown_reading_order_changes_output() {
    if !should_run_layout_tests() {
        eprintln!("Skipping layout-detection test (set XBERG_RUN_LAYOUT_TESTS=1 to enable)");
        return;
    }

    let content = load_test_pdf();

    // Extract with reading_order = false (baseline)
    let mut config_no_ro = ExtractionConfig {
        output_format: OutputFormat::Markdown,
        pdf_options: Some(PdfConfig {
            reading_order: false,
            ..Default::default()
        }),
        ..Default::default()
    };
    config_no_ro.layout = Some(Default::default()); // Enable layout detection
    config_no_ro.use_layout_for_markdown = true; // Needed for layout hints to be computed

    let result_no_ro = extract_bytes_sync(&content, "application/pdf", &config_no_ro)
        .expect("Failed to extract with reading_order=false");

    // Extract with reading_order = true
    let mut config_with_ro = ExtractionConfig {
        output_format: OutputFormat::Markdown,
        pdf_options: Some(PdfConfig {
            reading_order: true,
            ..Default::default()
        }),
        ..Default::default()
    };
    config_with_ro.layout = Some(Default::default()); // Enable layout detection
    config_with_ro.use_layout_for_markdown = true; // Needed for layout hints to be computed

    let result_with_ro = extract_bytes_sync(&content, "application/pdf", &config_with_ro)
        .expect("Failed to extract with reading_order=true");

    // Both should produce markdown
    assert!(
        !result_no_ro.content.is_empty(),
        "No-RO markdown extraction produced empty text"
    );
    assert!(
        !result_with_ro.content.is_empty(),
        "With-RO markdown extraction produced empty text"
    );

    // Markdown outputs should differ (if they're identical, the feature is broken)
    if result_no_ro.content == result_with_ro.content {
        eprintln!("WARNING: markdown outputs are IDENTICAL with vs without reading_order");
        eprintln!("This confirms the bug: reading_order is not wired to the markdown path.");
        eprintln!("No-RO length: {}", result_no_ro.content.len());
        eprintln!("With-RO length: {}", result_with_ro.content.len());
        // For now, report this as expected (the bug we're fixing)
        panic!("Markdown reading_order did not change output (this is the bug we're fixing)");
    } else {
        println!(
            "✓ Markdown outputs differ ({} vs {} bytes)",
            result_no_ro.content.len(),
            result_with_ro.content.len()
        );
    }
}
