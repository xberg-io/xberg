//! Integration test for the experimental PP-DocLayout-V3 layout model selector.
//!
//! Marked `#[ignore]` because it downloads the ~131 MB V3 weights and needs ONNX Runtime.

#![cfg(all(feature = "pdf", feature = "layout-detection"))]

mod helpers;

use helpers::{extract_uri_document_blocking, get_test_file_path, test_documents_available};
use xberg::core::config::{ExtractionConfig, OutputFormat, layout::LayoutDetectionConfig};
use xberg::{LayoutDetectionModel, LayoutStrategy};

#[test]
#[ignore = "requires PP-DocLayout-V3 model files (ORT inference, ~131 MB download)"]
fn pp_doclayout_v3_runs_on_a_public_pdf_page() {
    if !test_documents_available() {
        return;
    }

    let config = ExtractionConfig {
        output_format: OutputFormat::Markdown,
        layout: Some(LayoutDetectionConfig {
            model: LayoutDetectionModel::PpDocLayoutV3,
            strategy: LayoutStrategy::Always,
            ..Default::default()
        }),
        use_layout_for_markdown: true,
        ..Default::default()
    };
    let path = get_test_file_path("pdf/google_doc_document.pdf");
    let result = extract_uri_document_blocking(&path, None, &config).expect("V3 extraction should succeed");

    assert!(!result.content.trim().is_empty(), "extraction must produce text");
    let layout_warnings: Vec<_> = result
        .processing_warnings
        .iter()
        .filter(|warning| warning.source == "layout")
        .collect();
    assert!(
        layout_warnings.is_empty(),
        "V3 layout detection must run without layout warnings: {layout_warnings:?}"
    );
}
