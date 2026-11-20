//! Integration tests for table extraction from PDF structure trees.
//!
//! Tests table extraction, rendering, and markdown conversion according to
//! ISO 32000-1:2008 Section 14.8.4.3.4 table element specifications.

use xberg_native_pdf::geometry::Rect;
use xberg_native_pdf::layout::TextBlock;
use xberg_native_pdf::structure::table_extractor::{Table, TableCell, TableRow, extract_table};
use xberg_native_pdf::structure::types::{StructChild, StructElem, StructType};

/// Helper to create a mock text block with specific MCID
fn mock_text_block(text: &str, mcid: u32) -> TextBlock {
    TextBlock {
        text: text.to_string(),
        chars: vec![],
        bbox: Rect::new(0.0, 0.0, 100.0, 12.0),
        avg_font_size: 12.0,
        dominant_font: "Arial".to_string(),
        is_italic: false,
        is_bold: false,
        mcid: Some(mcid),
        mcid_scope: None,
        sequence: 0,
        rotation_degrees: 0.0,
    }
}

/// Helper to create marked content reference in structure element
fn add_mcid_ref(elem: &mut StructElem, mcid: u32, page: u32) {
    elem.add_child(StructChild::MarkedContentRef {
        mcid,
        page,
        scope: xberg_native_pdf::structure::McidScope::Page(page),
    });
}

#[test]
fn test_simple_2x2_table_extraction() {
    let mut table_elem = StructElem::new(StructType::Table);

    let mut row1 = StructElem::new(StructType::TR);
    let mut cell1 = StructElem::new(StructType::TD);
    add_mcid_ref(&mut cell1, 0, 0);
    row1.add_child(StructChild::StructElem(Box::new(cell1)));

    let mut cell2 = StructElem::new(StructType::TD);
    add_mcid_ref(&mut cell2, 1, 0);
    row1.add_child(StructChild::StructElem(Box::new(cell2)));

    table_elem.add_child(StructChild::StructElem(Box::new(row1)));

    let mut row2 = StructElem::new(StructType::TR);
    let mut cell3 = StructElem::new(StructType::TD);
    add_mcid_ref(&mut cell3, 2, 0);
    row2.add_child(StructChild::StructElem(Box::new(cell3)));

    let mut cell4 = StructElem::new(StructType::TD);
    add_mcid_ref(&mut cell4, 3, 0);
    row2.add_child(StructChild::StructElem(Box::new(cell4)));

    table_elem.add_child(StructChild::StructElem(Box::new(row2)));

    let text_blocks = vec![
        mock_text_block("A", 0),
        mock_text_block("B", 1),
        mock_text_block("C", 2),
        mock_text_block("D", 3),
    ];

    let table = extract_table(&table_elem, &text_blocks).unwrap();

    assert_eq!(table.rows.len(), 2);
    assert_eq!(table.col_count, 2);
    assert!(!table.has_header);

    assert_eq!(table.rows[0].cells.len(), 2);
    assert_eq!(table.rows[0].cells[0].text, "A");
    assert_eq!(table.rows[0].cells[1].text, "B");

    assert_eq!(table.rows[1].cells.len(), 2);
    assert_eq!(table.rows[1].cells[0].text, "C");
    assert_eq!(table.rows[1].cells[1].text, "D");
}

#[test]
fn test_table_with_header() {
    let mut table_elem = StructElem::new(StructType::Table);

    let mut thead = StructElem::new(StructType::THead);
    let mut header_row = StructElem::new(StructType::TR);

    let mut header1 = StructElem::new(StructType::TH);
    add_mcid_ref(&mut header1, 0, 0);
    header_row.add_child(StructChild::StructElem(Box::new(header1)));

    let mut header2 = StructElem::new(StructType::TH);
    add_mcid_ref(&mut header2, 1, 0);
    header_row.add_child(StructChild::StructElem(Box::new(header2)));

    thead.add_child(StructChild::StructElem(Box::new(header_row)));
    table_elem.add_child(StructChild::StructElem(Box::new(thead)));

    let mut tbody = StructElem::new(StructType::TBody);
    let mut body_row = StructElem::new(StructType::TR);

    let mut data1 = StructElem::new(StructType::TD);
    add_mcid_ref(&mut data1, 2, 0);
    body_row.add_child(StructChild::StructElem(Box::new(data1)));

    let mut data2 = StructElem::new(StructType::TD);
    add_mcid_ref(&mut data2, 3, 0);
    body_row.add_child(StructChild::StructElem(Box::new(data2)));

    tbody.add_child(StructChild::StructElem(Box::new(body_row)));
    table_elem.add_child(StructChild::StructElem(Box::new(tbody)));

    let text_blocks = vec![
        mock_text_block("Header1", 0),
        mock_text_block("Header2", 1),
        mock_text_block("Data1", 2),
        mock_text_block("Data2", 3),
    ];

    let table = extract_table(&table_elem, &text_blocks).unwrap();

    assert!(table.has_header);
    assert_eq!(table.rows.len(), 2);
    assert_eq!(table.col_count, 2);

    assert!(table.rows[0].is_header);
    assert_eq!(table.rows[0].cells[0].text, "Header1");
    assert_eq!(table.rows[0].cells[1].text, "Header2");

    assert!(!table.rows[1].is_header);
    assert_eq!(table.rows[1].cells[0].text, "Data1");
    assert_eq!(table.rows[1].cells[1].text, "Data2");
}

#[test]
fn test_table_cell_spans() {
    let mut table_elem = StructElem::new(StructType::Table);
    let mut row = StructElem::new(StructType::TR);

    let mut cell1 = StructElem::new(StructType::TD);
    add_mcid_ref(&mut cell1, 0, 0);
    row.add_child(StructChild::StructElem(Box::new(cell1)));

    table_elem.add_child(StructChild::StructElem(Box::new(row)));

    let text_blocks = vec![mock_text_block("Cell", 0)];

    let table = extract_table(&table_elem, &text_blocks).unwrap();

    assert_eq!(table.rows[0].cells[0].colspan, 1);
    assert_eq!(table.rows[0].cells[0].rowspan, 1);
}

#[test]
fn test_table_escape_pipes() {
    let mut table_elem = StructElem::new(StructType::Table);
    let mut row = StructElem::new(StructType::TR);

    let mut cell = StructElem::new(StructType::TD);
    add_mcid_ref(&mut cell, 0, 0);
    row.add_child(StructChild::StructElem(Box::new(cell)));

    table_elem.add_child(StructChild::StructElem(Box::new(row)));

    let text_blocks = vec![mock_text_block("A|B|C", 0)];

    let table = extract_table(&table_elem, &text_blocks).unwrap();

    assert_eq!(table.rows[0].cells[0].text, "A|B|C");
}

#[test]
fn test_empty_table() {
    let table_elem = StructElem::new(StructType::Table);
    let text_blocks = vec![];

    let table = extract_table(&table_elem, &text_blocks).unwrap();

    assert!(table.is_empty());
    assert_eq!(table.rows.len(), 0);
    assert_eq!(table.col_count, 0);
}

#[test]
fn test_table_row_operations() {
    let mut row = TableRow::new(false);
    assert!(!row.is_header);
    assert!(row.cells.is_empty());

    let cell1 = TableCell::new("A".to_string(), false);
    let cell2 = TableCell::new("B".to_string(), false);

    row.add_cell(cell1);
    row.add_cell(cell2);

    assert_eq!(row.cells.len(), 2);
    assert_eq!(row.cells[0].text, "A");
    assert_eq!(row.cells[1].text, "B");
}

#[test]
fn test_table_cell_with_mcids() {
    let mut cell = TableCell::new("Test".to_string(), false);
    assert!(cell.mcids.is_empty());

    cell.add_mcid(0);
    cell.add_mcid(1);

    assert_eq!(cell.mcids.len(), 2);
    assert_eq!(cell.mcids[0], 0);
    assert_eq!(cell.mcids[1], 1);
}

#[test]
fn test_table_multiline_cells() {
    let mut table_elem = StructElem::new(StructType::Table);
    let mut row = StructElem::new(StructType::TR);

    let mut cell = StructElem::new(StructType::TD);
    add_mcid_ref(&mut cell, 0, 0);
    add_mcid_ref(&mut cell, 1, 0);
    row.add_child(StructChild::StructElem(Box::new(cell)));

    table_elem.add_child(StructChild::StructElem(Box::new(row)));

    let text_blocks = vec![mock_text_block("Line1", 0), mock_text_block("Line2", 1)];

    let table = extract_table(&table_elem, &text_blocks).unwrap();

    assert!(table.rows[0].cells[0].text.contains("Line1"));
    assert!(table.rows[0].cells[0].text.contains("Line2"));
}

#[test]
fn test_table_3x3() {
    let mut table_elem = StructElem::new(StructType::Table);

    for row_idx in 0..3 {
        let mut row = StructElem::new(StructType::TR);

        for col_idx in 0..3 {
            let mcid = (row_idx * 3 + col_idx) as u32;
            let mut cell = StructElem::new(StructType::TD);
            add_mcid_ref(&mut cell, mcid, 0);
            row.add_child(StructChild::StructElem(Box::new(cell)));
        }

        table_elem.add_child(StructChild::StructElem(Box::new(row)));
    }

    let text_blocks: Vec<TextBlock> = (0..9).map(|i| mock_text_block(&format!("Cell{}", i), i)).collect();

    let table = extract_table(&table_elem, &text_blocks).unwrap();

    assert_eq!(table.rows.len(), 3);
    assert_eq!(table.col_count, 3);

    for (row_idx, row) in table.rows.iter().enumerate() {
        assert_eq!(row.cells.len(), 3);
        for (col_idx, cell) in row.cells.iter().enumerate() {
            let expected = format!("Cell{}", row_idx * 3 + col_idx);
            assert_eq!(cell.text, expected);
        }
    }
}

#[test]
fn test_table_missing_mcid() {
    let mut table_elem = StructElem::new(StructType::Table);
    let mut row = StructElem::new(StructType::TR);

    let mut cell = StructElem::new(StructType::TD);
    add_mcid_ref(&mut cell, 99, 0);
    row.add_child(StructChild::StructElem(Box::new(cell)));

    table_elem.add_child(StructChild::StructElem(Box::new(row)));

    let text_blocks = vec![mock_text_block("Other", 0)];

    let table = extract_table(&table_elem, &text_blocks).unwrap();

    assert_eq!(table.rows[0].cells.len(), 1);
    assert_eq!(table.rows[0].cells[0].text, "");
}

#[test]
fn test_table_operations() {
    let mut table = Table::new();

    assert!(table.is_empty());
    assert_eq!(table.col_count, 0);
    assert!(!table.has_header);

    let mut row = TableRow::new(false);
    row.add_cell(TableCell::new("A".to_string(), false));
    row.add_cell(TableCell::new("B".to_string(), false));

    table.add_row(row);

    assert_eq!(table.col_count, 2);
    assert_eq!(table.rows.len(), 1);
}
