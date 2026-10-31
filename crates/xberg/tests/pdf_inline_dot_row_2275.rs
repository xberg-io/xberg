#![cfg(feature = "pdf")]

mod helpers;

use helpers::{extract_bytes_document_blocking, get_test_file_path};
use sha2::{Digest, Sha256};
use xberg::core::config::{ExtractionConfig, OutputFormat, PdfBackend, PdfConfig};
use xberg::types::document_structure::{AnnotationKind, NodeContent};

#[test]
fn should_keep_aes_inline_multiplication_dot_as_ordinary_prose_2275() {
    let path = get_test_file_path("pdf/regression_pages/nist-fips-197-upd1-p16.pdf");
    let bytes =
        std::fs::read(&path).unwrap_or_else(|error| panic!("mandatory inline-dot source {}: {error}", path.display()));
    assert_eq!(
        hex::encode(Sha256::digest(&bytes)),
        "ddbb5a63987345c809b99487f583bc7eca2e9883131b38cc8e5fea59763ef33d"
    );
    assert!(bytes.starts_with(b"%PDF-"));
    let config = ExtractionConfig {
        output_format: OutputFormat::Markdown,
        include_document_structure: true,
        disable_ocr: true,
        use_cache: false,
        enable_quality_processing: false,
        pdf_options: Some(PdfConfig {
            backend: PdfBackend::Native,
            extract_tables: true,
            ..Default::default()
        }),
        ..Default::default()
    };
    let result = extract_bytes_document_blocking(&bytes, "application/pdf", &config).unwrap();
    let normalized = result
        .content
        .split_whitespace()
        .collect::<Vec<_>>()
        .join(" ")
        .replace('•', "·");
    assert!(
        normalized.contains("The symbol · denotes multiplication"),
        "the inline glyph must remain between its own prose neighbors: {}",
        result.content
    );
    assert!(
        result
            .content
            .lines()
            .all(|line| !matches!(line.trim(), "-" | "*" | "+" | "•" | "·")),
        "the inline dot must not create an empty list: {}",
        result.content
    );
    assert!(normalized.contains("Algebra [4]."), "retain the actual footnote body");
    let document = result.document.expect("requested public document structure");
    let node = document.nodes.iter().find(|node| {
        matches!(&node.content, NodeContent::Paragraph { text } if text.contains("The symbol") && text.contains(['•', '·']) && text.contains("denotes multiplication"))
    }).expect("the multiplication dot belongs to a prose node");
    let text = node.content.text().unwrap();
    let dot = u32::try_from(
        text.find(['•', '·'])
            .expect("actual ordinary multiplication-dot witness"),
    )
    .unwrap();
    assert!(
        node.annotations.iter().all(|annotation| {
            !matches!(annotation.kind, AnnotationKind::Superscript | AnnotationKind::Subscript)
                || annotation.end <= dot
                || annotation.start > dot
        }),
        "ordinary multiplication must not acquire a script annotation"
    );
}
