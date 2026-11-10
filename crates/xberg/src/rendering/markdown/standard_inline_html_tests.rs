use super::render_markdown;
use crate::types::builder::{bold, code, italic, subscript, superscript};
use crate::types::document_structure::{AnnotationKind, TextAnnotation};
use crate::types::internal::InternalDocument;
use crate::types::internal_builder::InternalDocumentBuilder;

fn paragraph(text: &str, annotations: Vec<TextAnnotation>) -> InternalDocument {
    let mut builder = InternalDocumentBuilder::new("test");
    builder.push_paragraph(text, annotations, None, None);
    builder.build()
}

fn underline(start: u32, end: u32) -> TextAnnotation {
    TextAnnotation {
        start,
        end,
        kind: AnnotationKind::Underline,
    }
}

fn commonmark_html(markdown: &str, tables: bool) -> String {
    // ~keep A standard reader must work without comrak's superscript or underline extensions.
    let mut options = comrak::Options::default();
    options.extension.table = tables;
    options.render.r#unsafe = true;
    comrak::markdown_to_html(markdown, &options)
}

#[test]
fn superscript_uses_html_without_changing_utf8_source_offsets() {
    let annotations = vec![superscript(3, 4)];
    let document = paragraph("éx2", annotations.clone());

    let markdown = render_markdown(&document);

    assert_eq!(markdown, "éx<sup>2</sup>\n");
    assert_eq!(commonmark_html(&markdown, false), "<p>éx<sup>2</sup></p>\n");
    assert_eq!(document.elements[0].text, "éx2");
    assert_eq!(document.elements[0].annotations, annotations);
    assert_eq!(render_markdown(&document), markdown);
}

#[test]
fn underline_uses_html_instead_of_commonmark_strong_emphasis() {
    let document = paragraph("a under z", vec![underline(2, 7)]);

    let markdown = render_markdown(&document);

    assert_eq!(markdown, "a <u>under</u> z\n");
    assert_eq!(commonmark_html(&markdown, false), "<p>a <u>under</u> z</p>\n");
}

#[test]
fn superscript_escapes_source_html_and_ampersands() {
    let source = "<img src=x> & <u>Y</u>";
    let document = paragraph(source, vec![superscript(0, source.len() as u32)]);

    let markdown = render_markdown(&document);

    assert_eq!(
        commonmark_html(&markdown, false),
        "<p><sup>&lt;img src=x&gt; &amp; &lt;u&gt;Y&lt;/u&gt;</sup></p>\n"
    );
    assert_eq!(document.elements[0].text, source);
    assert_eq!(document.elements[0].annotations, [superscript(0, source.len() as u32)]);
}

#[test]
fn underline_escapes_source_closing_tags() {
    let source = "</u><script>x&y</script>";
    let document = paragraph(source, vec![underline(0, source.len() as u32)]);

    let markdown = render_markdown(&document);

    assert_eq!(
        commonmark_html(&markdown, false),
        "<p><u>&lt;/u&gt;&lt;script&gt;x&amp;y&lt;/script&gt;</u></p>\n"
    );
    assert!(!markdown.contains("<script>"));
}

#[test]
fn adjacent_and_nested_annotations_keep_their_formatting_boundaries() {
    let document = paragraph(
        "ab2cde",
        vec![bold(0, 3), superscript(2, 3), underline(3, 6), italic(4, 5)],
    );

    let markdown = render_markdown(&document);

    assert_eq!(
        commonmark_html(&markdown, false),
        "<p><strong>ab<sup>2</sup></strong><u>c<em>d</em>e</u></p>\n"
    );
}

#[test]
fn table_scripts_use_html_without_exposing_literal_tags_or_pipes() {
    use crate::rendering::table_scripts::{CellScriptAnnotation, TABLE_CELL_SCRIPTS_ATTRIBUTE};

    let cells = vec![
        vec!["Value".into(), "Literal".into()],
        vec!["éx2".into(), "<u>x</u> ^2^".into()],
        vec!["x2|3".into(), "<sup>literal</sup>".into()],
    ];
    let records = [
        CellScriptAnnotation {
            row: 1,
            col: 0,
            annotation: superscript(3, 4),
        },
        CellScriptAnnotation {
            row: 2,
            col: 0,
            annotation: superscript(1, 4),
        },
    ];
    let mut builder = InternalDocumentBuilder::new("test");
    let table = builder.push_table_from_cells(&cells, None, None);
    builder.merge_attribute(
        table,
        TABLE_CELL_SCRIPTS_ATTRIBUTE,
        serde_json::to_string(&records).unwrap(),
    );
    let mut document = builder.build();
    document.tables[0].columns = Some(cells[0].clone());

    let annotations = crate::rendering::table_scripts::read_annotations(
        document.elements[table as usize].attributes.as_ref(),
        &cells,
    );
    assert_eq!(annotations.len(), 2);
    assert_eq!(annotations[&(1, 0)], [superscript(3, 4)]);
    assert_eq!(annotations[&(2, 0)], [superscript(1, 4)]);

    let markdown = render_markdown(&document);
    let html = commonmark_html(&markdown, true);

    assert!(html.contains("<td>éx<sup>2</sup></td>"), "{html}");
    assert!(html.contains("<td>x<sup>2|3</sup></td>"), "{html}");
    assert!(html.contains("<td>&lt;u&gt;x&lt;/u&gt; ^2^</td>"), "{html}");
    assert!(html.contains("<td>&lt;sup&gt;literal&lt;/sup&gt;</td>"), "{html}");
    assert_eq!(html.matches("<sup>").count(), 2);
    assert_eq!(document.tables[0].cells, cells);
}

#[test]
fn inline_and_fenced_code_keep_literal_html_and_caret_text() {
    use comrak::nodes::NodeValue;

    let literal = "<sup>x</sup> <u>y</u> ^2^";
    let x = literal.find('x').unwrap() as u32;
    let y = literal.find('y').unwrap() as u32;
    let mut builder = InternalDocumentBuilder::new("test");
    builder.push_paragraph(
        literal,
        vec![
            code(0, literal.len() as u32),
            superscript(x, x + 1),
            underline(y, y + 1),
        ],
        None,
        None,
    );
    builder.push_code(literal, Some("text"), None, None);
    let document = builder.build();

    let markdown = render_markdown(&document);
    let arena = comrak::Arena::new();
    let root = comrak::parse_document(&arena, &markdown, &comrak::Options::default());
    let literals: Vec<_> = root
        .descendants()
        .filter_map(|node| match &node.data.borrow().value {
            NodeValue::Code(code) => Some(code.literal.clone()),
            NodeValue::CodeBlock(code) => Some(code.literal.clone()),
            _ => None,
        })
        .collect();

    assert_eq!(literals, [literal.to_string(), format!("{literal}\n")]);
    assert_eq!(
        root.descendants()
            .filter(|node| matches!(
                node.data.borrow().value,
                NodeValue::HtmlInline(_) | NodeValue::HtmlBlock(_)
            ))
            .count(),
        0
    );
}

#[test]
fn unannotated_carets_and_html_like_text_remain_literal() {
    for escape_markdown in [false, true] {
        let mut document = paragraph("x^2^ <u>literal</u>", vec![]);
        document.escape_markdown = escape_markdown;

        let markdown = render_markdown(&document);

        assert_eq!(
            commonmark_html(&markdown, false),
            "<p>x^2^ &lt;u&gt;literal&lt;/u&gt;</p>\n",
            "escape_markdown={escape_markdown}: {markdown}"
        );
    }
}

#[test]
fn subscript_and_explicit_math_keep_their_existing_markdown_policy() {
    let document = paragraph(
        "H2O $x^2$",
        vec![
            subscript(1, 2),
            TextAnnotation {
                start: 4,
                end: 9,
                kind: AnnotationKind::math(false),
            },
        ],
    );

    assert!(crate::rendering::common::valid_math_annotation(
        &document.elements[0].text,
        &document.elements[0].annotations[1],
    ));
    assert_eq!(render_markdown(&document), "H~2~O $x^2$\n");
}

#[test]
fn other_renderers_keep_superscript_and_underline_semantics() {
    let document = paragraph("x2 U", vec![superscript(1, 2), underline(3, 4)]);

    assert_eq!(
        crate::rendering::render_html(&document),
        "<p>x<sup>2</sup> <u>U</u></p>\n"
    );
    assert_eq!(crate::rendering::render_djot(&document), "x^2^ [U]{.underline}\n");
}
