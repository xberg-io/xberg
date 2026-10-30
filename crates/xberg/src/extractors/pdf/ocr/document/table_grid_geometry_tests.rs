use super::*;
use crate::types::internal::{ElementKind, InternalDocument, InternalElement, NATIVE_TABLE_GRID_ATTRIBUTE};
use crate::types::{BoundingBox, GridCell, Table, TableGrid};

fn bbox(x0: f64, y0: f64, x1: f64, y1: f64) -> BoundingBox {
    BoundingBox { x0, y0, x1, y1 }
}

fn source_grid(bottom_left_grid: bool) -> TableGrid {
    let bounds = if bottom_left_grid {
        [
            bbox(10.0, 60.0, 110.0, 80.0),
            bbox(10.0, 20.0, 60.0, 60.0),
            bbox(60.0, 20.0, 110.0, 60.0),
        ]
    } else {
        [
            bbox(10.0, 20.0, 110.0, 40.0),
            bbox(10.0, 40.0, 60.0, 80.0),
            bbox(60.0, 40.0, 110.0, 80.0),
        ]
    };
    let cells = [("Span", 0, 0, 2), ("A", 1, 0, 1), ("B", 1, 1, 1)]
        .into_iter()
        .zip(bounds)
        .map(|((text, row, col, col_span), bbox)| GridCell {
            content: text.to_owned(),
            row,
            col,
            row_span: 1,
            col_span,
            is_header: row == 0,
            bbox: Some(bbox),
            heading_level: None,
            style_name: None,
        })
        .collect();
    TableGrid {
        rows: 2,
        cols: 2,
        cells,
    }
}

fn document(bottom_left_grid: bool) -> InternalDocument {
    let table = Table {
        cells: vec![
            vec!["Span".to_owned(), String::new()],
            vec!["A".to_owned(), "B".to_owned()],
        ],
        page_number: 4,
        bounding_box: Some(bbox(10.0, 20.0, 110.0, 80.0)),
        ..Default::default()
    };
    let grid = source_grid(bottom_left_grid);
    assert!(crate::extraction::derive::native_grid_matches_table(&table, &grid));
    let mut element = InternalElement::text(ElementKind::Table { table_index: 0 }, "", 0).with_page(4);
    element.bbox = table.bounding_box;
    element.attributes.get_or_insert_with(Default::default).insert(
        NATIVE_TABLE_GRID_ATTRIBUTE.to_owned(),
        serde_json::to_string(&grid).unwrap(),
    );
    let mut document = InternalDocument::new("pdf");
    document.tables.push(table);
    document.push_element(element);
    document
}

fn grid(document: &InternalDocument) -> TableGrid {
    let encoded = document.elements[0]
        .attributes
        .as_ref()
        .unwrap()
        .get(NATIVE_TABLE_GRID_ATTRIBUTE)
        .unwrap();
    serde_json::from_str(encoded).unwrap()
}

#[test]
fn detached_pdf_tables_and_retained_cells_reach_the_same_page_points() {
    let document = build_pipeline_ocr_page_document(
        Some(document(true)),
        vec![],
        vec![],
        "Span A B",
        4,
        (140, 100),
        (280.0, 300.0),
    )
    .expect("detached OCR document is retained");
    assert_eq!(document.tables.len(), 1);
    assert_eq!(document.tables[0].page_number, 4);
    assert_eq!(document.elements[0].page, Some(4));
    assert_eq!(document.tables[0].bounding_box, Some(bbox(20.0, 60.0, 220.0, 240.0)));
    assert_eq!(document.elements[0].bbox, document.tables[0].bounding_box);
    let grid = grid(&document);
    assert_eq!((grid.rows, grid.cols, grid.cells[0].col_span), (2, 2, 2));
    assert_eq!(grid.cells[0].bbox, Some(bbox(20.0, 180.0, 220.0, 240.0)));
    assert_eq!(grid.cells[1].bbox, Some(bbox(20.0, 60.0, 120.0, 180.0)));
    assert_eq!(grid.cells[2].bbox, Some(bbox(120.0, 60.0, 220.0, 180.0)));
}

#[test]
fn public_pdf_raster_frame_scales_and_flips_retained_cell_geometry() {
    let mut document = document(false);
    place_public_ocr_bboxes(
        &mut document,
        PublicOcrOutputFrame::PdfRaster {
            processed: (280, 200),
            raster: (140, 100),
        },
    );
    assert_eq!(document.elements[0].bbox, Some(bbox(5.0, 60.0, 55.0, 90.0)));
    assert_eq!(grid(&document).cells[0].bbox, Some(bbox(5.0, 80.0, 55.0, 90.0)));
    assert_eq!(grid(&document).cells[1].bbox, Some(bbox(5.0, 60.0, 30.0, 80.0)));
    assert_eq!(document.elements[0].page, Some(4));
    assert_eq!(document.tables[0].bounding_box, Some(bbox(5.0, 10.0, 55.0, 40.0)));
    let final_document = build_pipeline_ocr_page_document(
        Some(document),
        vec![],
        vec![],
        "Span A B",
        4,
        (140, 100),
        (280.0, 300.0),
    )
    .unwrap();
    assert_eq!(
        final_document.tables[0].bounding_box,
        Some(bbox(10.0, 180.0, 110.0, 270.0))
    );
    assert_eq!(final_document.elements[0].bbox, final_document.tables[0].bounding_box);
    assert_eq!(
        grid(&final_document).cells[0].bbox,
        Some(bbox(10.0, 240.0, 110.0, 270.0))
    );
}

#[test]
fn public_pdf_page_frame_converts_tables_and_retained_cells_together() {
    let mut document = document(false);
    place_public_ocr_bboxes(
        &mut document,
        PublicOcrOutputFrame::PdfPage {
            raster: (140, 100),
            points: (280.0, 300.0),
        },
    );
    assert_eq!(document.tables[0].bounding_box, Some(bbox(20.0, 60.0, 220.0, 240.0)));
    assert_eq!(document.elements[0].bbox, document.tables[0].bounding_box);
    assert_eq!(grid(&document).cells[0].bbox, Some(bbox(20.0, 180.0, 220.0, 240.0)));
    assert_eq!(grid(&document).cells[1].bbox, Some(bbox(20.0, 60.0, 120.0, 180.0)));
    assert_eq!(document.elements[0].page, Some(4));
}

#[test]
fn degenerate_raster_does_not_fabricate_retained_grid_coordinates() {
    let source = document(true);
    let table_bbox = source.tables[0].bounding_box;
    let expected = grid(&source);
    let document =
        build_pipeline_ocr_page_document(Some(source), vec![], vec![], "Span A B", 4, (0, 100), (280.0, 300.0))
            .expect("document remains available without a fabricated scale");
    assert_eq!(document.tables[0].bounding_box, table_bbox);
    assert_eq!(grid(&document), expected);
}

#[test]
fn raw_and_public_table_elements_reach_identical_pdf_geometry() {
    let raw = document(true);
    let mut public = document(false);
    place_public_ocr_bboxes(
        &mut public,
        PublicOcrOutputFrame::PdfRaster {
            processed: (140, 100),
            raster: (140, 100),
        },
    );
    for source in [raw, public] {
        let final_document =
            build_pipeline_ocr_page_document(Some(source), vec![], vec![], "Span A B", 4, (140, 100), (280.0, 300.0))
                .unwrap();
        assert_eq!(
            final_document.tables[0].bounding_box,
            Some(bbox(20.0, 60.0, 220.0, 240.0))
        );
        assert_eq!(final_document.elements[0].bbox, final_document.tables[0].bounding_box);
        assert_eq!(
            grid(&final_document).cells[0].bbox,
            Some(bbox(20.0, 180.0, 220.0, 240.0))
        );
    }
}

#[test]
fn missing_table_rectangle_keeps_the_element_flip_fallback() {
    let mut source = document(true);
    source.tables[0].bounding_box = None;
    let final_document =
        build_pipeline_ocr_page_document(Some(source), vec![], vec![], "Span A B", 4, (140, 100), (280.0, 300.0))
            .unwrap();
    assert_eq!(final_document.tables[0].bounding_box, None);
    assert_eq!(final_document.elements[0].bbox, Some(bbox(20.0, 60.0, 220.0, 240.0)));
    assert_eq!(
        grid(&final_document).cells[0].bbox,
        Some(bbox(20.0, 180.0, 220.0, 240.0))
    );
}
