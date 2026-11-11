//! Math written as dollar math must keep its TeX when a GitHub-style Markdown reader reads it.
//!
//! Such a reader applies the CommonMark escapes and the inline rules (emphasis, links,
//! strikethrough) to the TeX before it looks for math. The tests read the output with comrak, once
//! with its math extension off, so the escapes apply as they do on GitHub, and once with it on.

use crate::types::document_structure::{AnnotationKind, TextAnnotation};
use crate::types::internal_builder::InternalDocumentBuilder;
use comrak::nodes::{AstNode, NodeValue};
use comrak::{Arena, Options, parse_document};

fn github_reader() -> Options<'static> {
    let mut options = super::markdown::comrak_options();
    options.extension.math_dollars = false;
    options.extension.math_code = false;
    options
}

/// The visible text of every table cell, or of the whole document when it holds no table.
fn visible_text<'a>(node: &'a AstNode<'a>) -> String {
    node.descendants()
        .filter_map(|node| match &node.data.borrow().value {
            NodeValue::Text(text) => Some(text.to_string()),
            NodeValue::Code(code) => Some(code.literal.clone()),
            NodeValue::SoftBreak | NodeValue::LineBreak => Some("\n".to_string()),
            NodeValue::HtmlInline(html) => Some(format!("<HTML {html}>")),
            _ => None,
        })
        .collect()
}

fn read_text(markdown: &str) -> String {
    let arena = Arena::new();
    visible_text(parse_document(&arena, markdown, &github_reader()))
}

fn read_cells(markdown: &str) -> Vec<String> {
    let arena = Arena::new();
    let root = parse_document(&arena, markdown, &github_reader());
    root.descendants()
        .filter(|node| matches!(node.data.borrow().value, NodeValue::TableCell))
        .map(visible_text)
        .collect()
}

fn math_paragraph(text: &str, spans: &[&str]) -> String {
    let annotations = spans
        .iter()
        .map(|span| {
            let start = text.find(span).unwrap() as u32;
            TextAnnotation {
                start,
                end: start + span.len() as u32,
                kind: AnnotationKind::math(span.starts_with("$$")),
            }
        })
        .collect();
    let mut builder = InternalDocumentBuilder::new("test");
    builder.push_paragraph(text, annotations, None, None);
    super::render_markdown(&builder.build())
}

const TEX_CASES: [&str; 16] = [
    r"a \\ b",
    r"\{a\}",
    r"a \% b",
    r"\begin{aligned} a &= b \\ c &= d \end{aligned}",
    "a*b*c",
    "a_b_c",
    "a~~b~~c",
    "[a](u)",
    r"\$\$",
    "x<y",
    r"a\,b",
    r"\left\| x \right\|",
    r"\_ \# \( \) \- \=",
    "x_{i} + _y_ + a &b &#42; &amp;",
    "f[x](y) \\[2pt]",
    "p <b> q </i>",
];

#[test]
fn should_keep_inline_tex_when_a_reader_applies_markdown_escapes() {
    for tex in TEX_CASES {
        let source = format!("Let ${tex}$ hold.");
        let markdown = math_paragraph(&source, &[&format!("${tex}$")]);
        assert_eq!(read_text(&markdown), source, "markdown: {markdown:?}");
    }
}

#[test]
fn should_keep_display_tex_inside_prose_when_a_reader_applies_markdown_escapes() {
    for tex in TEX_CASES {
        let source = format!("Then $${tex}$$ holds.");
        let markdown = math_paragraph(&source, &[&format!("$${tex}$$")]);
        assert_eq!(read_text(&markdown), source, "markdown: {markdown:?}");
    }
}

#[test]
fn should_keep_tex_in_a_table_cell_when_a_reader_applies_markdown_escapes() {
    for tex in [r"\|x\|", r"a \\ b", r"|x|"] {
        let text = format!("${tex}$");
        let (cells, encoded) =
            super::table_math::split_cell_grid(vec![vec![(text.clone(), vec![(0..text.len(), false)])]]);
        let markdown = super::table_math::render_table_markdown(&cells, encoded.as_deref());
        assert_eq!(read_cells(&markdown), [text], "markdown: {markdown:?}");
    }
}

#[test]
fn should_not_leave_a_dollar_for_a_reader_to_misread_after_a_closing_dollar() {
    // A closing "$" followed by a digit is not a closer, and "$a$$b$" reads as a "$$" run.
    for (source, spans) in [
        ("Cost $x$5 here.", vec!["$x$"]),
        ("Pair $a$$b$ here.", vec!["$a$", "$b$"]),
    ] {
        let markdown = math_paragraph(source, &spans);
        let arena = Arena::new();
        let root = parse_document(&arena, &markdown, &super::markdown::comrak_options());
        let stray: Vec<_> = root
            .descendants()
            .filter_map(|node| match &node.data.borrow().value {
                NodeValue::Text(text) if text.contains('$') => Some(text.to_string()),
                _ => None,
            })
            .collect();
        assert!(stray.is_empty(), "markdown: {markdown:?}, stray: {stray:?}");
        assert!(!markdown.contains("$$"), "markdown: {markdown:?}");
    }
}

#[test]
fn should_keep_display_form_for_display_math_in_a_list_item() {
    let mut builder = InternalDocumentBuilder::new("test");
    builder.push_list(false);
    builder.push_list_item("item", false, vec![], None, None);
    builder.push_formula(r"a \\ b", None, None);
    builder.end_list();
    let markdown = super::render_markdown(&builder.build());
    assert!(
        markdown.lines().any(|line| line.trim() == "$$"),
        "display math in a list item must open and close on its own lines: {markdown:?}"
    );
}
