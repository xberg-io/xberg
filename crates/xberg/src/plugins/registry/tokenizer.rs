//! Tokenizer backend registry.
//!
//! In-process complement to the HuggingFace-loaded tokenizer path used by
//! token-budgeted chunking. Callers — from Rust or a host-language bridge —
//! register a [`TokenizerBackend`] once; `ChunkSizing::Tokenizer { model }`
//! then resolves the name against this registry before falling back to the
//! HuggingFace Hub, so chunks can be sized with the exact tokenizer the
//! consumer's embedder uses.

use crate::plugins::TokenizerBackend;
use crate::{Result, XbergError};
use ahash::AHashMap;
use std::sync::Arc;

/// Registry for tokenizer backend plugins.
///
/// Like [`super::EmbeddingBackendRegistry`], no default backends are registered —
/// tokenizer backends are always supplied by the caller at runtime.
///
/// Unlike the embedding registry there is no cached shape metadata: a tokenizer
/// backend is a pure `&str -> usize` counter, validated once at registration
/// with a probe (see [`Self::register`]).
#[cfg_attr(alef, alef(skip))]
pub struct TokenizerBackendRegistry {
    backends: AHashMap<String, Arc<dyn TokenizerBackend>>,
}

impl TokenizerBackendRegistry {
    /// Create a new empty tokenizer backend registry.
    pub fn new() -> Self {
        Self {
            backends: AHashMap::new(),
        }
    }

    /// Register a tokenizer backend.
    ///
    /// Runs the backend's `initialize()` first (so lazy-loading implementations
    /// can load their vocabulary), then probes `count_tokens("a")`: a backend
    /// that reports zero tokens for non-empty text would defeat chunk sizing —
    /// every span would appear to fit any budget — so it is rejected.
    ///
    /// # Errors
    ///
    /// - [`XbergError::Validation`] if the name is empty, contains whitespace,
    ///   or the probe reports zero tokens for non-empty text.
    /// - [`XbergError::Plugin`] if a backend with the same name is already registered.
    /// - Any error from the backend's `initialize()` method.
    #[tracing::instrument(skip(self, backend), fields(backend_name))]
    pub fn register(&mut self, backend: Arc<dyn TokenizerBackend>) -> Result<()> {
        let name = backend.name().to_string();
        tracing::Span::current().record("backend_name", name.as_str());

        super::validate_plugin_name(&name)?;

        if self.backends.contains_key(&name) {
            return Err(XbergError::Plugin {
                message: format!("Tokenizer backend '{name}' is already registered"),
                plugin_name: name,
            });
        }

        // Run initialize() first so that backends which lazy-load their
        // vocabulary can produce real counts from the probe below.
        backend.initialize()?;

        if backend.count_tokens("a") == 0 {
            // initialize() already ran; give the backend a chance to release
            // resources before we reject it.
            let _ = backend.shutdown();
            return Err(XbergError::Validation {
                message: format!("Tokenizer backend '{name}' must report a non-zero token count for non-empty text"),
                source: None,
            });
        }

        tracing::info!(backend = %name, "Tokenizer backend registered");
        self.backends.insert(name, backend);
        Ok(())
    }

    /// Get a tokenizer backend by name, or `None` if not registered.
    ///
    /// This is the dispatch-path accessor: chunking probes the registry with
    /// the configured tokenizer name and falls back to the HuggingFace path on
    /// `None`, so a miss here is not an error.
    pub fn lookup(&self, name: &str) -> Option<Arc<dyn TokenizerBackend>> {
        self.backends.get(name).cloned()
    }

    /// Get a tokenizer backend by name.
    ///
    /// # Errors
    ///
    /// [`XbergError::Plugin`] if no backend with that name is registered.
    #[tracing::instrument(skip(self), fields(registered_backends = ?self.backends.keys().collect::<Vec<_>>()))]
    pub fn get(&self, name: &str) -> Result<Arc<dyn TokenizerBackend>> {
        self.lookup(name).ok_or_else(|| {
            let available: Vec<String> = self.backends.keys().cloned().collect();
            XbergError::Plugin {
                message: format!(
                    "Tokenizer backend '{}' not registered. Available backends: {}",
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
    pub fn shutdown_all(&mut self) -> Result<()> {
        let names: Vec<_> = self.backends.keys().cloned().collect();
        for name in names {
            self.remove(&name)?;
        }
        Ok(())
    }

    /// Drain the registry. Alias for `shutdown_all` for parity with the other
    /// plugin registries.
    pub fn clear(&mut self) -> Result<()> {
        self.shutdown_all()
    }
}

impl Default for TokenizerBackendRegistry {
    fn default() -> Self {
        Self::new()
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::plugins::Plugin;

    struct MockTokenizer {
        name: String,
        tokens_per_call: usize,
    }

    impl Plugin for MockTokenizer {
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

    impl TokenizerBackend for MockTokenizer {
        fn count_tokens(&self, _text: &str) -> usize {
            self.tokens_per_call
        }
    }

    fn mock(name: &str) -> Arc<MockTokenizer> {
        Arc::new(MockTokenizer {
            name: name.to_string(),
            tokens_per_call: 1,
        })
    }

    #[test]
    fn register_and_retrieve() {
        let mut registry = TokenizerBackendRegistry::new();
        registry.register(mock("mock")).unwrap();

        let retrieved = registry.get("mock").unwrap();
        assert_eq!(retrieved.name(), "mock");
        assert_eq!(retrieved.count_tokens("anything"), 1);
    }

    #[test]
    fn empty_registry_has_no_backends() {
        let registry = TokenizerBackendRegistry::new();
        assert!(registry.list().is_empty());
    }

    #[test]
    fn lookup_missing_backend_returns_none() {
        let registry = TokenizerBackendRegistry::new();
        assert!(registry.lookup("never-registered").is_none());
    }

    #[test]
    fn get_missing_backend_returns_plugin_error() {
        let registry = TokenizerBackendRegistry::new();
        let result = registry.get("never-registered");
        assert!(matches!(result, Err(XbergError::Plugin { .. })));
    }

    #[test]
    fn rejects_empty_name() {
        let mut registry = TokenizerBackendRegistry::new();
        assert!(matches!(
            registry.register(mock("")),
            Err(XbergError::Validation { .. })
        ));
    }

    #[test]
    fn rejects_whitespace_in_name() {
        let mut registry = TokenizerBackendRegistry::new();
        assert!(matches!(
            registry.register(mock("has spaces")),
            Err(XbergError::Validation { .. })
        ));
    }

    #[test]
    fn rejects_zero_count_for_nonempty_text() {
        let mut registry = TokenizerBackendRegistry::new();
        let backend = Arc::new(MockTokenizer {
            name: "zero-count".to_string(),
            tokens_per_call: 0,
        });
        assert!(matches!(registry.register(backend), Err(XbergError::Validation { .. })));
    }

    #[test]
    fn rejects_duplicate_name() {
        let mut registry = TokenizerBackendRegistry::new();
        registry.register(mock("dup")).unwrap();
        let result = registry.register(mock("dup"));
        assert!(matches!(result, Err(XbergError::Plugin { .. })));
    }

    #[test]
    fn remove_backend_clears_entry() {
        let mut registry = TokenizerBackendRegistry::new();
        registry.register(mock("to-remove")).unwrap();
        registry.remove("to-remove").unwrap();
        assert!(registry.list().is_empty());
    }

    #[test]
    fn remove_missing_backend_is_noop() {
        let mut registry = TokenizerBackendRegistry::new();
        assert!(registry.remove("never-registered").is_ok());
    }

    #[test]
    fn shutdown_all_clears_all_backends() {
        let mut registry = TokenizerBackendRegistry::new();
        registry.register(mock("one")).unwrap();
        registry.register(mock("two")).unwrap();

        registry.shutdown_all().unwrap();
        assert!(registry.list().is_empty());
    }

    #[test]
    fn initialize_failure_propagates_and_backend_is_not_registered() {
        struct FailingInit;
        impl Plugin for FailingInit {
            fn name(&self) -> &str {
                "failing-init"
            }
            fn version(&self) -> String {
                "1.0.0".to_string()
            }
            fn initialize(&self) -> Result<()> {
                Err(XbergError::Plugin {
                    message: "boom".to_string(),
                    plugin_name: "failing-init".to_string(),
                })
            }
            fn shutdown(&self) -> Result<()> {
                Ok(())
            }
        }
        impl TokenizerBackend for FailingInit {
            fn count_tokens(&self, _text: &str) -> usize {
                1
            }
        }

        let mut registry = TokenizerBackendRegistry::new();
        assert!(registry.register(Arc::new(FailingInit)).is_err());
        assert!(registry.list().is_empty());
    }
}
