//! Embedding backend registry.
//!
//! In-process complement to the HTTP-based [`crate::core::config::EmbeddingModelType::Llm`]
//! path. Host-language bridges register an [`EmbeddingBackend`] once; kreuzberg then
//! calls back into it during chunking and standalone embed requests instead of
//! downloading an ONNX model from HuggingFace.

use crate::plugins::EmbeddingBackend;
use crate::{KreuzbergError, Result};
use ahash::AHashMap;
use std::sync::Arc;

/// Registry for embedding backend plugins.
///
/// Unlike [`super::OcrBackendRegistry`], no default backends are registered — embedding
/// backends are always supplied by the host language at runtime.
///
/// Entries cache the backend's `dimensions()` at registration time. Downstream
/// code should prefer [`Self::get_with_dimensions`] over calling
/// `backend.dimensions()` on each dispatch so that a backend returning an
/// inconsistent dimension post-registration cannot slip through shape
/// validation mid-session.
pub struct EmbeddingBackendRegistry {
    pub(super) backends: AHashMap<String, RegisteredBackend>,
}

/// A registered embedding backend and the dimensions it reported at
/// registration time. The cached value is what validation uses; the backend's
/// live `dimensions()` method is not called again.
#[derive(Clone)]
pub(crate) struct RegisteredBackend {
    pub(crate) backend: Arc<dyn EmbeddingBackend>,
    pub(crate) dimensions: usize,
}

impl EmbeddingBackendRegistry {
    /// Create a new empty embedding backend registry.
    pub fn new() -> Self {
        Self {
            backends: AHashMap::new(),
        }
    }

    /// Register an embedding backend.
    ///
    /// # Errors
    ///
    /// - [`KreuzbergError::Validation`] if the name is empty, contains whitespace, or
    ///   the backend reports zero dimensions.
    /// - [`KreuzbergError::Plugin`] if a backend with the same name is already registered.
    /// - Any error from the backend's `initialize()` method.
    #[tracing::instrument(skip(self, backend), fields(backend_name))]
    pub fn register(&mut self, backend: Arc<dyn EmbeddingBackend>) -> Result<()> {
        let name = backend.name().to_string();
        tracing::Span::current().record("backend_name", name.as_str());

        super::validate_plugin_name(&name)?;

        if self.backends.contains_key(&name) {
            return Err(KreuzbergError::Plugin {
                message: format!("Embedding backend '{name}' is already registered"),
                plugin_name: name,
            });
        }

        // Run initialize() first so that backends which lazy-load their model
        // (a common pattern — see the OCR Tesseract/Paddle backends) can
        // report their real dimension from dimensions() once init is done.
        backend.initialize()?;

        let dimensions = backend.dimensions();
        if dimensions == 0 {
            // initialize() already ran; give the backend a chance to release
            // resources before we reject it.
            let _ = backend.shutdown();
            return Err(KreuzbergError::Validation {
                message: format!("Embedding backend '{name}' must report dimensions() > 0"),
                source: None,
            });
        }

        tracing::info!(backend = %name, dimensions, "Embedding backend registered");
        self.backends.insert(name, RegisteredBackend { backend, dimensions });
        Ok(())
    }

    /// Get an embedding backend by name.
    ///
    /// Returns the backend only — use [`Self::get_with_dimensions`] when you
    /// need the dimensions captured at registration time (which is almost
    /// always the right thing for dispatch and shape validation).
    #[tracing::instrument(skip(self), fields(registered_backends = ?self.backends.keys().collect::<Vec<_>>()))]
    pub fn get(&self, name: &str) -> Result<Arc<dyn EmbeddingBackend>> {
        self.get_with_dimensions(name).map(|(backend, _)| backend)
    }

    /// Get an embedding backend and its registration-time dimensions by name.
    #[tracing::instrument(skip(self), fields(registered_backends = ?self.backends.keys().collect::<Vec<_>>()))]
    pub fn get_with_dimensions(&self, name: &str) -> Result<(Arc<dyn EmbeddingBackend>, usize)> {
        self.backends
            .get(name)
            .map(|entry| (entry.backend.clone(), entry.dimensions))
            .ok_or_else(|| {
                tracing::error!(
                    backend = name,
                    available = ?self.backends.keys().collect::<Vec<_>>(),
                    "Embedding backend not found in registry"
                );
                let available: Vec<String> = self.backends.keys().cloned().collect();
                KreuzbergError::Plugin {
                    message: format!(
                        "Embedding backend '{}' not registered. Available backends: {}",
                        name,
                        if available.is_empty() {
                            "(none registered)".to_string()
                        } else {
                            available.join(", ")
                        }
                    ),
                    plugin_name: name.to_string(),
                }
            })
    }

    /// List all registered backend names.
    pub fn list(&self) -> Vec<String> {
        self.backends.keys().cloned().collect()
    }

    /// Remove a backend from the registry, calling its `shutdown()` method.
    pub fn remove(&mut self, name: &str) -> Result<()> {
        if let Some(entry) = self.backends.remove(name) {
            entry.backend.shutdown()?;
        }
        Ok(())
    }

    /// Shutdown all backends and clear the registry.
    pub fn shutdown_all(&mut self) -> Result<()> {
        let names: Vec<_> = self.backends.keys().cloned().collect();
        for name in names {
            self.remove(&name)?;
        }
        Ok(())
    }
}

impl Default for EmbeddingBackendRegistry {
    fn default() -> Self {
        Self::new()
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::plugins::{EmbeddingBackend, Plugin};

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

    #[async_trait::async_trait]
    impl EmbeddingBackend for MockEmbeddingBackend {
        fn dimensions(&self) -> usize {
            self.dimensions
        }

        async fn embed(&self, texts: Vec<String>) -> Result<Vec<Vec<f32>>> {
            Ok(texts.iter().map(|_| vec![0.0; self.dimensions]).collect())
        }
    }

    #[test]
    fn register_and_retrieve() {
        let mut registry = EmbeddingBackendRegistry::new();
        let backend = Arc::new(MockEmbeddingBackend {
            name: "mock".to_string(),
            dimensions: 384,
        });
        registry.register(backend).unwrap();

        let retrieved = registry.get("mock").unwrap();
        assert_eq!(retrieved.name(), "mock");
        assert_eq!(retrieved.dimensions(), 384);
    }

    #[test]
    fn empty_registry_has_no_backends() {
        let registry = EmbeddingBackendRegistry::new();
        assert!(registry.list().is_empty());
    }

    #[test]
    fn get_missing_backend_returns_plugin_error() {
        let registry = EmbeddingBackendRegistry::new();
        let result = registry.get("never-registered");
        assert!(matches!(result, Err(KreuzbergError::Plugin { .. })));
    }

    #[test]
    fn rejects_empty_name() {
        let mut registry = EmbeddingBackendRegistry::new();
        let backend = Arc::new(MockEmbeddingBackend {
            name: String::new(),
            dimensions: 384,
        });
        assert!(matches!(
            registry.register(backend),
            Err(KreuzbergError::Validation { .. })
        ));
    }

    #[test]
    fn rejects_whitespace_in_name() {
        let mut registry = EmbeddingBackendRegistry::new();
        let backend = Arc::new(MockEmbeddingBackend {
            name: "has spaces".to_string(),
            dimensions: 384,
        });
        assert!(matches!(
            registry.register(backend),
            Err(KreuzbergError::Validation { .. })
        ));
    }

    #[test]
    fn rejects_zero_dimensions() {
        let mut registry = EmbeddingBackendRegistry::new();
        let backend = Arc::new(MockEmbeddingBackend {
            name: "zero-dim".to_string(),
            dimensions: 0,
        });
        assert!(matches!(
            registry.register(backend),
            Err(KreuzbergError::Validation { .. })
        ));
    }

    #[test]
    fn rejects_duplicate_name() {
        let mut registry = EmbeddingBackendRegistry::new();
        registry
            .register(Arc::new(MockEmbeddingBackend {
                name: "dup".to_string(),
                dimensions: 384,
            }))
            .unwrap();

        let result = registry.register(Arc::new(MockEmbeddingBackend {
            name: "dup".to_string(),
            dimensions: 768,
        }));
        assert!(matches!(result, Err(KreuzbergError::Plugin { .. })));
    }

    #[test]
    fn remove_backend_clears_entry() {
        let mut registry = EmbeddingBackendRegistry::new();
        registry
            .register(Arc::new(MockEmbeddingBackend {
                name: "to-remove".to_string(),
                dimensions: 128,
            }))
            .unwrap();
        registry.remove("to-remove").unwrap();
        assert!(registry.list().is_empty());
    }

    #[test]
    fn remove_missing_backend_is_noop() {
        let mut registry = EmbeddingBackendRegistry::new();
        assert!(registry.remove("never-registered").is_ok());
    }

    #[test]
    fn shutdown_all_clears_all_backends() {
        let mut registry = EmbeddingBackendRegistry::new();
        registry
            .register(Arc::new(MockEmbeddingBackend {
                name: "one".to_string(),
                dimensions: 384,
            }))
            .unwrap();
        registry
            .register(Arc::new(MockEmbeddingBackend {
                name: "two".to_string(),
                dimensions: 768,
            }))
            .unwrap();

        registry.shutdown_all().unwrap();
        assert!(registry.list().is_empty());
    }

    #[tokio::test]
    async fn mock_embedder_returns_batch_of_correct_shape() {
        let backend = MockEmbeddingBackend {
            name: "batch".to_string(),
            dimensions: 4,
        };
        let vectors = backend.embed(vec!["a".into(), "b".into(), "c".into()]).await.unwrap();
        assert_eq!(vectors.len(), 3);
        assert!(vectors.iter().all(|v| v.len() == 4));
    }
}
