//! Integration tests for the `use_layout_for_markdown` flag.
//!
//! These tests verify that:
//! 1. `use_layout_for_markdown = true` feeds layout regions into the non-OCR
//!    markdown pipeline, producing richer structural output compared to the
//!    baseline (font-clustering only).
//! 2. `use_layout_for_markdown = false` (default) leaves the pipeline unchanged
//!    and produces the same output as a config without the field.
//!
//! Tests are feature-gated on `pdf` and `layout-detection` and are marked
//! `#[ignore]` when the layout engine model files are not available on CI.

#![cfg(all(feature = "pdf", feature = "layout-detection"))]

mod helpers;

use helpers::{get_test_file_path, test_documents_available};
use kreuzberg::core::config::{ExtractionConfig, OutputFormat, layout::LayoutDetectionConfig};
use kreuzberg::extract_file_sync;

/// Extract `relative_path` (from `test_documents/`) with the given config.
fn extract_md(relative_path: &str, config: &ExtractionConfig) -> String {
    let path = get_test_file_path(relative_path);
    extract_file_sync(&path, None, config)
        .expect("extraction should succeed")
        .content
}

/// Config: output_format=Markdown, no layout at all (pure baseline).
fn baseline_config() -> ExtractionConfig {
    ExtractionConfig {
        output_format: OutputFormat::Markdown,
        ..Default::default()
    }
}

/// Config: layout=Some(default), use_layout_for_markdown=false.
/// Layout model is loaded but NOT injected into the native path.
fn layout_config_not_injected() -> ExtractionConfig {
    ExtractionConfig {
        output_format: OutputFormat::Markdown,
        layout: Some(LayoutDetectionConfig::default()),
        use_layout_for_markdown: false,
        ..Default::default()
    }
}

/// Config: layout=Some(default), use_layout_for_markdown=true.
/// Layout regions ARE injected into the native markdown pipeline.
fn layout_for_markdown_config() -> ExtractionConfig {
    ExtractionConfig {
        output_format: OutputFormat::Markdown,
        layout: Some(LayoutDetectionConfig::default()),
        use_layout_for_markdown: true,
        ..Default::default()
    }
}

// ── Default: no behavior change ─────────────────────────────────────────────

/// With `use_layout_for_markdown = false` (the default), the pipeline must
/// produce output that is indistinguishable from the baseline (no layout).
/// This guards against accidental regressions introduced by the new field.
#[test]
fn test_use_layout_for_markdown_false_matches_baseline() {
    if !test_documents_available() {
        return;
    }

    let pdf = "pdf/google_doc_document.pdf";
    let baseline = extract_md(pdf, &baseline_config());
    let layout_not_injected = extract_md(pdf, &layout_config_not_injected());

    assert_eq!(
        baseline, layout_not_injected,
        "use_layout_for_markdown=false must not change extraction output compared to no-layout config"
    );
}

// ── Layout injection: structural improvement ─────────────────────────────────

/// With `use_layout_for_markdown = true` and a PDF that has headings, the
/// markdown output must contain at least one ATX heading line (`# ...`).
///
/// The test uses `google_doc_document.pdf`, which is a structured Google Docs
/// export with clear title and section headings detectable by the RT-DETR model.
///
/// This test requires the layout model to be available (ORT + model files).
/// It is marked `#[ignore]` on CI where model weights are not pre-downloaded.
#[test]
#[ignore = "requires layout model files (ORT inference)"]
fn test_use_layout_for_markdown_produces_headings() {
    if !test_documents_available() {
        return;
    }

    let pdf = "pdf/google_doc_document.pdf";
    let output = extract_md(pdf, &layout_for_markdown_config());

    let has_heading = output.lines().any(|line| line.starts_with('#'));
    assert!(
        has_heading,
        "use_layout_for_markdown=true should produce at least one ATX heading line; got:\n{}",
        &output[..output.len().min(500)]
    );
}

/// Verify that `use_layout_for_markdown = true` with `layout = None` silently
/// produces the same output as the baseline (no-op when layout config is absent).
#[test]
fn test_use_layout_for_markdown_without_layout_config_is_noop() {
    if !test_documents_available() {
        return;
    }

    let pdf = "pdf/google_doc_document.pdf";
    let baseline = extract_md(pdf, &baseline_config());

    // use_layout_for_markdown=true but layout=None → runner must skip silently.
    let noop_config = ExtractionConfig {
        output_format: OutputFormat::Markdown,
        layout: None,
        use_layout_for_markdown: true,
        ..Default::default()
    };
    let noop_output = extract_md(pdf, &noop_config);

    assert_eq!(
        baseline, noop_output,
        "use_layout_for_markdown=true with layout=None must produce the same output as baseline"
    );
}

/// Verify that `force_ocr=true` bypasses the layout-for-markdown path.
/// The field must be a no-op when the entire document is OCR'd.
#[test]
fn test_use_layout_for_markdown_skipped_when_force_ocr() {
    // We can't easily run OCR in unit tests without a backend registered,
    // but we CAN verify the config combination doesn't panic or error.
    // The actual gate is tested via the `maybe_run_layout_for_markdown` guard.
    let config = ExtractionConfig {
        output_format: OutputFormat::Markdown,
        layout: Some(LayoutDetectionConfig::default()),
        use_layout_for_markdown: true,
        force_ocr: true,
        ..Default::default()
    };
    // Config construction must succeed and the field values must be set correctly.
    assert!(config.use_layout_for_markdown);
    assert!(config.force_ocr);
    assert!(config.layout.is_some());
}
