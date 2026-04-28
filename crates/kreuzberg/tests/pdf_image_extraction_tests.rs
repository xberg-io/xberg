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

// ─── Regression tests for issue #796 ────────────────────────────────────────
//
// Before the fix, setting `images.extract_images = false` (or
// `pdf_options.extract_images = false`) still caused full base64 image data to
// appear in `ExtractionResult.images` when `output_format` was `Markdown` or
// `Djot`. The root cause was that `inject_placeholders` in `extraction.rs`
// defaulted to `true` without checking `extract_images`, allowing the structure
// pipeline to call `populate_images_from_pdfium` unconditionally.

/// Helper: extract with a specific output format and images explicitly disabled
/// via `ImageExtractionConfig.extract_images = false`.
fn extract_no_images(relative_path: &str, fmt: OutputFormat) -> kreuzberg::types::ExtractionResult {
    use kreuzberg::core::config::ImageExtractionConfig;
    let path = test_documents_dir().join(relative_path);
    let config = ExtractionConfig {
        output_format: fmt,
        images: Some(ImageExtractionConfig {
            extract_images: false,
            ..Default::default()
        }),
        ..Default::default()
    };
    let rt = tokio::runtime::Runtime::new().unwrap();
    rt.block_on(kreuzberg::core::extractor::extract_file(&path, None, &config))
        .unwrap()
}

/// Helper: extract with a specific output format and images disabled via
/// `PdfConfig.extract_images = false`.
fn extract_no_images_via_pdf_options(relative_path: &str, fmt: OutputFormat) -> kreuzberg::types::ExtractionResult {
    use kreuzberg::core::config::pdf::PdfConfig;
    let path = test_documents_dir().join(relative_path);
    let config = ExtractionConfig {
        output_format: fmt,
        pdf_options: Some(PdfConfig {
            extract_images: false,
            ..Default::default()
        }),
        ..Default::default()
    };
    let rt = tokio::runtime::Runtime::new().unwrap();
    rt.block_on(kreuzberg::core::extractor::extract_file(&path, None, &config))
        .unwrap()
}

/// Regression #796: images must be absent when extract_images=false, output_format=Markdown.
///
/// Uses `embedded_images_tables.pdf` — a known-image PDF. Before the fix, this
/// returned `ExtractionResult.images` with full base64 data despite the flag.
#[test]
fn test_regression_796_markdown_no_images_when_disabled_via_images_config() {
    let result = extract_no_images("pdf/embedded_images_tables.pdf", OutputFormat::Markdown);
    assert!(
        result.images.as_ref().map(|v| v.is_empty()).unwrap_or(true),
        "images.extract_images=false must produce an empty images list even for \
         output_format=Markdown. Got {} image(s).",
        result.images.as_ref().map(|v| v.len()).unwrap_or(0)
    );
    // Confirm the text content was still extracted (no regression on content).
    assert!(
        !result.content.is_empty(),
        "Content must still be extracted when images are disabled"
    );
}

/// Regression #796: same assertion for Djot output format.
#[test]
fn test_regression_796_djot_no_images_when_disabled_via_images_config() {
    let result = extract_no_images("pdf/embedded_images_tables.pdf", OutputFormat::Djot);
    assert!(
        result.images.as_ref().map(|v| v.is_empty()).unwrap_or(true),
        "images.extract_images=false must produce an empty images list even for \
         output_format=Djot. Got {} image(s).",
        result.images.as_ref().map(|v| v.len()).unwrap_or(0)
    );
}

/// Regression #796: the pdf_options.extract_images path must also be respected
/// when output_format=Markdown.
#[test]
fn test_regression_796_markdown_no_images_when_disabled_via_pdf_options() {
    let result = extract_no_images_via_pdf_options("pdf/embedded_images_tables.pdf", OutputFormat::Markdown);
    assert!(
        result.images.as_ref().map(|v| v.is_empty()).unwrap_or(true),
        "pdf_options.extract_images=false must produce an empty images list even for \
         output_format=Markdown. Got {} image(s).",
        result.images.as_ref().map(|v| v.len()).unwrap_or(0)
    );
}

/// Sanity check: images must still appear when extract_images=true (no regression).
#[test]
fn test_regression_796_markdown_images_present_when_enabled() {
    use kreuzberg::core::config::ImageExtractionConfig;
    let path = test_documents_dir().join("pdf/embedded_images_tables.pdf");
    let config = ExtractionConfig {
        output_format: OutputFormat::Markdown,
        images: Some(ImageExtractionConfig {
            extract_images: true,
            ..Default::default()
        }),
        ..Default::default()
    };
    let rt = tokio::runtime::Runtime::new().unwrap();
    let result = rt
        .block_on(kreuzberg::core::extractor::extract_file(&path, None, &config))
        .unwrap();
    let images = result
        .images
        .as_ref()
        .expect("images must be Some when extract_images=true");
    assert!(
        !images.is_empty(),
        "images list must be non-empty when extract_images=true and the PDF contains images"
    );
}

/// Plain-text baseline: images must never appear for plain output (already passing
/// before the fix; kept as a safety net).
#[test]
fn test_regression_796_plain_no_images_when_disabled() {
    let result = extract_no_images("pdf/embedded_images_tables.pdf", OutputFormat::Plain);
    assert!(
        result.images.as_ref().map(|v| v.is_empty()).unwrap_or(true),
        "Plain output with extract_images=false must have no images. Got {} image(s).",
        result.images.as_ref().map(|v| v.len()).unwrap_or(0)
    );
}
