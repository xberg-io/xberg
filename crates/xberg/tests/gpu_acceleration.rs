//! GPU acceleration integration tests for all ORT-backed subsystems.
//!
//! Covers every code path that uses AccelerationConfig → apply_execution_providers:
//! 1. PaddleOCR (det + cls + rec) — feature: paddle-ocr
//! 2. Layout detection (RT-DETR) — feature: layout-detection
//! 3. Embeddings (ONNX models) — feature: embeddings
//! 4. Document orientation (auto-rotate via paddle-ocr)
//! 5. End-to-end extraction with CUDA acceleration
//!
//! All tests are `#[ignore]` and require:
//! - NVIDIA GPU with CUDA
//! - ONNX Runtime built with CUDA EP
//! - Network access (models auto-downloaded from HuggingFace)
//!
//! Run all GPU tests:
//!   cargo test -p xberg --features "full,ort-dynamic,cuda" --test gpu_acceleration -- --ignored
//!
//! The `ort-dynamic` feature overrides the bundled CPU-only ORT so a
//! GPU-enabled ONNX Runtime (via `ORT_DYLIB_PATH`) is loaded at runtime. The `cuda`
//! feature compiles in `ort::ep::CUDA` (gated out of `full` by default — see
//! `ort_discovery.rs` — since the plain `download-binaries` prebuilt has no CUDA support).

#![allow(clippy::print_stdout, clippy::print_stderr, clippy::dbg_macro)] // ~keep: test/bench binaries print by design; org logging policy exempts tests
#![allow(dead_code)]

mod helpers;

use std::path::PathBuf;
use std::sync::{Arc, Mutex};

fn test_documents_dir() -> PathBuf {
    if let Ok(dir) = std::env::var("TEST_DOCUMENTS_DIR") {
        return PathBuf::from(dir);
    }
    PathBuf::from(env!("CARGO_MANIFEST_DIR"))
        .parent()
        .unwrap()
        .parent()
        .unwrap()
        .join("test_documents")
}

#[cfg(paddle_ocr)]
fn test_cache_dir() -> PathBuf {
    std::env::temp_dir().join("xberg_gpu_test")
}

fn cuda_accel() -> xberg::AccelerationConfig {
    xberg::AccelerationConfig {
        provider: xberg::ExecutionProviderType::Cuda,
        device_id: 0,
    }
}

struct LogCapture {
    messages: Arc<Mutex<Vec<String>>>,
}

impl<S: tracing::Subscriber> tracing_subscriber::Layer<S> for LogCapture {
    fn on_event(&self, event: &tracing::Event<'_>, _ctx: tracing_subscriber::layer::Context<'_, S>) {
        let mut visitor = MessageVisitor(String::new());
        event.record(&mut visitor);
        if let Ok(mut msgs) = self.messages.lock() {
            msgs.push(visitor.0);
        }
    }
}

struct MessageVisitor(String);

impl tracing::field::Visit for MessageVisitor {
    fn record_debug(&mut self, field: &tracing::field::Field, value: &dyn std::fmt::Debug) {
        if field.name() == "message" {
            self.0 = format!("{:?}", value);
        }
    }

    fn record_str(&mut self, field: &tracing::field::Field, value: &str) {
        if field.name() == "message" {
            self.0 = value.to_string();
        }
    }
}

fn setup_log_capture() -> (Arc<Mutex<Vec<String>>>, tracing::subscriber::DefaultGuard) {
    use tracing_subscriber::layer::SubscriberExt;

    let captured = Arc::new(Mutex::new(Vec::<String>::new()));
    let layer = LogCapture {
        messages: Arc::clone(&captured),
    };
    let subscriber = tracing_subscriber::registry().with(layer);
    let guard = tracing::subscriber::set_default(subscriber);
    (captured, guard)
}

fn assert_cuda_requested(captured: &Arc<Mutex<Vec<String>>>) {
    let logs = captured.lock().unwrap();
    let cuda_active = logs
        .iter()
        .any(|msg| msg.contains("CUDA execution provider available") || msg.contains("CUDA available, using GPU"));
    assert!(
        cuda_active,
        "CUDA EP was NOT activated — AccelerationConfig not propagated to ORT session. \
         Captured logs:\n{}",
        logs.iter()
            .filter(|m| !m.is_empty())
            .cloned()
            .collect::<Vec<_>>()
            .join("\n")
    );
}

#[cfg(paddle_ocr)]
mod paddle_ocr_cuda {
    use super::*;
    use xberg::core::config::OcrConfig;
    use xberg::paddle_ocr::{PaddleOcrBackend, PaddleOcrConfig};
    use xberg::plugins::OcrBackend;

    #[tokio::test]
    #[ignore = "gpu: requires CUDA + ONNX Runtime CUDA EP"]
    async fn hello_world() {
        let (captured, _guard) = setup_log_capture();

        let image_path = test_documents_dir().join("images/test_hello_world.png");
        let image_bytes = std::fs::read(&image_path).expect("Failed to read test image");

        let paddle_config = PaddleOcrConfig::new("en").with_cache_dir(test_cache_dir());
        let backend = PaddleOcrBackend::with_config(paddle_config).expect("Failed to create backend");

        let ocr_config = OcrConfig {
            backend: "paddle-ocr".to_string(),
            language: vec!["en".to_string()],
            acceleration: Some(cuda_accel()),
            ..Default::default()
        };

        let result = backend.process_image(&image_bytes, &ocr_config).await;
        assert!(result.is_ok(), "PaddleOCR CUDA failed: {:?}", result.err());
        assert_cuda_requested(&captured);

        let text = result.unwrap().content.to_lowercase();
        assert!(
            text.contains("hello") || text.contains("helo"),
            "Expected 'hello': {text}"
        );
    }

    #[tokio::test]
    #[ignore = "gpu: requires CUDA + ONNX Runtime CUDA EP"]
    async fn complex_document() {
        let (captured, _guard) = setup_log_capture();

        let image_path = test_documents_dir().join("images/ocr_image.jpg");
        let image_bytes = std::fs::read(&image_path).expect("Failed to read test image");

        let paddle_config = PaddleOcrConfig::new("en").with_cache_dir(test_cache_dir());
        let backend = PaddleOcrBackend::with_config(paddle_config).expect("Failed to create backend");

        let ocr_config = OcrConfig {
            backend: "paddle-ocr".to_string(),
            language: vec!["en".to_string()],
            acceleration: Some(cuda_accel()),
            ..Default::default()
        };

        let result = backend.process_image(&image_bytes, &ocr_config).await;
        assert!(result.is_ok(), "PaddleOCR CUDA failed: {:?}", result.err());
        assert_cuda_requested(&captured);

        let text = result.unwrap().content.to_uppercase();
        assert!(text.contains("NASDAQ") || text.contains("NASOAQ"), "Expected 'NASDAQ'");
    }

    #[tokio::test]
    #[ignore = "gpu: requires CUDA + ONNX Runtime CUDA EP"]
    async fn chinese() {
        let (captured, _guard) = setup_log_capture();

        let image_path = test_documents_dir().join("images/chi_sim_image.jpeg");
        let image_bytes = std::fs::read(&image_path).expect("Failed to read test image");

        let paddle_config = PaddleOcrConfig::new("ch").with_cache_dir(test_cache_dir());
        let backend = PaddleOcrBackend::with_config(paddle_config).expect("Failed to create backend");

        let ocr_config = OcrConfig {
            backend: "paddle-ocr".to_string(),
            language: vec!["ch".to_string()],
            acceleration: Some(cuda_accel()),
            ..Default::default()
        };

        let result = backend.process_image(&image_bytes, &ocr_config).await;
        assert!(result.is_ok(), "PaddleOCR CUDA Chinese failed: {:?}", result.err());
        assert_cuda_requested(&captured);
        assert!(!result.unwrap().content.is_empty(), "Chinese OCR should produce text");
    }

    #[tokio::test]
    #[ignore = "gpu: requires CUDA + ONNX Runtime CUDA EP + ~170MB server models"]
    async fn server_tier() {
        let (captured, _guard) = setup_log_capture();

        let image_path = test_documents_dir().join("images/ocr_image.jpg");
        let image_bytes = std::fs::read(&image_path).expect("Failed to read test image");

        let paddle_config = PaddleOcrConfig::new("en")
            .with_model_tier("server")
            .with_cache_dir(test_cache_dir());
        let backend = PaddleOcrBackend::with_config(paddle_config).expect("Failed to create backend");

        let ocr_config = OcrConfig {
            backend: "paddle-ocr".to_string(),
            language: vec!["en".to_string()],
            acceleration: Some(cuda_accel()),
            ..Default::default()
        };

        let result = backend.process_image(&image_bytes, &ocr_config).await;
        assert!(result.is_ok(), "PaddleOCR CUDA server-tier failed: {:?}", result.err());
        assert_cuda_requested(&captured);
        assert!(!result.unwrap().content.is_empty());
    }
}

#[cfg(feature = "layout-detection")]
mod layout_detection_cuda {
    use super::*;
    use xberg::layout::{LayoutEngine, LayoutEngineConfig, ModelBackend};

    #[test]
    #[ignore = "gpu: requires CUDA + ONNX Runtime CUDA EP + layout model"]
    fn rtdetr_complex_document() {
        let (captured, _guard) = setup_log_capture();

        let image_path = test_documents_dir().join("images/complex_document.png");
        let img = image::open(&image_path).expect("Failed to open image").to_rgb8();

        let config = LayoutEngineConfig {
            backend: ModelBackend::RtDetr,
            acceleration: Some(cuda_accel()),
            ..Default::default()
        };

        let mut engine = LayoutEngine::from_config(config).expect("Failed to create layout engine");
        let result = engine.detect(&img);
        assert!(result.is_ok(), "Layout CUDA failed: {:?}", result.err());
        assert_cuda_requested(&captured);

        let detections = result.unwrap();
        assert!(!detections.detections.is_empty(), "Should detect layout regions");
        println!("CUDA RT-DETR detected {} regions", detections.detections.len());
    }

    #[test]
    #[ignore = "gpu: requires CUDA + ONNX Runtime CUDA EP + layout model"]
    fn rtdetr_table_detection() {
        let (captured, _guard) = setup_log_capture();

        let image_path = test_documents_dir().join("images/simple_table.png");
        let img = image::open(&image_path).expect("Failed to open image").to_rgb8();

        let config = LayoutEngineConfig {
            backend: ModelBackend::RtDetr,
            acceleration: Some(cuda_accel()),
            ..Default::default()
        };

        let mut engine = LayoutEngine::from_config(config).expect("Failed to create layout engine");
        let result = engine.detect(&img);
        assert!(result.is_ok(), "Layout CUDA table detection failed: {:?}", result.err());
        assert_cuda_requested(&captured);

        let detections = result.unwrap();
        let has_table = detections
            .detections
            .iter()
            .any(|d| d.class_name == xberg::layout::LayoutClass::Table);
        println!(
            "CUDA layout: {} regions, table detected: {}",
            detections.detections.len(),
            has_table
        );
    }
}

#[cfg(feature = "embeddings")]
mod embeddings_cuda {
    use super::*;
    use xberg::core::config::{EmbeddingConfig, EmbeddingModelType};

    #[test]
    #[ignore = "gpu: requires CUDA + ONNX Runtime CUDA EP + embedding model"]
    fn fast_preset() {
        let (captured, _guard) = setup_log_capture();

        let config = EmbeddingConfig {
            model: EmbeddingModelType::Preset {
                name: "fast".to_string(),
            },
            acceleration: Some(cuda_accel()),
            ..Default::default()
        };

        let result = xberg::embed_texts(
            vec![
                "Hello, world!".to_string(),
                "GPU-accelerated embeddings test".to_string(),
            ],
            &config,
        );
        assert!(result.is_ok(), "Embedding CUDA failed: {:?}", result.err());
        assert_cuda_requested(&captured);

        let embeddings = result.unwrap();
        assert_eq!(embeddings.len(), 2);
        assert_eq!(embeddings[0].len(), 384, "fast preset = 384 dims");
    }

    #[test]
    #[ignore = "gpu: requires CUDA + ONNX Runtime CUDA EP + embedding model"]
    fn balanced_preset() {
        let (captured, _guard) = setup_log_capture();

        let config = EmbeddingConfig {
            model: EmbeddingModelType::Preset {
                name: "balanced".to_string(),
            },
            acceleration: Some(cuda_accel()),
            ..Default::default()
        };

        let result = xberg::embed_texts(vec!["Document intelligence with GPU acceleration".to_string()], &config);
        assert!(result.is_ok(), "Embedding CUDA balanced failed: {:?}", result.err());
        assert_cuda_requested(&captured);

        let embeddings = result.unwrap();
        assert_eq!(embeddings[0].len(), 768, "balanced preset = 768 dims");
    }
}

#[cfg(paddle_ocr)]
mod doc_orientation_cuda {
    use super::*;
    use xberg::core::config::OcrConfig;
    use xberg::paddle_ocr::{PaddleOcrBackend, PaddleOcrConfig};
    use xberg::plugins::OcrBackend;

    #[tokio::test]
    #[ignore = "gpu: requires CUDA + ONNX Runtime CUDA EP + orientation model"]
    async fn auto_rotate_rotated_180() {
        let (captured, _guard) = setup_log_capture();

        let image_path = test_documents_dir().join("images/complex_document_rotated_180.png");
        let image_bytes = std::fs::read(&image_path).expect("Failed to read rotated image");

        let paddle_config = PaddleOcrConfig::new("en").with_cache_dir(test_cache_dir());
        let backend = PaddleOcrBackend::with_config(paddle_config).expect("Failed to create backend");

        let ocr_config = OcrConfig {
            backend: "paddle-ocr".to_string(),
            language: vec!["en".to_string()],
            auto_rotate: Some(true),
            acceleration: Some(cuda_accel()),
            ..Default::default()
        };

        let result = backend.process_image(&image_bytes, &ocr_config).await;
        assert!(result.is_ok(), "CUDA auto-rotate failed: {:?}", result.err());
        assert_cuda_requested(&captured);

        let text = result.unwrap().content;
        assert!(!text.is_empty(), "Auto-rotated CUDA OCR should produce text");
    }
}

#[cfg(paddle_ocr)]
mod e2e_cuda {
    use super::*;
    use xberg::core::config::OcrConfig;

    #[tokio::test]
    #[ignore = "gpu: requires CUDA + ONNX Runtime CUDA EP + models"]
    async fn extract_image_bytes() {
        let (captured, _guard) = setup_log_capture();

        let image_path = test_documents_dir().join("images/test_hello_world.png");
        let image_bytes = std::fs::read(&image_path).expect("Failed to read image");

        let config = xberg::ExtractionConfig {
            ocr: Some(OcrConfig {
                backend: "paddle-ocr".to_string(),
                language: vec!["en".to_string()],
                ..Default::default()
            }),
            acceleration: Some(cuda_accel()),
            ..Default::default()
        };

        let result = helpers::extract_bytes_document(&image_bytes, "image/png", &config).await;
        assert!(result.is_ok(), "E2E CUDA extraction failed: {:?}", result.err());
        assert_cuda_requested(&captured);

        let text = result.unwrap().content.to_lowercase();
        assert!(text.contains("hello"), "E2E CUDA should contain 'hello': {text}");
    }
}
