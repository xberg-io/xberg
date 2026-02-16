//! vLLM OCR backend using vision models.
//!
//! Sends images to a vLLM instance via the OpenAI-compatible `/v1/chat/completions`
//! endpoint and extracts text from the model's response. Works with any vLLM-hosted
//! vision model (e.g., `glm-ocr`, `Nanonets-OCR-s`, `LightOnOCR-2-1B`).
//!
//! # Quick Start
//!
//! Enable the `vllm-ocr` feature, then configure:
//!
//! ```toml
//! [dependencies]
//! kreuzberg = { version = "4.3", features = ["vllm-ocr"] }
//! ```
//!
//! The backend auto-registers as `"vllm"` and connects to `localhost:8000` by default.
//!
//! # Custom Configuration
//!
//! ```rust,no_run
//! use kreuzberg::vllm_ocr::VllmOcrBackend;
//! use kreuzberg::plugins::register_ocr_backend;
//! use std::sync::Arc;
//!
//! let backend = VllmOcrBackend::builder()
//!     .endpoint("http://my-gpu-server:8000")
//!     .model("zai-org/GLM-OCR")
//!     .api_key("my-key") // optional, for authenticated endpoints
//!     .build();
//!
//! register_ocr_backend(Arc::new(backend)).unwrap();
//! ```

mod backend;

pub use backend::{VllmOcrBackend, VllmOcrBuilder};
