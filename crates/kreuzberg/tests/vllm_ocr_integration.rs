//! Integration tests for vLLM OCR backend.
//!
//! These tests require a running vLLM instance with a vision model.
//! Run with: cargo test --features "vllm-ocr" --test vllm_ocr_integration -- --ignored

#![cfg(feature = "vllm-ocr")]

use kreuzberg::OcrConfig;
use kreuzberg::plugins::{OcrBackend, Plugin};
use kreuzberg::vllm_ocr::VllmOcrBackend;

const VLLM_MODELS_URL: &str = "http://localhost:8000/v1/models";

fn vllm_available() -> bool {
    ureq::get(VLLM_MODELS_URL).call().is_ok()
}

fn test_image_path(name: &str) -> std::path::PathBuf {
    std::path::PathBuf::from(env!("CARGO_MANIFEST_DIR"))
        .parent()
        .unwrap()
        .parent()
        .unwrap()
        .join("test_documents/images")
        .join(name)
}

/// Test basic English OCR with vLLM.
#[tokio::test]
#[ignore = "requires running vLLM with a vision model"]
async fn test_vllm_hello_world() {
    if !vllm_available() {
        eprintln!("Skipping: vLLM not available at localhost:8000");
        return;
    }

    let backend = VllmOcrBackend::new();
    let image_bytes = std::fs::read(test_image_path("test_hello_world.png")).expect("test image");

    let config = OcrConfig {
        backend: "vllm".to_string(),
        language: "eng".to_string(),
        ..Default::default()
    };

    let result = backend.process_image(&image_bytes, &config).await.unwrap();
    assert!(
        result.content.to_lowercase().contains("hello"),
        "Expected 'hello' in output, got: {}",
        result.content
    );
    assert!(
        result.content.to_lowercase().contains("world"),
        "Expected 'world' in output, got: {}",
        result.content
    );
}

/// Test Chinese OCR with vLLM.
#[tokio::test]
#[ignore = "requires running vLLM with a vision model"]
async fn test_vllm_chinese() {
    if !vllm_available() {
        eprintln!("Skipping: vLLM not available at localhost:8000");
        return;
    }

    let backend = VllmOcrBackend::new();
    let image_bytes = std::fs::read(test_image_path("chi_sim_image.jpeg")).expect("test image");

    let config = OcrConfig {
        backend: "vllm".to_string(),
        language: "chi".to_string(),
        ..Default::default()
    };

    let result = backend.process_image(&image_bytes, &config).await.unwrap();
    assert!(!result.content.is_empty(), "Chinese OCR should return text");
    assert!(
        result.content.contains('肌') || result.content.contains('肤'),
        "Expected Chinese characters in output, got: {}",
        &result.content[..result.content.len().min(200)]
    );
}

/// Test that process_file works (reads file then calls process_image).
#[tokio::test]
#[ignore = "requires running vLLM with a vision model"]
async fn test_vllm_process_file() {
    if !vllm_available() {
        eprintln!("Skipping: vLLM not available at localhost:8000");
        return;
    }

    let backend = VllmOcrBackend::new();
    let path = test_image_path("test_hello_world.png");

    let config = OcrConfig {
        backend: "vllm".to_string(),
        language: "eng".to_string(),
        ..Default::default()
    };

    let result = backend.process_file(&path, &config).await.unwrap();
    assert!(
        result.content.to_lowercase().contains("hello"),
        "Expected 'hello' from file OCR"
    );
}

/// Test custom endpoint via builder.
#[tokio::test]
#[ignore = "requires running vLLM with a vision model"]
async fn test_vllm_builder_config() {
    if !vllm_available() {
        eprintln!("Skipping: vLLM not available at localhost:8000");
        return;
    }

    let backend = VllmOcrBackend::builder()
        .endpoint("http://localhost:8000")
        .model("zai-org/GLM-OCR")
        .prompt("What text is in this image? Return only the text.")
        .build();

    let image_bytes = std::fs::read(test_image_path("test_hello_world.png")).expect("test image");

    let config = OcrConfig {
        backend: "vllm".to_string(),
        language: "eng".to_string(),
        ..Default::default()
    };

    let result = backend.process_image(&image_bytes, &config).await.unwrap();
    assert!(!result.content.is_empty(), "Builder-configured backend should work");
}

/// Test that wrong endpoint gives a clear error.
#[tokio::test]
#[ignore = "requires tokio runtime"]
async fn test_vllm_connection_error() {
    let backend = VllmOcrBackend::builder().endpoint("http://localhost:99999").build();

    let config = OcrConfig {
        backend: "vllm".to_string(),
        language: "eng".to_string(),
        ..Default::default()
    };

    let result = backend.process_image(b"fake image", &config).await;
    assert!(result.is_err(), "Should fail with bad endpoint");
    let err = result.unwrap_err().to_string();
    assert!(
        err.contains("vLLM") || err.contains("request") || err.contains("connect"),
        "Error should mention connection failure: {}",
        err
    );
}

/// Test plugin interface.
#[test]
fn test_vllm_plugin_interface() {
    let backend = VllmOcrBackend::builder().build();
    assert_eq!(backend.name(), "vllm");
    assert!(backend.supports_language("eng"));
    assert!(backend.supports_language("chi"));
    assert!(backend.supports_language("unknown_lang")); // vision models accept anything
}
