//! Every ONNX Runtime consumer in a process must run under one environment.
//!
//! Since ort 2.0.0-rc.14, `EnvironmentBuilder::build` refuses a second environment ("only one
//! environment is allowed per process"). The benchmark pipelines failed when the first ORT
//! consumer in a process (layout detection, the auto-rotate classifier) created the environment
//! and a later one (PaddleOCR, Sceptre) built its own. Each scenario re-runs itself in a fresh
//! child process: the environment is process-global, so a sibling test that created it first
//! would decide the order under test.
//!
//! The scenarios download the layout, PaddleOCR and Sceptre models, so they are ignored by default:
//!
//! ```text
//! cargo test -p xberg --features "pdf layout-detection paddle-ocr sceptre-ocr auto-rotate" \
//!     --test ort_shared_environment -- --ignored --nocapture
//! ```

#![allow(clippy::print_stderr)] // ~keep: a skipped scenario reports why on stderr
#![cfg(all(
    feature = "pdf",
    feature = "layout-detection",
    any(feature = "paddle-ocr-ort", feature = "sceptre-ocr-ort")
))]

mod helpers;

use std::process::Command;

use helpers::{get_test_file_path, test_documents_available};
use xberg::layout::{LayoutEngine, LayoutEngineConfig};
use xberg::types::ProcessingWarning;

const CHILD_ENV: &str = "XBERG_ORT_SHARED_ENVIRONMENT_CHILD";
const HELLO_WORLD_IMAGE: &str = "images/test_hello_world.png";

fn isolated(name: &str, run: impl FnOnce()) {
    if std::env::var(CHILD_ENV).as_deref() == Ok(name) {
        run();
        return;
    }
    if !test_documents_available() {
        eprintln!("SKIP {name}: test_documents is not populated, so {HELLO_WORLD_IMAGE} is missing");
        return;
    }
    let output = Command::new(std::env::current_exe().expect("locate the test binary"))
        .args(["--exact", name, "--include-ignored", "--nocapture", "--test-threads=1"])
        .env(CHILD_ENV, name)
        .output()
        .expect("run the scenario in a child process");
    let stdout = String::from_utf8_lossy(&output.stdout);
    assert!(
        output.status.success(),
        "{name} failed in its child process:\nstdout={stdout}\nstderr={}",
        String::from_utf8_lossy(&output.stderr),
    );
    assert!(
        stdout.contains("1 passed; 0 failed"),
        "{name} did not run in its child process:\n{stdout}"
    );
}

fn create_environment_with_layout_detection() {
    LayoutEngine::from_config(LayoutEngineConfig::default())
        .expect("layout detection must create the process ONNX Runtime environment");
}

fn hello_world_image() -> Vec<u8> {
    std::fs::read(get_test_file_path(HELLO_WORLD_IMAGE)).expect("read the hello-world fixture")
}

fn assert_recognized_hello(consumer: &str, content: &str) {
    let text = content.to_lowercase();
    assert!(
        text.contains("hello") || text.contains("helo"),
        "{consumer} recognized no greeting: {content:?}"
    );
}

fn assert_no_environment_warning(consumer: &str, warnings: &[ProcessingWarning]) {
    let environment_warnings: Vec<_> = warnings
        .iter()
        .filter(|warning| warning.message.contains("environment"))
        .collect();
    assert!(
        environment_warnings.is_empty(),
        "{consumer} reported an ONNX Runtime environment failure: {environment_warnings:?}"
    );
}

#[cfg(feature = "paddle-ocr-ort")]
#[test]
#[ignore = "downloads the layout and PaddleOCR models"]
fn paddle_ocr_after_layout_detection_reuses_the_environment() {
    isolated("paddle_ocr_after_layout_detection_reuses_the_environment", || {
        use xberg::core::config::OcrConfig;
        use xberg::paddle_ocr::{PaddleOcrBackend, PaddleOcrConfig};
        use xberg::plugins::OcrBackend;

        create_environment_with_layout_detection();
        let backend = PaddleOcrBackend::with_config(PaddleOcrConfig::new("en")).expect("create the PaddleOCR backend");
        let ocr = OcrConfig {
            backend: "paddle-ocr".to_string(),
            language: vec!["en".to_string()],
            ..Default::default()
        };
        let runtime = tokio::runtime::Builder::new_current_thread()
            .enable_all()
            .build()
            .expect("build a tokio runtime");
        let document = runtime
            .block_on(backend.process_image(&hello_world_image(), &ocr))
            .unwrap_or_else(|error| panic!("PaddleOCR after layout detection failed: {error}"));
        assert_no_environment_warning("PaddleOCR", &document.processing_warnings);
        assert_recognized_hello("PaddleOCR", &document.content);
    });
}

#[cfg(feature = "sceptre-ocr-ort")]
fn extract_with_sceptre_ort(auto_rotate: bool) {
    let config: xberg::core::config::ExtractionConfig = serde_json::from_value(serde_json::json!({
        "use_cache": false,
        "force_ocr": true,
        "ocr": {
            "backend": "sceptre",
            "backend_options": {"model": {"backend": "ort"}},
            "auto_rotate": auto_rotate,
        },
    }))
    .expect("valid Sceptre ORT extraction config");
    let document = helpers::extract_uri_document_blocking(get_test_file_path(HELLO_WORLD_IMAGE), None, &config)
        .unwrap_or_else(|error| panic!("Sceptre ORT extraction failed: {error}"));
    assert_no_environment_warning("Sceptre", &document.processing_warnings);
    assert_recognized_hello("Sceptre", &document.content);
}

#[cfg(feature = "sceptre-ocr-ort")]
#[test]
#[ignore = "downloads the layout and Sceptre models"]
fn sceptre_after_layout_detection_reuses_the_environment() {
    isolated("sceptre_after_layout_detection_reuses_the_environment", || {
        create_environment_with_layout_detection();
        extract_with_sceptre_ort(false);
    });
}

#[cfg(all(feature = "sceptre-ocr-ort", feature = "auto-rotate"))]
#[test]
#[ignore = "downloads the orientation classifier and Sceptre models"]
fn sceptre_with_auto_rotate_shares_the_environment() {
    isolated("sceptre_with_auto_rotate_shares_the_environment", || {
        extract_with_sceptre_ort(true);
    });
}
