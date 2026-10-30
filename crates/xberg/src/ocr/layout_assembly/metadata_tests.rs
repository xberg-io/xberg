use super::*;
use crate::layout::models::tatr::{TatrClass, TatrDetection, TatrResult, build_cell_grid_with_structure};
use crate::types::{OcrBoundingGeometry, OcrConfidence};

pub(crate) struct OcrTableFixture {
    pub(crate) outcome: RecognizedTablesOutcome,
    pub(crate) elements: Vec<OcrElement>,
}

fn detection(class_name: TatrClass, bbox: [f32; 4]) -> TatrDetection {
    TatrDetection {
        bbox,
        confidence: 0.99,
        class_name,
    }
}

pub(crate) fn recognized_fixture(header_rows: usize, span: Option<[f32; 4]>) -> OcrTableFixture {
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
    let (grid, structure) = build_cell_grid_with_structure(&result, result.table_bbox);
    let elements = ["North", "Region", "A", "B", "C", "D"]
        .into_iter()
        .enumerate()
        .map(|(index, text)| {
            OcrElement::new(
                text,
                OcrBoundingGeometry::Rectangle {
                    left: 20 + (index as u32 % 2) * 50,
                    top: 25 + (index as u32 / 2) * 20,
                    width: 20,
                    height: 8,
                },
                OcrConfidence::from_tesseract(99.0),
            )
            .with_level(OcrElementLevel::Word)
        })
        .collect::<Vec<_>>();
    let references = elements.iter().collect::<Vec<_>>();
    let TableRegionOutcome::Recognized(cells, markdown, grid) =
        table_region_outcome(&grid, &references, 10.0, 20.0, &structure)
    else {
        panic!("populated real post-inference OCR table must be recognized");
    };
    OcrTableFixture {
        outcome: RecognizedTablesOutcome {
            tables: vec![RecognizedTable {
                detection_bbox: BBox::new(10.0, 20.0, 110.0, 80.0),
                cells,
                markdown,
            }],
            grids: vec![grid],
            unrecognized_table_text: vec![],
        },
        elements,
    }
}
