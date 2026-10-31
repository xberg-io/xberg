use super::convert_extracted_table;
use crate::core::config::OutputFormat;
use crate::extraction::derive::derive_extraction_result;
use crate::types::document_structure::NodeContent;
use crate::types::internal::table_grid::{TableGridSidecar, attach_table_grids};
use crate::types::internal::{ElementKind, InternalDocument, InternalElement};
use crate::types::{BoundingBox, Table, TableGrid};
use xberg_native_pdf::structure::table_extractor::{Table as NativeTable, TableCell, TableRow};

fn cell(text: &str, is_header: bool, rowspan: u32, colspan: u32) -> TableCell {
    TableCell {
        text: text.to_owned(),
        spans: vec![],
        colspan,
        rowspan,
        mcids: vec![],
        bbox: None,
        is_header,
    }
}

fn row(cells: Vec<TableCell>, is_header: bool) -> TableRow {
    TableRow { cells, is_header }
}

fn native(rows: Vec<TableRow>, has_header: bool, col_count: usize) -> NativeTable {
    NativeTable {
        rows,
        has_header,
        col_count,
        bbox: None,
    }
}

fn converted(source: &NativeTable, page_number: u32) -> (Table, Option<TableGrid>) {
    let (cells, markdown, grid) = convert_extracted_table(source);
    (
        Table {
            cells,
            markdown,
            page_number,
            bounding_box: Some(BoundingBox {
                x0: 10.0,
                y0: 20.0,
                x1: 110.0,
                y1: 120.0,
            }),
            ..Default::default()
        },
        grid,
    )
}

fn render(tables: Vec<Table>, sidecars: &[TableGridSidecar]) -> (String, Vec<TableGrid>) {
    let mut document = InternalDocument::new("pdf");
    for (index, table) in tables.into_iter().enumerate() {
        let page = table.page_number;
        document.tables.push(table);
        document.push_element(
            InternalElement::text(
                ElementKind::Table {
                    table_index: index as u32,
                },
                "",
                0,
            )
            .with_page(page),
        );
    }
    attach_table_grids(&mut document, sidecars).expect("attach exact converted table identities");
    let result = derive_extraction_result(document, true, OutputFormat::Html);
    let grids = result
        .document
        .expect("derived document")
        .nodes
        .into_iter()
        .filter_map(|node| match node.content {
            NodeContent::Table { grid } => Some(grid),
            _ => None,
        })
        .collect();
    (result.formatted_content.expect("rendered HTML"), grids)
}

fn assert_roles(source: &NativeTable, expected: &[bool]) -> (String, TableGrid) {
    let (table, grid) = converted(source, 1);
    let grid = grid.expect("positive native header roles require retained metadata without geometric spans");
    assert_eq!(
        grid.cells.iter().map(|cell| cell.is_header).collect::<Vec<_>>(),
        expected
    );
    let (html, mut grids) = render(
        vec![table.clone()],
        &[TableGridSidecar {
            table,
            grid: grid.clone(),
        }],
    );
    assert_eq!(grids.len(), 1);
    assert_eq!(
        grids[0], grid,
        "real derive and sidecar matching retain every native cell role"
    );
    for cell in &grid.cells {
        assert_eq!(html.matches(&cell.content).count(), 1, "cell occurs once: {html}");
    }
    (html, grids.remove(0))
}

#[test]
fn positive_multiple_header_rows_survive_without_geometric_spans() {
    let source = native(
        vec![
            row(vec![cell("body-a", false, 1, 1), cell("body-b", false, 1, 1)], false),
            row(vec![cell("head-a", true, 1, 1), cell("head-b", true, 1, 1)], false),
            row(vec![cell("sub-a", true, 1, 1), cell("sub-b", true, 1, 1)], false),
            row(vec![cell("value-a", false, 1, 1), cell("value-b", false, 1, 1)], false),
        ],
        false,
        2,
    );
    let (html, grid) = assert_roles(&source, &[false, false, true, true, true, true, false, false]);
    assert_eq!((grid.rows, grid.cols), (4, 2));
    assert!(html.contains("<td>body-a</td>"), "{html}");
    assert!(html.contains("<th>head-a</th>"), "{html}");
    assert!(html.contains("<th>sub-b</th>"), "{html}");
    assert_eq!(html.matches("<th>").count(), 4);
    assert_eq!(html.matches("<td>").count(), 4);
}

#[test]
fn direct_th_cells_keep_mixed_roles_without_thead_or_row_flags() {
    let source = native(
        vec![
            row(vec![cell("left-a", false, 1, 1), cell("right-a", false, 1, 1)], false),
            row(
                vec![cell("row-head", true, 1, 1), cell("row-value", false, 1, 1)],
                false,
            ),
        ],
        false,
        2,
    );
    let (html, _) = assert_roles(&source, &[false, false, true, false]);
    assert!(html.contains("<th>row-head</th>"), "{html}");
    assert!(html.contains("<td>row-value</td>"), "{html}");
    assert_eq!(html.matches("<th>").count(), 1);
}

fn spanned_source() -> NativeTable {
    native(
        vec![
            row(
                vec![
                    cell("span-a", false, 2, 1),
                    cell("span-b", false, 1, 2),
                    cell("span-c", false, 1, 1),
                ],
                false,
            ),
            row(
                vec![
                    cell("span-d", true, 1, 1),
                    cell("span-e", false, 1, 1),
                    cell("span-f", true, 1, 1),
                ],
                false,
            ),
            row(
                vec![
                    cell("span-g", false, 1, 1),
                    cell("span-h", false, 1, 1),
                    cell("span-i", false, 1, 1),
                    cell("span-j", false, 1, 1),
                ],
                false,
            ),
        ],
        false,
        4,
    )
}

#[test]
fn positive_cell_roles_follow_source_ordinals_across_rowspan_and_colspan() {
    let source = spanned_source();
    let (html, grid) = assert_roles(
        &source,
        &[false, false, false, true, false, true, false, false, false, false],
    );
    assert_eq!((grid.rows, grid.cols), (3, 4));
    assert_eq!((grid.cells[0].row_span, grid.cells[1].col_span), (2, 2));
    assert_eq!((grid.cells[3].row, grid.cells[3].col), (1, 1));
    assert_eq!((grid.cells[4].row, grid.cells[4].col), (1, 2));
    assert_eq!((grid.cells[5].row, grid.cells[5].col), (1, 3));
    let (table, _) = converted(&source, 1);
    assert_eq!(table.cells[0], ["span-a", "span-b", "", "span-c"]);
    assert_eq!(table.cells[1], ["", "span-d", "span-e", "span-f"]);
    assert!(html.contains("<td rowspan=\"2\">span-a</td>"), "{html}");
    assert!(html.contains("<td colspan=\"2\">span-b</td>"), "{html}");
    assert!(html.contains("<th>span-d</th>"), "{html}");
    assert!(html.contains("<td>span-e</td>"), "{html}");
}

#[test]
fn thead_row_forces_headers_even_when_children_are_td() {
    let source = native(
        vec![
            row(
                vec![cell("section-a", false, 1, 1), cell("section-b", false, 1, 1)],
                true,
            ),
            row(
                vec![cell("number-a", false, 1, 1), cell("number-b", false, 1, 1)],
                false,
            ),
        ],
        true,
        2,
    );
    let (html, _) = assert_roles(&source, &[true, true, false, false]);
    assert_eq!(html.matches("<th>").count(), 2);
    assert!(html.contains("<td>number-a</td>"), "{html}");
}

#[test]
fn legacy_unknown_roles_without_spans_keep_no_sidecar() {
    let source = native(
        vec![
            row(
                vec![cell("unknown-a", false, 1, 1), cell("unknown-b", false, 1, 1)],
                false,
            ),
            row(
                vec![cell("unknown-c", false, 1, 1), cell("unknown-d", false, 1, 1)],
                false,
            ),
        ],
        false,
        2,
    );
    let (table, grid) = converted(&source, 1);
    assert!(grid.is_none());
    assert_eq!(
        table.cells,
        vec![vec!["unknown-a", "unknown-b"], vec!["unknown-c", "unknown-d"]]
    );
    assert_eq!(
        table.markdown,
        "| unknown-a | unknown-b |\n| --- | --- |\n| unknown-c | unknown-d |\n"
    );
}

#[test]
fn legacy_unknown_spanned_roles_keep_first_row_header_fallback() {
    let source = native(
        vec![
            row(vec![cell("legacy-group", false, 1, 2)], false),
            row(
                vec![cell("legacy-left", false, 1, 1), cell("legacy-right", false, 1, 1)],
                false,
            ),
        ],
        false,
        2,
    );
    let (html, grid) = assert_roles(&source, &[true, false, false]);
    assert_eq!(grid.cells[0].col_span, 2);
    assert!(html.contains("<th colspan=\"2\">legacy-group</th>"), "{html}");
    assert!(html.contains("<td>legacy-left</td>"), "{html}");
}

#[test]
fn opposite_positive_header_roles_match_their_source_page_after_sidecar_reordering() {
    let mut source = native(
        vec![
            row(vec![cell("page-a", true, 1, 1), cell("page-b", false, 1, 1)], false),
            row(vec![cell("page-c", false, 1, 1), cell("page-d", false, 1, 1)], false),
        ],
        false,
        2,
    );
    let (first, grid) = converted(&source, 1);
    let first_grid = grid.expect("first page's positive roles");
    source.rows[0].cells[0].is_header = false;
    source.rows[1].cells[1].is_header = true;
    let (second, grid) = converted(&source, 2);
    let second_grid = grid.expect("second page's positive roles");
    let sidecars = [
        TableGridSidecar {
            table: second.clone(),
            grid: second_grid.clone(),
        },
        TableGridSidecar {
            table: first.clone(),
            grid: first_grid.clone(),
        },
    ];
    let (html, grids) = render(vec![first, second], &sidecars);
    assert_eq!(grids, vec![first_grid, second_grid]);
    assert_eq!(
        grids[0].cells.iter().map(|cell| cell.is_header).collect::<Vec<_>>(),
        [true, false, false, false]
    );
    assert_eq!(
        grids[1].cells.iter().map(|cell| cell.is_header).collect::<Vec<_>>(),
        [false, false, false, true]
    );
    assert_eq!(html.matches("<th>page-a</th>").count(), 1);
    assert_eq!(html.matches("<td>page-a</td>").count(), 1);
    assert_eq!(html.matches("<th>page-d</th>").count(), 1);
    assert_eq!(html.matches("<td>page-d</td>").count(), 1);
}
