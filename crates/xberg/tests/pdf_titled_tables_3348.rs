//! Regression tests for adjacent tables that start with their own title
//! (xberg-io/xberg#3348).
//!
//! Two ParseBench records carry the defect:
//!
//! - `0dc78d10374b01a67aee` (`FBLB-134215544_page88`) prints four ruled tables
//!   stacked on one page. Their rules line up, the ruled detector joins them,
//!   and each table's title survives only as a full-width row.
//! - `078d0b5457f6a070d206` (`SERFF_TX_random_pages_1_page810`) prints three
//!   borderless tables side by side, each opening with a centered title over
//!   its own block of six columns. The region clusterer sees one slab and the
//!   titles land in the merged header row.
//!
//! Both fixtures print the same shape at the pinned pages' own geometry. A
//! full-width row only starts a table when a fresh header band follows it, so
//! the fixtures below also pin the shapes that must stay one table: a spanning
//! subtotal inside a body, and a header band that repeats with no title above
//! it (what a page break produces).

#![cfg(feature = "pdf")]

mod helpers;

use flate2::read::GzDecoder;
use helpers::extract_bytes_document_blocking;
use std::io::Read;
use xberg::{ExtractedDocument, ExtractionConfig};

const STACKED: &[u8] = include_bytes!("fixtures/pdf_titled_tables_3348/stacked-titled.pdf.gz");
const SIDE_BY_SIDE: &[u8] = include_bytes!("fixtures/pdf_titled_tables_3348/side-by-side-titled.pdf.gz");
const SPANNING_BODY_ROW: &[u8] = include_bytes!("fixtures/pdf_titled_tables_3348/spanning-body-row.pdf.gz");
const REPEATED_HEADER: &[u8] = include_bytes!("fixtures/pdf_titled_tables_3348/repeated-header-band.pdf.gz");

fn extract(fixture: &[u8]) -> ExtractedDocument {
    let mut pdf = Vec::new();
    GzDecoder::new(fixture).read_to_end(&mut pdf).unwrap();
    let config = ExtractionConfig {
        disable_ocr: true,
        use_cache: false,
        enable_quality_processing: false,
        ..Default::default()
    };
    extract_bytes_document_blocking(&pdf, "application/pdf", &config).unwrap()
}

fn is_full_width_row(row: &[String]) -> bool {
    row.first().is_some_and(|cell| !cell.is_empty()) && row.iter().skip(1).all(|cell| cell.is_empty())
}

fn column(row: &[String], index: usize) -> &str {
    row.get(index).map_or("", String::as_str)
}

#[test]
fn stacked_titled_tables_split_into_four_tables() {
    let document = extract(STACKED);

    let titles = [
        "Alpha Construction Factors",
        "Beta Services Factor",
        "Gamma Products",
        "Delta Coverage Factors",
    ];
    let rows = [5, 5, 4, 4];
    assert_eq!(
        document.tables.len(),
        titles.len(),
        "the four source tables must stay separate, got {} table(s): {}",
        document.tables.len(),
        document.content
    );
    for (table, (title, expected_rows)) in document.tables.iter().zip(titles.iter().zip(rows)) {
        assert_eq!(table.cells.len(), expected_rows, "{title}: {}", table.markdown);
        let header = &table.cells[0];
        assert_eq!(column(header, 0), *title, "each table opens with its own title");
        assert!(
            is_full_width_row(header),
            "{title}: the title owns the row, got {header:?}"
        );
    }

    let alpha = &document.tables[0].cells;
    assert_eq!(
        alpha[1],
        [
            "Dwelling Construction Year",
            "Peril 1",
            "Peril 2",
            "Peril 3",
            "Peril 4",
            "Peril 5",
            "Peril 6",
            "Peril 7",
            "Peril 8",
        ]
    );
    assert_eq!(alpha[2][0], "1950 & Prior");
    assert_eq!(alpha[4][0], "1971-1980");

    // The second source table prints a subtitle between its title and header.
    let beta = &document.tables[1].cells;
    assert_eq!(beta[1][0], "DW, MH");
    assert!(
        is_full_width_row(&beta[1]),
        "the subtitle owns its row, got {:?}",
        beta[1]
    );
    assert_eq!(beta[2][0], "Financial Product");

    let gamma = &document.tables[2].cells;
    assert_eq!(gamma[2][0], "Condo");
    assert_eq!(gamma[2][1], "0.110");
    assert_eq!(gamma[2][8], "NA");

    let delta = &document.tables[3].cells;
    assert_eq!(delta[2][0], "HPP, HPPO");
    assert_eq!(delta[3][0], "MHPP, MHPPO");
}

#[test]
fn side_by_side_titled_tables_split_into_three_tables() {
    let document = extract(SIDE_BY_SIDE);

    let titles = ["Zips All Others", "Zips 750-753, 760-762", "Zips 770-775"];
    assert_eq!(
        document.tables.len(),
        titles.len(),
        "the three side-by-side tables must stay separate, got {} table(s): {}",
        document.tables.len(),
        document.content
    );
    let header = [
        "Attained Age",
        "Mode",
        "Plan F 9743F",
        "Attained Age",
        "Mode",
        "Plan F 9743F",
    ];
    for (table, title) in document.tables.iter().zip(titles) {
        assert_eq!(table.cells.len(), 11, "{title}: {}", table.markdown);
        assert_eq!(column(&table.cells[0], 0), title);
        assert!(is_full_width_row(&table.cells[0]));
        assert_eq!(table.cells[1], header, "{title}: {}", table.markdown);
        assert_eq!(column(&table.cells[2], 0), "65");
        assert_eq!(column(&table.cells[2], 1), "Annual");
    }

    assert_eq!(document.tables[0].cells[2][2], "$4000.00");
    assert_eq!(document.tables[1].cells[2][2], "$4700.00");
    assert_eq!(document.tables[2].cells[2][2], "$5400.00");
}

#[test]
fn a_spanning_row_inside_a_body_does_not_split_the_table() {
    let document = extract(SPANNING_BODY_ROW);

    let [table] = document.tables.as_slice() else {
        panic!(
            "a spanning body row must not create a table, got {}: {}",
            document.tables.len(),
            document.content
        );
    };
    assert_eq!(table.cells.len(), 7, "{}", table.markdown);
    assert_eq!(column(&table.cells[0], 0), "Factor Schedule");
    assert_eq!(column(&table.cells[4], 0), "Subtotal");
    assert!(is_full_width_row(&table.cells[4]));
}

#[test]
fn a_repeated_header_band_does_not_split_the_table() {
    let document = extract(REPEATED_HEADER);

    let [table] = document.tables.as_slice() else {
        panic!(
            "a repeated header band must not create a table, got {}: {}",
            document.tables.len(),
            document.content
        );
    };
    assert_eq!(table.cells.len(), 7, "{}", table.markdown);
    assert_eq!(column(&table.cells[0], 0), "Factor Schedule");
    assert_eq!(table.cells[1], ["Product", "One", "Two", "Three"]);
    assert_eq!(table.cells[4], ["Product", "One", "Two", "Three"]);
}
