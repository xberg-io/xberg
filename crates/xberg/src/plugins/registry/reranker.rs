//! Reranker backend registry.
//!
//! In-process complement to the HTTP-based [`crate::core::config::RerankerModelType::Llm`]
//! path. Host-language bridges register a [`RerankerBackend`] once; xberg then
//! calls back into it during standalone rerank requests instead of running a local
//! ONNX cross-encoder or calling a provider API.
//!
//! Since v5.0.0.

use crate::plugins::RerankerBackend;
use crate::{Result, XbergError};
use ahash::AHashMap;
use std::sync::Arc;

/// Registry for reranker backend plugins.
///
/// Unlike [`super::OcrBackendRegistry`], no default backends are registered —
/// reranker backends are always supplied by the host language at runtime.
///
/// Unlike [`super::EmbeddingBackendRegistry`], there is no cached dimension —
/// cross-encoders return a single scalar per `(query, document)` pair, not a
/// vector of fixed length.
///
/// Since v5.0.0.
#[cfg_attr(alef, alef(skip))]
pub struct RerankerBackendRegistry {
    pub(super) backends: AHashMap<String, Arc<dyn RerankerBackend>>,
}

impl RerankerBackendRegistry {
    /// Create a new empty reranker backend registry.
    pub fn new() -> Self {
        Self {
            backends: AHashMap::new(),
        }
    }

    /// Register a reranker backend.
    ///
    /// # Errors
    ///
    /// - [`XbergError::Validation`] if the name is empty or contains whitespace.
    /// - [`XbergError::Plugin`] if a backend with the same name is already registered.
    /// - Any error from the backend's `initialize()` method.
    #[tracing::instrument(skip(self, backend), fields(backend_name))]
    pub fn register(&mut self, backend: Arc<dyn RerankerBackend>) -> Result<()> {
        let name = backend.name().to_string();
        tracing::Span::current().record("backend_name", name.as_str());

        super::validate_plugin_name(&name)?;

        if self.backends.contains_key(&name) {
            return Err(XbergError::Plugin {
                message: format!("Reranker backend '{name}' is already registered"),
                plugin_name: name,
            });
        }

        backend.initialize()?;

        tracing::info!(backend = %name, "Reranker backend registered");
        self.backends.insert(name, backend);
        Ok(())
    }

    /// Get a reranker backend by name.
    #[tracing::instrument(skip(self), fields(registered_backends = ?self.backends.keys().collect::<Vec<_>>()))]
    pub fn get(&self, name: &str) -> Result<Arc<dyn RerankerBackend>> {
        self.backends.get(name).cloned().ok_or_else(|| {
            tracing::error!(
                backend = name,
                available = ?self.backends.keys().collect::<Vec<_>>(),
                "Reranker backend not found in registry"
            );
            let available: Vec<String> = self.backends.keys().cloned().collect();
            XbergError::Plugin {
                message: format!(
                    "Reranker backend '{}' not registered. Available backends: {}",
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
        if let Some(backend) = self.backends.remove(name) {
            backend.shutdown()?;
        }
        Ok(())
    }

    /// Shutdown all backends and clear the registry.
    ///
    /// Best-effort: every backend's `shutdown()` is invoked even if an earlier
    /// one returned an error. All backends are removed from the registry
    /// regardless. The first error encountered is returned; subsequent errors
    /// are logged at `warn` and dropped. This keeps the registry consistent
    /// across partial failures — callers don't end up with half-shutdown
    /// backends still indexed.
    pub fn shutdown_all(&mut self) -> Result<()> {
        let names: Vec<_> = self.backends.keys().cloned().collect();
        let mut first_error: Option<crate::XbergError> = None;

        for name in names {
            if let Some(backend) = self.backends.remove(&name)
                && let Err(err) = backend.shutdown()
            {
                if first_error.is_none() {
                    first_error = Some(err);
                } else {
                    tracing::warn!(
                        backend = %name,
                        error = %err,
                        "Reranker backend shutdown failed during shutdown_all (already surfacing an earlier error)",
                    );
                }
            }
        }

        match first_error {
            Some(err) => Err(err),
            None => Ok(()),
        }
    }

    /// Drain the registry. Alias for `shutdown_all` used by alef trait-bridge codegen.
    pub fn clear(&mut self) -> Result<()> {
        self.shutdown_all()
    }
}

impl Default for RerankerBackendRegistry {
    fn default() -> Self {
        Self::new()
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::plugins::{Plugin, RerankerBackend};

    struct MockRerankerBackend {
        name: String,
    }

    impl Plugin for MockRerankerBackend {
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
    impl RerankerBackend for MockRerankerBackend {
        async fn rerank(&self, _query: String, documents: Vec<String>) -> Result<Vec<f32>> {
            Ok(documents.iter().map(|_| 0.5_f32).collect())
        }
    }

    #[test]
    fn register_and_retrieve() {
        let mut registry = RerankerBackendRegistry::new();
        let backend = Arc::new(MockRerankerBackend {
            name: "mock".to_string(),
        });
        registry.register(backend).unwrap();

        let retrieved = registry.get("mock").unwrap();
        assert_eq!(retrieved.name(), "mock");
    }

    #[test]
    fn empty_registry_has_no_backends() {
        let registry = RerankerBackendRegistry::new();
        assert!(registry.list().is_empty());
    }

    #[test]
    fn get_missing_backend_returns_plugin_error() {
        let registry = RerankerBackendRegistry::new();
        let result = registry.get("never-registered");
        assert!(matches!(result, Err(XbergError::Plugin { .. })));
    }

    #[test]
    fn rejects_empty_name() {
        let mut registry = RerankerBackendRegistry::new();
        let backend = Arc::new(MockRerankerBackend { name: String::new() });
        assert!(matches!(registry.register(backend), Err(XbergError::Validation { .. })));
    }

    #[test]
    fn rejects_whitespace_in_name() {
        let mut registry = RerankerBackendRegistry::new();
        let backend = Arc::new(MockRerankerBackend {
            name: "has spaces".to_string(),
        });
        assert!(matches!(registry.register(backend), Err(XbergError::Validation { .. })));
    }

    #[test]
    fn rejects_duplicate_name() {
        let mut registry = RerankerBackendRegistry::new();
        registry
            .register(Arc::new(MockRerankerBackend {
                name: "dup".to_string(),
            }))
            .unwrap();

        let result = registry.register(Arc::new(MockRerankerBackend {
            name: "dup".to_string(),
        }));
        assert!(matches!(result, Err(XbergError::Plugin { .. })));
    }

    #[test]
    fn remove_backend_clears_entry() {
        let mut registry = RerankerBackendRegistry::new();
        registry
            .register(Arc::new(MockRerankerBackend {
                name: "to-remove".to_string(),
            }))
            .unwrap();
        registry.remove("to-remove").unwrap();
        assert!(registry.list().is_empty());
    }

    #[test]
    fn remove_missing_backend_is_noop() {
        let mut registry = RerankerBackendRegistry::new();
        assert!(registry.remove("never-registered").is_ok());
    }

    #[test]
    fn shutdown_all_clears_all_backends() {
        let mut registry = RerankerBackendRegistry::new();
        registry
            .register(Arc::new(MockRerankerBackend {
                name: "one".to_string(),
            }))
            .unwrap();
        registry
            .register(Arc::new(MockRerankerBackend {
                name: "two".to_string(),
            }))
            .unwrap();

        registry.shutdown_all().unwrap();
        assert!(registry.list().is_empty());
    }

    #[tokio::test]
    async fn mock_reranker_returns_batch_of_correct_length() {
        let backend = MockRerankerBackend {
            name: "batch".to_string(),
        };
        let scores = backend
            .rerank("query".to_string(), vec!["a".into(), "b".into(), "c".into()])
            .await
            .unwrap();
        assert_eq!(scores.len(), 3);
    }
}
