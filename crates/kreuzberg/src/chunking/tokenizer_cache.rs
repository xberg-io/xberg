//! In-memory cache for HuggingFace tokenizers.
//!
//! Tokenizers are downloaded from HuggingFace Hub on first use and cached in-memory
//! for subsequent calls. File-level caching is handled by the `hf-hub` crate
//! (defaults to `~/.cache/huggingface/`, configurable via `HF_HOME` env var).

use std::collections::HashMap;
use std::sync::{Arc, RwLock};

use once_cell::sync::Lazy;

use crate::KreuzbergError;

/// Global in-memory cache for loaded tokenizers.
///
/// Keyed by model ID string. Once a tokenizer is loaded and parsed,
/// it's stored here to avoid re-downloading and re-parsing on subsequent calls.
static TOKENIZER_CACHE: Lazy<RwLock<HashMap<String, Arc<tokenizers::Tokenizer>>>> =
    Lazy::new(|| RwLock::new(HashMap::new()));

/// Get a cached tokenizer or initialize one from HuggingFace Hub.
///
/// Uses a two-phase locking strategy (read lock first, write lock on miss)
/// following the same pattern as the embeddings model cache in `embeddings.rs`.
///
/// # Arguments
///
/// * `model` - HuggingFace model ID (e.g., "Xenova/gpt-4o", "bert-base-uncased")
///
/// # Errors
///
/// Returns an error if the tokenizer cannot be downloaded or parsed.
pub(crate) fn get_or_init_tokenizer(model: &str) -> crate::Result<Arc<tokenizers::Tokenizer>> {
    // Phase 1: try read lock (fast path for cache hits)
    {
        let cache = TOKENIZER_CACHE
            .read()
            .map_err(|e| KreuzbergError::Other(format!("Tokenizer cache read lock poisoned: {}", e)))?;
        if let Some(tok) = cache.get(model) {
            return Ok(Arc::clone(tok));
        }
    }

    // Phase 2: write lock, double-check, then initialize
    let mut cache = TOKENIZER_CACHE
        .write()
        .map_err(|e| KreuzbergError::Other(format!("Tokenizer cache write lock poisoned: {}", e)))?;

    // Double-check after acquiring write lock (another thread may have initialized)
    if let Some(tok) = cache.get(model) {
        return Ok(Arc::clone(tok));
    }

    let tokenizer = tokenizers::Tokenizer::from_pretrained(model, None)
        .map_err(|e| KreuzbergError::validation(format!("Failed to load tokenizer '{}': {}", model, e)))?;

    let arc = Arc::new(tokenizer);
    cache.insert(model.to_string(), Arc::clone(&arc));
    Ok(arc)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_cache_returns_same_instance() {
        // This test requires network access to download a tokenizer.
        // Skip in CI by checking for a specific env var.
        if std::env::var("CI").is_ok() {
            return;
        }

        let model = "bert-base-uncased";
        let tok1 = get_or_init_tokenizer(model).unwrap();
        let tok2 = get_or_init_tokenizer(model).unwrap();

        // Same Arc instance (pointer equality)
        assert!(Arc::ptr_eq(&tok1, &tok2));
    }
}
