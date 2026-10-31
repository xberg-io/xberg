use super::*;
use xberg_native_pdf::geometry::Rect;
use xberg_native_pdf::structure::table_extractor::{Table as ExtractedTable, TableCell, TableRow};

fn cell(text: &str, colspan: u32, header: bool) -> TableCell {
    TableCell {
        text: text.into(),
        colspan,
        is_header: header,
        ..TableCell::new(String::new(), false)
    }
}

fn source() -> ExtractedTable {
    ExtractedTable {
        rows: vec![
            TableRow {
                cells: vec![
                    cell("Code", 1, false),
                    cell("Count", 1, false),
                    cell("Status", 1, false),
                ],
                is_header: false,
            },
            TableRow {
                cells: vec![cell("ItemA", 1, false), cell("17", 1, false), cell("Ready", 1, false)],
                is_header: false,
            },
        ],
        col_count: 3,
        has_header: false,
        bbox: Some(Rect::new(2.0, 4.0, 300.0, 60.0)),
    }
}

fn adapt(source: &ExtractedTable) -> NativeTable {
    let before = format!("{source:?}");
    let (cells, markdown, grid) = convert_extracted_table(source);
    let expected_cells = cells.clone();
    let expected_markdown = markdown.clone();
    let expected_grid = serde_json::to_value(&grid).unwrap();
    let adapted = finish_extracted_table(source, 7, &scripts::PageSpans::default(), cells, markdown, grid);
    assert_eq!(format!("{source:?}"), before);
    assert_eq!(adapted.table.cells, expected_cells);
    assert_eq!(adapted.table.markdown, expected_markdown);
    assert_eq!(serde_json::to_value(&adapted.grid).unwrap(), expected_grid);
    assert_eq!(adapted.table.page_number, 7);
    assert_eq!(
        adapted.table.bounding_box,
        Some(BoundingBox {
            x0: 2.0,
            y0: 4.0,
            x1: 302.0,
            y1: 64.0
        })
    );
    assert!(adapted.scripts.is_none());
    assert!(adapted.table.table_id.is_none());
    assert!(adapted.table.cell_styles.is_empty());
    adapted
}

#[test]
fn adapter_located_leading_row_produces_declared_columns_without_summary_flag() {
    let mut source = source();
    source.rows[0].is_header = true;
    assert!(!source.has_header);
    let adapted = adapt(&source);
    assert_eq!(adapted.table.cells[0], vec!["Code", "Count", "Status"]);
    assert_eq!(adapted.table.columns.as_ref(), adapted.table.cells.first());
    assert!(adapted.table.has_header_row());
}

#[test]
fn adapter_declared_columns_use_the_final_flattened_spanned_header() {
    let mut source = source();
    source.has_header = true;
    source.rows[0] = TableRow {
        cells: vec![cell("Group | A", 2, true), cell("Status", 1, true)],
        is_header: true,
    };
    let adapted = adapt(&source);
    let expected = vec!["Group | A".to_owned(), String::new(), "Status".to_owned()];
    assert_eq!(adapted.table.cells[0], expected);
    assert_eq!(adapted.table.columns, Some(expected));
    assert!(adapted.table.has_header_row());
}

#[test]
fn adapter_summary_has_header_alone_does_not_promote_the_first_row() {
    let mut source = source();
    source.has_header = true;
    assert!(adapt(&source).table.columns.is_none());
}

#[test]
fn adapter_nonleading_header_row_does_not_relabel_leading_body_cells() {
    let mut source = source();
    source.has_header = true;
    source.rows[1].is_header = true;
    for cell in &mut source.rows[1].cells {
        cell.is_header = true;
    }
    assert!(adapt(&source).table.columns.is_none());
}

#[test]
fn adapter_one_leading_row_header_cell_does_not_declare_column_labels() {
    let mut source = source();
    source.has_header = true;
    source.rows[0].cells[0].is_header = true;
    assert!(adapt(&source).table.columns.is_none());
}

#[test]
fn adapter_headerless_table_preserves_absent_columns() {
    assert!(adapt(&source()).table.columns.is_none());
}
