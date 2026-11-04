use super::*;

fn paragraph(content: &str, table: Option<DocTableMark>) -> DocParagraph {
    DocParagraph {
        content: content.to_string(),
        annotations: Vec::new(),
        list: None,
        heading_level: None,
        table,
    }
}

const CELL: Option<DocTableMark> = Some(DocTableMark {
    cell_end: true,
    row_end: false,
});
const ROW: Option<DocTableMark> = Some(DocTableMark {
    cell_end: false,
    row_end: true,
});
const IN_CELL: Option<DocTableMark> = Some(DocTableMark {
    cell_end: false,
    row_end: false,
});

fn element_kinds(doc: &InternalDocument) -> Vec<&'static str> {
    doc.elements
        .iter()
        .map(|element| match element.kind {
            ElementKind::Table { .. } => "table",
            ElementKind::Paragraph => "paragraph",
            ElementKind::Heading { .. } => "heading",
            ElementKind::ListStart { .. } => "list-start",
            ElementKind::ListItem { .. } => "list-item",
            ElementKind::ListEnd => "list-end",
            _ => "other",
        })
        .collect()
}

#[test]
fn should_emit_consecutive_table_paragraphs_as_one_table() {
    let paragraphs = [
        paragraph("A long introduction paragraph.", None),
        paragraph("Region", CELL),
        paragraph("Q1", CELL),
        paragraph("", ROW),
        paragraph("North", CELL),
        paragraph("", CELL),
        paragraph("", ROW),
        paragraph("After the table, more prose follows.", None),
    ];
    let mut doc = InternalDocument::new("doc");

    push_paragraph_elements(&mut doc, &paragraphs, &mut SecurityBudget::with_defaults())
        .expect("paragraphs fit the default budget");

    assert_eq!(element_kinds(&doc), ["paragraph", "table", "paragraph"]);
    assert_eq!(
        doc.tables[0].cells,
        [
            vec!["Region".to_string(), "Q1".to_string()],
            vec!["North".to_string(), String::new()]
        ]
    );
}

#[test]
fn should_join_the_paragraphs_of_a_cell_with_line_breaks() {
    let paragraphs = [
        paragraph("first", IN_CELL),
        paragraph("second", CELL),
        paragraph("", ROW),
    ];
    let mut doc = InternalDocument::new("doc");

    push_paragraph_elements(&mut doc, &paragraphs, &mut SecurityBudget::with_defaults())
        .expect("paragraphs fit the default budget");

    assert_eq!(doc.tables[0].cells, [vec!["first\nsecond".to_string()]]);
}

#[test]
fn should_leave_out_a_table_with_no_text() {
    let paragraphs = [paragraph("", CELL), paragraph("", CELL), paragraph("", ROW)];
    let mut doc = InternalDocument::new("doc");

    push_paragraph_elements(&mut doc, &paragraphs, &mut SecurityBudget::with_defaults())
        .expect("paragraphs fit the default budget");

    assert!(doc.elements.is_empty());
    assert!(doc.tables.is_empty());
}

#[test]
fn should_close_an_open_list_before_a_table() {
    let mut item = paragraph("An item", None);
    item.list = Some(crate::extraction::doc::DocListMembership {
        level: 0,
        ordered: false,
    });
    let paragraphs = [item, paragraph("cell", CELL), paragraph("", ROW)];
    let mut doc = InternalDocument::new("doc");

    push_paragraph_elements(&mut doc, &paragraphs, &mut SecurityBudget::with_defaults())
        .expect("paragraphs fit the default budget");

    assert_eq!(element_kinds(&doc), ["list-start", "list-item", "list-end", "table"]);
}

fn synthetic_table_doc(rows: &[&[&str]]) -> Vec<u8> {
    let mut text = String::new();
    let mut spans = Vec::new();
    for row in rows {
        let start = text.len();
        for cell in *row {
            text.push_str(cell);
            text.push('\x07');
        }
        spans.push((start..text.len(), &[0x16, 0x24, 0x01][..]));
        let start = text.len();
        text.push('\x07');
        spans.push((start..text.len(), &[0x16, 0x24, 0x01, 0x17, 0x24, 0x01][..]));
    }
    crate::extraction::doc::tests::build_utf16_doc_with_papx(&text, &spans)
}

async fn extract_with_cell_limit(bytes: &[u8], max_table_cells: usize) -> Result<InternalDocument> {
    let config = ExtractionConfig {
        security_limits: Some(crate::extractors::security::SecurityLimits {
            max_table_cells,
            ..Default::default()
        }),
        ..Default::default()
    };
    DocExtractor::new()
        .extract_content(bytes, LEGACY_WORD_MIME_TYPE, &config)
        .await
}

fn assert_cell_limit(error: crate::XbergError, cells: usize, max: usize) {
    let crate::XbergError::Security {
        source: Some(source), ..
    } = error
    else {
        panic!("expected a security error, got {error}");
    };
    assert!(
        matches!(source.downcast_ref::<crate::extractors::security::SecurityError>(),
        Some(crate::extractors::security::SecurityError::TooManyCells { cells: observed, max: configured })
            if *observed == cells && *configured == max),
        "{source}"
    );
}

#[tokio::test]
async fn should_enforce_configured_cell_limit_on_synthetic_doc() {
    let bytes = synthetic_table_doc(&[&["a", "b"], &["c", "d"]]);
    let error = extract_with_cell_limit(&bytes, 3)
        .await
        .expect_err("four cells exceed three");
    assert_cell_limit(error, 4, 3);
    let doc = extract_with_cell_limit(&bytes, 4).await.expect("exact four-cell limit");
    assert_eq!(doc.tables[0].cells, vec![vec!["a", "b"], vec!["c", "d"]]);
}

#[tokio::test]
async fn should_bound_padded_render_grid_on_synthetic_doc() {
    let bytes = synthetic_table_doc(&[&["a", "b", "c"], &["d"], &["e"]]);
    let error = extract_with_cell_limit(&bytes, 5)
        .await
        .expect_err("nine padded cells exceed five");
    assert_cell_limit(error, 6, 5);
    let doc = extract_with_cell_limit(&bytes, 9)
        .await
        .expect("exact nine-cell render grid");
    assert_eq!(doc.tables[0].cells, vec![vec!["a", "b", "c"], vec!["d"], vec!["e"]]);
}

#[tokio::test]
async fn should_charge_new_columns_for_rows_already_read() {
    let bytes = synthetic_table_doc(&[&["a"], &["b", "c", "d"]]);
    let error = extract_with_cell_limit(&bytes, 5)
        .await
        .expect_err("six padded cells exceed five");
    assert_cell_limit(error, 6, 5);
    let doc = extract_with_cell_limit(&bytes, 6)
        .await
        .expect("exact six-cell render grid");
    assert_eq!(doc.tables[0].cells, vec![vec!["a"], vec!["b", "c", "d"]]);
}

#[tokio::test]
async fn should_reject_a_table_with_a_zero_cell_budget() {
    let bytes = synthetic_table_doc(&[&["a"]]);
    let error = extract_with_cell_limit(&bytes, 0)
        .await
        .expect_err("one cell exceeds zero");
    assert_cell_limit(error, 1, 0);
}

#[tokio::test]
async fn should_bound_empty_cells_before_omitting_empty_tables() {
    let bytes = synthetic_table_doc(&[&["", ""], &["", ""]]);
    let error = extract_with_cell_limit(&bytes, 3)
        .await
        .expect_err("empty cells still consume the budget");
    assert_cell_limit(error, 4, 3);
    assert!(
        extract_with_cell_limit(&bytes, 4)
            .await
            .expect("four blank cells fit")
            .tables
            .is_empty()
    );
}

#[tokio::test]
async fn should_share_the_configured_cell_budget_across_separate_tables() {
    let bytes = crate::extraction::doc::tests::build_utf16_doc_with_papx(
        "a\x07\x07gap.\rb\x07\x07",
        &[
            (0..2, &[0x16, 0x24, 1]),
            (2..3, &[0x16, 0x24, 1, 0x17, 0x24, 1]),
            (3..8, &[]),
            (8..10, &[0x16, 0x24, 1]),
            (10..11, &[0x16, 0x24, 1, 0x17, 0x24, 1]),
        ],
    );
    let error = extract_with_cell_limit(&bytes, 1)
        .await
        .expect_err("two tables exceed one cell");
    assert_cell_limit(error, 2, 1);
    let doc = extract_with_cell_limit(&bytes, 2)
        .await
        .expect("two one-cell tables fit");
    assert_eq!(doc.tables.len(), 2);
    assert_eq!(doc.tables[0].cells, vec![vec!["a"]]);
    assert_eq!(doc.tables[1].cells, vec![vec!["b"]]);
}

#[test]
fn should_ignore_table_styles_when_classifying_body_headings() {
    let mut cell = paragraph("Cell heading", CELL);
    cell.heading_level = Some(1);
    let paragraphs = [
        cell,
        paragraph("", ROW),
        paragraph("Overview", None),
        paragraph("This is the much longer body paragraph after the heading.", None),
    ];
    let mut doc = InternalDocument::new("doc");
    push_paragraph_elements(&mut doc, &paragraphs, &mut SecurityBudget::with_defaults())
        .expect("paragraphs fit the default budget");
    assert_eq!(
        doc.elements
            .iter()
            .filter_map(|element| match element.kind {
                ElementKind::Heading { level } => Some((level, element.text.as_str())),
                _ => None,
            })
            .collect::<Vec<_>>(),
        [(2, "Overview")]
    );
}

fn script_table_paragraph(content: &str, table: Option<DocTableMark>, start: u32, end: u32) -> DocParagraph {
    let mut paragraph = paragraph(content, table);
    paragraph
        .annotations
        .push(crate::types::builder::superscript(start, end));
    paragraph
}

#[test]
fn should_preserve_doc_table_scripts_without_changing_plain_cells_gh2275() {
    let paragraphs = [
        paragraph("Value", CELL),
        paragraph("", ROW),
        script_table_paragraph("7,2 x 109/l", CELL, 8, 9),
        paragraph("", ROW),
    ];
    let mut doc = InternalDocument::new("doc");
    doc.escape_markdown = false;
    push_paragraph_elements(&mut doc, &paragraphs, &mut SecurityBudget::with_defaults()).unwrap();
    assert_eq!(doc.tables[0].cells, [vec!["Value"], vec!["7,2 x 109/l"]]);
    let markdown = crate::rendering::render_markdown(&doc);
    assert!(markdown.contains("7,2 x 10<sup>9</sup>/l"), "{markdown}");
    let html = crate::rendering::render_html(&doc);
    assert!(html.contains("7,2 x 10<sup>9</sup>/l"), "{html}");
    let plain = crate::rendering::render_plain(&doc);
    assert!(plain.contains("7,2 x 109/l"), "{plain}");
    assert!(!plain.contains('^'), "{plain}");
    assert!(doc.elements[0].public_attributes().is_none());
    let restored = serde_json::from_str(&serde_json::to_string(&doc).unwrap()).unwrap();
    assert_eq!(crate::rendering::render_markdown(&restored), markdown);
}

#[test]
fn should_project_doc_table_scripts_after_unicode_trim_and_paragraph_join_gh2275() {
    let mut second = paragraph(" H2O ", CELL);
    second.annotations.push(crate::types::builder::subscript(2, 3));
    let paragraphs = [
        script_table_paragraph(" é109 ", IN_CELL, 5, 6),
        second,
        paragraph("", ROW),
    ];
    let mut doc = InternalDocument::new("doc");
    doc.escape_markdown = false;
    push_paragraph_elements(&mut doc, &paragraphs, &mut SecurityBudget::with_defaults()).unwrap();
    assert_eq!(doc.tables[0].cells, [vec!["é109\nH2O"]]);
    let markdown = crate::rendering::render_markdown(&doc);
    assert!(markdown.contains("é10<sup>9</sup>"), "{markdown}");
    assert!(markdown.contains("H~2~O"), "{markdown}");
}

#[test]
fn should_keep_literal_script_markers_and_unannotated_tables_literal_gh2275() {
    let paragraphs = [paragraph("10^9^ and H~2~O", CELL), paragraph("", ROW)];
    let mut doc = InternalDocument::new("doc");
    push_paragraph_elements(&mut doc, &paragraphs, &mut SecurityBudget::with_defaults()).unwrap();
    assert!(doc.elements[0].attributes.is_none());
    let html = crate::rendering::render_html(&doc);
    assert!(!html.contains("<sup>"), "{html}");
    assert!(!html.contains("<sub>"), "{html}");
    assert!(html.contains("10^9^ and H~2~O"), "{html}");
}

#[test]
fn should_reset_doc_table_script_coordinates_between_tables_gh2275() {
    let paragraphs = [
        script_table_paragraph("109", CELL, 2, 3),
        paragraph("", ROW),
        paragraph("Between tables.", None),
        paragraph("plain 109", CELL),
        paragraph("", ROW),
    ];
    let mut doc = InternalDocument::new("doc");
    push_paragraph_elements(&mut doc, &paragraphs, &mut SecurityBudget::with_defaults()).unwrap();
    let html = crate::rendering::render_html(&doc);
    assert_eq!(html.matches("<sup>").count(), 1, "{html}");
    assert_eq!(doc.tables.len(), 2);
    assert!(doc.elements[2].attributes.is_none());
}
