use super::*;
use crate::layout::models::tatr::{TatrClass, TatrDetection, TatrResult, build_cell_grid_with_structure};
use crate::pdf::table_reconstruct::HocrWord;

fn detection(class_name: TatrClass, bbox: [f32; 4]) -> TatrDetection {
    TatrDetection {
        bbox,
        confidence: 0.99,
        class_name,
    }
}

fn recognized(header_rows: usize, span: Option<[f32; 4]>) -> RecognizedTatrTable {
    let result = TatrResult {
        rows: (0..3)
            .map(|row| detection(TatrClass::Row, [0.0, row as f32 * 20.0, 100.0, (row + 1) as f32 * 20.0]))
            .collect(),
        columns: vec![
            detection(TatrClass::Column, [0.0, 0.0, 50.0, 60.0]),
            detection(TatrClass::Column, [50.0, 0.0, 100.0, 60.0]),
        ],
        headers: (header_rows > 0)
            .then(|| detection(TatrClass::ColumnHeader, [0.0, 0.0, 100.0, header_rows as f32 * 20.0]))
            .into_iter()
            .collect(),
        spanning: span
            .map(|bbox| detection(TatrClass::SpanningCell, bbox))
            .into_iter()
            .collect(),
        table_bbox: Some([0.0, 0.0, 100.0, 60.0]),
    };
    let (cells, structure) = build_cell_grid_with_structure(&result, result.table_bbox);
    let words: Vec<_> = ["North", "Region", "A", "B", "C", "D"]
        .into_iter()
        .enumerate()
        .map(|(index, text)| HocrWord {
            text: text.to_owned(),
            left: 10 + (index as u32 % 2) * 50,
            top: 5 + (index as u32 / 2) * 20,
            width: 20,
            height: 8,
            confidence: 99.0,
        })
        .collect();
    let image = image::RgbImage::new(100, 60);
    let page_result = crate::pdf::structure::types::PageLayoutResult {
        page_width_pts: 100.0,
        page_height_pts: 100.0,
    };
    let crop = TatrCrop {
        image: image.clone(),
        px_left: 0,
        px_top: 0,
        width: 100,
        height: 60,
        sx: 1.0,
        sy: 1.0,
        extended_bottom_pt: 60.0,
    };
    let hint = LayoutHint {
        class_name: LayoutHintClass::Table,
        confidence: 0.99,
        left: 0.0,
        bottom: 40.0,
        right: 100.0,
        top: 100.0,
    };
    let context = TatrPageContext {
        image: &image,
        words: &words,
        page_result: &page_result,
        page_height: 100.0,
        page_index: 0,
        allow_single_column: false,
    };
    assemble_tatr_table(&TatrInference { cells, structure }, &crop, &hint, None, &context)
        .expect("populated three-row TATR table")
}

fn recognized_document(result: &RecognizedTatrTable) -> crate::types::internal::InternalDocument {
    use crate::types::internal::{ElementKind, InternalDocument, InternalElement};
    let mut doc = InternalDocument::new("pdf");
    doc.tables.push(result.table.clone());
    doc.push_element(InternalElement::text(ElementKind::Table { table_index: 0 }, "", 0).with_page(1));
    crate::pdf::structure::table_grid::attach_table_grids(
        &mut doc,
        &[crate::pdf::structure::table_grid::TableGridSidecar {
            table: result.table.clone(),
            grid: result.grid.clone().expect("native TATR structure"),
        }],
    )
    .unwrap();
    doc
}

fn derived_grid(result: &RecognizedTatrTable) -> crate::types::TableGrid {
    let doc = recognized_document(result);
    let derived =
        crate::extraction::derive::derive_extraction_result(doc, true, crate::core::config::OutputFormat::Markdown);
    let document = derived.document.expect("derived structure");
    document
        .nodes
        .into_iter()
        .find_map(|node| match node.content {
            crate::types::NodeContent::Table { grid } => Some(grid),
            _ => None,
        })
        .expect("derived TATR table")
}

#[test]
fn native_tatr_preserves_declared_header_labels() {
    let result = recognized(2, None);
    assert_eq!(
        result.table.columns,
        Some(vec!["North".to_owned(), "Region".to_owned()])
    );
    assert_eq!(result.table.cells.len(), 3);
    let grid = derived_grid(&result);
    assert_eq!((grid.rows, grid.cols), (3, 2));
    assert_eq!(grid.cells.iter().filter(|cell| cell.is_header).count(), 4);
}

#[test]
fn native_tatr_does_not_invent_header_labels_when_none_were_detected() {
    let result = recognized(0, None);
    assert_eq!(result.table.columns, None);
    assert_eq!(result.table.cells[0], ["North", "Region"]);
    assert_eq!(
        derived_grid(&result).cells.iter().filter(|cell| cell.is_header).count(),
        0
    );
}

#[test]
fn native_tatr_column_span_combines_source_fragments_once() {
    let result = recognized(1, Some([0.0, 0.0, 100.0, 20.0]));
    assert_eq!(result.table.cells[0], ["North Region", ""]);
    assert_eq!(result.table.cells[1], ["A", "B"]);
    assert_eq!(result.table.cells[2], ["C", "D"]);
    let grid = derived_grid(&result);
    assert_eq!(grid.cells.len(), 5);
    assert_eq!((grid.cells[0].row_span, grid.cells[0].col_span), (1, 2));
    assert_eq!(grid.cells[0].bbox.unwrap().x1, 100.0);
}

#[test]
fn native_tatr_row_span_combines_source_fragments_once() {
    let result = recognized(0, Some([0.0, 20.0, 50.0, 60.0]));
    assert_eq!(result.table.cells[1], ["A C", "B"]);
    assert_eq!(result.table.cells[2], ["", "D"]);
    let grid = derived_grid(&result);
    let merged = grid.cells.iter().find(|cell| cell.content == "A C").unwrap();
    assert_eq!((merged.row, merged.col, merged.row_span, merged.col_span), (1, 0, 2, 1));
    assert_eq!(grid.cells.len(), 5);
}

#[test]
fn native_tatr_combined_span_keeps_every_source_fragment_once() {
    let result = recognized(1, Some([0.0, 20.0, 100.0, 60.0]));
    assert_eq!(result.table.cells[0], ["North", "Region"]);
    assert_eq!(result.table.cells[1], ["A B C D", ""]);
    assert_eq!(result.table.cells[2], ["", ""]);
    let grid = derived_grid(&result);
    let merged = grid.cells.iter().find(|cell| cell.content == "A B C D").unwrap();
    assert_eq!((merged.row, merged.col, merged.row_span, merged.col_span), (1, 0, 2, 2));
    assert_eq!(grid.cells.len(), 3);
}

#[test]
fn native_tatr_ordinary_cells_retain_source_order_and_text() {
    let result = recognized(0, None);
    assert_eq!(result.table.cells, [["North", "Region"], ["A", "B"], ["C", "D"]]);
    assert_eq!(result.consumed_word_ids, (0..6).collect());
}

mod html {
    use super::{recognized, recognized_document};
    use crate::rendering::{render_html, render_markdown};
    use crate::types::internal::NATIVE_TABLE_GRID_ATTRIBUTE;

    fn header_cells(html: &str) -> usize {
        html.matches("<th>").count() + html.matches("<th ").count()
    }

    #[test]
    fn native_tatr_html_preserves_both_detected_header_rows() {
        let doc = recognized_document(&recognized(2, None));
        let html = render_html(&doc);
        assert_eq!(header_cells(&html), 4, "{html}");
        assert_eq!(html.matches("<td").count(), 2, "{html}");
    }

    #[test]
    fn native_tatr_html_keeps_a_column_span_and_label_once() {
        let doc = recognized_document(&recognized(1, Some([0.0, 0.0, 100.0, 20.0])));
        let html = render_html(&doc);
        assert!(html.contains("colspan=\"2\""), "{html}");
        assert_eq!(header_cells(&html), 1, "{html}");
        assert_eq!(html.matches("<td").count(), 4, "{html}");
        assert_eq!(html.matches("North Region").count(), 1, "{html}");
    }

    #[test]
    fn native_tatr_html_keeps_a_row_span_and_fragments_once() {
        let doc = recognized_document(&recognized(0, Some([0.0, 20.0, 50.0, 60.0])));
        let html = render_html(&doc);
        assert!(html.contains("rowspan=\"2\""), "{html}");
        assert_eq!(header_cells(&html), 0, "{html}");
        assert_eq!(html.matches("<td").count(), 5, "{html}");
        assert_eq!(html.matches("A C").count(), 1, "{html}");
    }

    #[test]
    fn native_tatr_html_keeps_combined_spans_without_covered_cells() {
        let doc = recognized_document(&recognized(1, Some([0.0, 20.0, 100.0, 60.0])));
        let html = render_html(&doc);
        assert!(html.contains("rowspan=\"2\""), "{html}");
        assert!(html.contains("colspan=\"2\""), "{html}");
        assert_eq!(header_cells(&html), 2, "{html}");
        assert_eq!(html.matches("<td").count(), 1, "{html}");
        assert_eq!(html.matches("A B C D").count(), 1, "{html}");
    }

    #[test]
    fn native_tatr_html_does_not_guess_headers_for_data_rows() {
        let doc = recognized_document(&recognized(0, None));
        let html = render_html(&doc);
        assert_eq!(header_cells(&html), 0, "{html}");
        assert_eq!(html.matches("<td").count(), 6, "{html}");
    }

    #[test]
    fn native_tatr_geometry_does_not_change_legacy_markdown_bytes() {
        let mut doc = recognized_document(&recognized(2, Some([0.0, 0.0, 100.0, 20.0])));
        let with_geometry = render_markdown(&doc);
        doc.elements[0]
            .attributes
            .as_mut()
            .unwrap()
            .remove(NATIVE_TABLE_GRID_ATTRIBUTE);
        assert_eq!(render_markdown(&doc), with_geometry);
    }

    #[test]
    fn native_tatr_html_rejects_stale_geometry_after_cell_edits() {
        let mut doc = recognized_document(&recognized(0, Some([0.0, 20.0, 50.0, 60.0])));
        doc.tables[0].cells[1][0] = "edited".to_owned();
        let html = render_html(&doc);
        assert!(!html.contains("rowspan="), "{html}");
        assert_eq!(html.matches("edited").count(), 1, "{html}");
    }

    #[test]
    fn native_tatr_html_rejects_out_of_bounds_span_geometry() {
        let mut result = recognized(0, None);
        result.grid.as_mut().unwrap().cells[0].row_span = u32::MAX;
        let doc = recognized_document(&result);
        let html = render_html(&doc);
        assert!(!html.contains("rowspan="), "{html}");
        assert_eq!(html.matches("<td").count(), 6, "{html}");
    }

    #[test]
    fn native_tatr_html_escapes_source_text_in_a_spanning_cell() {
        let mut result = recognized(1, Some([0.0, 0.0, 100.0, 20.0]));
        result.table.cells[0][0] = "<script> & source".to_owned();
        result.table.columns.as_mut().unwrap()[0] = result.table.cells[0][0].clone();
        result.grid.as_mut().unwrap().cells[0].content = result.table.cells[0][0].clone();
        let html = render_html(&recognized_document(&result));
        assert!(!html.contains("<script>"), "{html}");
        assert!(html.contains("&lt;script&gt; &amp; source"), "{html}");
    }

    #[test]
    fn native_tatr_html_retains_math_and_script_annotations_in_a_spanning_cell() {
        use crate::types::{AnnotationKind, TextAnnotation};
        let mut result = recognized(1, Some([0.0, 0.0, 100.0, 20.0]));
        result.table.cells[0][0] = "x2 $y$".to_owned();
        let math = TextAnnotation {
            start: 3,
            end: 6,
            kind: AnnotationKind::math(false),
        };
        assert!(crate::rendering::common::valid_math_annotation(
            &result.table.cells[0][0],
            &math
        ));
        result.table.columns.as_mut().unwrap()[0] = result.table.cells[0][0].clone();
        result.grid.as_mut().unwrap().cells[0].content = result.table.cells[0][0].clone();
        let mut doc = recognized_document(&result);
        let attributes = doc.elements[0].attributes.as_mut().unwrap();
        let scripts = vec![crate::rendering::table_scripts::CellScriptAnnotation {
            row: 0,
            col: 0,
            annotation: TextAnnotation {
                start: 1,
                end: 2,
                kind: AnnotationKind::Superscript,
            },
        }];
        attributes.insert(
            crate::rendering::table_scripts::TABLE_CELL_SCRIPTS_ATTRIBUTE.to_owned(),
            serde_json::to_string(&scripts).unwrap(),
        );
        attributes.insert(
            crate::rendering::table_math::TABLE_CELL_MATH_ATTRIBUTE.to_owned(),
            serde_json::json!([{
                "row": 0, "col": 0,
                "annotation": math,
            }])
            .to_string(),
        );
        let html = render_html(&doc);
        assert!(html.contains("<sup>2</sup>"), "{html}");
        assert!(html.contains("data-math-style=\"inline\""), "{html}");
        assert!(html.contains("colspan=\"2\""), "{html}");
    }

    #[test]
    fn native_grid_html_keeps_a_missing_empty_trailing_cell() {
        let mut result = recognized(0, None);
        result.table.cells[2][1].clear();
        let removed = result.grid.as_mut().unwrap().cells.pop().unwrap();
        assert_eq!((removed.row, removed.col), (2, 1));
        let html = render_html(&recognized_document(&result));
        assert_eq!(html.matches("<td").count(), 6, "{html}");
        assert_eq!(html.matches("<td></td>").count(), 1, "{html}");
    }

    #[test]
    fn native_grid_html_rejects_overlapping_blank_cell_anchors() {
        let mut result = recognized(0, None);
        result.table.cells[0][1].clear();
        let grid = result.grid.as_mut().unwrap();
        grid.cells[0].col_span = 2;
        grid.cells[1].content.clear();
        let html = render_html(&recognized_document(&result));
        assert!(!html.contains("colspan="), "{html}");
        assert_eq!(html.matches("<td").count(), 6, "{html}");
    }
}
