#![cfg(all(feature = "pdf", feature = "paddle-ocr-ort", feature = "tokio-runtime"))]

use sha2::{Digest, Sha256};
use xberg::{ExtractInput, ExtractionConfig};

const TINTED: &[u8] = include_bytes!("fixtures/ocr/paddle-ledger-tinted-2503.png");
const CLEAN: &[u8] = include_bytes!("fixtures/ocr/paddle-ledger-clean-2503.png");

fn model_cache() -> std::path::PathBuf {
    let cache = std::env::var_os("XBERG_2503_PADDLE_CACHE").expect("pinned Paddle model cache");
    let cache = std::path::PathBuf::from(cache);
    let models = cache.join(concat!(
        "models--xberg-io--paddleocr-onnx-models/snapshots/",
        "bc5ec866cf0e798e667808dfa51b0ba8ad0dafc8/v6"
    ));
    for (file, expected) in [
        (
            "det/small/model.onnx",
            "b1a4f07289eda88d29239890b94ea2f9e29f5635a33ff6e165bb1b27dcea25fc",
        ),
        (
            "rec/small/model.onnx",
            "0cc734c53747ea66968d286f6e6597fc1c91df121964ab4be5085c156fcac892",
        ),
        (
            "rec/small/dict.txt",
            "b5f2bfe2bdd9448429e3e82b51c789775d9b42f2403d082b00662eb77e401c5d",
        ),
    ] {
        let bytes = std::fs::read(models.join(file)).expect("cached Paddle artifact");
        assert_eq!(hex::encode(Sha256::digest(bytes)), expected, "{file}");
    }
    cache
}

#[tokio::test]
#[ignore = "requires ONNX Runtime and XBERG_2503_PADDLE_CACHE"]
async fn paddle_plain_page_retains_exact_rows_of_standalone_table() {
    let cache = model_cache();
    for (image, expected) in [(TINTED, TINTED_ROWS), (CLEAN, CLEAN_ROWS)] {
        let config: ExtractionConfig = serde_json::from_value(serde_json::json!({
            "use_cache": false,
            "output_format": "plain",
            "pages": {"extract_pages": true},
            "ocr": {
                "backend": "paddle-ocr",
                "language": ["eng"],
                "paddle_ocr_settings": {
                    "cache_dir": cache, "model_version": "pp-ocrv6", "model_tier": "small",
                    "enable_table_detection": true, "use_angle_cls": false
                }
            }
        }))
        .unwrap();
        let batch = xberg::extract(ExtractInput::from_bytes(image.to_vec(), "image/png", None), &config)
            .await
            .expect("standalone Paddle OCR succeeds");
        assert!(batch.errors.is_empty());
        let [doc] = batch.results.as_slice() else {
            panic!("expected one document")
        };
        let [page] = doc.pages.as_deref().expect("requested page output") else {
            panic!("expected one page")
        };
        let [table] = doc.tables.as_slice() else {
            panic!("expected one recognized table")
        };
        let [page_table] = page.tables.as_slice() else {
            panic!("expected one page table")
        };
        assert_eq!(table.cells.len(), 13);
        assert_eq!(table.cells, page_table.cells);
        assert_eq!(doc.content.trim(), expected);
        assert_eq!(
            page.content.trim(),
            expected,
            "Plain page must keep each recognized row together"
        );
    }
}

#[cfg(feature = "bundle-tessdata-eng")]
#[tokio::test]
async fn tesseract_plain_table_keeps_automatic_and_explicit_segmentation() {
    let models = tempfile::tempdir().unwrap();
    let english = xberg_tesseract::bundled_eng_traineddata().expect("bundled English");
    assert_eq!(
        hex::encode(Sha256::digest(english)),
        "7d4322bd2a7749724879683fc3912cb542f19906c83bcc1a52132556427170b2"
    );
    std::fs::write(models.path().join("eng.traineddata"), english).unwrap();
    for (psm, expected_psm) in [(None, "11"), (Some(6), "6")] {
        let mut config = serde_json::json!({
            "use_cache": false,
            "output_format": "plain",
            "pages": {"extract_pages": true},
            "ocr": {"backend": "tesseract", "language": ["eng"], "tessdata_path": models.path()}
        });
        if let Some(psm) = psm {
            config["ocr"]["tesseract_config"] = serde_json::json!({"psm": psm});
        }
        let config: ExtractionConfig = serde_json::from_value(config).unwrap();
        let batch = xberg::extract(ExtractInput::from_bytes(CLEAN.to_vec(), "image/png", None), &config)
            .await
            .expect("standalone Tesseract OCR succeeds");
        assert!(batch.errors.is_empty());
        let [doc] = batch.results.as_slice() else {
            panic!("expected one document")
        };
        let [page] = doc.pages.as_deref().expect("requested page output") else {
            panic!("expected one page")
        };
        assert_eq!(doc.metadata.additional["psm"], expected_psm);
        assert_eq!(doc.content.trim(), CLEAN_ROWS);
        assert_eq!(page.content.trim(), CLEAN_ROWS);
    }
}

const TINTED_ROWS: &str = "Item Count Budget Change
Code revlew 39 380,985 (6,250)
Tree planting 15 88,140 840.64
Signal timing 61 658,031 52,180
Pool repalrs 73 587,196 (6,697)
Snow removal 12 718,273 427.60
Fleld lights 24 679,264 51,642
Parks and trals 89 850,386 (1,865)
Water service 40 708,088 935.83
Street repalir 36 596,827 71,741
Llbrary hours 14 501,718 (3,276)
Fleet fuel 84 903,455 580.45
Bridge palint 27 163,782 91,463";

const CLEAN_ROWS: &str = "Item Count Budget Change
Code review 27 249,024 (7,425)
Tree planting 93 900,353 247.34
Signal timing 50 616,183 57,362
Pool repairs 16 591,902 (449)
Snow removal 63 195,184 606.67
Field lights 82 63,715 34,870
Parks and trails 33 980,402 (5,219)
Water service 75 146,286 693.88
Street repair 65 51,350 55,969
Library hours 42 519,291 (8,575)
Fleet fuel 94 947,938 324.72
Bridge paint 60 597,025 74,571";
