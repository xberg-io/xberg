use super::*;
use crate::ocr::layout_assembly::metadata_tests::recognized_fixture;
use crate::types::internal::table_grid::{TableGridSidecar, attach_recognized_table_grids, retained_grid};
use crate::types::internal::{ElementKind, InternalDocument, InternalElement, NATIVE_TABLE_GRID_ATTRIBUTE};
use crate::types::{BoundingBox, Table};

fn bbox(x0: f64, y0: f64, x1: f64, y1: f64) -> BoundingBox {
    BoundingBox { x0, y0, x1, y1 }
}

fn recognized(
    frame: RecognizedTableFrame,
    header_rows: usize,
    span: Option<[f32; 4]>,
) -> (Vec<Table>, Vec<TableGridSidecar>) {
    let fixture = recognized_fixture(header_rows, span);
    let mut tables = Vec::new();
    let mut grids = Vec::new();
    append_recognized_tables(&fixture.outcome, 4, frame, &mut tables, &mut grids);
    assert_eq!((tables.len(), grids.len()), (1, 1));
    (tables, grids)
}

fn document(tables: Vec<Table>) -> InternalDocument {
    let mut document = InternalDocument::new("pdf");
    for (index, table) in tables.iter().enumerate() {
        let mut element = InternalElement::text(
            ElementKind::Table {
                table_index: index as u32,
            },
            "",
            0,
        )
        .with_page(4);
        element.bbox = table.bounding_box;
        document.push_element(element);
    }
    document.tables = tables;
    document
}

#[test]
fn full_pdf_recognition_pairs_table_and_cell_page_coordinates() {
    let (tables, grids) = recognized(
        RecognizedTableFrame::PdfPage {
            raster: (140, 100),
            points: (280.0, 300.0),
        },
        2,
        None,
    );
    assert_eq!(tables[0].bounding_box, Some(bbox(20.0, 60.0, 220.0, 240.0)));
    assert_eq!(tables[0].page_number, 4);
    assert_eq!(tables[0].table_id.as_deref(), Some("table-1"));
    assert_eq!(grids[0].grid.cells[0].bbox, Some(bbox(20.0, 180.0, 120.0, 240.0)));
    assert_eq!(grids[0].grid.cells.iter().filter(|cell| cell.is_header).count(), 4);
    let mut document = document(tables);
    attach_recognized_table_grids(&mut document, &grids).unwrap();
    let retained = retained_grid(&document.tables[0], document.elements[0].attributes.as_ref()).unwrap();
    assert_eq!(retained, grids[0].grid);
}

#[test]
fn detached_pdf_recognition_is_flipped_and_scaled_exactly_once() {
    let (tables, grids) = recognized(
        RecognizedTableFrame::DetachedPdfRaster { height: 100 },
        1,
        Some([0.0, 0.0, 100.0, 40.0]),
    );
    assert_eq!(tables[0].bounding_box, Some(bbox(10.0, 20.0, 110.0, 80.0)));
    assert_eq!(grids[0].grid.cells[0].bbox, Some(bbox(10.0, 40.0, 110.0, 80.0)));
    let mut source = document(tables);
    attach_recognized_table_grids(&mut source, &grids).unwrap();
    let document = super::super::document::build_pipeline_ocr_page_document(
        Some(source),
        vec![],
        vec![],
        "North Region A B C D",
        4,
        (140, 100),
        (280.0, 300.0),
    )
    .unwrap();
    assert_eq!(document.tables[0].bounding_box, Some(bbox(20.0, 60.0, 220.0, 240.0)));
    let retained = retained_grid(&document.tables[0], document.elements[0].attributes.as_ref()).unwrap();
    assert_eq!(retained.cells[0].bbox, Some(bbox(20.0, 120.0, 220.0, 240.0)));
    assert_eq!((retained.cells[0].row_span, retained.cells[0].col_span), (2, 2));
    assert_eq!(document.elements[0].page, Some(4));
}

#[test]
fn rejected_replaced_or_duplicate_final_tables_do_not_gain_ocr_metadata() {
    let (tables, grids) = recognized(RecognizedTableFrame::ImagePixels, 0, Some([0.0, 0.0, 100.0, 20.0]));
    let mut replaced = tables[0].clone();
    replaced.cells[0][0] = "edited".to_owned();
    for final_tables in [vec![], vec![replaced], vec![tables[0].clone(), tables[0].clone()]] {
        let mut document = document(final_tables);
        attach_recognized_table_grids(&mut document, &grids).unwrap();
        assert!(document.elements.iter().all(|element| {
            element
                .attributes
                .as_ref()
                .is_none_or(|attributes| !attributes.contains_key(NATIVE_TABLE_GRID_ATTRIBUTE))
        }));
    }
}

#[test]
fn existing_authoritative_header_grid_and_labels_survive_an_ocr_candidate() {
    let (mut tables, grids) = recognized(RecognizedTableFrame::ImagePixels, 0, None);
    tables[0].columns = Some(tables[0].cells[0].clone());
    let expected_columns = tables[0].columns.clone();
    let mut existing = grids[0].grid.clone();
    for cell in &mut existing.cells {
        cell.is_header = cell.row == 0;
    }
    let mut document = document(tables);
    document.elements[0]
        .attributes
        .get_or_insert_with(Default::default)
        .insert(
            NATIVE_TABLE_GRID_ATTRIBUTE.to_owned(),
            serde_json::to_string(&existing).unwrap(),
        );
    attach_recognized_table_grids(&mut document, &grids).unwrap();
    assert_eq!(
        retained_grid(&document.tables[0], document.elements[0].attributes.as_ref()),
        Some(existing)
    );
    assert_eq!(document.tables[0].columns, expected_columns);
}
