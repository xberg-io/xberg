//! Regression tests for PDF image extraction in markdown output.
//!
//! Verifies that embedded images in PDFs produce proper `![](image_N.fmt)`
//! references instead of empty `![]()` placeholders.

#![cfg(feature = "pdf")]

use kreuzberg::core::config::{ExtractionConfig, OutputFormat};
use kreuzberg::core::extractor::extract_file;
use std::path::PathBuf;

mod helpers;

fn test_documents_dir() -> PathBuf {
    PathBuf::from(env!("CARGO_MANIFEST_DIR"))
        .parent()
        .unwrap()
        .parent()
        .unwrap()
        .join("test_documents")
}

fn extract_markdown(relative_path: &str) -> kreuzberg::types::ExtractionResult {
    let path = test_documents_dir().join(relative_path);
    let config = ExtractionConfig {
        output_format: OutputFormat::Markdown,
        ..Default::default()
    };
    let rt = tokio::runtime::Runtime::new().unwrap();
    rt.block_on(extract_file(&path, None, &config)).unwrap()
}

#[test]
fn test_multipage_marketing_no_empty_image_refs() {
    let result = extract_markdown("pdf/multipage_marketing.pdf");
    let content = &result.content;

    // Must not contain empty image references
    assert!(
        !content.contains("![]()"),
        "Markdown output must not contain empty image references ![](), got:\n{}",
        content
    );
}

#[test]
fn test_multipage_marketing_has_image_refs() {
    let result = extract_markdown("pdf/multipage_marketing.pdf");
    let content = &result.content;

    // Must contain at least one proper image reference
    assert!(
        content.contains("![](image_"),
        "Markdown output must contain image references like ![](image_N.png), got:\n{}",
        content
    );
}

#[test]
fn test_multipage_marketing_images_populated() {
    let result = extract_markdown("pdf/multipage_marketing.pdf");

    // Extraction result must have images with actual data
    let images = result.images.as_ref().expect("images field must be Some");
    assert!(!images.is_empty(), "Extraction result must contain extracted images");

    // At least some images should have non-empty data
    let images_with_data = images.iter().filter(|img| !img.data.is_empty()).count();
    assert!(
        images_with_data > 0,
        "At least some images should have actual pixel data, got {} images total but none with data",
        images.len()
    );
}

#[test]
fn test_docling_no_empty_image_refs() {
    let result = extract_markdown("pdf/docling.pdf");
    let content = &result.content;

    assert!(
        !content.contains("![]()"),
        "Docling markdown must not contain empty image references ![](), got:\n{}",
        content
    );
}

#[test]
fn test_docling_has_image_refs() {
    let result = extract_markdown("pdf/docling.pdf");
    let content = &result.content;

    // Docling has at least 1 figure
    assert!(
        content.contains("![](image_"),
        "Docling markdown must contain image references, got:\n{}",
        content
    );
}

#[test]
fn test_docling_content_quality() {
    let result = extract_markdown("pdf/docling.pdf");
    let content = &result.content;

    // Verify key content from the Docling technical report is present
    assert!(content.contains("Docling"), "Must contain 'Docling'");
    assert!(content.contains("PDF"), "Must contain 'PDF'");
    assert!(
        content.contains("table structure recognition") || content.contains("TableFormer"),
        "Must mention table structure recognition or TableFormer"
    );
}

/// Regression test for issue #752: structured output was ~1000x slower than text
/// on Ghostscript-produced PDFs with many inline images (~1,924 per page).
///
/// Root cause: `populate_images_from_pdfium` used `Vec::contains` (O(N)) inside
/// the per-page object loop — O(N²) total. Fixed by converting to `AHashSet` for
/// O(1) lookup before the loop.
///
/// This test skips when the repro file is absent (it is not committed to the
/// repository due to size). To reproduce locally, generate a Ghostscript vector
/// decomposition PDF and place it at:
///   test_documents/pdf/ghostscript_inline_images_repro.pdf
#[test]
fn test_ghostscript_inline_images_completes_in_reasonable_time() {
    let path = test_documents_dir().join("pdf/ghostscript_inline_images_repro.pdf");
    if !path.exists() {
        eprintln!("SKIP: test_documents/pdf/ghostscript_inline_images_repro.pdf not present");
        return;
    }

    let config = kreuzberg::core::config::ExtractionConfig {
        output_format: kreuzberg::core::config::OutputFormat::Markdown,
        ..Default::default()
    };
    let rt = tokio::runtime::Runtime::new().unwrap();

    let start = std::time::Instant::now();
    let result = rt
        .block_on(kreuzberg::core::extractor::extract_file(&path, None, &config))
        .expect("extraction must succeed for Ghostscript inline-image PDF");
    let elapsed = start.elapsed();

    // Before the fix, a single-page PDF with ~1,924 inline images took ~56 seconds.
    // After the fix it should complete in well under 10 seconds even on slow CI.
    assert!(
        elapsed.as_secs() < 10,
        "Ghostscript inline-image PDF must extract in under 10 seconds, took {elapsed:?}"
    );

    // The file has no text — content may be empty or minimal; that is expected.
    let _ = result;
}
