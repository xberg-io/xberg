#![cfg(feature = "pdf")]

mod helpers;

use helpers::extract_bytes_document_blocking;
use lopdf::{Document, Object, Stream, dictionary};
use xberg::core::config::{ExtractionConfig, OutputFormat, PdfBackend, PdfConfig};
use xberg::types::Table;
use xberg_native_pdf::structure::spatial_table_detector::{TableDetectionConfig, TableStrategy};

const ROWS: usize = 5;
const CELL_WIDTH: usize = 100;
const CELL_HEIGHT: usize = 30;
const TOP: usize = 680;
const LOWER: usize = 300;

#[derive(Clone, Copy)]
struct Grid {
    prefix: &'static str,
    columns: usize,
    top: usize,
}

const STRICT: Grid = Grid {
    prefix: "STR",
    columns: 3,
    top: TOP,
};
const BORDERED: Grid = Grid {
    prefix: "BDR",
    columns: 2,
    top: LOWER,
};
const SECOND_STRICT: Grid = Grid {
    prefix: "ALT",
    columns: 3,
    top: LOWER,
};

fn expected_cells(grid: Grid) -> Vec<Vec<String>> {
    (0..ROWS)
        .map(|row| {
            (0..grid.columns)
                .map(|col| format!("{}{row}{col}", grid.prefix))
                .collect()
        })
        .collect()
}

fn append_grid(content: &mut String, grid: Grid, header: Option<&[&str]>) {
    let left = 60;
    let right = left + grid.columns * CELL_WIDTH;
    let bottom = grid.top - ROWS * CELL_HEIGHT;
    for row in 0..=ROWS {
        let y = grid.top - row * CELL_HEIGHT;
        let (start, end) = if header.is_none() || row == ROWS {
            (left, right)
        } else {
            // Non-touching outer corners route to the existing clustered-grid detector.
            let end_gap = if row > 1 {
                CELL_WIDTH * 3 / 4
            } else {
                4 * usize::from(grid.columns > 2)
            };
            (left + 4, right - end_gap)
        };
        content.push_str(&format!("{start} {y} m {end} {y} l S\n"));
    }
    for col in 0..=grid.columns {
        let x = left + col * CELL_WIDTH;
        content.push_str(&format!("{x} {bottom} m {x} {} l S\n", grid.top));
    }
    for (row, cells) in expected_cells_with_header(grid, header).iter().enumerate() {
        for (col, text) in cells.iter().enumerate() {
            let x = left + col * CELL_WIDTH + 8;
            let y = grid.top - (row + 1) * CELL_HEIGHT + 11;
            content.push_str(&format!("BT /F1 10 Tf 1 0 0 1 {x} {y} Tm ({text}) Tj ET\n"));
        }
    }
}

fn expected_cells_with_header(grid: Grid, header: Option<&[&str]>) -> Vec<Vec<String>> {
    let mut cells = expected_cells(grid);
    if let Some(labels) = header {
        cells[0] = labels.iter().map(|label| (*label).to_owned()).collect();
    }
    cells
}

fn ruled_pdf(grids: &[Grid]) -> Vec<u8> {
    ruled_pdf_with_header(grids, None)
}

fn ruled_pdf_with_header(grids: &[Grid], header: Option<&[&str]>) -> Vec<u8> {
    let mut pdf = Document::with_version("1.7");
    let pages = pdf.new_object_id();
    let font = pdf.add_object(dictionary! {
        "Type" => "Font", "Subtype" => "Type1", "BaseFont" => "Helvetica",
    });
    let mut content = String::from("1 w 0 0 0 RG\n");
    for grid in grids {
        append_grid(&mut content, *grid, header);
    }
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
    pdf.save_to(&mut bytes).expect("synthetic ruled PDF serializes");
    bytes
}

fn bordered_config() -> TableDetectionConfig {
    // ~keep Mirror the production bordered tier, not the text-based relaxed() preset.
    TableDetectionConfig {
        enabled: true,
        horizontal_strategy: TableStrategy::Lines,
        vertical_strategy: TableStrategy::Lines,
        column_tolerance: 3.0,
        row_tolerance: 2.0,
        min_table_cells: 4,
        min_table_columns: 2,
        regular_row_ratio: 0.5,
        max_table_columns: 15,
        column_merge_threshold: 12.0,
        v_split_gap: 4.0,
        text_fallback: false,
    }
}

fn detected_cells(bytes: &[u8], config: TableDetectionConfig) -> Vec<Vec<Vec<String>>> {
    let pdf = xberg_native_pdf::PdfDocument::from_bytes(bytes.to_vec()).expect("native opens ruled PDF");
    assert_eq!(pdf.page_count().expect("native page count"), 1);
    let mut cells: Vec<_> = pdf
        .extract_tables_with_config(0, config)
        .expect("direct tier detects")
        .into_iter()
        .map(|table| {
            table
                .rows
                .into_iter()
                .map(|row| row.cells.into_iter().map(|cell| cell.text).collect())
                .collect()
        })
        .collect();
    cells.sort();
    cells
}

#[test]
fn should_qualify_exact_strict_and_bordered_fixture_tiers() {
    let cases = [
        vec![STRICT],
        vec![BORDERED],
        vec![STRICT, BORDERED],
        vec![STRICT, SECOND_STRICT],
    ];
    assert_eq!(cases.len(), 4, "fixture qualification must cover four cases");
    let mut executed_cases = 0;
    let mut executed_calls = 0;
    for grids in cases {
        let bytes = ruled_pdf(&grids);
        let mut strict: Vec<_> = grids
            .iter()
            .filter(|grid| grid.columns == 3)
            .map(|grid| expected_cells(*grid))
            .collect();
        let mut bordered: Vec<_> = grids.iter().map(|grid| expected_cells(*grid)).collect();
        strict.sort();
        bordered.sort();
        assert_eq!(detected_cells(&bytes, TableDetectionConfig::strict()), strict);
        executed_calls += 1;
        assert_eq!(detected_cells(&bytes, bordered_config()), bordered);
        executed_calls += 1;
        executed_cases += 1;
    }
    assert_eq!(executed_cases, 4, "all four fixture cases must execute");
    assert_eq!(executed_calls, 8, "all eight direct detector calls must execute");
}

fn assert_public_tables(grids: &[Grid]) {
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
    let result = extract_bytes_document_blocking(&ruled_pdf(grids), "application/pdf", &config)
        .expect("public extraction succeeds");
    assert_eq!(
        result.tables.len(),
        grids.len(),
        "no table may disappear, merge or duplicate"
    );
    for grid in grids {
        let expected = expected_cells(*grid);
        let matches: Vec<&Table> = result.tables.iter().filter(|table| table.cells == expected).collect();
        assert_eq!(matches.len(), 1, "exact {} cell matrix must occur once", grid.prefix);
        let table = matches[0];
        assert_eq!(table.page_number, 1);
        assert_eq!(table.columns, None, "unmarked ruled rows do not declare column labels");
        assert_eq!(table.cells.len(), ROWS);
        assert_eq!(
            table.cells.iter().map(Vec::len).collect::<Vec<_>>(),
            vec![grid.columns; ROWS]
        );
        let bbox = table.bounding_box.as_ref().expect("ruled table has geometry");
        assert!(bbox.x0.is_finite() && bbox.y0.is_finite() && bbox.x1.is_finite() && bbox.y1.is_finite());
        assert!(bbox.x0 < bbox.x1 && bbox.y0 < bbox.y1);
        for text in expected.iter().flatten() {
            assert_eq!(
                table.markdown.matches(text).count(),
                1,
                "cell appears once in table markdown"
            );
            assert_eq!(
                result.content.matches(text).count(),
                1,
                "cell appears once in public markdown"
            );
        }
    }
    if result.tables.len() == 2 {
        let a = result.tables[0].bounding_box.as_ref().expect("first geometry");
        let b = result.tables[1].bounding_box.as_ref().expect("second geometry");
        assert!(a.y1 < b.y0 || b.y1 < a.y0, "distinct regions must remain separate");
    }
}

#[test]
fn should_preserve_disjoint_strict_and_bordered_tables_on_one_page() {
    assert_public_tables(&[STRICT, BORDERED]);
}

#[test]
fn should_emit_strict_table_once_when_both_tiers_detect_it() {
    assert_public_tables(&[STRICT]);
}

#[test]
fn should_preserve_bordered_table_without_strict_detection() {
    assert_public_tables(&[BORDERED]);
}

#[test]
fn should_keep_two_separate_strict_regions_without_duplicate_relaxed_detections() {
    assert_public_tables(&[STRICT, SECOND_STRICT]);
}

fn assert_public_columns_preserve_native_header(grid: Grid, detector: TableDetectionConfig) {
    let header = Some(&["Item", "Count", "Status"][..grid.columns]);
    let bytes = ruled_pdf_with_header(&[grid], header);
    let native = xberg_native_pdf::PdfDocument::from_bytes(bytes.clone()).expect("native opens ruled PDF");
    let source = native
        .extract_tables_with_config(0, detector)
        .expect("native table detection");
    assert_eq!(source.len(), 1);
    let expected = expected_cells_with_header(grid, header);
    assert_eq!(
        source[0].rows[0]
            .cells
            .iter()
            .map(|cell| cell.text.as_str())
            .collect::<Vec<_>>(),
        expected[0]
    );
    let source_roles: Vec<_> = source[0]
        .rows
        .iter()
        .enumerate()
        .flat_map(|(row, source)| {
            source
                .cells
                .iter()
                .enumerate()
                .map(move |(column, cell)| (row as u32, column as u32, source.is_header || cell.is_header))
        })
        .collect();
    assert_eq!(
        source_roles.iter().filter(|(_, _, is_header)| *is_header).count(),
        grid.columns,
        "fixture must have exactly one complete header row in the actual producer"
    );

    let cases = [
        (OutputFormat::Html, true),
        (OutputFormat::Markdown, true),
        (OutputFormat::Plain, false),
    ];
    for (format, structure) in cases {
        let config = ExtractionConfig {
            use_cache: false,
            disable_ocr: true,
            output_format: format.clone(),
            include_document_structure: structure,
            pdf_options: Some(PdfConfig {
                backend: PdfBackend::Native,
                extract_tables: true,
                ..Default::default()
            }),
            ..Default::default()
        };
        let result = extract_bytes_document_blocking(&bytes, "application/pdf", &config).expect("public extraction");
        assert_eq!(result.tables.len(), 1, "{format:?}");
        assert_eq!(result.tables[0].cells, expected, "{format:?}");
        assert_eq!(
            result.tables[0].columns.as_ref(),
            expected.first(),
            "{format:?}: complete native header must also label the public table columns"
        );
        let output = result.formatted_content.as_deref().unwrap_or(&result.content);
        for value in expected.iter().flatten() {
            assert_eq!(output.matches(value).count(), 1, "{format:?}: {value} must occur once");
        }
        if structure {
            assert_public_grid_roles(&result.document.expect("public structured extraction"), &source_roles);
        } else {
            assert!(result.document.is_none());
        }
    }
}

fn assert_public_grid_roles(document: &xberg::types::DocumentStructure, source_roles: &[(u32, u32, bool)]) {
    let grids: Vec<_> = document
        .nodes
        .iter()
        .filter_map(|node| match &node.content {
            xberg::types::NodeContent::Table { grid } => Some(grid),
            _ => None,
        })
        .collect();
    assert_eq!(grids.len(), 1);
    assert_eq!(
        grids[0]
            .cells
            .iter()
            .map(|cell| (cell.row, cell.col, cell.is_header))
            .collect::<Vec<_>>(),
        source_roles,
        "public grid roles must exactly match the native producer"
    );
}

#[test]
fn should_expose_strict_ruled_table_column_labels_alongside_grid_roles() {
    assert_public_columns_preserve_native_header(STRICT, TableDetectionConfig::strict());
}

#[test]
fn should_expose_bordered_ruled_table_column_labels_alongside_grid_roles() {
    assert_public_columns_preserve_native_header(BORDERED, bordered_config());
}
