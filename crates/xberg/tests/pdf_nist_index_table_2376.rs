#![cfg(all(feature = "pdf", feature = "tokio-runtime"))]

mod helpers;

use sha2::{Digest, Sha256};
use xberg::core::config::{PdfBackend, PdfConfig};
use xberg::types::document_structure::NodeContent;
use xberg::{ExtractionConfig, OutputFormat};

#[test]
fn byte_index_cells_span_their_eight_source_bit_columns() {
    let path = std::env::var_os("XBERG_NIST_INDEX_SOURCE")
        .map(std::path::PathBuf::from)
        .unwrap_or_else(|| helpers::get_test_file_path("pdf/regression_pages/nist-fips-197-upd1-p14.pdf"));
    let bytes = std::fs::read(&path).unwrap_or_else(|error| panic!("required source {}: {error}", path.display()));
    assert_eq!(
        hex::encode(Sha256::digest(&bytes)),
        "6963302f29e38d120c367f5d175f7f556efbfef7d029bf6f08c7653c1211f7dc"
    );
    let config = ExtractionConfig {
        disable_ocr: true,
        use_cache: false,
        include_document_structure: true,
        output_format: OutputFormat::Html,
        pdf_options: Some(PdfConfig {
            backend: PdfBackend::Native,
            extract_tables: true,
            ..Default::default()
        }),
        ..Default::default()
    };
    let result = helpers::extract_bytes_document_blocking(&bytes, "application/pdf", &config).unwrap();
    assert_eq!(result.tables.len(), 1);
    let document = result.document.as_ref().expect("requested public structure");
    let grids: Vec<_> = document
        .nodes
        .iter()
        .filter_map(|node| match &node.content {
            NodeContent::Table { grid } => Some(grid),
            _ => None,
        })
        .collect();
    let [grid] = grids.as_slice() else {
        panic!("expected a single source index table");
    };
    assert_eq!((grid.rows, grid.cols), (3, 18));
    let byte_index = grid
        .cells
        .iter()
        .filter(|cell| cell.row == 1 && !cell.content.trim().is_empty())
        .map(|cell| (cell.col, cell.col_span, cell.content.as_str()))
        .collect::<Vec<_>>();
    assert_eq!(
        byte_index,
        [(0, 1, "Byte index"), (1, 8, "0"), (9, 8, "1"), (17, 1, ". . .")]
    );
    assert!(result.content.contains("colspan=\"8\">0</"));
    assert!(result.content.contains("colspan=\"8\">1</"));
}
