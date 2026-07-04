//! Tokenizer backend plugin trait.
//!
//! Defines the trait for supplying a custom tokenizer to token-budgeted
//! chunking — the in-process complement to the HuggingFace-loaded path behind
//! `ChunkSizing::Tokenizer`. A [`TokenizerBackend`] is a caller-supplied object
//! that counts tokens in a text span; xberg never owns the vocabulary.
//!
//! # Typical use
//!
//! Callers whose embedder tokenizes with something xberg can't load itself
//! (a llama.cpp/GGUF vocabulary, a SentencePiece model, a tuned custom vocab)
//! register the wrapper once and reference it by name in config:
//!
//! ```rust
//! use xberg::plugins::{Plugin, TokenizerBackend, register_tokenizer_backend};
//! use xberg::Result;
//! use std::sync::Arc;
//!
//! struct MyTokenizer;
//!
//! impl Plugin for MyTokenizer {
//!     fn name(&self) -> &str { "my-tokenizer" }
//!     fn version(&self) -> String { "1.0.0".to_string() }
//!     fn initialize(&self) -> Result<()> { Ok(()) }
//!     fn shutdown(&self) -> Result<()> { Ok(()) }
//! }
//!
//! impl TokenizerBackend for MyTokenizer {
//!     fn count_tokens(&self, text: &str) -> usize {
//!         // Delegate to your real tokenizer here.
//!         text.split_whitespace().count()
//!     }
//! }
//!
//! register_tokenizer_backend(Arc::new(MyTokenizer))?;
//! // ChunkSizing::Tokenizer { model: "my-tokenizer".into(), cache_dir: None }
//! // now resolves to this backend, and `max_characters` becomes its token budget.
//! # xberg::plugins::unregister_tokenizer_backend("my-tokenizer")?;
//! # Ok::<(), xberg::XbergError>(())
//! ```

use crate::Result;
use crate::plugins::Plugin;
use std::sync::Arc;

/// Trait for in-process tokenizer backend plugins.
///
/// Unlike [`crate::plugins::EmbeddingBackend`], this trait is **synchronous**:
/// the chunk splitter calls [`Self::count_tokens`] inside its boundary search,
/// many times per chunk, so counting must be a direct call with no async
/// dispatch. Host-language bridges (PyO3, napi-rs, etc.) invoke their host
/// callable synchronously on the calling thread; implementations should keep
/// `count_tokens` cheap — it dominates chunking time when the backend is slow.
///
/// # Lifecycle
///
/// `initialize()` is called once during registration, before any
/// `count_tokens` call; lazy-loading implementations should load their
/// vocabulary there. After registration succeeds, `count_tokens` may be called
/// from any thread, concurrently. `shutdown()` runs on unregistration and may
/// overlap an in-flight `count_tokens` call from a chunking run that resolved
/// the backend earlier — implementations must tolerate this, e.g. by keeping
/// the resources `count_tokens` needs alive via `Arc`.
///
/// # Thread safety
///
/// Backends must be `Send + Sync + 'static` (inherited from [`Plugin`]). They
/// are stored in `Arc<dyn TokenizerBackend>` and called concurrently from
/// xberg's chunking pipeline. If the underlying tokenizer isn't thread-safe,
/// the backend must serialize access internally.
///
/// # Contract
///
/// - `count_tokens` must return a non-zero count for non-empty text. The
///   registry probes this once at registration and rejects backends that
///   report zero — a zero count would make every span appear to fit any
///   budget. At runtime, a zero count for non-empty text is not trusted:
///   the chunker substitutes the character count and logs the substitution.
///   (An implementation may still return 0 for the empty string.)
/// - `count_tokens` must not panic; return a best-effort count for text the
///   tokenizer can't fully process.
/// - Counting should be deterministic for a given input — the splitter may
///   evaluate overlapping spans of the same text repeatedly.
pub trait TokenizerBackend: Plugin {
    /// Count the tokens in `text` according to this backend's tokenizer.
    fn count_tokens(&self, text: &str) -> usize;
}

/// Register a tokenizer backend with the global registry.
///
/// The backend is keyed by its `Plugin::name()`. Token-budgeted chunking
/// resolves `ChunkSizing::Tokenizer { model }` against this registry first —
/// a registered name takes precedence over a HuggingFace model id — and falls
/// back to the HuggingFace path when the name is not registered.
///
/// # Errors
///
/// - [`crate::XbergError::Validation`] if the name is empty, contains
///   whitespace, or the backend reports zero tokens for non-empty text.
/// - [`crate::XbergError::Plugin`] if a backend with that name is already registered.
/// - Any error from the backend's `initialize()` method.
#[cfg_attr(alef, alef(skip))]
pub fn register_tokenizer_backend(backend: Arc<dyn TokenizerBackend>) -> Result<()> {
    use crate::plugins::registry::get_tokenizer_backend_registry;

    let registry = get_tokenizer_backend_registry();
    let mut registry = registry.write();
    registry.register(backend)
}

/// Unregister a tokenizer backend by name, calling its `shutdown()` method.
///
/// No-op if the backend is not registered.
///
/// # Errors
///
/// - Any error returned by the backend's `shutdown()` method.
#[cfg_attr(alef, alef(skip))]
pub fn unregister_tokenizer_backend(name: &str) -> Result<()> {
    use crate::plugins::registry::get_tokenizer_backend_registry;

    let registry = get_tokenizer_backend_registry();
    let mut registry = registry.write();
    registry.remove(name)
}

/// Clear all tokenizer backends from the global registry.
///
/// Calls `shutdown()` on every registered backend, then empties the registry.
///
/// # Errors
///
/// - Any error returned by a backend's `shutdown()` method. The first error
///   encountered stops processing of remaining backends.
pub fn clear_tokenizer_backends() -> Result<()> {
    use crate::plugins::registry::get_tokenizer_backend_registry;

    let registry = get_tokenizer_backend_registry();
    let mut registry = registry.write();
    registry.shutdown_all()
}

/// List the names of all registered tokenizer backends.
///
/// Used by `xberg-cli`, the api/mcp endpoints, and generated language
/// bindings.
pub fn list_tokenizer_backends() -> Result<Vec<String>> {
    use crate::plugins::registry::get_tokenizer_backend_registry;

    let registry = get_tokenizer_backend_registry();
    let registry = registry.read();
    Ok(registry.list())
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::XbergError;
    use crate::plugins::Plugin;
    use std::sync::atomic::{AtomicU64, Ordering};

    struct MockTokenizerBackend {
        name: String,
    }

    impl Plugin for MockTokenizerBackend {
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

    impl TokenizerBackend for MockTokenizerBackend {
        fn count_tokens(&self, text: &str) -> usize {
            text.split_whitespace().count().max(usize::from(!text.is_empty()))
        }
    }

    /// Unique per-test name so parallel test runs don't collide in the shared
    /// global `TOKENIZER_BACKEND_REGISTRY`.
    fn unique_name(suffix: &str) -> String {
        static COUNTER: AtomicU64 = AtomicU64::new(0);
        let id = COUNTER.fetch_add(1, Ordering::SeqCst);
        format!("mock-tokenizer-{suffix}-{id}")
    }

    #[test]
    fn register_list_unregister_roundtrip() {
        let name = unique_name("roundtrip");
        register_tokenizer_backend(Arc::new(MockTokenizerBackend { name: name.clone() })).unwrap();

        assert!(list_tokenizer_backends().unwrap().contains(&name));

        unregister_tokenizer_backend(&name).unwrap();
        assert!(!list_tokenizer_backends().unwrap().contains(&name));
    }

    #[test]
    fn empty_name_rejected_via_global_api() {
        let result = register_tokenizer_backend(Arc::new(MockTokenizerBackend { name: String::new() }));
        assert!(matches!(result, Err(XbergError::Validation { .. })));
    }

    #[test]
    fn register_clear_roundtrip() {
        let name = unique_name("clear");
        register_tokenizer_backend(Arc::new(MockTokenizerBackend { name: name.clone() })).unwrap();

        assert!(list_tokenizer_backends().unwrap().contains(&name));

        clear_tokenizer_backends().unwrap();
        assert!(!list_tokenizer_backends().unwrap().contains(&name));
    }

    #[test]
    fn mock_backend_counts_words() {
        let backend = MockTokenizerBackend {
            name: "counter".to_string(),
        };
        assert_eq!(backend.count_tokens("one two three"), 3);
        assert_eq!(backend.count_tokens(""), 0);
    }
}
