#![cfg(feature = "pdf")]

use lopdf::{Document, Object, Stream, dictionary};
use xberg_native_pdf::PdfDocument;
use xberg_native_pdf::structure::spatial_table_detector::TableDetectionConfig;

const PROSE: &str = "A separate paragraph identifies these records.";
const ROWS: usize = 6;
const COLS: usize = 3;
const TOP: usize = 680;
const CELL_HEIGHT: usize = 30;

fn expected_rows() -> Vec<Vec<String>> {
    (0..ROWS)
        .map(|row| (0..COLS).map(|col| format!("R{row}C{col}")).collect())
        .collect()
}

fn ruled_pdf(sections: &[(usize, usize)]) -> Vec<u8> {
    let mut pdf = Document::with_version("1.7");
    let pages = pdf.new_object_id();
    let font = pdf.add_object(dictionary! {
        "Type" => "Font", "Subtype" => "Type1", "BaseFont" => "Helvetica",
    });
    let stream = pdf.add_object(Stream::new(dictionary! {}, ruled_content(sections).into_bytes()));
    let page = pdf.add_object(dictionary! {
        "Type" => "Page", "Parent" => pages, "Contents" => stream,
        "MediaBox" => vec![0.into(), 0.into(), 612.into(), 792.into()],
        "Resources" => dictionary! { "Font" => dictionary! { "F1" => font } },
    });
    pdf.objects.insert(
        pages,
        Object::Dictionary(dictionary! {
            "Type" => "Pages", "Kids" => vec![page.into()], "Count" => 1,
        }),
    );
    let catalog = pdf.add_object(dictionary! { "Type" => "Catalog", "Pages" => pages });
    pdf.trailer.set("Root", catalog);
    let mut bytes = Vec::new();
    pdf.save_to(&mut bytes).expect("serialize the ruled PDF");
    bytes
}

fn ruled_content(sections: &[(usize, usize)]) -> String {
    let mut content = format!("1 w 0 0 0 RG\nBT /F1 10 Tf 72 730 Td ({PROSE}) Tj ET\n");
    let left = 60;
    let cell_width = 75;
    let right = left + COLS * cell_width;
    let row_count = sections.last().unwrap().1;
    for row in 0..=row_count {
        let y = TOP - row * CELL_HEIGHT;
        content.push_str(&format!("{left} {y} m {right} {y} l S\n"));
    }
    for &(start, end) in sections {
        for column in 0..=COLS {
            let x = left + column * cell_width;
            let bottom = TOP - end * CELL_HEIGHT;
            let upper = TOP - start * CELL_HEIGHT;
            content.push_str(&format!("{x} {bottom} m {x} {upper} l S\n"));
        }
    }
    for (row, cells) in expected_rows().iter().take(row_count).enumerate() {
        for (column, value) in cells.iter().enumerate() {
            let x = left + column * cell_width + 8;
            let y = TOP - (row + 1) * CELL_HEIGHT + 11;
            content.push_str(&format!("BT /F1 10 Tf {x} {y} Td ({value}) Tj ET\n"));
        }
    }
    content
}

#[derive(Debug, PartialEq)]
struct CapturedSection {
    rows: Vec<Vec<String>>,
    vertical_bounds: (f32, f32),
}

#[test]
fn should_place_each_admitted_row_on_its_side_of_the_pdf_section_rule() {
    let actual = capture_native_sections(&[(0, 3), (3, 6)]);
    let expected = expected_rows();
    assert_eq!(
        actual.iter().flat_map(|section| &section.rows).collect::<Vec<_>>(),
        expected.iter().collect::<Vec<_>>()
    );
    assert_eq!(
        actual,
        [
            CapturedSection {
                rows: expected[..3].to_vec(),
                vertical_bounds: (590.0, 680.0),
            },
            CapturedSection {
                rows: expected[3..].to_vec(),
                vertical_bounds: (500.0, 590.0),
            },
        ],
        "the y=590 painted rule separates the two three-row sections"
    );
}

#[test]
fn should_keep_the_same_six_rows_in_one_grid_when_vertical_rules_cross() {
    assert_eq!(
        capture_native_sections(&[(0, 6)]),
        [CapturedSection {
            rows: expected_rows(),
            vertical_bounds: (500.0, 680.0),
        }]
    );
}

fn capture_native_sections(sections: &[(usize, usize)]) -> Vec<CapturedSection> {
    let pdf = PdfDocument::from_bytes(ruled_pdf(sections)).expect("native opens the ruled PDF");
    assert_eq!(pdf.page_count().unwrap(), 1);
    let tables = pdf
        .extract_tables_with_config(0, TableDetectionConfig::strict())
        .expect("actual public native table detection");
    tables
        .into_iter()
        .map(|table| {
            let bbox = table.bbox.expect("section has ruled geometry");
            let rows = table
                .rows
                .into_iter()
                .map(|row| row.cells.into_iter().map(|cell| cell.text).collect())
                .collect();
            CapturedSection {
                rows,
                vertical_bounds: (bbox.y, bbox.y + bbox.height),
            }
        })
        .collect()
}
