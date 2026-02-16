//! Integration tests for Ollama OCR backend.
//!
//! These tests require a running Ollama instance with the glm-ocr model.
//! Run with: cargo test --features "ollama-ocr" --test ollama_ocr_integration -- --ignored

#![cfg(feature = "ollama-ocr")]

use kreuzberg::OcrConfig;
use kreuzberg::ollama_ocr::OllamaOcrBackend;
use kreuzberg::plugins::{OcrBackend, Plugin};

const OLLAMA_TAGS_URL: &str = "http://localhost:11434/api/tags";

fn model_available(model: &str) -> bool {
    let Ok(mut resp) = ureq::get(OLLAMA_TAGS_URL).call() else {
        return false;
    };
    let body: serde_json::Value = resp.body_mut().read_json().unwrap_or_default();
    body["models"].as_array().is_some_and(|models| {
        models
            .iter()
            .any(|m| m["name"].as_str().unwrap_or("").starts_with(model))
    })
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

/// Test basic English OCR with glm-ocr.
#[tokio::test]
#[ignore = "requires running Ollama with glm-ocr model"]
async fn test_ollama_hello_world() {
    if !model_available("glm-ocr") {
        eprintln!("Skipping: glm-ocr model not available");
        return;
    }

    let backend = OllamaOcrBackend::new();
    let image_bytes = std::fs::read(test_image_path("test_hello_world.png")).expect("test image");

    let config = OcrConfig {
        backend: "ollama".to_string(),
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

/// Test Chinese OCR with glm-ocr.
#[tokio::test]
#[ignore = "requires running Ollama with glm-ocr model"]
async fn test_ollama_chinese() {
    if !model_available("glm-ocr") {
        eprintln!("Skipping: glm-ocr model not available");
        return;
    }

    let backend = OllamaOcrBackend::new();
    let image_bytes = std::fs::read(test_image_path("chi_sim_image.jpeg")).expect("test image");

    let config = OcrConfig {
        backend: "ollama".to_string(),
        language: "chi".to_string(),
        ..Default::default()
    };

    let result = backend.process_image(&image_bytes, &config).await.unwrap();
    assert!(!result.content.is_empty(), "Chinese OCR should return text");
    // The image contains Chinese characters about skincare
    assert!(
        result.content.contains('肌') || result.content.contains('肤'),
        "Expected Chinese characters in output, got: {}",
        &result.content[..result.content.len().min(200)]
    );
}

/// Test that process_file works (reads file then calls process_image).
#[tokio::test]
#[ignore = "requires running Ollama with glm-ocr model"]
async fn test_ollama_process_file() {
    if !model_available("glm-ocr") {
        eprintln!("Skipping: glm-ocr model not available");
        return;
    }

    let backend = OllamaOcrBackend::new();
    let path = test_image_path("test_hello_world.png");

    let config = OcrConfig {
        backend: "ollama".to_string(),
        language: "eng".to_string(),
        ..Default::default()
    };

    let result = backend.process_file(&path, &config).await.unwrap();
    assert!(
        result.content.to_lowercase().contains("hello"),
        "Expected 'hello' from file OCR"
    );
}

/// Test custom model via builder.
#[tokio::test]
#[ignore = "requires running Ollama with glm-ocr model"]
async fn test_ollama_builder_config() {
    if !model_available("glm-ocr") {
        eprintln!("Skipping: glm-ocr model not available");
        return;
    }

    let backend = OllamaOcrBackend::builder()
        .endpoint("http://localhost:11434")
        .model("glm-ocr")
        .prompt("What text is in this image? Return only the text.")
        .build();

    let image_bytes = std::fs::read(test_image_path("test_hello_world.png")).expect("test image");

    let config = OcrConfig {
        backend: "ollama".to_string(),
        language: "eng".to_string(),
        ..Default::default()
    };

    let result = backend.process_image(&image_bytes, &config).await.unwrap();
    assert!(!result.content.is_empty(), "Builder-configured backend should work");
}

/// Test that wrong endpoint gives a clear error.
#[tokio::test]
#[ignore = "requires tokio runtime"]
async fn test_ollama_connection_error() {
    let backend = OllamaOcrBackend::builder().endpoint("http://localhost:99999").build();

    let config = OcrConfig {
        backend: "ollama".to_string(),
        language: "eng".to_string(),
        ..Default::default()
    };

    let result = backend.process_image(b"fake image", &config).await;
    assert!(result.is_err(), "Should fail with bad endpoint");
    let err = result.unwrap_err().to_string();
    assert!(
        err.contains("Ollama") || err.contains("request") || err.contains("connect"),
        "Error should mention connection failure: {}",
        err
    );
}

/// Test plugin interface.
#[test]
fn test_ollama_plugin_interface() {
    let backend = OllamaOcrBackend::builder().build();
    assert_eq!(backend.name(), "ollama");
    assert!(backend.supports_language("eng"));
    assert!(backend.supports_language("chi"));
    assert!(backend.supports_language("unknown_lang")); // vision models accept anything
}
