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
    use kreuzberg::core::config::ImageExtractionConfig;
    let path = test_documents_dir().join(relative_path);
    let config = ExtractionConfig {
        output_format: OutputFormat::Markdown,
        images: Some(ImageExtractionConfig {
            extract_images: true,
            ..Default::default()
        }),
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
/// Root cause: `populate_images_from_oxide` used `Vec::contains` (O(N)) inside
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
// pipeline to call `populate_images_from_oxide` unconditionally.

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

// ─── Content-level image suppression tests ───────────────────────────────────
//
// The earlier #796 tests only assert `result.images.is_empty()`. That field is
// gated separately (extraction.rs:112) and is always empty when
// `extract_images=false`, even if the `inject_placeholders` guard at line 216 is
// removed. The guard controls whether `ElementKind::Image` elements are injected
// into the InternalDocument — which in turn controls whether image placeholder
// references (`![]()` / `![](image_N.fmt)`) appear in `result.content`.
//
// The Djot renderer (`djot.rs`) lacked the `doc.images.get()` None check that
// comrak_bridge, html_styled, and plain all have. Removing the guard would cause
// `![]()` to leak into Djot content with no test catching it.
//
// JSON renderer gap (out of scope): json.rs emits `{"type":"image","alt":null,"src":null}`
// for orphaned elements — null fields are valid structured JSON and produce no broken
// markup, so it is intentionally not addressed here.

/// Djot content must not contain image markup when `extract_images=false`.
///
/// End-to-end contract test: requires both the `inject_placeholders` guard in
/// `extraction.rs` AND the Djot renderer's `None` guard to be absent before it
/// fails. The renderer-level unit test `test_djot_renderer_skips_orphaned_image_element`
/// in `djot.rs` is the minimal proof that the renderer fix works independently.
#[test]
fn test_djot_content_has_no_image_refs_when_extraction_disabled() {
    let result = extract_no_images("pdf/embedded_images_tables.pdf", OutputFormat::Djot);
    assert!(
        !result.content.contains("![]()"),
        "Djot output must not contain empty ![]() refs when extract_images=false.\n\
         Got content:\n{}",
        &result.content[..result.content.len().min(400)]
    );
    assert!(
        !result.content.contains("![](image_"),
        "Djot output must not contain image placeholder refs when extract_images=false.\n\
         Got content:\n{}",
        &result.content[..result.content.len().min(400)]
    );
    // Text content must still be present — no regression on extraction.
    assert!(
        !result.content.is_empty(),
        "Djot content must not be empty when images are disabled"
    );
}

/// Markdown content must not contain image markup when `extract_images=false`.
///
/// comrak_bridge already has a None guard so this would pass even without the
/// extraction-level guard, but it pins the end-to-end contract explicitly.
#[test]
fn test_markdown_content_has_no_image_refs_when_extraction_disabled() {
    let result = extract_no_images("pdf/embedded_images_tables.pdf", OutputFormat::Markdown);
    assert!(
        !result.content.contains("![]()"),
        "Markdown output must not contain empty ![]() refs when extract_images=false.\n\
         Got content:\n{}",
        &result.content[..result.content.len().min(400)]
    );
    assert!(
        !result.content.contains("![](image_"),
        "Markdown output must not contain image placeholder refs when extract_images=false.\n\
         Got content:\n{}",
        &result.content[..result.content.len().min(400)]
    );
}

/// Djot content must not contain image markup when disabled via `pdf_options.extract_images`.
///
/// Verifies both config paths are covered — mirrors the existing `result.images`
/// test for the pdf_options path.
#[test]
fn test_djot_content_has_no_image_refs_when_disabled_via_pdf_options() {
    let result = extract_no_images_via_pdf_options("pdf/embedded_images_tables.pdf", OutputFormat::Djot);
    assert!(
        !result.content.contains("![]()"),
        "Djot output (pdf_options path) must not contain ![]() when extract_images=false"
    );
    assert!(
        !result.content.contains("![](image_"),
        "Djot output (pdf_options path) must not contain image refs when extract_images=false"
    );
}

// ─── Page-level and chunk-level image index references ────────────────────────
//
// Pages carry `image_indices: Vec<usize>` — zero-based indices into the
// top-level `ExtractionResult.images` collection. Chunks carry the same field.

fn extract_with_pages_and_images(relative_path: &str) -> kreuzberg::types::ExtractionResult {
    use kreuzberg::core::config::{ImageExtractionConfig, PageConfig};
    let path = test_documents_dir().join(relative_path);
    let config = ExtractionConfig {
        output_format: OutputFormat::Markdown,
        pages: Some(PageConfig {
            extract_pages: true,
            ..Default::default()
        }),
        images: Some(ImageExtractionConfig {
            extract_images: true,
            ..Default::default()
        }),
        ..Default::default()
    };
    let rt = tokio::runtime::Runtime::new().unwrap();
    rt.block_on(extract_file(&path, None, &config)).unwrap()
}

/// Pages that contain images must have non-empty `image_indices` pointing into
/// `ExtractionResult.images`. Every index must be in-bounds.
#[test]
fn test_page_image_indices_are_valid_when_images_extracted() {
    let result = extract_with_pages_and_images("pdf/embedded_images_tables.pdf");

    let images = result.images.as_ref().expect("images must be Some");
    assert!(!images.is_empty(), "fixture must have extracted images");

    let pages = result
        .pages
        .as_ref()
        .expect("pages must be Some when extract_pages=true");
    assert!(!pages.is_empty(), "fixture must have pages");

    // At least one page must carry image_indices (not all pages need images).
    let pages_with_images: Vec<_> = pages.iter().filter(|p| !p.image_indices.is_empty()).collect();
    assert!(
        !pages_with_images.is_empty(),
        "at least one page must have image_indices populated when the PDF contains images"
    );

    // Every index must be in-bounds and the referenced image must report
    // belonging to this page (cross-validation: wrong-page bugs would pass a
    // bounds-only check).
    for page in pages {
        for &idx in &page.image_indices {
            assert!(
                idx < images.len(),
                "page {} image_indices[{}] = {} is out of bounds (images.len() = {})",
                page.page_number,
                idx,
                idx,
                images.len()
            );
            let img_page = images[idx].page_number;
            assert_eq!(
                img_page,
                Some(page.page_number),
                "image at index {} has page_number {:?} but is referenced by page {}",
                idx,
                img_page,
                page.page_number
            );
        }
    }
}

/// `image_indices` on pages must be empty when image extraction is disabled.
#[test]
fn test_page_image_indices_empty_when_images_disabled() {
    use kreuzberg::core::config::{ImageExtractionConfig, PageConfig};
    let path = test_documents_dir().join("pdf/embedded_images_tables.pdf");
    let config = ExtractionConfig {
        output_format: OutputFormat::Markdown,
        pages: Some(PageConfig {
            extract_pages: true,
            ..Default::default()
        }),
        images: Some(ImageExtractionConfig {
            extract_images: false,
            ..Default::default()
        }),
        ..Default::default()
    };
    let rt = tokio::runtime::Runtime::new().unwrap();
    let result = rt.block_on(extract_file(&path, None, &config)).unwrap();

    if let Some(pages) = result.pages.as_ref() {
        for page in pages {
            assert!(
                page.image_indices.is_empty(),
                "page {} must have no image_indices when extract_images=false",
                page.page_number
            );
        }
    }
}

#[cfg(feature = "chunking")]
fn extract_with_pages_images_and_chunks(relative_path: &str) -> kreuzberg::types::ExtractionResult {
    use kreuzberg::core::config::{ChunkingConfig, ImageExtractionConfig, PageConfig};
    let path = test_documents_dir().join(relative_path);
    let config = ExtractionConfig {
        output_format: OutputFormat::Markdown,
        pages: Some(PageConfig {
            extract_pages: true,
            ..Default::default()
        }),
        images: Some(ImageExtractionConfig {
            extract_images: true,
            ..Default::default()
        }),
        chunking: Some(ChunkingConfig {
            max_characters: 500,
            ..Default::default()
        }),
        ..Default::default()
    };
    let rt = tokio::runtime::Runtime::new().unwrap();
    rt.block_on(extract_file(&path, None, &config)).unwrap()
}

/// Chunks that span pages containing images must have non-empty `image_indices`.
/// Every index must be in-bounds, and the referenced image's `page_number` must
/// fall within the chunk's `[first_page, last_page]` range.
#[cfg(feature = "chunking")]
#[test]
fn test_chunk_image_indices_are_valid_when_images_extracted() {
    let result = extract_with_pages_images_and_chunks("pdf/embedded_images_tables.pdf");

    let images = result.images.as_ref().expect("images must be Some");
    assert!(!images.is_empty(), "fixture must have extracted images");

    let chunks = result
        .chunks
        .as_ref()
        .expect("chunks must be Some when chunking is configured");
    assert!(!chunks.is_empty(), "fixture must produce chunks");

    // At least one chunk must carry image_indices.
    let chunks_with_images: Vec<_> = chunks.iter().filter(|c| !c.metadata.image_indices.is_empty()).collect();
    assert!(
        !chunks_with_images.is_empty(),
        "at least one chunk must have image_indices when the PDF contains images"
    );

    for chunk in chunks {
        for &idx in &chunk.metadata.image_indices {
            // In-bounds check.
            assert!(
                idx < images.len(),
                "chunk image_indices[{}] = {} is out of bounds (images.len() = {})",
                idx,
                idx,
                images.len()
            );

            // Cross-validation: referenced image must belong to a page within
            // the chunk's page range.
            if let (Some(first), Some(last)) = (chunk.metadata.first_page, chunk.metadata.last_page) {
                let img_page = images[idx]
                    .page_number
                    .expect("image referenced by a chunk must have a page_number set");
                assert!(
                    img_page >= first && img_page <= last,
                    "image at index {} is on page {} but chunk covers pages [{}, {}]",
                    idx,
                    img_page,
                    first,
                    last
                );
            }
        }
    }
}

/// `image_indices` on chunks must be empty when image extraction is disabled.
#[cfg(feature = "chunking")]
#[test]
fn test_chunk_image_indices_empty_when_images_disabled() {
    use kreuzberg::core::config::{ChunkingConfig, ImageExtractionConfig};
    let path = test_documents_dir().join("pdf/embedded_images_tables.pdf");
    let config = ExtractionConfig {
        output_format: OutputFormat::Markdown,
        images: Some(ImageExtractionConfig {
            extract_images: false,
            ..Default::default()
        }),
        chunking: Some(ChunkingConfig {
            max_characters: 500,
            ..Default::default()
        }),
        ..Default::default()
    };
    let rt = tokio::runtime::Runtime::new().unwrap();
    let result = rt.block_on(extract_file(&path, None, &config)).unwrap();

    if let Some(chunks) = result.chunks.as_ref() {
        for chunk in chunks {
            assert!(
                chunk.metadata.image_indices.is_empty(),
                "chunk must have no image_indices when extract_images=false"
            );
        }
    }
}
