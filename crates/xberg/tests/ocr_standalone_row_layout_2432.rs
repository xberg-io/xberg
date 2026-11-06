#![cfg(all(feature = "pdf", feature = "ocr", feature = "tokio-runtime"))]

mod helpers;

use xberg::{ExtractionConfig, OcrConfig, OutputFormat, PageConfig};

const IMAGE: &[u8] = include_bytes!("fixtures/ocr_source_row_layout_2432/image-11-no-leaders.png");

fn config() -> ExtractionConfig {
    ExtractionConfig {
        use_cache: false,
        output_format: OutputFormat::Plain,
        ocr: Some(OcrConfig::default()),
        pages: Some(PageConfig {
            extract_pages: true,
            ..Default::default()
        }),
        ..Default::default()
    }
}

fn expected_rows() -> Vec<Vec<&'static str>> {
    let mut rows = include_str!("fixtures/ocr_source_row_layout_2432/image-11-source-rows.tsv")
        .lines()
        .map(|row| row.split('\t').collect::<Vec<_>>())
        .collect::<Vec<_>>();
    assert_eq!(rows.len(), 13);
    assert_eq!(rows[7], ["Storm drains", "86", "346,028", "(1,463)"]);
    rows[7][3] = "(1.463)";
    rows
}

#[test]
fn sparse_image_recovers_rows_without_correcting_the_recognized_amount() {
    let result = helpers::extract_bytes_document_blocking(IMAGE, "image/png", &config()).unwrap();
    assert!(result.metadata.ocr_used);
    let expected = expected_rows();
    assert!(result.content.contains("(1.463)"));
    let [table] = result.tables.as_slice() else {
        panic!("expected one certified standalone table, found {}", result.tables.len());
    };
    assert_eq!(table.cells, expected);
    assert_eq!(table.page_number, 1);
    let pages = result.pages.as_ref().expect("requested page output");
    assert_eq!(pages.len(), 1);
    let [page_table] = pages[0].tables.as_slice() else {
        panic!("expected one page table");
    };
    assert_eq!(page_table.cells, expected);
    let expected_text = expected.iter().map(|row| row.join(" ")).collect::<Vec<_>>().join("\n");
    assert_eq!(result.content.trim(), expected_text);
    assert_eq!(pages[0].content.trim(), expected_text);
}

#[test]
fn accepted_tinted_table_keeps_the_header_and_first_record_separate() {
    let image = include_bytes!("fixtures/ocr_source_row_layout_2432/image-20-tinted.png");
    let mut expected = include_str!("fixtures/ocr_source_row_layout_2432/image-20-tinted-source-rows.tsv")
        .lines()
        .map(|row| row.split('\t').collect::<Vec<_>>())
        .collect::<Vec<_>>();
    assert_eq!(expected.len(), 13);
    assert_eq!(expected[1], ["Signal timing", "59", "860,596", "(3,091)"]);
    expected[1][1] = "BY)";

    let result = helpers::extract_bytes_document_blocking(image, "image/png", &config()).unwrap();
    assert!(result.metadata.ocr_used);
    let [table] = result.tables.as_slice() else {
        panic!("expected one recognized standalone table");
    };
    assert_eq!(table.cells.len(), 13, "header and twelve source rows");
    assert_eq!(table.cells, expected);
    let [page] = result.pages.as_deref().expect("requested page output") else {
        panic!("expected one page");
    };
    let [page_table] = page.tables.as_slice() else {
        panic!("expected one page table");
    };
    assert_eq!(page_table.cells, expected);
    let expected_text = expected.iter().map(|row| row.join(" ")).collect::<Vec<_>>().join("\n");
    assert_eq!(result.content.trim(), expected_text);
    assert_eq!(page.content.trim(), expected_text);
}

#[test]
fn explicit_sparse_mode_and_disabled_tables_keep_standalone_rows_unassociated() {
    for mode in [Some(11), None] {
        let mut config = config();
        config.ocr.as_mut().unwrap().tesseract_config = Some(xberg::types::TesseractConfig {
            psm: mode,
            enable_table_detection: mode.is_some(),
            ..Default::default()
        });
        let result = helpers::extract_bytes_document_blocking(IMAGE, "image/png", &config).unwrap();
        assert!(result.metadata.ocr_used);
        assert!(result.content.contains("(1.463)"));
        assert!(result.tables.is_empty());
    }
}

#[test]
fn requested_document_structure_keeps_the_original_standalone_blocks() {
    let mut config = config();
    config.include_document_structure = true;
    let result = helpers::extract_bytes_document_blocking(IMAGE, "image/png", &config).unwrap();
    assert!(result.metadata.ocr_used);
    assert!(result.tables.is_empty());
    let document = result.document.as_ref().expect("requested document structure");
    assert!(document.nodes.len() > expected_rows().len());
    assert!(result.content.contains("(1.463)"));
}

#[test]
fn standalone_row_claim_keeps_the_public_ocr_elements() {
    use xberg::types::{OcrElementConfig, OcrElementLevel, TesseractConfig};

    let mut config = config();
    let ocr = config.ocr.as_mut().unwrap();
    ocr.element_config = Some(OcrElementConfig {
        include_elements: true,
        min_level: OcrElementLevel::Word,
        build_hierarchy: true,
        ..Default::default()
    });
    let selected = helpers::extract_bytes_document_blocking(IMAGE, "image/png", &config).unwrap();
    config.ocr.as_mut().unwrap().tesseract_config = Some(TesseractConfig {
        psm: Some(11),
        ..Default::default()
    });
    let ordinary = helpers::extract_bytes_document_blocking(IMAGE, "image/png", &config).unwrap();
    assert_eq!(selected.tables.len(), 1);
    assert!(ordinary.tables.is_empty());
    let elements = selected.ocr_elements.expect("requested OCR elements");
    assert_eq!(
        elements
            .iter()
            .filter(|element| element.level == OcrElementLevel::Word)
            .count(),
        65
    );
    assert_eq!(
        serde_json::to_value(elements).unwrap(),
        serde_json::to_value(ordinary.ocr_elements.expect("explicit sparse elements")).unwrap()
    );
}
