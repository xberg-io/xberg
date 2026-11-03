//! Parenthesis escapes preserve typed code and links without changing prose mode.

use super::{build_comrak_ast, comrak_options, render_markdown};
use crate::types::builder::{code, link};
use crate::types::document_structure::TextAnnotation;
use crate::types::internal::InternalDocument;
use crate::types::internal_builder::InternalDocumentBuilder;
use comrak::nodes::NodeValue;
use comrak::{Arena, parse_document};

fn paragraph(source: &str, annotations: Vec<TextAnnotation>, escape: bool) -> InternalDocument {
    let mut builder = InternalDocumentBuilder::new("test");
    builder.push_paragraph(source, annotations.clone(), None, None);
    let mut doc = builder.build();
    doc.escape_markdown = escape;
    assert_eq!(doc.elements[0].text, source);
    assert_eq!(doc.elements[0].annotations, annotations);
    doc
}

fn raw_markdown(doc: &InternalDocument) -> String {
    let arena = Arena::new();
    let root = build_comrak_ast(doc, &arena);
    let mut markdown = String::new();
    comrak::format_commonmark(root, &comrak_options(), &mut markdown).expect("format actual source AST");
    markdown
}

fn visible(markdown: &str) -> String {
    let arena = Arena::new();
    let root = parse_document(&arena, markdown, &comrak_options());
    root.descendants()
        .filter_map(|node| match &node.data.borrow().value {
            NodeValue::Text(text) => Some(text.to_string()),
            _ => None,
        })
        .collect()
}

fn code_values(markdown: &str) -> Vec<String> {
    let arena = Arena::new();
    let root = parse_document(&arena, markdown, &comrak_options());
    root.descendants()
        .filter_map(|node| match &node.data.borrow().value {
            NodeValue::Code(code) => Some(code.literal.clone()),
            NodeValue::CodeBlock(code) => Some(code.literal.clone()),
            _ => None,
        })
        .collect()
}

fn destinations(markdown: &str) -> Vec<(String, String)> {
    let arena = Arena::new();
    let root = parse_document(&arena, markdown, &comrak_options());
    root.descendants()
        .filter_map(|node| match &node.data.borrow().value {
            NodeValue::Link(link) => Some((link.url.clone(), link.title.clone())),
            _ => None,
        })
        .collect()
}

fn natural_prose(escape: bool) {
    let source = "Before (natural) and closing ) plus opening ( after";
    let doc = paragraph(source, vec![], escape);
    assert_eq!(visible(&raw_markdown(&doc)), source);
    let markdown = render_markdown(&doc);
    assert_eq!(markdown, format!("{source}\n"));
    assert_eq!(visible(&markdown), source);
    assert_eq!(doc.elements[0].text, source);
}

#[test]
fn ordinary_prose_parentheses_stay_literal_with_escaping_enabled() {
    natural_prose(true);
}

#[test]
fn ordinary_prose_parentheses_stay_literal_with_escaping_disabled() {
    natural_prose(false);
}

#[test]
fn leading_numeric_parenthesis_prose_round_trips_when_escaping_is_enabled() {
    let source = "3) Literal prose clause";
    let doc = paragraph(source, vec![], true);
    assert_eq!(visible(&raw_markdown(&doc)), source);
    let markdown = render_markdown(&doc);
    assert!(markdown.contains("Literal prose clause"));
    assert_eq!(doc.elements[0].text, source);
    assert_eq!(visible(&markdown), source, "{markdown:?}");
    let arena = Arena::new();
    let root = parse_document(&arena, &markdown, &comrak_options());
    assert!(
        !root
            .descendants()
            .any(|node| matches!(node.data.borrow().value, NodeValue::List(_)))
    );
}

#[test]
fn leading_numeric_parenthesis_prose_keeps_the_documented_unescaped_mode() {
    let source = "3) Literal prose clause";
    let doc = paragraph(source, vec![], false);
    assert_eq!(visible(&raw_markdown(&doc)), source);
    let markdown = render_markdown(&doc);
    assert_eq!(doc.elements[0].text, source);
    assert_eq!(markdown, format!("{source}\n"));
    let arena = Arena::new();
    let root = parse_document(&arena, &markdown, &comrak_options());
    let starts: Vec<_> = root
        .descendants()
        .filter_map(|node| match &node.data.borrow().value {
            NodeValue::List(list) => Some(list.start),
            _ => None,
        })
        .collect();
    assert_eq!(starts, [3]);
    assert_eq!(visible(&markdown), "Literal prose clause");
}

fn inline_code(escape: bool, source: &str) {
    let annotations = vec![code(0, source.len() as u32)];
    let doc = paragraph(source, annotations.clone(), escape);
    assert_eq!(code_values(&raw_markdown(&doc)), [source.to_string()]);
    let markdown = render_markdown(&doc);
    assert!(markdown.contains("literal"));
    assert_eq!(doc.elements[0].text, source);
    assert_eq!(doc.elements[0].annotations, annotations);
    assert_eq!(code_values(&markdown), [source.to_string()], "{markdown:?}");
}

#[test]
fn inline_code_keeps_literal_backslash_parentheses_with_escaping_enabled() {
    inline_code(true, r"literal \(left\) and \) \(");
}

#[test]
fn inline_code_keeps_natural_parentheses_with_escaping_disabled() {
    inline_code(false, "literal (left) and ) (");
}

fn fenced_code(escape: bool, source: &str) {
    let mut builder = InternalDocumentBuilder::new("test");
    builder.push_code(source, Some("text"), None, None);
    let mut doc = builder.build();
    doc.escape_markdown = escape;
    let expected = format!("{source}\n");
    assert_eq!(doc.elements[0].text, source);
    assert_eq!(code_values(&raw_markdown(&doc)), std::slice::from_ref(&expected));
    let markdown = render_markdown(&doc);
    assert!(markdown.contains("literal") && markdown.contains("unmatched"));
    assert_eq!(doc.elements[0].text, source);
    assert_eq!(code_values(&markdown), [expected], "{markdown:?}");
}

#[test]
fn fenced_code_keeps_literal_backslash_parentheses_with_escaping_enabled() {
    fenced_code(true, "literal \\(left\\)\n\\) unmatched \\(");
}

#[test]
fn fenced_code_keeps_natural_parentheses_with_escaping_disabled() {
    fenced_code(false, "literal (left)\n) unmatched (");
}

fn link_outputs(escape: bool) -> Vec<(String, String)> {
    let label = "Open destination (label)";
    let title = "Original (title)";
    [
        "https://example.test/a(b)c",
        "https://example.test/a)b",
        "https://example.test/a(b",
        r"https://example.test/a\(b\)c",
    ]
    .into_iter()
    .map(|url| {
        let annotations = vec![link(0, label.len() as u32, url, Some(title))];
        let doc = paragraph(label, annotations.clone(), escape);
        let raw = raw_markdown(&doc);
        assert_eq!(visible(&raw), label);
        assert_eq!(destinations(&raw), [(url.to_string(), title.to_string())]);
        let markdown = render_markdown(&doc);
        assert!(markdown.contains(label));
        assert_eq!(doc.elements[0].text, label);
        assert_eq!(doc.elements[0].annotations, annotations);
        (url.to_string(), markdown)
    })
    .collect()
}

fn link_round_trip(escape: bool) {
    let outputs = link_outputs(escape);
    assert_eq!(outputs.len(), 4);
    for (url, markdown) in outputs {
        assert_eq!(visible(&markdown), "Open destination (label)", "{markdown:?}");
        assert_eq!(
            destinations(&markdown),
            [(url, "Original (title)".to_string())],
            "{markdown:?}"
        );
    }
}

#[test]
fn typed_link_destinations_keep_parentheses_with_escaping_enabled() {
    link_round_trip(true);
}

#[test]
fn plain_typed_link_destinations_remain_intact_with_escaping_disabled() {
    let label = "Open ordinary destination";
    let url = "https://example.test/plain-path";
    let annotations = vec![link(0, label.len() as u32, url, None)];
    let doc = paragraph(label, annotations.clone(), false);
    let raw = raw_markdown(&doc);
    assert_eq!(visible(&raw), label);
    assert_eq!(destinations(&raw), [(url.to_string(), String::new())]);
    let markdown = render_markdown(&doc);
    assert_eq!(doc.elements[0].annotations, annotations);
    assert_eq!(visible(&markdown), label);
    assert_eq!(destinations(&markdown), [(url.to_string(), String::new())]);
}
