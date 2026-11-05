#![cfg(feature = "pdf")]

mod helpers;

use helpers::extract_bytes_document_blocking;
use lopdf::{Document, Object, Stream, dictionary};
use xberg::core::config::{ExtractionConfig, OutputFormat, PdfBackend, PdfConfig};
use xberg_native_pdf::PdfDocument;
use xberg_native_pdf::structure::spatial_table_detector::TableDetectionConfig;

const PROSE: &str = "A separate paragraph identifies these records.";
const ROWS: usize = 6;
const COLS: usize = 3;
const TOP: usize = 680;
const CELL_HEIGHT: usize = 30;

fn expected_rows(row_count: usize) -> Vec<Vec<String>> {
    (0..row_count)
        .map(|row| (0..COLS).map(|col| format!("R{row}C{col}")).collect())
        .collect()
}

fn ruled_pdf(sections: &[(usize, usize)]) -> Vec<u8> {
    pdf_with_ruled_content(ruled_content(sections))
}

fn pdf_with_ruled_content(content: String) -> Vec<u8> {
    let mut pdf = Document::with_version("1.7");
    let pages = pdf.new_object_id();
    let font = pdf.add_object(dictionary! {
        "Type" => "Font", "Subtype" => "Type1", "BaseFont" => "Helvetica",
    });
    let stream = pdf.add_object(Stream::new(dictionary! {}, content.into_bytes()));
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
    for (row, cells) in expected_rows(row_count).iter().enumerate() {
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
    let expected = expected_rows(ROWS);
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
            rows: expected_rows(ROWS),
            vertical_bounds: (500.0, 680.0),
        }]
    );
}

#[test]
fn undersized_ruled_section_keeps_all_rows_in_native_and_core() {
    let sections = [(0, 3), (3, 4), (4, 7)];
    let rows = expected_rows(7);
    let expected = [rows[..3].to_vec(), rows[3..].to_vec()];
    let native: Vec<_> = capture_native_sections(&sections)
        .into_iter()
        .map(|section| section.rows)
        .collect();
    assert_eq!(native, expected);

    let config = ExtractionConfig {
        use_cache: false,
        disable_ocr: true,
        output_format: OutputFormat::Markdown,
        pdf_options: Some(PdfConfig {
            backend: PdfBackend::Native,
            extract_tables: true,
            ..Default::default()
        }),
        ..Default::default()
    };
    let result = extract_bytes_document_blocking(&ruled_pdf(&sections), "application/pdf", &config)
        .expect("extract the synthetic PDF through Core");
    let core: Vec<_> = result.tables.into_iter().map(|table| table.cells).collect();
    assert_eq!(core, expected);
    assert_eq!(result.content.matches(PROSE).count(), 1);
    for value in rows.iter().flatten() {
        assert_eq!(result.content.matches(value).count(), 1, "{value} in public output");
    }
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

fn stacked_rows(prefix: char) -> Vec<Vec<String>> {
    (0..3)
        .map(|row| (0..COLS).map(|column| format!("{prefix}{row}C{column}")).collect())
        .collect()
}

fn stacked_pdf(gap: f32) -> Vec<u8> {
    let mut content = format!("1 w 0 0 0 RG\nBT /F1 10 Tf 72 730 Td ({PROSE}) Tj ET\n");
    for (top, prefix) in [(TOP as f32, 'A'), ((TOP - 3 * CELL_HEIGHT) as f32 - gap, 'B')] {
        for row in 0..=3 {
            let y = top - (row * CELL_HEIGHT) as f32;
            content.push_str(&format!("60 {y} m 285 {y} l S\n"));
        }
        for column in 0..=COLS {
            let x = 60 + 75 * column;
            let bottom = top - (3 * CELL_HEIGHT) as f32;
            content.push_str(&format!("{x} {bottom} m {x} {top} l S\n"));
        }
        for (row, cells) in stacked_rows(prefix).iter().enumerate() {
            for (column, value) in cells.iter().enumerate() {
                let x = 68 + 75 * column;
                let y = top - ((row + 1) * CELL_HEIGHT - 11) as f32;
                content.push_str(&format!("BT /F1 10 Tf {x} {y} Td ({value}) Tj ET\n"));
            }
        }
    }
    pdf_with_ruled_content(content)
}

fn assert_stacked_rows(gap: f32, merged: bool) {
    let bytes = stacked_pdf(gap);
    let mut expected = vec![stacked_rows('A'), stacked_rows('B')];
    if merged {
        expected = vec![expected.into_iter().flatten().collect()];
    }

    let pdf = PdfDocument::from_bytes(bytes.clone()).expect("native opens the stacked grids");
    assert_eq!(pdf.page_count().unwrap(), 1);
    let native = pdf
        .extract_tables_with_config(0, TableDetectionConfig::strict())
        .expect("native detects ruled grids");
    let native_rows: Vec<Vec<Vec<&str>>> = native
        .iter()
        .map(|table| {
            table
                .rows
                .iter()
                .map(|row| row.cells.iter().map(|cell| cell.text.as_str()).collect())
                .collect()
        })
        .collect();

    let config = ExtractionConfig {
        use_cache: false,
        disable_ocr: true,
        output_format: OutputFormat::Markdown,
        pdf_options: Some(PdfConfig {
            backend: PdfBackend::Native,
            extract_tables: true,
            ..Default::default()
        }),
        ..Default::default()
    };
    let result =
        extract_bytes_document_blocking(&bytes, "application/pdf", &config).expect("Core extracts the stacked grids");
    let core_rows: Vec<_> = result.tables.iter().map(|table| &table.cells).collect();

    assert_eq!(
        native_rows, expected,
        "native row order with {gap}pt gap; Core: {core_rows:?}"
    );
    assert_eq!(
        core_rows,
        expected.iter().collect::<Vec<_>>(),
        "Core row order with {gap}pt gap"
    );
    assert_eq!(result.content.matches(PROSE).count(), 1);
    let mut end = 0;
    for cell in expected.iter().flatten().flatten() {
        let index = result.content.find(cell).expect("cell appears in public output");
        assert!(index >= end, "{cell} in public reading order");
        assert_eq!(result.content.matches(cell).count(), 1, "{cell} in public output");
        end = index + cell.len();
    }
    if merged {
        assert_merged_table_geometry(&native[0], &result.tables[0], gap);
    }
}

fn assert_merged_table_geometry(native: &xberg_native_pdf::structure::Table, core: &xberg::types::Table, gap: f32) {
    let bounds = native.bbox.expect("native ruled table has geometry");
    let core_bounds = core.bounding_box.as_ref().expect("Core retains table geometry");
    assert_eq!(
        (bounds.x, bounds.y, bounds.x + bounds.width, bounds.y + bounds.height),
        (60.0, 500.0 - gap, 285.0, 680.0)
    );
    assert_eq!(
        (core_bounds.x0, core_bounds.y0, core_bounds.x1, core_bounds.y1),
        (60.0, 500.0 - f64::from(gap), 285.0, 680.0)
    );
    assert_eq!(core.page_number, 1);
    assert_eq!(native.col_count, COLS);
    assert!(!native.has_header);
    for row in &native.rows {
        assert!(!row.is_header);
        for cell in &row.cells {
            assert_eq!((cell.colspan, cell.rowspan), (1, 1));
            assert!(!cell.is_header);
            assert!(cell.mcids.is_empty());
            assert_eq!(
                cell.spans.iter().map(|span| span.text.as_str()).collect::<String>(),
                cell.text
            );
            let cell_bounds = cell.bbox.expect("ruled cell has geometry");
            assert_eq!((cell_bounds.width, cell_bounds.height), (75.0, 30.0));
        }
    }
}

#[test]
fn stacked_ruled_grids_read_top_to_bottom_after_merging_at_three_and_a_half_points() {
    assert_stacked_rows(3.5, true);
}

#[test]
fn stacked_ruled_grids_read_top_to_bottom_after_merging_at_six_points() {
    assert_stacked_rows(6.0, true);
}

#[test]
fn stacked_ruled_grids_keep_two_tables_at_touching_and_narrow_gaps() {
    for gap in [0.0, 1.0, 2.0] {
        assert_stacked_rows(gap, false);
    }
}
