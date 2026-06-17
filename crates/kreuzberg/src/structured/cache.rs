//! Vision-call caching.
//!
//! The [`VisionCallCache`] trait lets callers deduplicate identical vision LLM calls across runs.
//! The in-process [`MokaVisionCache`] is the default; the cloud injects a distributed (NATS-backed)
//! implementation. The trait object is intentionally excluded from the FFI/binding surface — bindings
//! get the cache-less path.

/// Identifies a single vision call for caching.
///
/// Two calls with equal keys are guaranteed to produce equivalent vision responses, so a cache hit
/// can be returned without re-calling the model.
#[derive(Debug, Clone, PartialEq, Eq, Hash)]
pub struct CacheKey {
    /// Hash of the rendered page bytes covered by this call.
    pub content_hash: String,
    /// Inclusive 1-indexed page range `(first, last)` covered by this call.
    pub page_range: (u32, u32),
    /// Fingerprint of the resolved preset (schema + prompt + settings).
    pub preset_fingerprint: String,
    /// Hash of the fully-built prompt.
    pub prompt_hash: String,
    /// Model identifier used for the call.
    pub model: String,
}

/// A cache for vision LLM call results, keyed by [`CacheKey`].
///
/// Implementations must be cheap to call and safe to share across threads. `get` returns the cached
/// structured JSON value when present; `put` stores a freshly computed value.
pub trait VisionCallCache: Send + Sync + std::fmt::Debug {
    /// Look up a cached vision response.
    fn get(&self, key: &CacheKey) -> Option<serde_json::Value>;
    /// Store a vision response.
    fn put(&self, key: CacheKey, value: serde_json::Value);
}

/// Default maximum number of cached vision-call results for [`MokaVisionCache`].
const DEFAULT_CACHE_CAPACITY: u64 = 1_024;

/// In-process vision-call cache backed by a [`moka::sync::Cache`].
///
/// The cache is bounded by `max_capacity` entries. Eviction is LRU/TinyLFU as implemented by moka.
/// Cloning this value shares the same underlying cache (moka handles the internal `Arc`).
///
/// # Example
///
/// ```
/// use kreuzberg::structured::cache::{CacheKey, MokaVisionCache, VisionCallCache};
///
/// let cache = MokaVisionCache::with_default_capacity();
/// let key = CacheKey {
///     content_hash: "abc".into(),
///     page_range: (1, 1),
///     preset_fingerprint: "fp".into(),
///     prompt_hash: "ph".into(),
///     model: "gpt-4o".into(),
/// };
/// assert!(cache.get(&key).is_none());
/// cache.put(key.clone(), serde_json::json!({"result": "ok"}));
/// ```
#[derive(Debug, Clone)]
pub struct MokaVisionCache {
    inner: moka::sync::Cache<CacheKey, serde_json::Value>,
}

impl MokaVisionCache {
    /// Create a new cache with the given maximum entry capacity.
    pub fn new(max_capacity: u64) -> Self {
        Self {
            inner: moka::sync::Cache::new(max_capacity),
        }
    }

    /// Create a new cache with [`DEFAULT_CACHE_CAPACITY`] maximum entries.
    pub fn with_default_capacity() -> Self {
        Self::new(DEFAULT_CACHE_CAPACITY)
    }
}

impl Default for MokaVisionCache {
    fn default() -> Self {
        Self::with_default_capacity()
    }
}

impl VisionCallCache for MokaVisionCache {
    fn get(&self, key: &CacheKey) -> Option<serde_json::Value> {
        self.inner.get(key)
    }

    fn put(&self, key: CacheKey, value: serde_json::Value) {
        self.inner.insert(key, value);
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn make_key(content_hash: &str) -> CacheKey {
        CacheKey {
            content_hash: content_hash.into(),
            page_range: (1, 1),
            preset_fingerprint: "fp".into(),
            prompt_hash: "ph".into(),
            model: "test-model".into(),
        }
    }

    #[test]
    fn put_then_get_returns_stored_value() {
        let cache = MokaVisionCache::with_default_capacity();
        let key = make_key("hash-a");
        let value = serde_json::json!({"text": "hello"});

        cache.put(key.clone(), value.clone());
        // run_pending_tasks ensures the write is flushed before the read.
        cache.inner.run_pending_tasks();

        let result = cache.get(&key);
        assert_eq!(result, Some(value));
    }

    #[test]
    fn get_on_absent_key_returns_none() {
        let cache = MokaVisionCache::with_default_capacity();
        let key = make_key("hash-absent");
        assert_eq!(cache.get(&key), None);
    }

    #[test]
    fn distinct_keys_do_not_collide() {
        let cache = MokaVisionCache::with_default_capacity();
        let key_a = make_key("hash-a");
        let key_b = make_key("hash-b");
        let value_a = serde_json::json!({"tag": "a"});
        let value_b = serde_json::json!({"tag": "b"});

        cache.put(key_a.clone(), value_a.clone());
        cache.put(key_b.clone(), value_b.clone());
        cache.inner.run_pending_tasks();

        assert_eq!(cache.get(&key_a), Some(value_a));
        assert_eq!(cache.get(&key_b), Some(value_b));
    }

    #[test]
    fn default_capacity_is_nonzero() {
        assert!(DEFAULT_CACHE_CAPACITY > 0);
    }

    #[test]
    fn clone_shares_same_cache() {
        let cache = MokaVisionCache::with_default_capacity();
        let clone = cache.clone();
        let key = make_key("shared");
        let value = serde_json::json!(42);

        cache.put(key.clone(), value.clone());
        cache.inner.run_pending_tasks();

        assert_eq!(clone.get(&key), Some(value));
    }
}
