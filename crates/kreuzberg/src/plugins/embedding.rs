//! Embedding backend plugin trait.
//!
//! Defines the trait for implementing custom embedding backends — the in-process
//! complement to the HTTP-based [`crate::core::config::EmbeddingModelType::Llm`]
//! variant. An [`EmbeddingBackend`] is a caller-supplied object that turns a batch
//! of texts into vectors; kreuzberg never owns the model.
//!
//! # Typical use
//!
//! Callers that already load their own embedder (e.g. `llama-cpp-python`,
//! `sentence-transformers`, or a tuned ONNX model) register the wrapper once and
//! reference it by name in config:
//!
//! ```rust,no_run
//! use kreuzberg::plugins::{EmbeddingBackend, Plugin, register_embedding_backend};
//! use kreuzberg::Result;
//! use std::sync::Arc;
//!
//! struct MyEmbedder;
//!
//! impl Plugin for MyEmbedder {
//!     fn name(&self) -> &str { "my-embedder" }
//!     fn version(&self) -> String { "1.0.0".to_string() }
//!     fn initialize(&self) -> Result<()> { Ok(()) }
//!     fn shutdown(&self) -> Result<()> { Ok(()) }
//! }
//!
//! #[async_trait::async_trait]
//! impl EmbeddingBackend for MyEmbedder {
//!     fn dimensions(&self) -> usize { 768 }
//!     async fn embed(&self, texts: Vec<String>) -> Result<Vec<Vec<f32>>> {
//!         Ok(texts.iter().map(|_| vec![0.0; 768]).collect())
//!     }
//! }
//!
//! register_embedding_backend(Arc::new(MyEmbedder))?;
//! # Ok::<(), kreuzberg::KreuzbergError>(())
//! ```

use crate::Result;
use crate::plugins::Plugin;
use async_trait::async_trait;
use std::sync::Arc;

/// Trait for in-process embedding backend plugins.
///
/// Async to match the convention used by [`crate::plugins::OcrBackend`],
/// [`crate::plugins::DocumentExtractor`], and [`crate::plugins::PostProcessor`].
/// Host-language bridges (PyO3, napi-rs, Rustler, extendr, magnus, ext-php-rs,
/// C FFI, etc.) wrap their synchronous host callables in `spawn_blocking` or the
/// equivalent to satisfy the async signature.
///
/// # Thread safety
///
/// Backends must be `Send + Sync + 'static`. They are stored in
/// `Arc<dyn EmbeddingBackend>` and called concurrently from kreuzberg's chunking
/// pipeline. If the backend's underlying model isn't thread-safe, the backend
/// itself must serialize access internally (e.g. via `Mutex<Inner>`).
///
/// # Contract
///
/// - `embed(texts)` MUST return exactly `texts.len()` vectors, each of length
///   `self.dimensions()`. The dispatcher in [`crate::embeddings::embed_texts`]
///   validates this before returning to downstream consumers; a non-conforming
///   backend surfaces as a `KreuzbergError::Validation`, not a panic.
/// - `embed` may be called from any thread. Its future must be `Send`
///   (enforced by `async_trait` when `#[async_trait]` is used on non-WASM targets).
/// - `dimensions()` is called exactly once at registration, immediately after
///   `initialize()` succeeds. The returned value is cached by the registry and
///   used for all subsequent shape validation. Lazy-loading implementations can
///   defer model loading into `initialize()` and report the real dimension
///   afterwards. Later mutations of the backend's reported dimension are not
///   observed by kreuzberg — implementations that need to change dimension
///   must unregister and re-register.
/// - `shutdown()` (inherited from [`crate::plugins::Plugin`]) may be invoked
///   concurrently with an in-flight `embed()` call. Implementations must
///   tolerate this — e.g. by letting in-flight calls finish using resources
///   held via the `Arc<dyn EmbeddingBackend>` reference, and only releasing
///   shared state that isn't needed by `embed`.
///
/// # Runtime
///
/// The synchronous [`crate::embed_texts`] entry uses
/// [`tokio::task::block_in_place`] to await the trait's async `embed`, which
/// requires a multi-thread tokio runtime. Callers running inside a
/// `current_thread` runtime (e.g. `#[tokio::test]` without `flavor = "multi_thread"`,
/// or `tokio::runtime::Builder::new_current_thread()`) must use
/// [`crate::embed_texts_async`] instead, which awaits directly without
/// `block_in_place`.
#[cfg_attr(not(target_arch = "wasm32"), async_trait)]
#[cfg_attr(target_arch = "wasm32", async_trait(?Send))]
pub trait EmbeddingBackend: Plugin {
    /// Embedding vector dimension. Must be `> 0` and must match the length of
    /// every vector returned by `embed`.
    fn dimensions(&self) -> usize;

    /// Embed a batch of texts, returning one vector per input in order.
    ///
    /// # Errors
    ///
    /// Implementations should return [`crate::KreuzbergError::Plugin`] for
    /// backend-specific failures. The dispatcher layers its own validation
    /// (length, per-vector dimension) on top.
    async fn embed(&self, texts: Vec<String>) -> Result<Vec<Vec<f32>>>;
}

/// Register an embedding backend with the global registry.
///
/// The backend will be keyed by its `Plugin::name()` and can be referenced from
/// [`crate::core::config::EmbeddingModelType::Plugin`] by the same name.
///
/// # Errors
///
/// - [`crate::KreuzbergError::Validation`] if the name is empty, contains whitespace,
///   or `dimensions()` is zero.
/// - [`crate::KreuzbergError::Plugin`] if a backend with that name is already registered.
/// - Any error from the backend's `initialize()` method.
pub fn register_embedding_backend(backend: Arc<dyn EmbeddingBackend>) -> Result<()> {
    use crate::plugins::registry::get_embedding_backend_registry;

    let registry = get_embedding_backend_registry();
    let mut registry = registry.write();
    registry.register(backend)
}

/// Unregister an embedding backend by name, calling its `shutdown()` method.
///
/// No-op if the backend is not registered.
pub fn unregister_embedding_backend(name: &str) -> Result<()> {
    use crate::plugins::registry::get_embedding_backend_registry;

    let registry = get_embedding_backend_registry();
    let mut registry = registry.write();
    registry.remove(name)
}

/// List the names of all registered embedding backends.
pub fn list_embedding_backends() -> Result<Vec<String>> {
    use crate::plugins::registry::get_embedding_backend_registry;

    let registry = get_embedding_backend_registry();
    let registry = registry.read();
    Ok(registry.list())
}

/// Shutdown and remove every registered embedding backend.
pub fn clear_embedding_backends() -> Result<()> {
    use crate::plugins::registry::get_embedding_backend_registry;

    let registry = get_embedding_backend_registry();
    let mut registry = registry.write();
    registry.shutdown_all()
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::KreuzbergError;
    use crate::plugins::Plugin;
    use std::sync::atomic::{AtomicU64, Ordering};

    struct MockEmbeddingBackend {
        name: String,
        dimensions: usize,
    }

    impl Plugin for MockEmbeddingBackend {
        fn name(&self) -> &str {
            &self.name
        }
        fn version(&self) -> String {
            "1.0.0".to_string()
        }
        fn initialize(&self) -> Result<()> {
            Ok(())
        }
        fn shutdown(&self) -> Result<()> {
            Ok(())
        }
    }

    #[async_trait]
    impl EmbeddingBackend for MockEmbeddingBackend {
        fn dimensions(&self) -> usize {
            self.dimensions
        }

        async fn embed(&self, texts: Vec<String>) -> Result<Vec<Vec<f32>>> {
            Ok(texts.iter().map(|_| vec![0.5; self.dimensions]).collect())
        }
    }

    /// Unique per-test name so parallel test runs don't collide in the shared
    /// global `EMBEDDING_BACKEND_REGISTRY`.
    fn unique_name(suffix: &str) -> String {
        static COUNTER: AtomicU64 = AtomicU64::new(0);
        let id = COUNTER.fetch_add(1, Ordering::SeqCst);
        format!("mock-{suffix}-{id}")
    }

    #[test]
    fn register_list_unregister_roundtrip() {
        let name = unique_name("roundtrip");
        register_embedding_backend(Arc::new(MockEmbeddingBackend {
            name: name.clone(),
            dimensions: 384,
        }))
        .unwrap();

        assert!(list_embedding_backends().unwrap().contains(&name));

        unregister_embedding_backend(&name).unwrap();
        assert!(!list_embedding_backends().unwrap().contains(&name));
    }

    #[test]
    fn empty_name_rejected_via_global_api() {
        let result = register_embedding_backend(Arc::new(MockEmbeddingBackend {
            name: String::new(),
            dimensions: 384,
        }));
        assert!(matches!(result, Err(KreuzbergError::Validation { .. })));
    }

    #[test]
    fn zero_dimensions_rejected_via_global_api() {
        let result = register_embedding_backend(Arc::new(MockEmbeddingBackend {
            name: unique_name("zero"),
            dimensions: 0,
        }));
        assert!(matches!(result, Err(KreuzbergError::Validation { .. })));
    }

    #[tokio::test]
    async fn mock_backend_returns_expected_shape() {
        let backend = MockEmbeddingBackend {
            name: "local".to_string(),
            dimensions: 5,
        };
        let vectors = backend
            .embed(vec!["one".into(), "two".into(), "three".into()])
            .await
            .unwrap();
        assert_eq!(vectors.len(), 3);
        assert!(vectors.iter().all(|v| v.len() == 5));
    }
}
