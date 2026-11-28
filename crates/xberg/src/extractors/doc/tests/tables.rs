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
    depth: 1,
});
const ROW: Option<DocTableMark> = Some(DocTableMark {
    cell_end: false,
    row_end: true,
    depth: 1,
});
const IN_CELL: Option<DocTableMark> = Some(DocTableMark {
    cell_end: false,
    row_end: false,
    depth: 1,
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
        label: None,
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

const fn inner(cell_end: bool, row_end: bool, depth: u8) -> Option<DocTableMark> {
    Some(DocTableMark {
        cell_end,
        row_end,
        depth,
    })
}

fn push_all(paragraphs: &[DocParagraph]) -> InternalDocument {
    let mut doc = InternalDocument::new("doc");
    push_paragraph_elements(&mut doc, paragraphs, &mut SecurityBudget::with_defaults())
        .expect("paragraphs fit the default budget");
    doc
}

fn all_text(doc: &InternalDocument) -> String {
    doc.tables
        .iter()
        .flat_map(|table| table.cells.iter().flatten())
        .map(String::as_str)
        .collect::<Vec<_>>()
        .join("|")
}

#[test]
fn should_emit_a_table_that_starts_at_level_two_with_its_row_and_column_pairs_gh3438() {
    let paragraphs = [
        paragraph("A long introduction paragraph.", None),
        paragraph("INNER_A", inner(true, false, 2)),
        paragraph("INNER_B", inner(true, false, 2)),
        paragraph("", inner(false, true, 2)),
        paragraph("INNER_C", inner(true, false, 2)),
        paragraph("INNER_D", inner(true, false, 2)),
        paragraph("", inner(false, true, 2)),
        paragraph("", CELL),
        paragraph("OUTER_RIGHT", CELL),
        paragraph("", ROW),
        paragraph("AFTER_LEFT", CELL),
        paragraph("AFTER_RIGHT", CELL),
        paragraph("", ROW),
    ];

    let doc = push_all(&paragraphs);

    assert_eq!(element_kinds(&doc), ["paragraph", "table", "table"]);
    assert_eq!(
        doc.tables[0].cells,
        [
            vec!["INNER_A".to_string(), "INNER_B".to_string()],
            vec!["INNER_C".to_string(), "INNER_D".to_string()]
        ]
    );
    assert_eq!(
        doc.tables[1].cells,
        [
            vec![String::new(), "OUTER_RIGHT".to_string()],
            vec!["AFTER_LEFT".to_string(), "AFTER_RIGHT".to_string()]
        ]
    );
}

#[test]
fn should_emit_a_table_nested_in_a_level_one_cell_beside_its_parent_gh3438() {
    let paragraphs = [
        paragraph("outer text", IN_CELL),
        paragraph("N1", inner(true, false, 2)),
        paragraph("N2", inner(true, false, 2)),
        paragraph("", inner(false, true, 2)),
        paragraph("", CELL),
        paragraph("right", CELL),
        paragraph("", ROW),
    ];

    let doc = push_all(&paragraphs);

    assert_eq!(doc.tables[0].cells, [vec!["N1".to_string(), "N2".to_string()]]);
    assert_eq!(
        doc.tables[1].cells,
        [vec!["outer text".to_string(), "right".to_string()]]
    );
}

#[test]
fn should_nest_three_levels_deep_without_losing_text_gh3438() {
    let paragraphs = [
        paragraph("L2", inner(false, false, 2)),
        paragraph("L3", inner(true, false, 3)),
        paragraph("", inner(false, true, 3)),
        paragraph("", inner(true, false, 2)),
        paragraph("", inner(false, true, 2)),
        paragraph("L1", CELL),
        paragraph("", ROW),
    ];

    let doc = push_all(&paragraphs);

    assert_eq!(doc.tables.len(), 3);
    assert_eq!(all_text(&doc), "L3|L2|L1");
}

#[test]
fn should_flatten_a_depth_that_skips_a_level_instead_of_dropping_its_text_gh3438() {
    let paragraphs = [
        paragraph("start", IN_CELL),
        paragraph("deep", inner(true, false, 4)),
        paragraph("", inner(false, true, 4)),
        paragraph("end", CELL),
        paragraph("", ROW),
    ];

    let doc = push_all(&paragraphs);

    assert_eq!(doc.tables.len(), 1);
    assert_eq!(doc.tables[0].cells, [vec!["start\ndeep\nend".to_string()]]);
}

#[test]
fn should_flatten_a_nested_table_with_no_row_end_into_the_enclosing_cell_gh3438() {
    let paragraphs = [
        paragraph("outer", IN_CELL),
        paragraph("x", inner(true, false, 2)),
        paragraph("y", inner(true, false, 2)),
        paragraph("", CELL),
        paragraph("", ROW),
    ];

    let doc = push_all(&paragraphs);

    assert_eq!(doc.tables.len(), 1);
    assert_eq!(doc.tables[0].cells, [vec!["outer\nx\ny".to_string()]]);
}

#[test]
fn should_keep_a_nested_table_left_open_at_the_end_of_the_document_gh3438() {
    let paragraphs = [
        paragraph("a", inner(true, false, 2)),
        paragraph("", inner(false, true, 2)),
    ];

    let doc = push_all(&paragraphs);

    assert_eq!(doc.tables[0].cells, [vec!["a".to_string()]]);
}

/// Level-two paragraphs end with paragraph marks; `sprmPItap` 2 and the inner
/// cell and row sprms are what make them cells and rows.
#[tokio::test]
async fn should_read_a_level_two_table_from_papx_depth_and_inner_marks_gh3438() {
    const ITAP2: [u8; 6] = [0x49, 0x66, 2, 0, 0, 0];
    let cell = [&ITAP2[..], &[0x4B, 0x24, 1][..]].concat();
    let row = [&ITAP2[..], &[0x4C, 0x24, 1][..]].concat();
    let outer_cell = [0x16u8, 0x24, 1];
    let outer_row = [0x16u8, 0x24, 1, 0x17, 0x24, 1];
    let mut text = String::from("intro\r");
    let mut spans: Vec<(std::ops::Range<usize>, &[u8])> = vec![(0..6, &[][..])];
    for (piece, which) in [
        ("A\r", 0),
        ("B\r", 0),
        ("\r", 1),
        ("C\r", 0),
        ("D\r", 0),
        ("\r", 1),
        ("\x07", 2),
        ("RIGHT\x07", 2),
        ("\x07", 3),
    ] {
        let range = text.len()..text.len() + piece.len();
        text.push_str(piece);
        let grpprl: &[u8] = match which {
            0 => &cell,
            1 => &row,
            2 => &outer_cell,
            _ => &outer_row,
        };
        spans.push((range, grpprl));
    }
    let bytes = crate::extraction::doc::tests::build_utf16_doc_with_papx(&text, &spans);

    let doc = DocExtractor::new()
        .extract_content(&bytes, LEGACY_WORD_MIME_TYPE, &ExtractionConfig::default())
        .await
        .expect("extraction succeeds");

    assert_eq!(
        doc.tables[0].cells,
        [
            vec!["A".to_string(), "B".to_string()],
            vec!["C".to_string(), "D".to_string()]
        ]
    );
    assert_eq!(doc.tables[1].cells, [vec![String::new(), "RIGHT".to_string()]]);
}
