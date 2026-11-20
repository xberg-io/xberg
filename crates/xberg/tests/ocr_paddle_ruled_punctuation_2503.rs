#![cfg(all(feature = "pdf", feature = "paddle-ocr-ort", feature = "tokio-runtime"))]

use sha2::{Digest, Sha256};
use xberg::{ExtractInput, ExtractedDocument, ExtractionConfig};

async fn extract(image: &[u8], mime: &str, punctuation: bool, split_rows: bool) -> ExtractedDocument {
    extract_with_word_recovery(image, mime, punctuation, split_rows, false).await
}

async fn extract_with_word_recovery(
    image: &[u8],
    mime: &str,
    punctuation: bool,
    split_rows: bool,
    fused_words: bool,
) -> ExtractedDocument {
    let cache = std::env::var("XBERG_2503_PADDLE_CACHE").expect("pinned Paddle model cache");
    let models = std::path::Path::new(&cache)
        .join("models--xberg-io--paddleocr-onnx-models/snapshots/bc5ec866cf0e798e667808dfa51b0ba8ad0dafc8/v6");
    for (file, digest) in [
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
        assert_eq!(
            hex::encode(Sha256::digest(std::fs::read(models.join(file)).unwrap())),
            digest
        );
    }
    let config: ExtractionConfig = serde_json::from_value(serde_json::json!({
        "use_cache": false, "output_format": "plain", "pages": {"extract_pages": true},
        "ocr": {
            "backend": "paddle-ocr", "language": ["eng"],
            "backend_options": {
                "paddle_ruled_punctuation": punctuation,
                "paddle_ruled_row_recovery": split_rows,
                "paddle_ruled_word_recovery": fused_words
            },
            "paddle_ocr_settings": {
                "cache_dir": cache, "model_version": "pp-ocrv6", "model_tier": "small",
                "enable_table_detection": true, "use_angle_cls": false
            }
        }
    }))
    .unwrap();
    let mut batch = xberg::extract(ExtractInput::from_bytes(image.to_vec(), mime, None), &config)
        .await
        .unwrap();
    assert!(batch.errors.is_empty());
    assert_eq!(batch.results.len(), 1);
    batch.results.pop().unwrap()
}

fn fixture(name: &str, hash: &str) -> Vec<u8> {
    let dir = std::env::var_os("XBERG_2503_RULED_FIXTURES").expect("directory of pinned synthetic fixtures");
    let bytes = std::fs::read(std::path::Path::new(&dir).join(name)).unwrap();
    assert_eq!(hex::encode(Sha256::digest(&bytes)), hash);
    bytes
}

#[tokio::test]
#[ignore = "requires offline PP-OCRv6 Small models and XBERG_2503_RULED_FIXTURES"]
async fn paddle_corrects_only_supported_ruled_table_punctuation() {
    let ruled = fixture(
        "fresh_1_9px.jpg",
        "7046863cfab54bffd783a1b64ddda73bbe72af32c30b42cf014adca5bcd84f1e",
    );
    let baseline = extract(&ruled, "image/jpeg", false, false).await;
    let corrected = extract(&ruled, "image/jpeg", true, true).await;
    assert_eq!(baseline.tables.len(), 1);
    assert_eq!(corrected.tables.len(), 1);
    let mut expected = baseline.tables[0].cells.clone();
    assert_eq!(expected[3][2], "90.302");
    assert_eq!(expected[9][2], "608.387");
    expected[3][2] = "90,302".into();
    expected[9][2] = "608,387".into();
    assert_eq!(corrected.tables[0].cells, expected);
    let numeric_rows = expected
        .iter()
        .skip(1)
        .map(|row| row[1..].join(" "))
        .collect::<Vec<_>>()
        .join("\n");
    assert_eq!(
        numeric_rows,
        concat!(
            "17 104,705 (9,871)\n",
            "63 502,311 811.03\n",
            "42 90,302 -18.25\n",
            "15 201,780 +7.07\n",
            "77 49,002 48,199\n",
            "36 871,459 (2,634)\n",
            "50 337,509 101.90\n",
            "24 109,066 -30.08\n",
            "92 608,387 (1,017)\n",
            "19 78,411 223.93\n",
            "68 451,205 12,901\n",
            "31 283,870 (8,896)",
        )
    );
    let [page] = corrected.pages.as_deref().unwrap() else {
        panic!("one standalone page expected")
    };
    assert_eq!(page.tables[0].cells, expected);
    let rows = expected.iter().map(|row| row.join(" ")).collect::<Vec<_>>().join("\n");
    assert_eq!(corrected.content.trim(), rows);
    assert_eq!(page.content.trim(), rows);

    let merged = fixture(
        "fresh_2_9px.jpg",
        "ed9f44ffa7a2393abef04ef3ba8a7bb15c3e82998d1e16c00bf3dcf36ad94089",
    );
    for (bytes, mime) in [
        (merged.as_slice(), "image/jpeg"),
        (
            include_bytes!("fixtures/ocr/paddle-ledger-clean-2503.png").as_slice(),
            "image/png",
        ),
        (
            include_bytes!("fixtures/ocr/paddle-ledger-tinted-2503.png").as_slice(),
            "image/png",
        ),
    ] {
        let baseline = extract(bytes, mime, false, false).await;
        let candidate = extract(bytes, mime, true, mime != "image/jpeg").await;
        if mime == "image/jpeg" {
            assert_eq!(baseline.tables[0].cells[8][2], "780.090-780.090");
            assert!(baseline.tables[0].cells[8][3].is_empty());
        }
        assert_eq!(candidate.tables[0].cells, baseline.tables[0].cells);
        assert_eq!(candidate.content, baseline.content);
        assert_eq!(
            candidate.pages.as_ref().unwrap()[0].content,
            baseline.pages.as_ref().unwrap()[0].content
        );
    }
}

#[tokio::test]
#[ignore = "requires offline PP-OCRv6 Small models and XBERG_2503_RULED_FIXTURES"]
async fn paddle_splits_table_cells_only_when_both_source_detections_survive() {
    let cases = [
        (
            "fresh_2_7px.jpg",
            "069e09e1c041ab1d34bd1ee4a0c32ebf8f4e3c34548b91d45406fc2b5e35760d",
            &[
                (5, 1, "39 940,501", "39", "940,501"),
                (6, 2, "55.505 (55.505)", "55.505", "(55.505)"),
                (11, 2, "811,070 [811,070]", "811,070", "[811,070]"),
                (12, 2, "303.909 (303,909)", "303.909", "(303,909)"),
            ][..],
        ),
        (
            "fresh_2_9px.jpg",
            "ed9f44ffa7a2393abef04ef3ba8a7bb15c3e82998d1e16c00bf3dcf36ad94089",
            &[
                (8, 2, "780.090-780.090", "780.090", "-780.090"),
                (10, 2, "777.008 777,008", "777.008", "777,008"),
            ][..],
        ),
    ];
    for (name, digest, edits) in cases {
        let bytes = fixture(name, digest);
        let baseline = extract(&bytes, "image/jpeg", false, false).await;
        let candidate = extract(&bytes, "image/jpeg", false, true).await;
        assert_eq!(baseline.tables.len(), 1, "{name}");
        assert_eq!(candidate.tables.len(), 1, "{name}");
        let mut expected = baseline.tables[0].cells.clone();
        for &(row, col, merged, left, right) in edits {
            assert_eq!(expected[row][col], merged, "{name}");
            assert!(expected[row][col + 1].is_empty(), "{name}");
            expected[row][col] = left.to_owned();
            expected[row][col + 1] = right.to_owned();
        }
        if name == "fresh_2_9px.jpg" {
            assert_eq!(expected[11][2..], ["811,070 [811.070]", ""]);
            assert_eq!(expected[12][2..], ["303.909 (303,909)", ""]);
        }
        assert_eq!(candidate.tables[0].cells, expected, "{name}");
        let [page] = candidate.pages.as_deref().unwrap() else {
            panic!("one standalone page expected")
        };
        assert_eq!(page.tables[0].cells, expected, "{name}");
        let rows: Vec<_> = expected.iter().map(|row| row.join(" ").trim_end().to_owned()).collect();
        for output in [candidate.content.as_str(), page.content.as_str()] {
            assert_eq!(
                output.trim().lines().map(str::trim_end).collect::<Vec<_>>(),
                rows,
                "{name}"
            );
        }
    }
}

#[tokio::test]
#[ignore = "requires offline PP-OCRv6 Small models and XBERG_2503_RULED_FIXTURES"]
async fn paddle_recovers_exact_words_from_fused_9px_detections() {
    let bytes = fixture(
        "fresh_2_9px.jpg",
        "ed9f44ffa7a2393abef04ef3ba8a7bb15c3e82998d1e16c00bf3dcf36ad94089",
    );
    let line_only = extract(&bytes, "image/jpeg", false, true).await;
    let candidate = extract_with_word_recovery(&bytes, "image/jpeg", false, true, true).await;
    assert_eq!(line_only.tables.len(), 1);
    assert_eq!(candidate.tables.len(), 1);
    let mut expected = line_only.tables[0].cells.clone();
    for (row, joined, left, right) in [
        (11, "811,070 [811.070]", "811,070", "[811.070]"),
        (12, "303.909 (303,909)", "303.909", "(303,909)"),
    ] {
        assert_eq!(expected[row][2..], [joined, ""]);
        expected[row][2] = left.to_owned();
        expected[row][3] = right.to_owned();
    }
    assert_eq!(candidate.tables[0].cells, expected);
    let [page] = candidate.pages.as_deref().unwrap() else {
        panic!("one standalone page expected")
    };
    assert_eq!(page.tables[0].cells, expected);
    let rows = expected.iter().map(|row| row.join(" ")).collect::<Vec<_>>().join("\n");
    assert_eq!(candidate.content.trim(), rows);
    assert_eq!(page.content.trim(), rows);
}

#[tokio::test]
#[ignore = "requires offline PP-OCRv6 Small models and XBERG_2503_RULED_FIXTURES"]
async fn paddle_repairs_independently_detected_commas_alongside_fused_rows() {
    let bytes = fixture(
        "fresh_2_9px.jpg",
        "ed9f44ffa7a2393abef04ef3ba8a7bb15c3e82998d1e16c00bf3dcf36ad94089",
    );
    let stock = extract_with_word_recovery(&bytes, "image/jpeg", false, true, true).await;
    let candidate = extract_with_word_recovery(&bytes, "image/jpeg", true, true, true).await;
    let mut expected = stock.tables[0].cells.clone();
    for (row, column, before, after) in [
        (3, 2, "99.901", "99,901"),
        (5, 2, "940.501", "940,501"),
        (8, 3, "-780.090", "-780,090"),
    ] {
        assert_eq!(expected[row][column], before);
        expected[row][column] = after.to_owned();
    }
    assert_eq!(expected[9][2], "59.118");
    assert_eq!(expected[11][2..], ["811,070", "[811.070]"]);
    assert_eq!(expected[12][2..], ["303.909", "(303,909)"]);
    let [table] = candidate.tables.as_slice() else {
        panic!("one table expected");
    };
    assert_eq!(table.cells, expected);
    let [page] = candidate.pages.as_deref().unwrap() else {
        panic!("one standalone page expected");
    };
    assert_eq!(page.tables[0].cells, expected);
    let rows = expected.iter().map(|row| row.join(" ")).collect::<Vec<_>>().join("\n");
    assert_eq!(candidate.content.trim(), rows);
    assert_eq!(page.content.trim(), rows);
}

#[tokio::test]
#[ignore = "requires offline PP-OCRv6 Small and XBERG_2503_ORIGINAL_RULED_FIXTURES"]
async fn paddle_plain_page_retains_rows_after_source_proven_label_token_recovery() {
    let directory = std::env::var_os("XBERG_2503_ORIGINAL_RULED_FIXTURES").expect("original ruled fixture directory");
    let image = std::fs::read(std::path::Path::new(&directory).join("jpg_9_grid_lead.jpg")).unwrap();
    assert_eq!(
        hex::encode(Sha256::digest(&image)),
        "e2cb25fa92fdf4488b56c4a66c9c3c1a168621bc7664276c6cf98db418d4ed3c"
    );
    let before = extract_with_word_recovery(&image, "image/jpeg", true, false, false).await;
    let after = extract_with_word_recovery(&image, "image/jpeg", true, true, true).await;
    assert_eq!(before.tables.len(), 1);
    assert_eq!(after.tables.len(), 1);
    let mut cells = before.tables[0].cells.clone();
    assert_eq!(cells.len(), 13);
    assert_eq!(cells[12][0], "Street repair. .");
    assert!(
        before.pages.as_ref().unwrap()[0]
            .content
            .lines()
            .any(|line| line == "Street repair..")
    );

    cells[12][0] = "Street repair..".to_owned();
    assert_eq!(after.tables[0].cells, cells);
    let [page] = after.pages.as_deref().unwrap() else {
        panic!("one standalone page expected")
    };
    assert_eq!(page.tables[0].cells, cells);
    let rows = cells.iter().map(|row| row.join(" ")).collect::<Vec<_>>().join("\n");
    assert_eq!(after.content.trim(), rows);
    assert_eq!(page.content.trim(), rows);
}
