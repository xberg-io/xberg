use super::attach_native_table_grids;
use crate::types::internal::{ElementKind, InternalDocument, InternalElement, NATIVE_TABLE_GRID_ATTRIBUTE};
use crate::types::{BoundingBox, GridCell, Table, TableGrid};

fn table() -> Table {
    Table {
        cells: vec![vec!["A".to_owned(), "B".to_owned()]],
        page_number: 1,
        bounding_box: Some(BoundingBox {
            x0: 0.0,
            y0: 0.0,
            x1: 100.0,
            y1: 20.0,
        }),
        ..Default::default()
    }
}

fn grid(table: &Table, is_header: bool) -> TableGrid {
    TableGrid {
        rows: 1,
        cols: 2,
        cells: table.cells[0]
            .iter()
            .enumerate()
            .map(|(col, content)| GridCell {
                content: content.clone(),
                row: 0,
                col: col as u32,
                row_span: 1,
                col_span: 1,
                is_header,
                bbox: None,
                heading_level: None,
                style_name: None,
            })
            .collect(),
    }
}

fn document(table: Table, retained: Option<&TableGrid>) -> InternalDocument {
    let mut doc = InternalDocument::new("pdf");
    doc.tables.push(table);
    let mut element = InternalElement::text(ElementKind::Table { table_index: 0 }, "", 0).with_page(1);
    if let Some(retained) = retained {
        element.attributes.get_or_insert_with(Default::default).insert(
            NATIVE_TABLE_GRID_ATTRIBUTE.to_owned(),
            serde_json::to_string(retained).unwrap(),
        );
    }
    doc.push_element(element);
    doc
}

fn retained(doc: &InternalDocument) -> Option<TableGrid> {
    doc.elements[0]
        .attributes
        .as_ref()
        .and_then(|attrs| attrs.get(NATIVE_TABLE_GRID_ATTRIBUTE))
        .map(|encoded| serde_json::from_str(encoded).unwrap())
}

#[test]
fn native_grid_attaches_to_the_same_final_table() {
    let original = table();
    let geometry = grid(&original, true);
    let mut doc = document(original.clone(), None);
    attach_native_table_grids(&mut doc, &[original], &[(Some(geometry.clone()), None)]).unwrap();
    assert_eq!(retained(&doc), Some(geometry));
}

#[test]
fn native_grid_does_not_attach_after_final_cells_are_edited() {
    let original = table();
    let mut edited = original.clone();
    edited.cells[0][0] = "edited".to_owned();
    let mut doc = document(edited, None);
    attach_native_table_grids(
        &mut doc,
        std::slice::from_ref(&original),
        &[(Some(grid(&original, true)), None)],
    )
    .unwrap();
    assert_eq!(retained(&doc), None);
}

#[test]
fn original_index_does_not_replace_recognized_table_geometry() {
    let original = table();
    let mut recognized = original.clone();
    recognized.cells[0] = vec!["recognized".to_owned(), "value".to_owned()];
    let geometry = grid(&recognized, false);
    let mut doc = document(recognized, Some(&geometry));
    attach_native_table_grids(
        &mut doc,
        std::slice::from_ref(&original),
        &[(Some(grid(&original, true)), None)],
    )
    .unwrap();
    assert_eq!(retained(&doc), Some(geometry));
}

#[test]
fn native_grid_preserves_already_valid_recognizer_metadata() {
    let original = table();
    let recognized = grid(&original, false);
    let mut doc = document(original.clone(), Some(&recognized));
    attach_native_table_grids(
        &mut doc,
        std::slice::from_ref(&original),
        &[(Some(grid(&original, true)), None)],
    )
    .unwrap();
    assert_eq!(retained(&doc), Some(recognized));
}
