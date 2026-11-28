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

/// `(table, row, col)` of the cell holding the nested table at `tables[index]`.
fn nested_parent(doc: &InternalDocument, index: u32) -> Option<(u64, u64, u64)> {
    let element = doc
        .elements
        .iter()
        .find(|element| matches!(element.kind, ElementKind::Table { table_index } if table_index == index))?;
    let encoded = element
        .attributes
        .as_ref()?
        .get(crate::types::internal::DOC_NESTED_TABLE_PARENT_ATTRIBUTE)?;
    let value: serde_json::Value = serde_json::from_str(encoded).expect("the tag is JSON");
    Some((value["table"].as_u64()?, value["row"].as_u64()?, value["col"].as_u64()?))
}

fn row(cells: &[&str]) -> Vec<String> {
    cells.iter().map(|cell| (*cell).to_string()).collect()
}

/// A nested table of one row, `a` and `b`, as the paragraphs Word writes for it.
fn nested_row(a: &str, b: &str, depth: u8) -> Vec<DocParagraph> {
    vec![
        paragraph(a, inner(true, false, depth)),
        paragraph(b, inner(true, false, depth)),
        paragraph("", inner(false, true, depth)),
    ]
}

#[test]
fn should_keep_a_level_two_table_in_its_cell_and_emit_it_after_the_outer_table_gh3438() {
    let mut paragraphs = vec![paragraph("A long introduction paragraph.", None)];
    paragraphs.extend(nested_row("INNER_A", "INNER_B", 2));
    paragraphs.extend(nested_row("INNER_C", "INNER_D", 2));
    paragraphs.extend([
        paragraph("", CELL),
        paragraph("OUTER_RIGHT", CELL),
        paragraph("", ROW),
        paragraph("AFTER_LEFT", CELL),
        paragraph("AFTER_RIGHT", CELL),
        paragraph("", ROW),
    ]);

    let doc = push_all(&paragraphs);

    assert_eq!(element_kinds(&doc), ["paragraph", "table", "table"]);
    assert_eq!(
        doc.tables[0].cells,
        [
            row(&["INNER_A\nINNER_B\nINNER_C\nINNER_D", "OUTER_RIGHT"]),
            row(&["AFTER_LEFT", "AFTER_RIGHT"])
        ]
    );
    assert_eq!(
        doc.tables[1].cells,
        [row(&["INNER_A", "INNER_B"]), row(&["INNER_C", "INNER_D"])]
    );
    assert_eq!(nested_parent(&doc, 0), None);
    assert_eq!(nested_parent(&doc, 1), Some((0, 0, 0)));
}

#[test]
fn should_place_a_nested_table_in_a_later_row_by_its_cell_gh3438() {
    let mut paragraphs = vec![paragraph("r0c0", CELL), paragraph("r0c1", CELL), paragraph("", ROW)];
    paragraphs.push(paragraph("x", CELL));
    paragraphs.push(paragraph("before", IN_CELL));
    paragraphs.extend(nested_row("N1", "N2", 2));
    paragraphs.extend([paragraph("after", CELL), paragraph("", ROW)]);

    let doc = push_all(&paragraphs);

    assert_eq!(
        doc.tables[0].cells,
        [row(&["r0c0", "r0c1"]), row(&["x", "before\nN1\nN2\nafter"])]
    );
    assert_eq!(doc.tables[1].cells, [row(&["N1", "N2"])]);
    assert_eq!(nested_parent(&doc, 1), Some((0, 1, 1)));
}

#[test]
fn should_emit_nested_tables_of_different_rows_in_order_with_their_own_parents_gh3438() {
    let mut paragraphs = vec![paragraph("head", CELL), paragraph("", ROW)];
    paragraphs.extend(nested_row("first-a", "first-b", 2));
    paragraphs.extend([paragraph("", CELL), paragraph("", ROW)]);
    paragraphs.extend([paragraph("mid", CELL), paragraph("", ROW)]);
    paragraphs.extend(nested_row("second-a", "second-b", 2));
    paragraphs.extend([paragraph("", CELL), paragraph("", ROW)]);

    let doc = push_all(&paragraphs);

    assert_eq!(element_kinds(&doc), ["table", "table", "table"]);
    assert_eq!(doc.tables[1].cells, [row(&["first-a", "first-b"])]);
    assert_eq!(doc.tables[2].cells, [row(&["second-a", "second-b"])]);
    assert_eq!(nested_parent(&doc, 1), Some((0, 1, 0)));
    assert_eq!(nested_parent(&doc, 2), Some((0, 3, 0)));
}

#[test]
fn should_link_a_level_three_table_to_the_level_two_table_that_holds_it_gh3438() {
    let mut paragraphs = vec![paragraph("L2", inner(false, false, 2))];
    paragraphs.extend(nested_row("L3a", "L3b", 3));
    paragraphs.extend([
        paragraph("", inner(true, false, 2)),
        paragraph("", inner(false, true, 2)),
        paragraph("L1", CELL),
        paragraph("", ROW),
    ]);

    let doc = push_all(&paragraphs);

    assert_eq!(doc.tables.len(), 3);
    assert_eq!(doc.tables[0].cells, [row(&["L2\nL3a\nL3b\nL1"])]);
    assert_eq!(doc.tables[1].cells, [row(&["L2"])]);
    assert_eq!(doc.tables[2].cells, [row(&["L3a", "L3b"])]);
    assert_eq!(nested_parent(&doc, 1), Some((0, 0, 0)));
    assert_eq!(nested_parent(&doc, 2), Some((1, 0, 0)));
}

#[test]
fn should_keep_only_the_flattened_text_for_a_depth_that_skips_a_level_gh3438() {
    let paragraphs = [
        paragraph("start", IN_CELL),
        paragraph("deep", inner(true, false, 4)),
        paragraph("", inner(false, true, 4)),
        paragraph("end", CELL),
        paragraph("", ROW),
    ];

    let doc = push_all(&paragraphs);

    assert_eq!(doc.tables.len(), 1);
    assert_eq!(doc.tables[0].cells, [row(&["start\ndeep\nend"])]);
}

#[test]
fn should_keep_only_the_flattened_text_of_a_nested_table_with_no_row_end_gh3438() {
    let paragraphs = [
        paragraph("outer", IN_CELL),
        paragraph("x", inner(true, false, 2)),
        paragraph("y", inner(true, false, 2)),
        paragraph("", CELL),
        paragraph("", ROW),
    ];

    let doc = push_all(&paragraphs);

    assert_eq!(doc.tables.len(), 1);
    assert_eq!(doc.tables[0].cells, [row(&["outer\nx\ny"])]);
}

#[test]
fn should_close_a_nested_table_left_open_at_the_end_of_the_document_gh3438() {
    let doc = push_all(&nested_row("a", "b", 2));

    assert_eq!(doc.tables[0].cells, [row(&["a\nb"])]);
    assert_eq!(doc.tables[1].cells, [row(&["a", "b"])]);
}

#[test]
fn should_read_depth_zero_as_one_and_flatten_a_depth_past_the_maximum_gh3438() {
    let paragraphs = [
        paragraph("a", inner(true, false, 0)),
        paragraph("", inner(false, true, 0)),
        paragraph("deep", inner(true, false, 200)),
        paragraph("", inner(false, true, 200)),
        paragraph("b", CELL),
        paragraph("", ROW),
    ];

    let doc = push_all(&paragraphs);

    assert_eq!(doc.tables.len(), 1);
    assert_eq!(doc.tables[0].cells, [row(&["a"]), row(&["deep\nb"])]);
}

#[test]
fn should_count_nested_cells_against_the_table_cell_budget_gh3438() {
    let limits = crate::extractors::security::SecurityLimits {
        max_table_cells: 2,
        ..Default::default()
    };
    let mut paragraphs = vec![paragraph("outer", IN_CELL)];
    paragraphs.extend(nested_row("a", "b", 2));
    paragraphs.extend([paragraph("", CELL), paragraph("", ROW)]);
    let mut doc = InternalDocument::new("doc");

    let error = push_paragraph_elements(&mut doc, &paragraphs, &mut SecurityBudget::from_limits(&limits))
        .expect_err("one outer and two nested cells exceed two");

    assert!(matches!(error, crate::XbergError::Security { .. }), "{error}");
}

/// Word's own encoding: level-one paragraphs carry `sprmPItap` 1, level-two
/// ones carry 2 with the inner cell and row sprms, and end with paragraph marks.
#[tokio::test]
async fn should_read_a_nested_table_from_papx_with_itap_on_every_level_gh3438() {
    let itap = |depth: u8| [0x49, 0x66, depth, 0, 0, 0];
    let cell2 = [&itap(2)[..], &[0x4B, 0x24, 1][..]].concat();
    let row2 = [&itap(2)[..], &[0x4C, 0x24, 1][..]].concat();
    let cell1 = [&[0x16, 0x24, 1][..], &itap(1)[..]].concat();
    let row1 = [&[0x16, 0x24, 1, 0x17, 0x24, 1][..], &itap(1)[..]].concat();
    let intro = "A long introduction paragraph.\r";
    let mut text = String::from(intro);
    let mut spans: Vec<(std::ops::Range<usize>, &[u8])> = vec![(0..intro.len(), &[][..])];
    for (piece, grpprl) in [
        ("A\r", &cell2),
        ("B\r", &cell2),
        ("\r", &row2),
        ("C\r", &cell2),
        ("D\r", &cell2),
        ("\r", &row2),
        ("\x07", &cell1),
        ("RIGHT\x07", &cell1),
        ("\x07", &row1),
    ] {
        let range = text.len()..text.len() + piece.len();
        text.push_str(piece);
        spans.push((range, grpprl));
    }
    let bytes = crate::extraction::doc::tests::build_utf16_doc_with_papx(&text, &spans);

    let doc = DocExtractor::new()
        .extract_content(&bytes, LEGACY_WORD_MIME_TYPE, &ExtractionConfig::default())
        .await
        .expect("extraction succeeds");

    assert_eq!(doc.tables.len(), 2);
    assert_eq!(element_kinds(&doc), ["paragraph", "table", "table"]);
    assert_eq!(doc.tables[0].cells, [row(&["A\nB\nC\nD", "RIGHT"])]);
    assert_eq!(doc.tables[1].cells, [row(&["A", "B"]), row(&["C", "D"])]);
    assert_eq!(nested_parent(&doc, 1), Some((0, 0, 0)));
}
