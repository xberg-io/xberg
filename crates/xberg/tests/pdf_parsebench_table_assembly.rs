//! Native table assembly on ParseBench pages from the bucket-fetched corpus. Each test prints a
//! SKIP line and returns when its page is not present.

#![cfg(feature = "pdf")]
#![allow(clippy::print_stdout, clippy::print_stderr, clippy::dbg_macro)] // ~keep: test/bench binaries print by design; org logging policy exempts tests

mod helpers;

use helpers::{extract_bytes_document_blocking, get_test_file_path, require_test_fixtures};
use xberg::{ExtractedDocument, ExtractionConfig, OutputFormat};

fn extract(relative: &str) -> Option<ExtractedDocument> {
    let path = get_test_file_path(relative);
    let pdf = match std::fs::read(&path) {
        Ok(pdf) => pdf,
        Err(error) if require_test_fixtures() => panic!("required fixture {} unavailable: {error}", path.display()),
        Err(error) => {
            eprintln!(
                "SKIP: fixture {} not available ({error}); run `python3 test_documents/scripts/fetch_corpus.py`",
                path.display()
            );
            return None;
        }
    };
    let config = ExtractionConfig {
        disable_ocr: true,
        use_cache: false,
        output_format: OutputFormat::Markdown,
        ..Default::default()
    };
    Some(extract_bytes_document_blocking(&pdf, "application/pdf", &config).expect("fixture page must extract"))
}

fn row_index(cells: &[Vec<String>], starts_with: &str) -> usize {
    cells
        .iter()
        .position(|row| row.first().is_some_and(|cell| cell.starts_with(starts_with)))
        .unwrap_or_else(|| panic!("no row starts with {starts_with:?}: {cells:?}"))
}

/// `pb_sample_page_16_page1`: a header band whose cells stack one to three centred label lines.
#[test]
fn stacked_header_band_is_one_header_row() {
    let Some(document) = extract(".corpus-cache/pdf/pb_sample_page_16_page1.pdf") else {
        return;
    };
    let [table] = document.tables.as_slice() else {
        panic!(
            "expected one table, got {}: {}",
            document.tables.len(),
            document.content
        );
    };

    assert_eq!(
        table.cells[0],
        [
            "UW Tier Group",
            "Vehicle Type Group",
            "Minimum Vehicle Miles",
            "Maximum Vehicle Miles",
            "BI",
            "PD",
            "CSL",
            "PIP",
            "UM BI",
            "UM PD",
            "UM CSL",
            "UIMBI",
            "UIMCSL",
            "COMPO",
            "COMPA",
            "COMPG",
            "COMPC",
            "COLL",
            "Comp Auto Loan /Lease",
            "Coll Auto Loan /Lease",
            "OLTE",
            "Excess Custom Equipment",
            "Excess Electronic Equipment",
            "Roadside Assistance",
        ]
    );
    assert_eq!(&table.cells[1][..4], ["002", "AO", "1000", "1999"]);
}

/// `pb_SERFF_TX_random_pages_1_page1075`: two unruled multiplier columns beside a ruled rate grid.
#[test]
fn unruled_multiplier_columns_join_the_ruled_rate_table() {
    let Some(document) = extract(".corpus-cache/pdf/pb_SERFF_TX_random_pages_1_page1075.pdf") else {
        return;
    };
    let table = document
        .tables
        .iter()
        .find(|table| {
            table
                .cells
                .iter()
                .any(|row| row.first().is_some_and(|cell| cell == "18-29"))
        })
        .unwrap_or_else(|| panic!("no rate table: {}", document.content));

    assert!(
        table
            .cells
            .iter()
            .any(|row| row.iter().any(|cell| cell == "2-10yr BP") && row.iter().any(|cell| cell == "LifeBP")),
        "multiplier header missing: {:?}",
        &table.cells[..3]
    );
    let first = &table.cells[row_index(&table.cells, "18-29")];
    assert_eq!(&first[first.len() - 3..], ["57.00", "1.44", "1.00"]);
    let age_91 = &table.cells[row_index(&table.cells, "91+")];
    assert_eq!(&age_91[age_91.len() - 2..], ["1.08", "1.00"]);
    let table_start = document.content.find("18-29").expect("rate rows rendered");
    assert!(
        !document.content[..table_start].contains("1.44"),
        "multipliers must not precede the table: {}",
        &document.content[..table_start]
    );
}

/// `pb_fqr-retail-blackrock-global-allocation-fund-inc_page8`: a holdings table whose only row
/// with a digit in every cell is a label continuation deep in the body.
#[test]
fn holdings_rows_keep_source_order_around_a_label_with_digits() {
    let Some(document) = extract(".corpus-cache/pdf/pb_fqr-retail-blackrock-global-allocation-fund-inc_page8.pdf")
    else {
        return;
    };
    let table = document
        .tables
        .iter()
        .find(|table| table.cells.iter().flatten().any(|cell| cell.starts_with("NXP")))
        .unwrap_or_else(|| panic!("no holdings table with NXP: {}", document.content));

    assert!(
        table.cells[0]
            .iter()
            .all(|cell| !cell.contains("NXP") && !cell.contains("Teya")),
        "body rows lifted into the header: {:?}",
        table.cells[0]
    );
    for holdings in &document.tables {
        assert_eq!(
            holdings.cells[0],
            ["Security", "Shares", "Value"],
            "{:?}",
            &holdings.cells[..2]
        );
    }
    let vopak = row_index(&table.cells, "Koninklijke Vopak");
    let nxp = row_index(&table.cells, "NXP");
    let teya = row_index(&table.cells, "Teya");
    let continuation = row_index(&table.cells, "06/03/24");
    assert!(
        vopak < nxp && nxp < teya && teya < continuation,
        "rows out of order: Vopak {vopak}, NXP {nxp}, Teya {teya}, 06/03/24 {continuation}"
    );
}
