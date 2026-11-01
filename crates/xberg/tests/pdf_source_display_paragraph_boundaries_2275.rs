#![cfg(all(
    feature = "full",
    feature = "formula-recognition",
    feature = "bundle-tessdata-eng"
))]

mod helpers;

use helpers::{extract_uri_document_blocking, get_test_file_path};
use sha2::{Digest, Sha256};
use xberg::core::config::{ExtractionConfig, OutputFormat, PdfBackend};
use xberg::types::document_structure::{
    AnnotationKind, DocumentNode, DocumentStructure, NodeContent,
};

fn source_document() -> xberg::ExtractedDocument {
    let path = get_test_file_path("pdf/regression_pages/gutenberg-33283-p33.pdf");
    let bytes = std::fs::read(&path).expect("mandatory visually inspected public source page");
    assert!(bytes.starts_with(b"%PDF-"));
    assert_eq!(
        hex::encode(Sha256::digest(&bytes)),
        "f0accfac5c9d9b1f819d1f475d947c59aeda0366bf9e98c9e90653720c75eaa3",
        "exact source page identity"
    );
    let config = ExtractionConfig {
        disable_ocr: true,
        use_cache: false,
        enable_quality_processing: false,
        output_format: OutputFormat::Markdown,
        include_document_structure: true,
        ..Default::default()
    };
    assert!(config.pdf_options.is_none(), "canonical PDF defaults");
    assert!(matches!(PdfBackend::default(), PdfBackend::Native));
    let result = extract_uri_document_blocking(&path, Some("application/pdf"), &config)
        .expect("actual canonical public URI extraction");
    assert!(result.content.contains("CALCULUS MADE EASY"));
    assert!(!result
        .document
        .as_ref()
        .expect("requested native structure")
        .nodes
        .is_empty());
    result
}

fn unique_paragraph<'a>(
    document: &'a DocumentStructure,
    anchor: &str,
) -> (usize, &'a DocumentNode) {
    let nodes: Vec<_> = document
        .nodes
        .iter()
        .enumerate()
        .filter(|(_, node)| {
            node.content
                .text()
                .is_some_and(|text| text.contains(anchor))
        })
        .collect();
    assert_eq!(nodes.len(), 1, "one actual source context for {anchor:?}");
    let (index, node) = nodes[0];
    assert!(matches!(node.content, NodeContent::Paragraph { .. }));
    (index, node)
}

fn assert_opening_powers(document: &DocumentStructure) {
    let (_, node) = unique_paragraph(document, "and subtracting y = x");
    let text = node.content.text().unwrap();
    assert!(
        text.contains("exactly as we supposed."),
        "complete opening source explanation"
    );
    let powers: Vec<_> = node
        .annotations
        .iter()
        .filter(|annotation| annotation.kind == AnnotationKind::Superscript)
        .map(|annotation| {
            let start = annotation.start as usize;
            let end = annotation.end as usize;
            assert!(text
                .get(..start)
                .expect("valid native UTF-8 start")
                .ends_with('x'));
            text.get(start..end)
                .expect("valid native UTF-8 power range")
        })
        .collect();
    assert_eq!(
        powers,
        ["5", "4", "4"],
        "preserve all three source-owned opening powers"
    );
}

#[test]
fn should_keep_source_prose_starts_separate_from_the_derivative_display_2275() {
    let result = source_document();
    let document = result.document.as_ref().unwrap();
    assert_opening_powers(document);
    let (display_index, _) = unique_paragraph(document, "= nx");
    let (example_index, example) = unique_paragraph(document, "For example, let n = 8");
    let (rule_index, rule) =
        unique_paragraph(document, "And, indeed, the rule that differentiating");
    assert!(example
        .content
        .text()
        .unwrap()
        .contains("differentiating it would"));
    assert!(rule
        .content
        .text()
        .unwrap()
        .contains("whole number and positive."));
    unique_paragraph(document, "question whether it is true for cases");
    assert_eq!(
        (
            example.content.text().unwrap().starts_with("For example,"),
            rule.content.text().unwrap().starts_with("And, indeed,"),
            display_index < example_index && example_index < rule_index,
        ),
        (true, true, true),
        "both visually indented source prose paragraphs start after the preceding display"
    );
}
