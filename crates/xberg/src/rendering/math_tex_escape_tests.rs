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
fn should_write_display_math_in_a_list_item_on_one_line_with_escaped_tex() {
    // Redaction cannot map a `$$` block that the list indent breaks, so a list item keeps one line.
    let mut builder = InternalDocumentBuilder::new("test");
    builder.push_list(false);
    builder.push_list_item("item", false, vec![], None, None);
    builder.push_formula(r"a \\ b", None, None);
    builder.end_list();
    let markdown = super::render_markdown(&builder.build());
    assert!(markdown.contains(r"$$a \\\ b$$"), "{markdown:?}");
}

#[test]
fn should_keep_a_dollar_in_tex_inside_its_math_for_a_reader_with_math() {
    // A "$" in the TeX is escaped for the reader, and the escaping must not turn into a bare "$"
    // that closes the math early.
    for tex in [r"a \$ b", r"\$5 + \$6", r"a $ b", r"\$\$"] {
        let source = format!("Let ${tex}$ hold, then.");
        let markdown = math_paragraph(&source, &[&format!("${tex}$")]);
        let arena = Arena::new();
        let root = parse_document(&arena, &markdown, &super::markdown::comrak_options());
        let math: Vec<_> = root
            .descendants()
            .filter_map(|node| match &node.data.borrow().value {
                NodeValue::Math(math) => Some(math.literal.clone()),
                _ => None,
            })
            .collect();
        let stray: Vec<_> = root
            .descendants()
            .filter_map(|node| match &node.data.borrow().value {
                NodeValue::Text(text) if text.contains('$') => Some(text.to_string()),
                _ => None,
            })
            .collect();
        let inner = &markdown[markdown.find('$').unwrap() + 1..markdown.rfind('$').unwrap()];
        for (at, _) in inner.match_indices('$') {
            let run = inner[..at].bytes().rev().take_while(|byte| *byte == b'\\').count();
            assert!(run % 2 == 1, "a bare dollar closes the math early in {markdown:?}");
        }
        assert_eq!(math.len(), 1, "markdown: {markdown:?}");
        assert!(stray.is_empty(), "markdown: {markdown:?}, stray: {stray:?}");
        assert_eq!(
            super::common::unescape_markdown_text(&math[0]).replace(r"\$", "$"),
            tex.replace(r"\$", "$"),
            "markdown: {markdown:?}"
        );
    }
}

/// TeX that ends in one, two, three and four backslashes.
const BACKSLASH_TAILS: [&str; 4] = ["x\\", "x\\\\", "x\\\\\\", "x\\\\\\\\"];

/// The display flag and literal of every math node that a reader with math on finds.
fn math_nodes(markdown: &str) -> Vec<(bool, String)> {
    let arena = Arena::new();
    let root = parse_document(&arena, markdown, &super::markdown::comrak_options());
    root.descendants()
        .filter_map(|node| match &node.data.borrow().value {
            NodeValue::Math(math) => Some((math.display_math, math.literal.clone())),
            _ => None,
        })
        .collect()
}

#[test]
fn should_close_display_math_whose_tex_ends_in_a_backslash() {
    // GitHub and comrak close `$$` after a backslash only when a space comes between them.
    for tex in BACKSLASH_TAILS {
        let source = format!("Then $${tex}$$ holds.");
        let markdown = math_paragraph(&source, &[&format!("$${tex}$$")]);
        let math = math_nodes(&markdown);
        assert_eq!(math.len(), 1, "markdown: {markdown:?}");
        assert!(math[0].0, "markdown: {markdown:?}");
        assert_eq!(
            super::common::unescape_markdown_text(&math[0].1).trim_end(),
            tex,
            "markdown: {markdown:?}"
        );
        assert_eq!(
            read_text(&markdown),
            format!("Then $${tex} $$ holds."),
            "markdown: {markdown:?}"
        );
    }
}

#[test]
fn should_write_inline_math_whose_tex_ends_in_a_backslash_as_code() {
    // No reader closes a single `$` right after a backslash, so the TeX is kept as code.
    for tex in BACKSLASH_TAILS {
        let source = format!("Let ${tex}$ hold.");
        let markdown = math_paragraph(&source, &[&format!("${tex}$")]);
        assert_eq!(markdown, format!("Let `{tex}` hold.\n"));
        assert!(math_nodes(&markdown).is_empty(), "markdown: {markdown:?}");
        assert_eq!(
            read_text(&markdown),
            format!("Let {tex} hold."),
            "markdown: {markdown:?}"
        );
    }
}

/// How GitHub reads what the writer wrote. Each row is the TeX, whether it is display math in prose,
/// the paragraph the writer produces, and the TeX that GitHub's own renderer returned for that
/// paragraph (`POST /markdown` with `mode=gfm`, 2026-11-12): the text of its math element, or of
/// the code span for a row that is written as code.
const GITHUB_READS: [(&str, bool, &str, &str); 10] = [
    ("a \\\\ b", false, "Let $a \\\\\\ b$ hold.\n", "a \\\\ b"),
    ("\\{a\\}", false, "Let $\\\\{a\\\\}$ hold.\n", "\\{a\\}"),
    ("a \\% b", false, "Let $a \\\\% b$ hold.\n", "a \\% b"),
    ("a*b*c", false, "Let $a&#42;b&#42;c$ hold.\n", "a*b*c"),
    ("a_b_c", false, "Let $a_b_c$ hold.\n", "a_b_c"),
    ("[a](u)", false, "Let $&#91;a&#93;(u)$ hold.\n", "[a](u)"),
    ("\\_ \\#", false, "Let $&#92;&#95; &#92;#$ hold.\n", "\\_ \\#"),
    ("a\\$ b", false, "Let $a\\\\\\$ b$ hold.\n", "a\\$ b"),
    ("x\\\\", false, "Let `x\\\\` hold.\n", "x\\\\"),
    ("x\\\\", true, "Then $$x\\\\\\ $$ holds.\n", "x\\\\ "),
];

#[test]
fn should_write_the_markdown_that_github_read_back_as_the_tex() {
    for (tex, display, written, github) in GITHUB_READS {
        let (source, span) = if display {
            (format!("Then $${tex}$$ holds."), format!("$${tex}$$"))
        } else {
            (format!("Let ${tex}$ hold."), format!("${tex}$"))
        };
        assert_eq!(math_paragraph(&source, &[&span]), written, "tex: {tex:?}");
        assert_eq!(github.trim_end(), tex, "recorded GitHub read of {written:?}");
    }
}
