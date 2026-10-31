use super::*;
use crate::ocr::layout_assembly::metadata_tests::recognized_fixture;
use crate::types::{NodeContent, TableGrid};

fn recognized_document(header_rows: usize, span: Option<[f32; 4]>) -> InternalDocument {
    let fixture = recognized_fixture(header_rows, span);
    let whole = InternalDocument::new("image");
    let detections = [crate::layout::LayoutDetection::new(
        crate::layout::LayoutClass::Table,
        0.99,
        fixture.outcome.tables[0].detection_bbox,
    )];
    let mut document = assemble_cached_layout_elements(
        &whole,
        &detections,
        &fixture.outcome.tables,
        &fixture.elements,
        140,
        100,
    );
    crate::types::internal::table_grid::attach_recognized_table_grids(&mut document, &fixture.outcome.grid_sidecars(1))
        .unwrap();
    document
}

fn derived_grid(document: &InternalDocument) -> TableGrid {
    let derived = crate::extraction::derive::derive_extraction_result(
        document.clone(),
        true,
        crate::core::config::OutputFormat::Html,
    );
    derived
        .document
        .expect("derived image document")
        .nodes
        .into_iter()
        .find_map(|node| match node.content {
            NodeContent::Table { grid } => Some(grid),
            _ => None,
        })
        .expect("recognized image table reaches document structure")
}

#[test]
fn image_ocr_keeps_both_detected_header_rows() {
    let document = recognized_document(2, None);
    let grid = derived_grid(&document);
    assert_eq!((grid.rows, grid.cols), (3, 2));
    assert_eq!(grid.cells.iter().filter(|cell| cell.is_header).count(), 4);
    let html = crate::rendering::render_html(&document);
    assert_eq!(html.matches("<th>").count(), 4, "{html}");
    assert_eq!(html.matches("<td>").count(), 2, "{html}");
}

#[test]
fn image_ocr_does_not_invent_a_header_when_tatr_detected_none() {
    let document = recognized_document(0, None);
    assert_eq!(document.tables[0].columns, None);
    assert!(derived_grid(&document).cells.iter().all(|cell| !cell.is_header));
    let html = crate::rendering::render_html(&document);
    assert_eq!(html.matches("<th>").count(), 0, "{html}");
    assert_eq!(html.matches("<td>").count(), 6, "{html}");
}

fn assert_span(span: [f32; 4], expected: (u32, u32), cells: usize) {
    let document = recognized_document(1, Some(span));
    let grid = derived_grid(&document);
    assert_eq!(grid.cells.len(), cells);
    let anchor = grid.cells.iter().find(|cell| cell.row == 0 && cell.col == 0).unwrap();
    assert_eq!((anchor.row_span, anchor.col_span), expected);
    let html = crate::rendering::render_html(&document);
    if expected.0 > 1 {
        assert!(html.contains("rowspan=\"2\""), "{html}");
    }
    if expected.1 > 1 {
        assert!(html.contains("colspan=\"2\""), "{html}");
    }
}

#[test]
fn image_ocr_keeps_a_column_span_in_html_and_document_structure() {
    assert_span([0.0, 0.0, 100.0, 20.0], (1, 2), 5);
}

#[test]
fn image_ocr_keeps_a_row_span_in_html_and_document_structure() {
    assert_span([0.0, 0.0, 50.0, 40.0], (2, 1), 5);
}

#[test]
fn image_ocr_keeps_a_combined_span_in_html_and_document_structure() {
    assert_span([0.0, 0.0, 100.0, 40.0], (2, 2), 3);
}

#[test]
fn image_ocr_keeps_every_recognized_fragment_once() {
    let document = recognized_document(1, Some([0.0, 0.0, 100.0, 40.0]));
    let grid = derived_grid(&document);
    let plain = crate::rendering::render_plain(&document);
    for text in ["North", "Region", "A", "B", "C", "D"] {
        assert_eq!(
            grid.cells
                .iter()
                .filter(|cell| cell.content.split_whitespace().any(|part| part == text))
                .count(),
            1
        );
        assert_eq!(
            plain.split_whitespace().filter(|part| *part == text).count(),
            1,
            "{plain}"
        );
    }
}

#[test]
fn image_ocr_does_not_reuse_geometry_after_dense_cells_change() {
    let mut document = recognized_document(1, Some([0.0, 0.0, 100.0, 20.0]));
    document.tables[0].cells[0][0] = "changed".to_owned();
    let grid = derived_grid(&document);
    assert!(grid.cells.iter().all(|cell| cell.row_span == 1 && cell.col_span == 1));
    let html = crate::rendering::render_html(&document);
    assert!(!html.contains("colspan="), "{html}");
    assert!(html.contains("changed"), "{html}");
}
