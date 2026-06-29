//! The [`CacheBackend`] seam: an optional, content-addressed byte cache.
//!
//! The in-core default is [`NoopCache`], which stores nothing — exactly the
//! behavior xberg exhibits today, where the extraction path performs no
//! engine-level result caching. Alternative backends (Redis, on-disk, an
//! in-memory LRU) implement this trait and are injected via
//! [`EngineBuilder::with_cache_backend`](super::super::EngineBuilder::with_cache_backend).

use std::time::Duration;

use async_trait::async_trait;

/// A content-addressed byte cache keyed by opaque string keys.
///
/// # Thread safety
///
/// Implementations are `Send + Sync + 'static` and held behind
/// `Arc<dyn CacheBackend>`; they may be called concurrently.
#[cfg_attr(not(target_arch = "wasm32"), async_trait)]
#[cfg_attr(target_arch = "wasm32", async_trait(?Send))]
pub trait CacheBackend: Send + Sync + 'static {
    /// Fetch the bytes stored under `key`, or `None` if absent or expired.
    async fn get(&self, key: &str) -> Option<Vec<u8>>;

    /// Store `value` under `key`, optionally expiring after `ttl`.
    async fn put(&self, key: &str, value: Vec<u8>, ttl: Option<Duration>);
}

/// In-core default: a cache that stores nothing and never hits.
///
/// This reproduces today's behavior exactly — the default extraction path does
/// no engine-level caching, so [`get`](CacheBackend::get) always returns `None`
/// and [`put`](CacheBackend::put) is inert.
#[derive(Debug, Default, Clone, Copy)]
pub struct NoopCache;

#[cfg_attr(not(target_arch = "wasm32"), async_trait)]
#[cfg_attr(target_arch = "wasm32", async_trait(?Send))]
impl CacheBackend for NoopCache {
    async fn get(&self, _key: &str) -> Option<Vec<u8>> {
        None
    }

    async fn put(&self, _key: &str, _value: Vec<u8>, _ttl: Option<Duration>) {}
}
