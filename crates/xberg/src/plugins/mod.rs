//! Plugin system for extending Xberg functionality.
//!
//! The plugin system provides a trait-based architecture that allows extending
//! Xberg with custom extractors, OCR backends, post-processors, and validators.
//!
//! # Plugin Types
//!
//! - [`Plugin`] - Base trait that all plugins must implement
//! - [`OcrBackend`] - OCR processing plugins
//! - [`EmbeddingBackend`] - In-process embedding backend plugins
//! - [`DocumentExtractor`] - Document format extraction plugins
//! - [`PostProcessor`] - Content post-processing plugins
//! - [`Validator`] - Validation plugins
//!
//! # Language Support
//!
//! Plugins can be implemented in:
//! - **Rust** (native, highest performance)
//! - **Python** (via PyO3 FFI bridge)
//! - **Node.js** (future - via napi-rs FFI bridge)
//!
//! # Lifecycle Pattern
//!
//! Plugins are stored in `Arc<dyn Trait>` for thread-safe shared access:
//!
//! ```rust
//! use xberg::plugins::{Plugin, DocumentExtractor};
//! use xberg::plugins::registry::get_document_extractor_registry;
//! use std::sync::Arc;
//!
//! # struct MyExtractor;
//! # use xberg::{ExtractInput, ExtractionConfig, ExtractedDocument};
//! # impl xberg::plugins::Plugin for MyExtractor {
//! #     fn name(&self) -> &str { "my" }
//! #     fn version(&self) -> String { "1.0.0".to_string() }
//! #     fn initialize(&self) -> xberg::Result<()> { Ok(()) }
//! #     fn shutdown(&self) -> xberg::Result<()> { Ok(()) }
//! # }
//! # #[async_trait::async_trait]
//! # impl DocumentExtractor for MyExtractor {
//! #     async fn extract(&self, _: ExtractInput, _: &ExtractionConfig) -> xberg::Result<ExtractedDocument> {
//! #         Ok(ExtractedDocument::default())
//! #     }
//! #     fn supported_mime_types(&self) -> &[&str] { &[] }
//! #     fn priority(&self) -> i32 { 50 }
//! # }
//! // 1. Create plugin instance
//! let plugin = MyExtractor;
//!
//! // 2. Wrap in Arc for registration
//! let plugin = Arc::new(plugin);
//!
//! // 3. Register with registry (calls initialize internally)
//! let registry = get_document_extractor_registry();
//! let mut registry = registry.write();
//! registry.register(plugin)?;
//! # Ok::<(), xberg::XbergError>(())
//! ```
//!
//! # Example: Custom Document Extractor
//!
//! ```rust
//! use xberg::plugins::{Plugin, DocumentExtractor};
//! use xberg::{ExtractInput, ExtractionConfig, Result};
//! use xberg::types::{ExtractedDocument, Metadata};
//! use async_trait::async_trait;
//!
//! struct CustomJsonExtractor;
//!
//! impl Plugin for CustomJsonExtractor {
//!     fn name(&self) -> &str { "custom-json-extractor" }
//!     fn version(&self) -> String { "1.0.0".to_string() }
//!     fn initialize(&self) -> Result<()> {
//!         println!("JSON extractor initialized");
//!         Ok(())
//!     }
//!     fn shutdown(&self) -> Result<()> {
//!         println!("JSON extractor shutdown");
//!         Ok(())
//!     }
//! }
//!
//! #[async_trait]
//! impl DocumentExtractor for CustomJsonExtractor {
//!     async fn extract(&self, input: ExtractInput, _config: &ExtractionConfig)
//!         -> Result<ExtractedDocument> {
//!         // Parse JSON and extract all string values
//!         let content = input.bytes.unwrap_or_default();
//!         let json: serde_json::Value = serde_json::from_slice(&content)?;
//!         let extracted_text = extract_strings_from_json(&json);
//!
//!         let mut metadata = Metadata::default();
//!         metadata.additional.insert("extracted_fields".to_string().into(), serde_json::json!(true));
//!
//!         Ok(ExtractedDocument {
//!             content: extracted_text,
//!             mime_type: std::borrow::Cow::Borrowed("application/json"),
//!             metadata,
//!             ..Default::default()
//!         })
//!     }
//!
//!     fn supported_mime_types(&self) -> &[&str] {
//!         &["application/json", "text/json"]
//!     }
//!
//!     fn priority(&self) -> i32 { 50 } // Default priority
//! }
//!
//! fn extract_strings_from_json(value: &serde_json::Value) -> String {
//!     match value {
//!         serde_json::Value::String(s) => format!("{}\n", s),
//!         serde_json::Value::Array(arr) => {
//!             arr.iter().map(extract_strings_from_json).collect()
//!         }
//!         serde_json::Value::Object(obj) => {
//!             obj.values().map(extract_strings_from_json).collect()
//!         }
//!         _ => String::new(),
//!     }
//! }
//! ```
//!
//! # Safety and Threading
//!
//! **CRITICAL**: All plugins must be `Send + Sync` because they are:
//! - Stored in `Arc<dyn Trait>` for shared ownership
//! - Accessed concurrently from multiple threads
//! - Called with `&self` (shared references)
//!
//! **Interior Mutability Pattern**:
//! Since plugins receive `&self` (not `&mut self`), use these for mutable state:
//! - `Mutex<T>` - Exclusive access, blocking
//! - `RwLock<T>` - Shared read, exclusive write
//! - `AtomicBool` / `AtomicU64` - Lock-free primitives
//! - `OnceCell<T>` - One-time initialization
//!
//! ```rust
//! use xberg::plugins::Plugin;
//! use std::sync::Mutex;
//!
//! struct StatefulPlugin {
//!     // Use interior mutability for state
//!     call_count: std::sync::atomic::AtomicU64,
//!     cache: Mutex<Option<Vec<String>>>,
//! }
//!
//! impl Plugin for StatefulPlugin {
//!     fn name(&self) -> &str { "stateful-plugin" }
//!     fn version(&self) -> String { "1.0.0".to_string() }
//!
//!     fn initialize(&self) -> xberg::Result<()> {
//!         // Modify through interior mutability
//!         let mut cache = self.cache.lock().unwrap();
//!         *cache = Some(vec!["initialized".to_string()]);
//!         Ok(())
//!     }
//!
//!     fn shutdown(&self) -> xberg::Result<()> {
//!         self.call_count.store(0, std::sync::atomic::Ordering::Release);
//!         Ok(())
//!     }
//! }
//! ```

pub(crate) mod embedding;
pub(crate) mod extractor;
mod ocr;
pub mod processor;
pub mod registry;
pub mod renderer;
pub(crate) mod reranker;
pub mod startup_validation;
pub(crate) mod tokenizer;
mod traits;
pub mod validator;

pub use embedding::{
    EmbeddingBackend, clear_embedding_backends, list_embedding_backends, register_embedding_backend,
    unregister_embedding_backend,
};
pub use extractor::{
    DocumentExtractor, InternalDocumentExtractor, clear_document_extractors, list_document_extractors,
    register_document_extractor, unregister_document_extractor,
};
pub use ocr::{
    OcrBackend, OcrBackendType, clear_ocr_backends, list_ocr_backends, register_ocr_backend, unregister_ocr_backend,
};
pub use processor::{
    PostProcessor, ProcessingStage, clear_post_processors, list_post_processors, register_post_processor,
    unregister_post_processor,
};
pub(crate) use renderer::InternalRenderer;
pub use renderer::{Renderer, clear_renderers, list_renderers, register_renderer, unregister_renderer};
pub use reranker::{
    RerankerBackend, clear_reranker_backends, list_reranker_backends, register_reranker_backend,
    unregister_reranker_backend,
};
pub use tokenizer::{
    TokenizerBackend, clear_tokenizer_backends, list_tokenizer_backends, register_tokenizer_backend,
    unregister_tokenizer_backend,
};
pub use traits::Plugin;
pub use validator::{Validator, clear_validators, list_validators, register_validator, unregister_validator};

// Alef trait-bridge codegen derives `xberg::plugins::{trait_snake}::{fn_name}`
// from the `registry_getter = "...::get_{trait_snake}_registry"` config (see
// `host_function_path` in alef-codegen). Our actual modules are named differently
// (`ocr` not `ocr_backend`, `processor` not `post_processor`, `embedding` not
// `embedding_backend`), and `validator` / `renderer` are private. These public
// alias modules expose the lifecycle wrappers at the alef-derived path so the
// generated code resolves without forcing a xberg-side rename or alef-side
// path-override field.
/// Re-exports for the OCR backend plugin type, used by alef-generated bindings.
pub mod ocr_backend {
    pub use super::{OcrBackend, clear_ocr_backends, list_ocr_backends, register_ocr_backend, unregister_ocr_backend};
}
/// Re-exports for the post-processor plugin type, used by alef-generated bindings.
pub mod post_processor {
    pub use super::{
        PostProcessor, clear_post_processors, list_post_processors, register_post_processor, unregister_post_processor,
    };
}
/// Re-exports for the embedding backend plugin type, used by alef-generated bindings.
pub mod embedding_backend {
    pub use super::{
        EmbeddingBackend, clear_embedding_backends, list_embedding_backends, register_embedding_backend,
        unregister_embedding_backend,
    };
}
/// Re-exports for the reranker backend plugin type, used by alef-generated bindings.
///
/// Since v5.0.0.
pub mod reranker_backend {
    pub use super::{
        RerankerBackend, clear_reranker_backends, list_reranker_backends, register_reranker_backend,
        unregister_reranker_backend,
    };
}
/// Re-exports for the tokenizer backend plugin type, used by alef-generated bindings.
pub mod tokenizer_backend {
    pub use super::{
        TokenizerBackend, clear_tokenizer_backends, list_tokenizer_backends, register_tokenizer_backend,
        unregister_tokenizer_backend,
    };
}
/// Re-exports for the document extractor plugin type, used by alef-generated bindings.
pub mod document_extractor {
    pub use super::{
        DocumentExtractor, clear_document_extractors, list_document_extractors, register_document_extractor,
        unregister_document_extractor,
    };
}

#[cfg(feature = "embeddings")]
pub(crate) use registry::get_embedding_backend_registry;

// Self-healing initializer for the global OCR backend registry. Re-exported at
// crate visibility so the image extractor can re-seed the built-in backends
// after `clear_ocr_backends` empties the registry.
#[cfg(any(feature = "ocr", feature = "ocr-wasm", feature = "ocr-pipeline"))]
pub(crate) use ocr::ensure_ocr_backends_initialized;
