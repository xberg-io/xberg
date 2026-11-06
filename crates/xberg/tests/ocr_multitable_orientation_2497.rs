#![cfg(all(
    feature = "pdf",
    feature = "ocr",
    any(feature = "auto-rotate", feature = "auto-rotate-tract")
))]

use std::{env, fs, process::Command};

use sha2::{Digest, Sha256};
use xberg::{BoundingBox, ExtractInput, ExtractedDocument, ExtractionConfig, OcrConfig};

const CASE: &str = "implicit_rotation_retains_primary_when_two_tables_lack_complete_primary_evidence";
const CHILD: &str = "XBERG_MULTITABLE_2497_CHILD";
const MODEL_CACHE_PATH: &str = concat!(
    "models--xberg-io--paddleocr-onnx-models/snapshots/bc5ec866cf0e798e667808dfa51b0ba8ad0dafc8/",
    "v2/classifiers/PP-LCNet_x1_0_doc_ori.onnx"
);
const MODEL_HASH: &str = "6b742aebce6f0f7f71f747931ac7becfc7c96c51641e14943b291eeb334e7947";

fn in_pinned_child(case: &str) -> bool {
    if env::var(CHILD).as_deref() == Ok(case) {
        return true;
    }
    let Some(model) = env::var_os("XBERG_ORIENTATION_TEST_MODEL") else {
        assert_ne!(
            env::var("XBERG_REQUIRE_MODELS").as_deref(),
            Ok("1"),
            "provide the pinned orientation model"
        );
        return false;
    };
    let temp = tempfile::tempdir().unwrap();
    let hub = temp.path().join("hub");
    let model_path = hub.join(MODEL_CACHE_PATH);
    fs::create_dir_all(model_path.parent().unwrap()).unwrap();
    let bytes = fs::read(model).unwrap();
    assert_eq!(hex::encode(Sha256::digest(&bytes)), MODEL_HASH);
    fs::write(model_path, bytes).unwrap();
    let tessdata = temp.path().join("tessdata");
    fs::create_dir(&tessdata).unwrap();
    let english = xberg_tesseract::bundled_eng_traineddata().expect("enable bundle-tessdata-eng");
    assert_eq!(
        hex::encode(Sha256::digest(english)),
        "7d4322bd2a7749724879683fc3912cb542f19906c83bcc1a52132556427170b2"
    );
    fs::write(tessdata.join("eng.traineddata"), english).unwrap();
    let output = Command::new(env::current_exe().unwrap())
        .args(["--exact", case, "--nocapture"])
        .env(CHILD, case)
        .env("HF_HOME", temp.path())
        .env("HF_HUB_CACHE", hub)
        .env("HF_HUB_OFFLINE", "1")
        .env("TESSDATA_PREFIX", tessdata)
        .env("XBERG_CACHE_DIR", temp.path().join("xberg"))
        .output()
        .unwrap();
    let stdout = String::from_utf8_lossy(&output.stdout);
    assert!(
        output.status.success() && stdout.contains("test result: ok. 1 passed; 0 failed; 0 ignored;"),
        "{stdout}\n{}",
        String::from_utf8_lossy(&output.stderr),
    );
    false
}

async fn extract(pdf: &[u8], ocr: Option<OcrConfig>) -> ExtractedDocument {
    extract_with_structure(pdf, ocr, false).await
}

async fn extract_with_structure(
    pdf: &[u8],
    ocr: Option<OcrConfig>,
    include_document_structure: bool,
) -> ExtractedDocument {
    let config = ExtractionConfig {
        force_ocr: true,
        use_cache: false,
        include_document_structure,
        ocr,
        pages: Some(xberg::core::config::PageConfig {
            extract_pages: true,
            ..Default::default()
        }),
        ..Default::default()
    };
    let output = xberg::extract(ExtractInput::from_bytes(pdf.to_vec(), "application/pdf", None), &config)
        .await
        .expect("real OCR extraction");
    assert!(output.errors.is_empty());
    let [document] = <[_; 1]>::try_from(output.results).unwrap();
    assert!(document.metadata.ocr_used);
    document
}

#[track_caller]
fn assert_tables(document: &ExtractedDocument, expected: &[Vec<Vec<String>>], frame: (f64, f64)) {
    assert_eq!(document.tables.len(), 2, "two independently owned tables");
    let [page] = document.pages.as_deref().unwrap() else {
        panic!("one extracted page")
    };
    assert_eq!(page.tables.len(), document.tables.len());
    for (table, page_table) in document.tables.iter().zip(&page.tables) {
        assert_eq!(table.cells, page_table.cells);
        assert_eq!(table.bounding_box, page_table.bounding_box);
    }
    for (table, expected) in document.tables.iter().zip(expected) {
        assert_eq!(table.page_number, 1);
        assert_eq!(&table.cells, expected);
        let BoundingBox { x0, y0, x1, y1 } = table.bounding_box.unwrap();
        assert!(0. <= x0 && x0 < x1 && x1 <= frame.0 && 0. <= y0 && y0 < y1 && y1 <= frame.1);
    }
    let words = expected
        .iter()
        .flatten()
        .flatten()
        .flat_map(|cell| cell.split_whitespace())
        .collect::<Vec<_>>();
    assert_eq!(words.len(), 200);
    assert_eq!(document.content.split_whitespace().collect::<Vec<_>>(), words);
    let page_tables = document
        .tables
        .iter()
        .map(|table| table.markdown.trim())
        .collect::<Vec<_>>()
        .join("\n\n");
    assert_eq!(page.content.trim(), page_tables);
    assert!(document.ocr_elements.is_none());
    assert!(
        document
            .processing_warnings
            .iter()
            .all(|warning| warning.source != "ocr")
    );
}

#[cfg(feature = "paddle-ocr-ort")]
fn assert_detached_upright_structure(document: &ExtractedDocument) {
    let structure = serde_json::to_value(document.document.as_ref().expect("requested structure")).unwrap();
    let nodes = structure["nodes"].as_array().unwrap();
    assert_eq!(nodes.len(), 2);
    let source_rectangles = [[38.4, 459.84, 475.2, 742.08], [38.4, 75.84, 475.2, 358.08]];
    for ((node, table), expected) in nodes.iter().zip(&document.tables).zip(source_rectangles) {
        let bbox = table.bounding_box.unwrap();
        for (actual, expected) in [bbox.x0, bbox.y0, bbox.x1, bbox.y1].into_iter().zip(expected) {
            assert!(
                (actual - expected).abs() < 0.01,
                "point coordinate {actual} != {expected}"
            );
        }
        assert_eq!(node["content"]["node_type"], "table");
        let cells = node["content"]["grid"]["cells"].as_array().unwrap();
        assert_eq!(cells.len(), 80);
        for (cell, text) in cells.iter().zip(table.cells.iter().flatten()) {
            assert_eq!(cell["content"], *text);
            assert!(cell.get("bbox").is_none_or(serde_json::Value::is_null));
        }
        assert_eq!(node["bbox"], serde_json::to_value(bbox).unwrap());
    }
}

#[tokio::test]
async fn implicit_rotation_retains_primary_when_two_tables_lack_complete_primary_evidence() {
    if !in_pinned_child(CASE) {
        return;
    }
    let expected: Vec<Vec<Vec<String>>> =
        serde_json::from_str(include_str!("fixtures/ocr/multitable_orientation_2497/cells.json")).unwrap();
    assert_eq!(expected.iter().map(Vec::len).collect::<Vec<_>>(), [20, 20]);
    let upright = include_bytes!("fixtures/ocr/multitable_orientation_2497/two-tables-0.pdf");
    assert_tables(&extract(upright, None).await, &expected, (612.48, 792.));
    #[cfg(feature = "paddle-ocr-ort")]
    {
        let structured = extract_with_structure(upright, None, true).await;
        assert_tables(&structured, &expected, (612.48, 792.));
        assert_detached_upright_structure(&structured);
    }
    for rotated in [
        include_bytes!("fixtures/ocr/multitable_orientation_2497/two-tables-90.pdf").as_slice(),
        include_bytes!("fixtures/ocr/multitable_orientation_2497/two-tables-270.pdf").as_slice(),
    ] {
        let forced = extract(
            rotated,
            Some(OcrConfig {
                auto_rotate: true,
                ..Default::default()
            }),
        )
        .await;
        assert_tables(&forced, &expected, (792., 612.48));
        let disabled = extract(rotated, Some(OcrConfig::default())).await;
        let implicit = extract(rotated, None).await;
        assert!(implicit.tables.is_empty());
        assert!(implicit.ocr_elements.is_none());
        assert_eq!(implicit.content, disabled.content);
        assert!(
            implicit
                .processing_warnings
                .iter()
                .any(|warning| warning.message.contains("recognition noise"))
        );
    }

    let expected: Vec<Vec<Vec<String>>> = serde_json::from_str(include_str!(
        "fixtures/ocr/multitable_orientation_2497/strict-cells.json"
    ))
    .unwrap();
    assert_eq!(expected.iter().map(Vec::len).collect::<Vec<_>>(), [20, 20]);
    let upright = include_bytes!("fixtures/ocr/multitable_orientation_2497/strict-two-tables-0.pdf");
    assert_tables(&extract(upright, None).await, &expected, (612.48, 792.));
    let rotated = include_bytes!("fixtures/ocr/multitable_orientation_2497/strict-two-tables-90.pdf");
    let forced = extract(
        rotated,
        Some(OcrConfig {
            auto_rotate: true,
            ..Default::default()
        }),
    )
    .await;
    assert_tables(&forced, &expected, (792., 612.48));
    let implicit = extract(rotated, None).await;
    assert_tables(&implicit, &expected, (792., 612.48));
    for (actual, control) in implicit.tables.iter().zip(forced.tables) {
        assert_eq!(actual.bounding_box, control.bounding_box);
    }
}

#[tokio::test]
async fn implicit_sideways_heading_recovers_exact_source_and_table_bounds() {
    if !in_pinned_child("implicit_sideways_heading_recovers_exact_source_and_table_bounds") {
        return;
    }
    let pdf = include_bytes!("fixtures/ocr/detached_heading_frame_2497/heading-table-90.pdf");
    assert_eq!(
        hex::encode(Sha256::digest(pdf)),
        "1128fc7e3586699c33b14372e71265e2cec284311c493a7a85ac1c0ac30073a0"
    );
    let expected: Vec<Vec<String>> =
        serde_json::from_str(include_str!("fixtures/ocr/detached_heading_frame_2497/cells.json")).unwrap();
    assert_eq!(expected.len(), 46);
    assert!(expected.iter().all(|row| row.len() == 8));
    let forced = extract(
        pdf,
        Some(OcrConfig {
            auto_rotate: true,
            ..Default::default()
        }),
    )
    .await;
    assert_heading_and_table(&forced, &expected);
    let implicit = extract(pdf, None).await;
    assert_heading_and_table(&implicit, &expected);
    assert_eq!(implicit.tables[0].bounding_box, forced.tables[0].bounding_box);
    assert!(implicit.ocr_elements.is_none());
}

#[track_caller]
fn assert_heading_and_table(document: &ExtractedDocument, expected: &[Vec<String>]) {
    const HEADING: &str = "Cedar Company Ledger Summary For The March Quarter Report 2026";
    let [table] = document.tables.as_slice() else {
        panic!("expected one table, found {}", document.tables.len());
    };
    assert_eq!(table.cells.as_slice(), expected);
    assert_eq!(table.page_number, 1);
    let [page] = document.pages.as_deref().unwrap() else {
        panic!("expected one extracted page");
    };
    let [page_table] = page.tables.as_slice() else {
        panic!("expected one page table");
    };
    assert_eq!(page_table.cells, table.cells);
    assert_eq!(page_table.bounding_box, table.bounding_box);
    assert_eq!(page.content.trim(), format!("{HEADING}\n\n{}", table.markdown.trim()));
    let words = HEADING
        .split_whitespace()
        .chain(expected.iter().flatten().flat_map(|cell| cell.split_whitespace()))
        .collect::<Vec<_>>();
    assert_eq!(words.len(), 470);
    assert_eq!(document.content.split_whitespace().collect::<Vec<_>>(), words);
    let bbox = table.bounding_box.expect("table bounds in rotated PDF page points");
    let source_bounds = [53.76, 36.48, 689.28, 583.68];
    for (actual, expected) in [bbox.x0, bbox.y0, bbox.x1, bbox.y1].into_iter().zip(source_bounds) {
        assert!(
            (actual - expected).abs() < 0.01,
            "point coordinate {actual} != {expected}"
        );
    }
}
