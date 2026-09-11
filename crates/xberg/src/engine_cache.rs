//! Process-wide cache of loaded model engines with an optional bound on the
//! number of resident engines. Least recently used engines leave first.

use std::hash::Hash;
use std::num::NonZeroUsize;
use std::sync::{Arc, PoisonError, RwLock, RwLockWriteGuard};

use lru::LruCache;

/// Engines keyed by the parameters they were loaded with.
///
/// Every engine is shared through an [`Arc`], so removing it from the cache
/// frees its memory once the last caller drops its handle.
pub(crate) struct EngineCache<K, V> {
    inner: RwLock<LruCache<K, Arc<V>>>,
}

impl<K: Hash + Eq + Clone, V> EngineCache<K, V> {
    /// A cache with no bound on the number of resident engines.
    pub(crate) fn unbounded() -> Self {
        Self {
            inner: RwLock::new(LruCache::unbounded()),
        }
    }

    fn write(&self) -> RwLockWriteGuard<'_, LruCache<K, Arc<V>>> {
        self.inner.write().unwrap_or_else(PoisonError::into_inner)
    }

    /// Return the resident engine for `key`, or build one with `init` and keep it.
    ///
    /// The cache stays locked while `init` runs, so a second caller for the
    /// same key waits for the first engine instead of building another. When
    /// the cache is at its bound, the least recently used engine is dropped to
    /// make room.
    pub(crate) fn get_or_try_init<E>(&self, key: K, init: impl FnOnce() -> Result<V, E>) -> Result<Arc<V>, E> {
        let mut cache = self.write();
        if let Some(engine) = cache.get(&key) {
            return Ok(Arc::clone(engine));
        }
        let engine = Arc::new(init()?);
        cache.put(key, Arc::clone(&engine));
        Ok(engine)
    }

    /// Remove every engine whose key matches `matches`. Returns the number removed.
    pub(crate) fn evict_where(&self, mut matches: impl FnMut(&K) -> bool) -> usize {
        let mut cache = self.write();
        let keys: Vec<K> = cache
            .iter()
            .filter(|(key, _)| matches(key))
            .map(|(key, _)| key.clone())
            .collect();
        for key in &keys {
            cache.pop(key);
        }
        keys.len()
    }

    /// Remove every engine. Returns the number removed.
    pub(crate) fn clear(&self) -> usize {
        let mut cache = self.write();
        let removed = cache.len();
        cache.clear();
        removed
    }

    /// Bound the number of resident engines, or lift the bound with `None`.
    ///
    /// Lowering the bound below the current count drops the least recently
    /// used engines at once.
    pub(crate) fn set_limit(&self, max_resident: Option<NonZeroUsize>) {
        self.write().resize(max_resident.unwrap_or(NonZeroUsize::MAX));
    }

    /// The number of resident engines.
    #[cfg(test)]
    pub(crate) fn len(&self) -> usize {
        self.inner.read().unwrap_or_else(PoisonError::into_inner).len()
    }

    /// Whether an engine for `key` is resident. Does not change its recency.
    #[cfg(test)]
    pub(crate) fn contains(&self, key: &K) -> bool {
        self.inner.read().unwrap_or_else(PoisonError::into_inner).contains(key)
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::sync::Weak;

    fn cache() -> EngineCache<String, u32> {
        EngineCache::unbounded()
    }

    fn load(cache: &EngineCache<String, u32>, name: &str, value: u32) -> Arc<u32> {
        cache
            .get_or_try_init(name.to_string(), || Ok::<_, ()>(value))
            .expect("init cannot fail")
    }

    #[test]
    fn get_or_try_init_builds_once_per_key() {
        let cache = cache();
        let first = load(&cache, "a", 1);
        let second = cache
            .get_or_try_init("a".to_string(), || Ok::<_, ()>(99))
            .expect("init cannot fail");
        assert!(Arc::ptr_eq(&first, &second));
        assert_eq!(*second, 1);
        assert_eq!(cache.len(), 1);
    }

    #[test]
    fn get_or_try_init_keeps_nothing_when_init_fails() {
        let cache = cache();
        let result = cache.get_or_try_init("a".to_string(), || Err::<u32, _>("download failed"));
        assert_eq!(result.unwrap_err(), "download failed");
        assert_eq!(cache.len(), 0);
    }

    #[test]
    fn bound_of_n_with_n_plus_one_models_drops_the_least_recently_used() {
        let cache = cache();
        cache.set_limit(NonZeroUsize::new(2));
        load(&cache, "a", 1);
        load(&cache, "b", 2);
        // Touch "a" so "b" becomes the least recently used engine.
        load(&cache, "a", 1);
        load(&cache, "c", 3);
        assert_eq!(cache.len(), 2);
        assert!(cache.contains(&"a".to_string()));
        assert!(!cache.contains(&"b".to_string()));
        assert!(cache.contains(&"c".to_string()));
    }

    #[test]
    fn lowering_the_limit_drops_engines_at_once() {
        let cache = cache();
        load(&cache, "a", 1);
        load(&cache, "b", 2);
        load(&cache, "c", 3);
        cache.set_limit(NonZeroUsize::new(1));
        assert_eq!(cache.len(), 1);
        assert!(cache.contains(&"c".to_string()));
        cache.set_limit(None);
        load(&cache, "a", 1);
        load(&cache, "b", 2);
        assert_eq!(cache.len(), 3);
    }

    #[test]
    fn evict_where_removes_only_matching_keys_and_drops_the_engine() {
        let cache = cache();
        let held: Weak<u32> = Arc::downgrade(&load(&cache, "a", 1));
        load(&cache, "b", 2);
        assert!(held.upgrade().is_some());
        assert_eq!(cache.evict_where(|key| key == "a"), 1);
        assert!(held.upgrade().is_none(), "the cache held the last handle");
        assert!(!cache.contains(&"a".to_string()));
        assert!(cache.contains(&"b".to_string()));
        assert_eq!(cache.evict_where(|key| key == "a"), 0);
    }

    #[test]
    fn a_caller_holding_a_handle_keeps_the_engine_alive_after_eviction() {
        let cache = cache();
        let handle = load(&cache, "a", 1);
        assert_eq!(cache.clear(), 1);
        assert_eq!(*handle, 1);
        let rebuilt = load(&cache, "a", 7);
        assert!(!Arc::ptr_eq(&handle, &rebuilt), "a fresh call builds a new engine");
        assert_eq!(*rebuilt, 7);
    }

    #[test]
    fn clear_reports_the_count_and_empties_the_cache() {
        let cache = cache();
        load(&cache, "a", 1);
        load(&cache, "b", 2);
        assert_eq!(cache.clear(), 2);
        assert_eq!(cache.len(), 0);
        assert_eq!(cache.clear(), 0);
    }
}
