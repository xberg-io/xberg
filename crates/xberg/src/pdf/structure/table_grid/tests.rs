use super::*;
use crate::types::internal::InternalElement;
use crate::types::{BoundingBox, GridCell};

fn table() -> Table {
    Table {
        cells: vec![vec!["A".to_owned(), "B".to_owned()]],
        page_number: 1,
        bounding_box: Some(BoundingBox {
            x0: 1.0,
            y0: 2.0,
            x1: 3.0,
            y1: 4.0,
        }),
        ..Default::default()
    }
}

fn sidecar(table: &Table) -> TableGridSidecar {
    TableGridSidecar {
        table: table.clone(),
        grid: TableGrid {
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
                    is_header: false,
                    bbox: None,
                    heading_level: None,
                    style_name: None,
                })
                .collect(),
        },
    }
}

fn document(tables: Vec<Table>) -> InternalDocument {
    let mut doc = InternalDocument::new("pdf");
    for (index, table) in tables.iter().enumerate() {
        doc.push_element(
            InternalElement::text(
                ElementKind::Table {
                    table_index: index as u32,
                },
                "",
                0,
            )
            .with_page(table.page_number),
        );
    }
    doc.tables = tables;
    doc
}

#[test]
fn matching_final_table_keeps_exact_geometry_and_text() {
    let table = table();
    let source = sidecar(&table);
    let mut doc = document(vec![table.clone()]);
    attach_table_grids(&mut doc, &[source]).unwrap();
    assert_eq!(
        retained_grid(&table, doc.elements[0].attributes.as_ref()),
        Some(sidecar(&table).grid)
    );
}

#[test]
fn edited_cells_page_or_bbox_cannot_reuse_geometry() {
    let original = table();
    let mut changed = [original.clone(), original.clone(), original.clone()];
    changed[0].cells[0][0] = "edited".to_owned();
    changed[1].page_number = 2;
    changed[2].bounding_box.as_mut().unwrap().x0 = 9.0;
    for table in changed {
        let mut doc = document(vec![table.clone()]);
        attach_table_grids(&mut doc, &[sidecar(&original)]).unwrap();
        assert_eq!(retained_grid(&table, doc.elements[0].attributes.as_ref()), None);
    }
}

#[test]
fn rejected_table_does_not_create_an_element_or_payload() {
    let mut doc = document(vec![]);
    attach_table_grids(&mut doc, &[sidecar(&table())]).unwrap();
    assert_eq!(doc.elements.len(), 0);
    assert_eq!(doc.tables.len(), 0);
}

#[test]
fn deduplicated_producers_cannot_select_arbitrary_geometry() {
    let table = table();
    let mut doc = document(vec![table.clone()]);
    attach_table_grids(&mut doc, &[sidecar(&table), sidecar(&table)]).unwrap();
    assert_eq!(retained_grid(&table, doc.elements[0].attributes.as_ref()), None);
}

#[test]
fn identical_final_tables_are_ambiguous() {
    let table = table();
    let mut doc = document(vec![table.clone(), table.clone()]);
    attach_table_grids(&mut doc, &[sidecar(&table)]).unwrap();
    for element in &doc.elements {
        assert_eq!(retained_grid(&table, element.attributes.as_ref()), None);
    }
}

#[test]
fn reordered_table_indices_are_matched_by_final_identity() {
    let first = table();
    let mut second = first.clone();
    second.cells[0][0] = "second".to_owned();
    let mut doc = document(vec![second.clone(), first.clone()]);
    attach_table_grids(&mut doc, &[sidecar(&first)]).unwrap();
    assert_eq!(retained_grid(&second, doc.elements[0].attributes.as_ref()), None);
    assert_eq!(
        retained_grid(&first, doc.elements[1].attributes.as_ref()),
        Some(sidecar(&first).grid)
    );
}

#[test]
fn rejected_page_cannot_attach_geometry_to_another_page_element() {
    let table = table();
    let mut doc = document(vec![table.clone()]);
    doc.elements[0].page = Some(2);
    attach_table_grids(&mut doc, &[sidecar(&table)]).unwrap();
    assert_eq!(retained_grid(&table, doc.elements[0].attributes.as_ref()), None);
}

#[test]
fn invalid_source_grid_does_not_attach() {
    let table = table();
    let mut source = sidecar(&table);
    source.grid.cells[0].content = "wrong".to_owned();
    let mut doc = document(vec![table.clone()]);
    attach_table_grids(&mut doc, &[source]).unwrap();
    assert_eq!(retained_grid(&table, doc.elements[0].attributes.as_ref()), None);
}
