#![cfg(all(feature = "ocr", feature = "tokio-runtime"))]

mod helpers;

use sha2::{Digest, Sha256};
use xberg::ExtractionConfig;
use xberg::core::config::OcrConfig;

const EXPECTED: &str = "Docling bundles PDF document conversion to JSON and Markdown in an easy self contained package";

#[test]
fn budgeted_sparse_retry_preserves_the_rotated_png_sentence() {
    let path = helpers::get_test_documents_dir().join("images/ocr_test_rotated_90.png");
    let bytes = std::fs::read(&path).unwrap_or_else(|error| panic!("required source {}: {error}", path.display()));
    assert_eq!(
        hex::encode(Sha256::digest(&bytes)),
        "9a67a012fe80a9a07c639a9a59ddb98feed64f4604d4d7dacd698e0a7d642004"
    );
    let result = helpers::extract_bytes_document_blocking(
        &bytes,
        "image/png",
        &ExtractionConfig {
            use_cache: false,
            ocr: Some(OcrConfig::default()),
            ..Default::default()
        },
    )
    .expect("OCR succeeds with the default workspace");
    assert_eq!(
        result.content.split_whitespace().collect::<Vec<_>>().join(" "),
        EXPECTED
    );
    assert_eq!(
        result.metadata.additional.get("psm").and_then(|psm| psm.as_str()),
        Some("3")
    );
    let dpi = result
        .metadata
        .image_preprocessing
        .expect("actual contrast retry normalization");
    assert_eq!(dpi.target_dpi, 300);
    assert!((225..250).contains(&dpi.final_dpi), "reported DPI: {dpi:?}");
    assert_eq!(dpi.calculated_dpi, Some(dpi.final_dpi));
}
