#![cfg(all(feature = "pdf", feature = "tokio-runtime"))]

mod helpers;

use sha2::{Digest, Sha256};
use xberg::core::config::{PdfBackend, PdfConfig};
use xberg::types::document_structure::NodeContent;
use xberg::{ExtractionConfig, OutputFormat};

#[test]
fn byte_index_cells_span_their_eight_source_bit_columns() {
    let bytes = source_pdf();
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
    assert_eq!(
        result.content.matches("Byte index").count(),
        1,
        "source-spanned row text must not also remain as body text: {}",
        result.content
    );
}

#[test]
fn native_ruled_grid_admission_honors_the_explicit_column_cap() {
    let document = xberg_native_pdf::PdfDocument::from_bytes(source_pdf()).unwrap();
    assert!(
        document
            .extract_tables_with_config(0, native_ruled_config(15))
            .unwrap()
            .is_empty()
    );
    let tables = document.extract_tables_with_config(0, native_ruled_config(18)).unwrap();
    let [table] = tables.as_slice() else {
        panic!("expected the single source-spanned wide table");
    };
    assert_eq!(table.col_count, 18);
    assert_eq!(
        table.rows.iter().map(|row| row.cells.len()).collect::<Vec<_>>(),
        [18, 4, 18]
    );
    let middle = table.rows[1]
        .cells
        .iter()
        .map(|cell| (cell.text.as_str(), cell.colspan))
        .collect::<Vec<_>>();
    assert_eq!(middle, [("Byte index", 1), ("0", 8), ("1", 8), (". . .", 1)]);
}

#[test]
fn native_wide_ruled_admission_declines_clipped_transparent_or_dashed_boundaries() {
    let neutral = wrapped_source_tables(b"q\n", false);
    assert_eq!(
        neutral.len(),
        1,
        "a neutral q/Q wrapper must still certify, or the declines below prove nothing"
    );
    for (control, prefix) in [
        ("clip", b"q 0 0 1 1 re W n\n".as_slice()),
        ("transparent", b"q /InvisibleStroke gs\n".as_slice()),
        ("dash", b"q [1 12] 0 d\n".as_slice()),
    ] {
        let tables = wrapped_source_tables(prefix, control == "transparent");
        assert!(tables.is_empty(), "{control} boundary unexpectedly certified a table");
    }
}

fn wrapped_source_tables(
    prefix: &[u8],
    transparent_stroke: bool,
) -> Vec<xberg_native_pdf::structure::table_extractor::Table> {
    use lopdf::{Document, dictionary};

    let mut pdf = Document::load_mem(&source_pdf()).unwrap();
    let page = pdf.get_pages()[&1];
    if transparent_stroke {
        let resources = pdf
            .get_object_mut(page)
            .unwrap()
            .as_dict_mut()
            .unwrap()
            .get_mut(b"Resources")
            .unwrap()
            .as_dict_mut()
            .unwrap();
        resources.set(
            "ExtGState",
            dictionary! { "InvisibleStroke" => dictionary! { "CA" => 0 } },
        );
    }
    let [stream] = <[_; 1]>::try_from(pdf.get_page_contents(page)).unwrap();
    let content = pdf.get_page_content(page);
    let wrapped = [prefix, content.as_slice(), b"\nQ\n"].concat();
    pdf.get_object_mut(stream)
        .unwrap()
        .as_stream_mut()
        .unwrap()
        .set_plain_content(wrapped);
    let mut bytes = Vec::new();
    pdf.save_to(&mut bytes).unwrap();

    let document = xberg_native_pdf::PdfDocument::from_bytes(bytes).unwrap();
    document.extract_tables_with_config(0, native_ruled_config(18)).unwrap()
}

fn native_ruled_config(
    max_table_columns: usize,
) -> xberg_native_pdf::structure::spatial_table_detector::TableDetectionConfig {
    use xberg_native_pdf::structure::spatial_table_detector::{TableDetectionConfig, TableStrategy};

    TableDetectionConfig {
        horizontal_strategy: TableStrategy::Lines,
        vertical_strategy: TableStrategy::Lines,
        column_tolerance: 3.0,
        row_tolerance: 2.0,
        regular_row_ratio: 0.5,
        column_merge_threshold: 12.0,
        v_split_gap: 4.0,
        text_fallback: false,
        max_table_columns,
        ..Default::default()
    }
}

fn source_pdf() -> Vec<u8> {
    let path = std::env::var_os("XBERG_NIST_INDEX_SOURCE")
        .map(std::path::PathBuf::from)
        .unwrap_or_else(|| helpers::get_test_file_path("pdf/regression_pages/nist-fips-197-upd1-p14.pdf"));
    let bytes = std::fs::read(&path).unwrap_or_else(|error| panic!("required source {}: {error}", path.display()));
    assert_eq!(
        hex::encode(Sha256::digest(&bytes)),
        "6963302f29e38d120c367f5d175f7f556efbfef7d029bf6f08c7653c1211f7dc"
    );
    bytes
}

#[test]
fn native_wide_ruled_admission_declines_translated_rules() {
    let identity = wrapped_source_tables(b"q 1 0 0 1 0 0 cm\n", false);
    assert_eq!(identity.len(), 1, "an identity cm must still certify");
    let moved = wrapped_source_tables(b"q 1 0 0 1 200 0 cm\n", false);
    assert!(moved.is_empty(), "a table with translated rules was certified");
}
