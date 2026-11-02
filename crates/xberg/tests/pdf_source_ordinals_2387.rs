//! PDF source ordinals remain semantic decimal markers without losing printed labels.

#![cfg(all(feature = "pdf", feature = "tokio-runtime"))]

mod helpers;

use helpers::extract_bytes_document_blocking;
use lopdf::{Document, Object, Stream, dictionary};
use sha2::{Digest, Sha256};
use xberg::core::config::{OutputFormat, PdfBackend, PdfConfig};
use xberg::types::document_structure::{AnnotationKind, NodeContent};
use xberg::{ExtractedDocument, ExtractionConfig};

fn pdf(lines: &[(&str, i64)]) -> Vec<u8> {
    let content: String = lines
        .iter()
        .enumerate()
        .map(|(index, (text, x))| format!("BT /F1 12 Tf {x} {} Td ({text}) Tj ET\n", 720 - index * 30))
        .collect();
    let mut doc = Document::with_version("1.7");
    let pages = doc.new_object_id();
    let font = doc.add_object(dictionary! { "Type" => "Font", "Subtype" => "Type1", "BaseFont" => "Helvetica" });
    let stream = doc.add_object(Stream::new(dictionary! {}, content.into_bytes()));
    let page = doc.add_object(dictionary! {
        "Type" => "Page", "Parent" => pages, "Contents" => stream,
        "MediaBox" => vec![0.into(), 0.into(), 612.into(), 792.into()],
        "Resources" => dictionary! { "Font" => dictionary! { "F1" => font } },
    });
    doc.objects.insert(
        pages,
        Object::Dictionary(dictionary! {
            "Type" => "Pages", "Kids" => vec![page.into()], "Count" => 1,
        }),
    );
    let catalog = doc.add_object(dictionary! { "Type" => "Catalog", "Pages" => pages });
    doc.trailer.set("Root", catalog);
    let mut bytes = Vec::new();
    doc.save_to(&mut bytes).expect("serialize genuine source PDF");
    let native = xberg_native_pdf::PdfDocument::from_bytes(bytes.clone()).expect("independently open source PDF");
    assert_eq!(native.page_count().unwrap(), 1);
    let raw = native.extract_text(0).expect("extract actual source text");
    for (line, _) in lines {
        assert!(raw.contains(line), "source text missing {line:?}: {raw:?}");
    }
    bytes
}

fn extract(bytes: &[u8], format: OutputFormat) -> ExtractedDocument {
    let config = ExtractionConfig {
        output_format: format,
        include_document_structure: true,
        use_cache: false,
        disable_ocr: true,
        pdf_options: Some(PdfConfig {
            backend: PdfBackend::Native,
            ..Default::default()
        }),
        ..Default::default()
    };
    let document = extract_bytes_document_blocking(bytes, "application/pdf", &config)
        .expect("actual public native PDF extraction");
    assert_eq!(document.mime_type, "application/pdf");
    assert!(!document.content.is_empty());
    assert!(document.document.is_some());
    document
}

fn label_for<'a>(document: &'a ExtractedDocument, body: &str) -> &'a str {
    let nodes = &document.document.as_ref().unwrap().nodes;
    let matched: Vec<_> = nodes
        .iter()
        .filter(|node| matches!(&node.content, NodeContent::ListItem { text } if text.contains(body)))
        .collect();
    assert_eq!(matched.len(), 1, "one actual list item must carry {body:?}: {nodes:?}");
    matched[0]
        .attributes
        .as_ref()
        .and_then(|attrs| attrs.get("list_marker"))
        .expect("actual producer retains the printed source marker")
}

fn assert_decimal_line(document: &ExtractedDocument, label: &str, body: &str) {
    let lines: Vec<_> = document.content.lines().filter(|line| line.contains(body)).collect();
    assert_eq!(lines.len(), 1, "one rendered source body: {body:?}");
    assert!(
        lines[0].trim_start().starts_with(&format!("{label} ")),
        "a source decimal ordinal must be a CommonMark marker, not a bullet or escaped literal: {:?}",
        lines[0]
    );
    assert!(
        !lines[0].contains("\\."),
        "ordinal must not render as literal escaped punctuation"
    );
}

fn qualify_labels(bytes: &[u8], expected: &[(&str, &str)]) -> ExtractedDocument {
    let plain = extract(bytes, OutputFormat::Plain);
    let markdown = extract(bytes, OutputFormat::Markdown);
    for (label, body) in expected {
        assert_eq!(label_for(&plain, body), *label);
        assert_eq!(label_for(&markdown, body), *label);
        assert!(plain.content.contains(label) && plain.content.contains(body));
        assert!(markdown.content.contains(body));
    }
    markdown
}

#[test]
fn public_contract_keeps_its_first_decimal_clause_as_an_ordered_item() {
    let path = std::path::Path::new(env!("CARGO_MANIFEST_DIR")).join("../../test_documents/pdf/sample_contract.pdf");
    let bytes =
        std::fs::read(&path).unwrap_or_else(|error| panic!("mandatory public source {}: {error}", path.display()));
    assert_eq!(bytes.len(), 49647);
    assert_eq!(
        hex::encode(Sha256::digest(&bytes)),
        "5c970b36c96f043f2be2ffe4f02746075e7b9210536130128f53e8d62894eb1a"
    );
    let native = xberg_native_pdf::PdfDocument::from_bytes(bytes.clone()).expect("open pinned public PDF");
    assert!(native.page_count().unwrap() > 0);
    let source: String = (0..native.page_count().unwrap())
        .map(|page| native.extract_text(page).unwrap())
        .collect();
    assert!(source.contains("DUTIES") && source.contains("CONSULTANT"));
    let document = qualify_labels(&bytes, &[("1.", "DUTIES")]);
    let node = document
        .document
        .as_ref()
        .unwrap()
        .nodes
        .iter()
        .find(|node| matches!(&node.content, NodeContent::ListItem { text } if text.contains("DUTIES")))
        .unwrap();
    assert!(
        node.annotations
            .iter()
            .any(|annotation| matches!(annotation.kind, AnnotationKind::Bold))
    );
    assert_decimal_line(&document, "1.", "DUTIES");
}

#[test]
fn consecutive_decimal_pdf_labels_render_as_ordered_markers() {
    let bytes = pdf(&[
        ("1. First obligation remains binding.", 72),
        ("2. Second obligation remains binding.", 72),
    ]);
    let document = qualify_labels(&bytes, &[("1.", "First obligation"), ("2.", "Second obligation")]);
    assert_decimal_line(&document, "1.", "First obligation");
    assert_decimal_line(&document, "2.", "Second obligation");
}

#[test]
fn non_one_decimal_start_preserves_actual_printed_numbers() {
    let bytes = pdf(&[
        ("7. Seventh obligation remains binding.", 72),
        ("8. Eighth obligation remains binding.", 72),
    ]);
    let document = qualify_labels(&bytes, &[("7.", "Seventh obligation"), ("8.", "Eighth obligation")]);
    assert_decimal_line(&document, "7.", "Seventh obligation");
    assert_decimal_line(&document, "8.", "Eighth obligation");
    assert!(
        !document
            .content
            .lines()
            .any(|line| line.trim_start().starts_with("1. "))
    );
}

#[test]
fn lettered_clauses_do_not_demote_the_preceding_decimal_clause() {
    let bytes = pdf(&[
        ("1. Principal obligation remains binding.", 72),
        ("A. Alpha qualification remains binding.", 96),
        ("B. Beta qualification remains binding.", 96),
    ]);
    let document = qualify_labels(
        &bytes,
        &[
            ("1.", "Principal obligation"),
            ("A.", "Alpha qualification"),
            ("B.", "Beta qualification"),
        ],
    );
    assert!(document.content.contains("A. Alpha qualification") && document.content.contains("B. Beta qualification"));
    assert_decimal_line(&document, "1.", "Principal obligation");
}

#[test]
fn nondecimal_source_labels_remain_literal_without_synthesized_ordinals() {
    let bytes = pdf(&[
        ("B. Beta obligation remains binding.", 72),
        ("D. Delta obligation remains binding.", 72),
    ]);
    let document = qualify_labels(&bytes, &[("B.", "Beta obligation"), ("D.", "Delta obligation")]);
    for (label, body) in [("B.", "Beta obligation"), ("D.", "Delta obligation")] {
        assert!(
            document
                .content
                .lines()
                .any(|line| line.trim_start().starts_with(&format!("- {label} ")) && line.contains(body))
        );
    }
    assert!(
        !document
            .content
            .lines()
            .any(|line| line.trim_start().starts_with("1. "))
    );
}

#[test]
fn ordinary_prose_and_unordered_source_items_do_not_gain_decimal_markers() {
    let bytes = pdf(&[
        ("Version 3.14 remains literal prose.", 72),
        ("- Actual unordered alpha item.", 72),
        ("- Actual unordered beta item.", 72),
    ]);
    let plain = extract(&bytes, OutputFormat::Plain);
    let document = extract(&bytes, OutputFormat::Markdown);
    for body in ["Version 3.14", "Actual unordered alpha", "Actual unordered beta"] {
        assert!(plain.content.contains(body) && document.content.contains(body));
    }
    assert!(
        document
            .content
            .lines()
            .any(|line| line.trim_start().starts_with("- Actual unordered alpha"))
    );
    assert!(
        document
            .content
            .lines()
            .any(|line| line.trim_start().starts_with("- Actual unordered beta"))
    );
    assert!(
        !document
            .content
            .lines()
            .any(|line| line.trim_start().starts_with("1. "))
    );
}

fn contract_clause_nodes(
    document: &ExtractedDocument,
) -> Vec<(&'static str, &xberg::types::document_structure::DocumentNode)> {
    [
        ("1.", "DUTIES"),
        ("A.", "exercise special skill"),
        ("B.", "personnel listed"),
    ]
    .into_iter()
    .map(|(label, phrase)| {
        assert_eq!(label_for(document, phrase), label);
        let nodes: Vec<_> = document
            .document
            .as_ref()
            .unwrap()
            .nodes
            .iter()
            .filter(|node| matches!(&node.content, NodeContent::ListItem { text } if text.contains(phrase)))
            .collect();
        assert_eq!(nodes.len(), 1);
        (label, nodes[0])
    })
    .collect()
}

fn full_contract_clauses() -> ExtractedDocument {
    let path = std::path::Path::new(env!("CARGO_MANIFEST_DIR")).join("../../test_documents/pdf/sample_contract.pdf");
    let bytes = std::fs::read(path).expect("mandatory pinned public contract");
    assert_eq!(bytes.len(), 49647);
    assert_eq!(
        hex::encode(Sha256::digest(&bytes)),
        "5c970b36c96f043f2be2ffe4f02746075e7b9210536130128f53e8d62894eb1a"
    );
    let plain = extract(&bytes, OutputFormat::Plain);
    let markdown = extract(&bytes, OutputFormat::Markdown);
    let clauses = contract_clause_nodes(&markdown);
    let plain_clauses = contract_clause_nodes(&plain);
    for ((label, node), (plain_label, plain_node)) in clauses.iter().zip(&plain_clauses) {
        assert_eq!(label, plain_label);
        assert_eq!(node.content, plain_node.content);
        let NodeContent::ListItem { text } = &node.content else {
            unreachable!()
        };
        assert!(plain.content.contains(&format!("{label} {text}")));
    }
    assert!(clauses[0].1.parent.is_some());
    assert!(clauses.iter().all(|(_, node)| node.parent == clauses[0].1.parent));
    let parent = clauses[0].1.parent.unwrap().0 as usize;
    assert!(matches!(
        markdown.document.as_ref().unwrap().nodes[parent].content,
        NodeContent::List { ordered: true }
    ));
    assert!(
        clauses[0]
            .1
            .annotations
            .iter()
            .any(|annotation| matches!(annotation.kind, AnnotationKind::Bold))
    );
    markdown
}

fn contract_paragraph_text<'a>(node: &'a comrak::nodes::AstNode<'a>) -> String {
    node.descendants()
        .filter_map(|child| match &child.data.borrow().value {
            comrak::nodes::NodeValue::Text(text) => Some(text.to_string()),
            _ => None,
        })
        .collect()
}

#[test]
fn public_contract_flat_alphabetic_clauses_are_paragraphs_beside_the_decimal_clause() {
    use comrak::nodes::NodeValue;
    let document = full_contract_clauses();
    let clauses = contract_clause_nodes(&document);
    let arena = comrak::Arena::new();
    let root = comrak::parse_document(&arena, &document.content, &comrak::Options::default());
    let nodes: Vec<_> = root.descendants().collect();
    let paragraphs: Vec<_> = clauses
        .iter()
        .map(|(label, node)| {
            let NodeContent::ListItem { text } = &node.content else {
                unreachable!()
            };
            let expected = if *label == "1." {
                text.clone()
            } else {
                format!("{label} {text}")
            };
            let matches: Vec<_> = nodes
                .iter()
                .copied()
                .filter(|node| {
                    matches!(node.data.borrow().value, NodeValue::Paragraph)
                        && contract_paragraph_text(node) == expected
                })
                .collect();
            assert_eq!(
                matches.len(),
                1,
                "complete original clause body must reach the reader: {label}"
            );
            matches[0]
        })
        .collect();
    let positions: Vec<_> = paragraphs
        .iter()
        .map(|paragraph| nodes.iter().position(|node| node.same_node(paragraph)).unwrap())
        .collect();
    assert!(positions.windows(2).all(|pair| pair[0] < pair[1]));
    assert!(
        paragraphs[0]
            .descendants()
            .any(|node| matches!(node.data.borrow().value, NodeValue::Strong))
    );
    let duty_item = paragraphs[0].parent().unwrap();
    assert!(matches!(duty_item.data.borrow().value, NodeValue::Item(_)));
    assert!(
        matches!(duty_item.parent().unwrap().data.borrow().value, NodeValue::List(list) if list.list_type == comrak::nodes::ListType::Ordered && list.start == 1)
    );
    for paragraph in &paragraphs[1..] {
        assert!(
            !paragraph
                .ancestors()
                .any(|node| matches!(node.data.borrow().value, NodeValue::List(_))),
            "flat source A/B clauses must remain literal paragraphs without invented nesting: {:?}",
            paragraph.data.borrow().value
        );
    }
}
