#![cfg(feature = "pdf")]

use xberg::extraction::derive::derive_extraction_result;
use xberg::types::document_structure::NodeContent;
use xberg::types::internal::{ElementKind, InternalDocument, InternalElement};
use xberg::{OutputFormat, Table};

fn document(rowspan: u32) -> InternalDocument {
    let mut doc = InternalDocument::new("pdf");
    let columns = vec!["Header".into(), String::new()];
    doc.tables.push(Table {
        columns: Some(columns.clone()),
        cells: vec![columns, vec!["Record".into(), "Value".into()]],
        page_number: 1,
        ..Default::default()
    });
    let mut element = InternalElement::text(ElementKind::Table { table_index: 0 }, "", 0);
    element.page = Some(1);
    let sidecar = serde_json::json!({
        "rows": 2,
        "cols": 2,
        "cells": [
            {"row": 0, "col": 0, "row_span": rowspan, "col_span": 2, "is_header": true, "content": "Header"},
            {"row": 1, "col": 0, "content": "Record"},
            {"row": 1, "col": 1, "content": "Value"}
        ]
    });
    element
        .attributes
        .get_or_insert_with(Default::default)
        .insert("xberg:internal:native-table-grid".into(), sidecar.to_string());
    doc.push_element(element);
    doc
}

#[test]
fn published_html_and_grid_agree_on_retained_span_fallback() {
    for (rowspan, valid) in [(1, true), (2, false)] {
        let result = derive_extraction_result(document(rowspan), true, OutputFormat::Html);
        assert_eq!(result.tables.len(), 1);
        assert_eq!(result.tables[0].cells, [["Header", ""], ["Record", "Value"]]);
        let html = result.formatted_content.as_deref().expect("HTML output");
        assert_eq!(html.matches("<table>").count(), 1, "{html}");
        assert_eq!(html.matches("</table>").count(), 1);
        for text in ["Header", "Record", "Value"] {
            assert_eq!(html.matches(text).count(), 1, "{html}");
        }
        assert_eq!(html.contains("<th colspan=\"2\">Header</th>"), valid);
        assert_eq!(html.contains("<th>Header</th>"), !valid);
        assert!(!html.contains("rowspan="));

        let structure = result.document.unwrap();
        let grids: Vec<_> = structure
            .nodes
            .iter()
            .filter_map(|node| match &node.content {
                NodeContent::Table { grid } => Some(grid),
                _ => None,
            })
            .collect();
        assert_eq!(grids.len(), 1);
        let grid = grids[0];
        assert_eq!((grid.rows, grid.cols), (2, 2));
        let cells: Vec<_> = grid
            .cells
            .iter()
            .map(|cell| {
                (
                    cell.row,
                    cell.col,
                    cell.row_span,
                    cell.col_span,
                    cell.is_header,
                    cell.content.as_str(),
                )
            })
            .collect();
        let mut expected = vec![(0, 0, 1, if valid { 2 } else { 1 }, true, "Header")];
        if !valid {
            expected.push((0, 1, 1, 1, true, ""));
        }
        expected.extend([(1, 0, 1, 1, false, "Record"), (1, 1, 1, 1, false, "Value")]);
        assert_eq!(cells, expected);
    }
}
