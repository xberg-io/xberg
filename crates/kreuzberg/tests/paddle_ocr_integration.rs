//! Integration tests for PaddleOCR functionality.
//!
//! These tests require:
//! - Network access to download models from HuggingFace
//! - ONNX Runtime installed on the system
//!
//! Run with: `cargo test -p kreuzberg --features paddle-ocr --test paddle_ocr_integration -- --ignored`

#![cfg(feature = "paddle-ocr")]

use std::path::PathBuf;

use kreuzberg::core::config::OcrConfig;
use kreuzberg::paddle_ocr::{ModelManager, PaddleOcrBackend, PaddleOcrConfig};
use kreuzberg::plugins::OcrBackend;
use kreuzberg::types::ExtractionResult;

/// Helper to get the test documents directory
fn test_documents_dir() -> PathBuf {
    PathBuf::from(env!("CARGO_MANIFEST_DIR"))
        .parent()
        .unwrap()
        .parent()
        .unwrap()
        .join("test_documents")
}

/// Helper to get a temporary cache directory for tests
fn test_cache_dir() -> PathBuf {
    std::env::temp_dir().join("kreuzberg_paddle_test")
}

/// Test that model manager can download models from HuggingFace.
///
/// This test downloads actual models and verifies they are cached correctly.
/// It's ignored by default since it requires network access and ~100MB download.
#[tokio::test]
#[ignore = "requires network access and ~100MB download"]
async fn test_model_download_from_huggingface() {
    let cache_dir = test_cache_dir();

    // Clean up any existing cache
    let _ = std::fs::remove_dir_all(&cache_dir);

    let manager = ModelManager::new(cache_dir.clone());

    // Verify cache is empty
    assert!(!manager.are_models_cached());

    // Download models (synchronous now)
    let result = manager.ensure_models_exist();
    assert!(result.is_ok(), "Model download failed: {:?}", result.err());

    let paths: kreuzberg::paddle_ocr::ModelPaths = result.unwrap();

    // Verify all model directories exist
    assert!(paths.det_model.exists(), "Detection model dir not found");
    assert!(paths.cls_model.exists(), "Classification model dir not found");
    assert!(paths.rec_model.exists(), "Recognition model dir not found");

    // Verify ONNX model files exist within directories
    assert!(paths.det_model.join("model.onnx").exists(), "Detection ONNX file not found");
    assert!(paths.cls_model.join("model.onnx").exists(), "Classification ONNX file not found");
    assert!(paths.rec_model.join("model.onnx").exists(), "Recognition ONNX file not found");

    // Verify dictionary file exists
    assert!(paths.dict_file.exists(), "Dictionary file not found");

    // Verify cache reports correctly
    assert!(manager.are_models_cached());

    // Check cache stats
    let stats = manager.cache_stats().unwrap();
    // 3 model dirs, each containing model.onnx (rec/ also has dict.txt)
    assert!(stats.model_count >= 3, "Expected at least 3 cached items, got {}", stats.model_count);
    // Models should be > 1MB each
    assert!(stats.total_size_bytes > 1_000_000);

    println!("Cache stats: {:?}", stats);
    println!("Detection model: {:?}", paths.det_model);
    println!("Classification model: {:?}", paths.cls_model);
    println!("Recognition model: {:?}", paths.rec_model);
    println!("Dictionary file: {:?}", paths.dict_file);
}

/// Test OCR on a simple English "Hello World" image.
///
/// This test requires ONNX Runtime and downloaded models.
#[tokio::test]
#[ignore = "requires ONNX Runtime and downloaded models"]
async fn test_ocr_hello_world_english() {
    let image_path = test_documents_dir().join("images/test_hello_world.png");
    assert!(image_path.exists(), "Test image not found: {:?}", image_path);

    let image_bytes = std::fs::read(&image_path).expect("Failed to read image");

    let config = PaddleOcrConfig::new("en").with_cache_dir(test_cache_dir());

    let backend = PaddleOcrBackend::with_config(config).expect("Failed to create backend");

    let ocr_config = OcrConfig {
        backend: "paddle-ocr".to_string(),
        language: "en".to_string(),
        ..Default::default()
    };

    let result: kreuzberg::Result<ExtractionResult> = backend.process_image(&image_bytes, &ocr_config).await;
    assert!(result.is_ok(), "OCR failed: {:?}", result.err());

    let extraction: ExtractionResult = result.unwrap();
    let text = extraction.content.to_lowercase();

    println!("OCR result: {}", extraction.content);

    // Should contain "hello" and "world"
    assert!(
        text.contains("hello") || text.contains("helo"),
        "Expected 'hello' in OCR result: {}",
        text
    );
    assert!(
        text.contains("world") || text.contains("worid"),
        "Expected 'world' in OCR result: {}",
        text
    );
}

/// Test OCR on a complex English document (newspaper).
#[tokio::test]
#[ignore = "requires ONNX Runtime and downloaded models"]
async fn test_ocr_newspaper_english() {
    let image_path = test_documents_dir().join("images/ocr_image.jpg");
    assert!(image_path.exists(), "Test image not found: {:?}", image_path);

    let image_bytes = std::fs::read(&image_path).expect("Failed to read image");

    let config = PaddleOcrConfig::new("en").with_cache_dir(test_cache_dir());

    let backend = PaddleOcrBackend::with_config(config).expect("Failed to create backend");

    let ocr_config = OcrConfig {
        backend: "paddle-ocr".to_string(),
        language: "en".to_string(),
        ..Default::default()
    };

    let result: kreuzberg::Result<ExtractionResult> = backend.process_image(&image_bytes, &ocr_config).await;
    assert!(result.is_ok(), "OCR failed: {:?}", result.err());

    let extraction: ExtractionResult = result.unwrap();
    let text = extraction.content.to_uppercase();

    println!(
        "OCR result (first 500 chars): {}",
        &extraction.content[..extraction.content.len().min(500)]
    );

    // Should contain "NASDAQ" and "AMEX" from the header
    assert!(
        text.contains("NASDAQ") || text.contains("NASOAQ"),
        "Expected 'NASDAQ' in OCR result"
    );
    assert!(
        text.contains("AMEX") || text.contains("STOCK"),
        "Expected 'AMEX' or 'STOCK' in OCR result"
    );
}

/// Test OCR on Chinese text image.
///
/// Note: Currently using English-only recognition model (en_PP-OCRv4_rec_infer.onnx).
/// This test verifies the pipeline handles non-English images without crashing,
/// but cannot produce Chinese characters until a Chinese recognition model is added.
#[tokio::test]
#[ignore = "requires ONNX Runtime and downloaded models"]
async fn test_ocr_chinese_text() {
    let image_path = test_documents_dir().join("images/chi_sim_image.jpeg");
    assert!(image_path.exists(), "Test image not found: {:?}", image_path);

    let image_bytes = std::fs::read(&image_path).expect("Failed to read image");

    // Use Chinese language setting
    let config = PaddleOcrConfig::new("ch").with_cache_dir(test_cache_dir());

    let backend = PaddleOcrBackend::with_config(config).expect("Failed to create backend");

    let ocr_config = OcrConfig {
        backend: "paddle-ocr".to_string(),
        language: "ch".to_string(),
        ..Default::default()
    };

    let result: kreuzberg::Result<ExtractionResult> = backend.process_image(&image_bytes, &ocr_config).await;
    assert!(result.is_ok(), "OCR failed: {:?}", result.err());

    let extraction: ExtractionResult = result.unwrap();

    println!("OCR result: {}", extraction.content);

    // The pipeline should produce some output without crashing.
    // With the English-only model, Chinese characters are not recognized,
    // but the detection and recognition pipeline should still function.
    assert!(
        !extraction.content.is_empty(),
        "Expected non-empty OCR result for Chinese image"
    );
}

/// Test that the backend correctly reports supported languages.
#[test]
fn test_supported_languages() {
    let backend = PaddleOcrBackend::new().expect("Failed to create backend");

    // Direct PaddleOCR codes
    assert!(backend.supports_language("ch"));
    assert!(backend.supports_language("en"));
    assert!(backend.supports_language("japan"));
    assert!(backend.supports_language("korean"));

    // Mapped Tesseract/ISO codes
    assert!(backend.supports_language("chi_sim"));
    assert!(backend.supports_language("eng"));
    assert!(backend.supports_language("jpn"));
    assert!(backend.supports_language("fra"));
    assert!(backend.supports_language("deu"));

    // Unsupported
    assert!(!backend.supports_language("xyz"));
    assert!(!backend.supports_language("klingon"));
}

/// Test that empty image returns an error.
#[tokio::test]
async fn test_empty_image_error() {
    let backend = PaddleOcrBackend::new().expect("Failed to create backend");

    let ocr_config = OcrConfig {
        backend: "paddle-ocr".to_string(),
        language: "en".to_string(),
        ..Default::default()
    };

    let result: kreuzberg::Result<ExtractionResult> = backend.process_image(&[], &ocr_config).await;
    assert!(result.is_err(), "Expected error for empty image");
}

/// Test that invalid image data returns an error (requires ONNX Runtime).
#[tokio::test]
#[ignore = "requires ONNX Runtime and downloaded models"]
async fn test_invalid_image_error() {
    let config = PaddleOcrConfig::new("en").with_cache_dir(test_cache_dir());
    let backend = PaddleOcrBackend::with_config(config).expect("Failed to create backend");

    let ocr_config = OcrConfig {
        backend: "paddle-ocr".to_string(),
        language: "en".to_string(),
        ..Default::default()
    };

    // Random bytes that aren't a valid image
    let invalid_bytes = vec![0u8, 1, 2, 3, 4, 5, 6, 7, 8, 9];

    let result: kreuzberg::Result<ExtractionResult> = backend.process_image(&invalid_bytes, &ocr_config).await;
    assert!(result.is_err(), "Expected error for invalid image data");
}

/// Test processing an image file directly.
#[tokio::test]
#[ignore = "requires ONNX Runtime and downloaded models"]
async fn test_process_file() {
    let image_path = test_documents_dir().join("images/test_hello_world.png");
    assert!(image_path.exists(), "Test image not found: {:?}", image_path);

    let config = PaddleOcrConfig::new("en").with_cache_dir(test_cache_dir());
    let backend = PaddleOcrBackend::with_config(config).expect("Failed to create backend");

    let ocr_config = OcrConfig {
        backend: "paddle-ocr".to_string(),
        language: "en".to_string(),
        ..Default::default()
    };

    let result: kreuzberg::Result<ExtractionResult> = backend.process_file(&image_path, &ocr_config).await;
    assert!(result.is_ok(), "OCR failed: {:?}", result.err());

    let extraction: ExtractionResult = result.unwrap();
    let text = extraction.content.to_lowercase();

    assert!(
        text.contains("hello") || text.contains("helo"),
        "Expected 'hello' in OCR result"
    );
}

/// Test that explicit cache_dir in config overrides default.
#[test]
fn test_cache_dir_explicit_config() {
    // Set explicit config - this should always work regardless of env vars
    let config = PaddleOcrConfig::new("en").with_cache_dir(PathBuf::from("/explicit/path"));
    let resolved = config.resolve_cache_dir();

    // Explicit config should always win
    assert_eq!(resolved, PathBuf::from("/explicit/path"));
}

/// Test that OCR elements have proper geometry (quadrilateral bounding boxes).
#[tokio::test]
#[ignore = "requires ONNX Runtime and downloaded models"]
async fn test_paddle_ocr_elements_geometry() {
    use kreuzberg::types::OcrBoundingGeometry;

    let image_path = test_documents_dir().join("images/test_hello_world.png");
    assert!(image_path.exists(), "Test image not found: {:?}", image_path);

    let image_bytes = std::fs::read(&image_path).expect("Failed to read image");

    let config = PaddleOcrConfig::new("en").with_cache_dir(test_cache_dir());
    let backend = PaddleOcrBackend::with_config(config).expect("Failed to create backend");

    let ocr_config = OcrConfig {
        backend: "paddle-ocr".to_string(),
        language: "en".to_string(),
        ..Default::default()
    };

    let result: kreuzberg::Result<ExtractionResult> = backend.process_image(&image_bytes, &ocr_config).await;
    assert!(result.is_ok(), "OCR failed: {:?}", result.err());

    let extraction: ExtractionResult = result.unwrap();

    // Check that OCR elements are present
    assert!(
        extraction.ocr_elements.is_some(),
        "Expected ocr_elements to be populated"
    );

    let elements = extraction.ocr_elements.as_ref().unwrap();
    assert!(!elements.is_empty(), "Expected at least one OCR element");

    // Verify each element has geometry
    for element in elements {
        // Check geometry based on variant
        match &element.geometry {
            OcrBoundingGeometry::Quadrilateral { points } => {
                // Quadrilateral should have 4 points
                assert_eq!(points.len(), 4, "Quadrilateral should have 4 points");
                println!("Quadrilateral with 4 points");
            }
            OcrBoundingGeometry::Rectangle {
                left,
                top,
                width,
                height,
            } => {
                assert!(*width > 0, "Width should be positive");
                assert!(*height > 0, "Height should be positive");
                println!("Rectangle at ({}, {}) size {}x{}", left, top, width, height);
            }
        }
    }

    println!("Found {} OCR elements with valid geometry", elements.len());
}

/// Test that OCR elements have confidence scores.
#[tokio::test]
#[ignore = "requires ONNX Runtime and downloaded models"]
async fn test_paddle_ocr_elements_confidence() {
    let image_path = test_documents_dir().join("images/test_hello_world.png");
    assert!(image_path.exists(), "Test image not found: {:?}", image_path);

    let image_bytes = std::fs::read(&image_path).expect("Failed to read image");

    let config = PaddleOcrConfig::new("en").with_cache_dir(test_cache_dir());
    let backend = PaddleOcrBackend::with_config(config).expect("Failed to create backend");

    let ocr_config = OcrConfig {
        backend: "paddle-ocr".to_string(),
        language: "en".to_string(),
        ..Default::default()
    };

    let result: kreuzberg::Result<ExtractionResult> = backend.process_image(&image_bytes, &ocr_config).await;
    assert!(result.is_ok(), "OCR failed: {:?}", result.err());

    let extraction: ExtractionResult = result.unwrap();

    assert!(
        extraction.ocr_elements.is_some(),
        "Expected ocr_elements to be populated"
    );

    let elements = extraction.ocr_elements.as_ref().unwrap();
    assert!(!elements.is_empty(), "Expected at least one OCR element");

    // Verify each element has confidence score
    for element in elements {
        // Recognition confidence should be between 0 and 1
        assert!(
            element.confidence.recognition >= 0.0 && element.confidence.recognition <= 1.0,
            "Recognition confidence should be between 0 and 1, got {}",
            element.confidence.recognition
        );

        // PaddleOCR also provides detection confidence
        if let Some(det_conf) = element.confidence.detection {
            assert!(
                (0.0..=1.0).contains(&det_conf),
                "Detection confidence should be between 0 and 1, got {}",
                det_conf
            );
        }

        println!(
            "Element '{}' has recognition confidence: {:.2}%",
            element.text,
            element.confidence.recognition * 100.0
        );
    }
}

/// Test rotation detection via angle classification.
#[tokio::test]
#[ignore = "requires ONNX Runtime and downloaded models"]
async fn test_paddle_ocr_rotation_detection() {
    // Use an image that might have rotated text
    let image_path = test_documents_dir().join("images/ocr_image.jpg");
    assert!(image_path.exists(), "Test image not found: {:?}", image_path);

    let image_bytes = std::fs::read(&image_path).expect("Failed to read image");

    // Enable angle classification
    let config = PaddleOcrConfig::new("en").with_cache_dir(test_cache_dir());

    let backend = PaddleOcrBackend::with_config(config).expect("Failed to create backend");

    let ocr_config = OcrConfig {
        backend: "paddle-ocr".to_string(),
        language: "en".to_string(),
        ..Default::default()
    };

    let result: kreuzberg::Result<ExtractionResult> = backend.process_image(&image_bytes, &ocr_config).await;
    assert!(result.is_ok(), "OCR failed: {:?}", result.err());

    let extraction: ExtractionResult = result.unwrap();

    assert!(
        extraction.ocr_elements.is_some(),
        "Expected ocr_elements to be populated"
    );

    let elements = extraction.ocr_elements.as_ref().unwrap();

    // Check that rotation info is populated when available
    let elements_with_rotation = elements.iter().filter(|e| e.rotation.is_some()).count();

    println!(
        "Found {} elements total, {} with rotation info",
        elements.len(),
        elements_with_rotation
    );

    // For elements with rotation, verify the angle is valid
    for element in elements.iter().filter(|e| e.rotation.is_some()) {
        let rotation = element.rotation.as_ref().unwrap();
        // Rotation should be in degrees (typically 0, 90, 180, 270)
        assert!(
            rotation.angle_degrees >= 0.0 && rotation.angle_degrees < 360.0,
            "Rotation angle should be between 0 and 360, got {}",
            rotation.angle_degrees
        );
    }
}

/// Test table reconstruction from OCR elements.
#[tokio::test]
#[ignore = "requires ONNX Runtime and downloaded models"]
async fn test_paddle_ocr_table_reconstruction() {
    let image_path = test_documents_dir().join("images/simple_table.png");
    assert!(image_path.exists(), "Test image not found: {:?}", image_path);

    let image_bytes = std::fs::read(&image_path).expect("Failed to read image");

    // Enable table detection
    let config = PaddleOcrConfig::new("en")
        .with_cache_dir(test_cache_dir())
        .with_table_detection(true);

    let backend = PaddleOcrBackend::with_config(config).expect("Failed to create backend");

    let ocr_config = OcrConfig {
        backend: "paddle-ocr".to_string(),
        language: "en".to_string(),
        ..Default::default()
    };

    let result: kreuzberg::Result<ExtractionResult> = backend.process_image(&image_bytes, &ocr_config).await;
    assert!(result.is_ok(), "OCR failed: {:?}", result.err());

    let extraction: ExtractionResult = result.unwrap();

    println!(
        "OCR result (first 500 chars): {}",
        &extraction.content[..extraction.content.len().min(500)]
    );

    // Check if tables were detected
    if !extraction.tables.is_empty() {
        println!("Found {} tables", extraction.tables.len());
        for (i, table) in extraction.tables.iter().enumerate() {
            println!(
                "Table {}: {} rows x {} cols",
                i,
                table.cells.len(),
                table.cells.first().map(|r| r.len()).unwrap_or(0)
            );
        }
    }

    // OCR elements should also be populated
    if let Some(elements) = &extraction.ocr_elements {
        println!("Found {} OCR elements", elements.len());

        // Elements should have text content
        let non_empty_elements = elements.iter().filter(|e| !e.text.is_empty()).count();
        assert!(non_empty_elements > 0, "Expected at least one element with text");
    }
}

/// Test default cache directory when no explicit config is set.
#[test]
fn test_cache_dir_default() {
    // Save and clear env var to test default behavior
    let original = std::env::var("KREUZBERG_CACHE_DIR").ok();

    // SAFETY: This is a test that manipulates environment variables.
    // Tests should be run with --test-threads=1 if this causes issues.
    unsafe {
        std::env::remove_var("KREUZBERG_CACHE_DIR");
    }

    let config = PaddleOcrConfig::new("en");
    let resolved = config.resolve_cache_dir();

    // Default should use .kreuzberg/paddle-ocr/
    assert!(resolved.to_string_lossy().contains(".kreuzberg"));
    assert!(resolved.to_string_lossy().contains("paddle-ocr"));

    // Restore
    unsafe {
        if let Some(val) = original {
            std::env::set_var("KREUZBERG_CACHE_DIR", val);
        }
    }
}
