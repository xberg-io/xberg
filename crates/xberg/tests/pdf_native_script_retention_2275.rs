#![cfg(feature = "pdf")]

mod helpers;

use helpers::{extract_bytes_document_blocking, get_test_documents_dir};
use sha2::{Digest, Sha256};
use std::path::PathBuf;
use xberg::{ExtractionConfig, OutputFormat};

fn extract_retention_page(name: &str, expected_hash: &str) -> xberg::ExtractedDocument {
    let root = std::env::var_os("XBERG_SCRIPT_RETENTION_FIXTURE_ROOT")
        .map(PathBuf::from)
        .unwrap_or_else(get_test_documents_dir);
    let bytes = std::fs::read(root.join("pdf/regression_pages").join(name)).expect("mandatory retention fixture");
    assert_eq!(
        hex::encode(Sha256::digest(&bytes)),
        expected_hash,
        "exact retention source identity"
    );
    let config = ExtractionConfig {
        disable_ocr: true,
        use_cache: false,
        enable_quality_processing: false,
        output_format: OutputFormat::Markdown,
        include_document_structure: true,
        ..Default::default()
    };
    let result =
        extract_bytes_document_blocking(&bytes, "application/pdf", &config).expect("native retention extraction");
    if let Some(directory) = std::env::var_os("XBERG_SCRIPT_RETENTION_RECEIPTS") {
        let directory = PathBuf::from(directory);
        std::fs::create_dir_all(&directory).expect("receipt directory");
        std::fs::write(
            directory.join(format!("{name}.result.json")),
            serde_json::to_vec_pretty(&result).expect("serialize actual result"),
        )
        .expect("write actual result");
        let native = xberg_native_pdf::PdfDocument::from_bytes(bytes).expect("native source document");
        let page = native
            .extract_page_text_with_script_geometry(0, xberg_native_pdf::document::ReadingOrder::TopToBottom)
            .expect("native preserved spans");
        std::fs::write(
            directory.join(format!("{name}.native-spans.json")),
            serde_json::to_vec_pretty(&page.spans).expect("serialize actual native spans"),
        )
        .expect("write actual spans");
    }
    result
}

#[test]
fn should_retain_all_arxiv_reference_identifiers_after_native_script_grouping() {
    let result = extract_retention_page(
        "arxiv-2312.13591v2-p16.pdf",
        "a8d2e0673d25e4bb72e9babeecacfd0e454bcb5c307b634784df1b134c9d9e37",
    );
    assert_eq!(
        result.content.matches("arXiv:").count(),
        36,
        "retain every baseline-known reference identifier"
    );
    for sentinel in [
        "2108.12730",
        "2311.13968",
        "2307.04168",
        "2311.04695",
        "1805.08384",
        "1410.7069",
    ] {
        assert!(
            result.content.contains(sentinel),
            "retain source reference tail {sentinel}"
        );
    }
}

#[test]
fn should_preserve_existing_gutenberg_denominator_script_and_word_joining() {
    let result = extract_retention_page(
        "gutenberg-33283-p27.pdf",
        "4d7fe54cedd4daf177144e6b8a7d863b7e071e637d259b4bb4006835c07cf2d3",
    );
    assert!(
        result.content.contains("dx<sup>2</sup>"),
        "retain the existing denominator superscript"
    );
    assert!(
        result.content.contains("following way."),
        "retain existing line-break dehyphenation"
    );
}
