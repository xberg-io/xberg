//! General-purpose caching utilities.
//!
//! This module contains cache types and helper traits that are used across
//! multiple modules (e.g., `document`, `fonts::cmap`).

use std::collections::HashMap;
use std::sync::Mutex;

/// Extension trait for Mutex to recover from poisoned locks.
///
/// When a thread panics while holding a Mutex, the lock becomes "poisoned"
/// and the standard `.lock().unwrap()` would cascade panics. This trait
/// provides `.lock_or_recover()` which discards the poison flag and returns
/// the inner data, since the Mutexes in PdfDocument protect caches and
/// bookkeeping (not safety-critical invariants).
pub(crate) trait MutexExt<T> {
    /// Lock the mutex, recovering from poison if needed.
    fn lock_or_recover(&self) -> std::sync::MutexGuard<'_, T>;
}

impl<T> MutexExt<T> for Mutex<T> {
    fn lock_or_recover(&self) -> std::sync::MutexGuard<'_, T> {
        self.lock().unwrap_or_else(|poisoned| {
            tracing::warn!("mutex was poisoned by a panicking thread; recovering");
            poisoned.into_inner()
        })
    }
}

/// Entry-count-bounded cache with LRU-style eviction.
///
/// Used for caches where estimating per-entry byte size is impractical
/// (e.g., `Vec<TextSpan>`, `Vec<PdfImage>`). On insert, when at capacity,
/// the least-recently-used entry is evicted. `get()` promotes the accessed
/// key to the most-recently-used position so hot entries survive eviction.
pub(crate) struct BoundedEntryCache<K: Eq + std::hash::Hash + Copy, V> {
    map: HashMap<K, V>,
    insertion_order: std::collections::VecDeque<K>,
    max_entries: usize,
}

impl<K: Eq + std::hash::Hash + Copy, V> BoundedEntryCache<K, V> {
    pub(crate) fn new(max_entries: usize) -> Self {
        Self {
            map: HashMap::new(),
            insertion_order: std::collections::VecDeque::new(),
            max_entries,
        }
    }

    pub(crate) fn get(&mut self, key: &K) -> Option<&V> {
        if self.map.contains_key(key) {
            if let Some(pos) = self.insertion_order.iter().position(|k| k == key) {
                self.insertion_order.remove(pos);
            }
            self.insertion_order.push_back(*key);
        }
        self.map.get(key)
    }

    pub(crate) fn insert(&mut self, key: K, value: V) {
        use std::collections::hash_map::Entry;
        // On re-insert: replace value and promote to most-recently-used position
        // so LRU eviction order stays accurate. ~keep
        if let Entry::Occupied(mut e) = self.map.entry(key) {
            e.insert(value);
            if let Some(pos) = self.insertion_order.iter().position(|k| k == &key) {
                self.insertion_order.remove(pos);
            }
            self.insertion_order.push_back(key);
            return;
        }
        while self.map.len() >= self.max_entries && !self.insertion_order.is_empty() {
            if let Some(old_key) = self.insertion_order.pop_front() {
                self.map.remove(&old_key);
            }
        }
        self.map.insert(key, value);
        self.insertion_order.push_back(key);
    }

    pub(crate) fn clear(&mut self) {
        self.map.clear();
        self.insertion_order.clear();
    }

    pub(crate) fn remove(&mut self, key: &K) {
        if self.map.remove(key).is_some()
            && let Some(pos) = self.insertion_order.iter().position(|k| k == key)
        {
            self.insertion_order.remove(pos);
        }
    }

    pub(crate) fn len(&self) -> usize {
        self.map.len()
    }
}

/// Cache bounded by an estimated byte budget that pins entries while the budget
/// lasts and never evicts them, backed by a small LRU for entries past it.
///
/// Whole-document passes read pages 0..N one after another. Under LRU or FIFO
/// eviction alone a capacity smaller than the document makes every later pass
/// miss on every page, because each page is evicted before the next pass reaches
/// it; pinning until full keeps the first pages that fit warm for every pass. The
/// LRU keeps the back-to-back reads of one page past the budget served. ~keep
pub(crate) struct AdmissionBudgetCache<K: Eq + std::hash::Hash + Copy, V> {
    pinned: HashMap<K, (V, usize)>,
    used_bytes: usize,
    budget_bytes: usize,
    recent: BoundedEntryCache<K, V>,
}

impl<K: Eq + std::hash::Hash + Copy, V> AdmissionBudgetCache<K, V> {
    pub(crate) fn new(budget_bytes: usize, recent_entries: usize) -> Self {
        Self {
            pinned: HashMap::new(),
            used_bytes: 0,
            budget_bytes,
            recent: BoundedEntryCache::new(recent_entries),
        }
    }

    pub(crate) fn get(&mut self, key: &K) -> Option<&V> {
        if self.pinned.contains_key(key) {
            return self.pinned.get(key).map(|(value, _)| value);
        }
        self.recent.get(key)
    }

    /// Store `value` under `key`, replacing any existing entry: pinned when the
    /// budget still fits its estimated `bytes`, otherwise in the LRU.
    pub(crate) fn insert(&mut self, key: K, value: V, bytes: usize) {
        if let Some((_, replaced)) = self.pinned.remove(&key) {
            self.used_bytes -= replaced;
        }
        if bytes <= self.budget_bytes - self.used_bytes {
            self.recent.remove(&key);
            self.used_bytes += bytes;
            self.pinned.insert(key, (value, bytes));
        } else {
            self.recent.insert(key, value);
        }
    }

    pub(crate) fn clear(&mut self) {
        self.pinned.clear();
        self.used_bytes = 0;
        self.recent.clear();
    }

    #[cfg(test)]
    pub(crate) fn len(&self) -> usize {
        self.pinned.len() + self.recent.len()
    }
}
