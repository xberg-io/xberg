//! Public-API check that selecting PP-DocLayout-V3 on a build without the ORT layout engine
//! fails loudly instead of silently skipping layout.

#![cfg(all(feature = "layout-types", not(feature = "layout-detection")))]

mod helpers;

use helpers::extract_bytes_document_blocking;
use xberg::{ExtractInput, ExtractionConfig, FileExtractionConfig, LayoutDetectionConfig, LayoutDetectionModel};

fn v3_layout() -> LayoutDetectionConfig {
    LayoutDetectionConfig {
        model: LayoutDetectionModel::PpDocLayoutV3,
        ..Default::default()
    }
}

#[test]
fn extract_rejects_v3_in_the_request_config() {
    let config = ExtractionConfig {
        layout: Some(v3_layout()),
        ..Default::default()
    };
    let error = extract_bytes_document_blocking(b"hello", "text/plain", &config)
        .expect_err("V3 must not silently fall back or skip layout");
    assert!(error.to_string().contains("pp_doc_layout_v3"), "{error}");
}

#[test]
fn extract_rejects_v3_in_a_per_input_override() {
    let mut input = ExtractInput::from_bytes(b"hello".to_vec(), "text/plain".to_string(), None);
    input.config = Some(FileExtractionConfig {
        layout: Some(v3_layout()),
        ..Default::default()
    });
    let runtime = tokio::runtime::Builder::new_current_thread()
        .enable_all()
        .build()
        .unwrap();
    let error = runtime
        .block_on(xberg::extract(input, &ExtractionConfig::default()))
        .expect_err("a per-input V3 selection must be rejected");
    assert!(error.to_string().contains("pp_doc_layout_v3"), "{error}");
}

#[test]
fn extract_accepts_the_default_layout_model() {
    let config = ExtractionConfig {
        layout: Some(LayoutDetectionConfig::default()),
        ..Default::default()
    };
    extract_bytes_document_blocking(b"hello", "text/plain", &config).expect("RT-DETR selection stays valid");
}
