#![cfg(all(feature = "pdf", feature = "ocr", feature = "paddle-ocr-ort"))]

use sha2::{Digest, Sha256};
use xberg::{BoundingBox, ExtractInput, ExtractedDocument, ExtractionConfig};

const HEADING: &str = "Cedar Company Ledger Summary For The March Quarter Report 2026";

async fn extract(pdf: &[u8]) -> ExtractedDocument {
    let config = ExtractionConfig {
        force_ocr: true,
        use_cache: false,
        include_document_structure: true,
        pages: Some(xberg::core::config::PageConfig {
            extract_pages: true,
            ..Default::default()
        }),
        ..Default::default()
    };
    let output = xberg::extract(ExtractInput::from_bytes(pdf.to_vec(), "application/pdf", None), &config)
        .await
        .expect("extract the pinned raster PDF");
    assert!(output.errors.is_empty(), "{:?}", output.errors);
    let [document] = <[_; 1]>::try_from(output.results).unwrap();
    assert!(document.metadata.ocr_used);
    document
}

fn coordinates(bbox: BoundingBox) -> [f64; 4] {
    [bbox.x0, bbox.y0, bbox.x1, bbox.y1]
}

fn matches_points(actual: [f64; 4], expected: [f64; 4]) -> bool {
    actual
        .into_iter()
        .zip(expected)
        .all(|(actual, expected)| (actual - expected).abs() < 0.01)
}

fn assert_exact_payload(document: &ExtractedDocument, expected: &[Vec<String>]) {
    let [table] = document.tables.as_slice() else {
        panic!("one owned table")
    };
    let [page] = document.pages.as_deref().unwrap() else {
        panic!("one extracted page")
    };
    let [page_table] = page.tables.as_slice() else {
        panic!("one table on the extracted page")
    };
    assert_eq!(table.page_number, 1);
    assert_eq!(table.cells.as_slice(), expected);
    assert_eq!(page_table.cells.as_slice(), expected);
    assert_eq!(page_table.bounding_box, table.bounding_box);
    assert_eq!(page.content.trim(), format!("{HEADING}\n\n{}", table.markdown.trim()));
    let words = HEADING
        .split_whitespace()
        .chain(expected.iter().flatten().flat_map(|cell| cell.split_whitespace()))
        .collect::<Vec<_>>();
    assert_eq!(words.len(), 470);
    assert_eq!(document.content.split_whitespace().collect::<Vec<_>>(), words);
}

#[tokio::test]
async fn detached_heading_and_table_share_the_pdf_page_frame() {
    let pdf = include_bytes!("fixtures/ocr/detached_heading_frame_2497/heading-table-0.pdf");
    assert_eq!(
        hex::encode(Sha256::digest(pdf)),
        "0007e630efa28f59639975016a8a2560729af47a696f137920557d8bed16344f"
    );
    let expected: Vec<Vec<String>> =
        serde_json::from_str(include_str!("fixtures/ocr/detached_heading_frame_2497/cells.json")).unwrap();
    assert_eq!(expected.len(), 46);
    assert!(expected.iter().all(|row| row.len() == 8));
    let document = extract(pdf).await;
    assert_exact_payload(&document, &expected);
    let preprocess = document
        .metadata
        .image_preprocessing
        .as_ref()
        .expect("OCR frame metadata");
    let original = preprocess.original_dimensions;
    let processed = preprocess.new_dimensions.expect("Tesseract resampled the input");
    assert_eq!((original.width, original.height), (1276, 1650));
    assert_eq!((processed.width, processed.height), (2552, 3300));
    let [table] = document.tables.as_slice() else {
        panic!("one owned table")
    };

    let structure = serde_json::to_value(document.document.as_ref().expect("requested structure")).unwrap();
    let [heading, table_node] = structure["nodes"].as_array().unwrap().as_slice() else {
        panic!("one paragraph followed by one table")
    };
    assert_eq!(heading["content"]["node_type"], "paragraph");
    assert_eq!(heading["content"]["text"], HEADING);
    assert_eq!(table_node["content"]["node_type"], "table");
    let node_cells = table_node["content"]["grid"]["cells"].as_array().unwrap();
    assert_eq!(node_cells.len(), 368);
    for (cell, text) in node_cells.iter().zip(expected.iter().flatten()) {
        assert_eq!(cell["content"], *text);
    }
    let heading_bbox = coordinates(serde_json::from_value(heading["bbox"].clone()).unwrap());
    let table_bbox = coordinates(table.bounding_box.unwrap());
    assert_eq!(table_node["bbox"], serde_json::to_value(table.bounding_box).unwrap());
    assert!(
        matches_points(heading_bbox, [29.28, 737.76, 363.6, 746.88])
            && matches_points(table_bbox, [28.8, 53.76, 576.0, 689.28]),
        "expected both boxes in bottom-left PDF points, got heading {heading_bbox:?}, table {table_bbox:?}"
    );
}

#[tokio::test]
async fn detached_table_only_page_preserves_both_ordered_grids_and_point_bounds() {
    let document = extract(include_bytes!(
        "fixtures/ocr/multitable_orientation_2497/two-tables-0.pdf"
    ))
    .await;
    let expected: Vec<Vec<Vec<String>>> =
        serde_json::from_str(include_str!("fixtures/ocr/multitable_orientation_2497/cells.json")).unwrap();
    let structure = serde_json::to_value(document.document.as_ref().expect("requested structure")).unwrap();
    let nodes = structure["nodes"].as_array().unwrap();
    assert_eq!(document.tables.len(), 2);
    assert_eq!(nodes.len(), 2);
    let point_bounds = [[38.4, 459.84, 475.2, 742.08], [38.4, 75.84, 475.2, 358.08]];
    for (((table, node), cells), points) in document.tables.iter().zip(nodes).zip(expected).zip(point_bounds) {
        let bbox = table.bounding_box.unwrap();
        assert!(matches_points(coordinates(bbox), points), "{bbox:?}");
        assert_eq!(table.cells, cells);
        assert_eq!(node["content"]["node_type"], "table");
        assert_eq!(node["bbox"], serde_json::to_value(bbox).unwrap());
    }
}
