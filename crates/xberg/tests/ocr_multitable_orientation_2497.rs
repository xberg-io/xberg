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

fn in_pinned_child() -> bool {
    if env::var(CHILD).as_deref() == Ok(CASE) {
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
    fs::write(tessdata.join("eng.traineddata"), english).unwrap();
    let output = Command::new(env::current_exe().unwrap())
        .args(["--exact", CASE, "--nocapture"])
        .env(CHILD, CASE)
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
    let config = ExtractionConfig {
        force_ocr: true,
        use_cache: false,
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

#[tokio::test]
async fn implicit_rotation_retains_primary_when_two_tables_lack_complete_primary_evidence() {
    if !in_pinned_child() {
        return;
    }
    let expected: Vec<Vec<Vec<String>>> =
        serde_json::from_str(include_str!("fixtures/ocr/multitable_orientation_2497/cells.json")).unwrap();
    assert_eq!(expected.iter().map(Vec::len).collect::<Vec<_>>(), [20, 20]);
    let upright = include_bytes!("fixtures/ocr/multitable_orientation_2497/two-tables-0.pdf");
    assert_tables(&extract(upright, None).await, &expected, (612.48, 792.));
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
