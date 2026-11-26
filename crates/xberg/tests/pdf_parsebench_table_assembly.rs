//! Native table assembly on ParseBench pages from the bucket-fetched corpus. Each test prints a
//! SKIP line and returns when its page is not present.

#![cfg(feature = "pdf")]

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
    let vopak = row_index(&table.cells, "Koninklijke Vopak");
    let nxp = row_index(&table.cells, "NXP");
    let teya = row_index(&table.cells, "Teya");
    let continuation = row_index(&table.cells, "06/03/24");
    assert!(
        vopak < nxp && nxp < teya && teya < continuation,
        "rows out of order: Vopak {vopak}, NXP {nxp}, Teya {teya}, 06/03/24 {continuation}"
    );
}
