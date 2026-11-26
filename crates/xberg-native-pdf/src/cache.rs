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

    pub(crate) fn len(&self) -> usize {
        self.map.len()
    }
}

/// Cache bounded by an estimated byte budget that admits entries while the
/// budget lasts and never evicts.
///
/// Whole-document passes read pages 0..N one after another. Under LRU or FIFO
/// eviction a budget smaller than the document makes every later pass miss on
/// every page, because each page is evicted before the next pass reaches it;
/// admitting until full keeps the first pages that fit warm for every pass. ~keep
pub(crate) struct AdmissionBudgetCache<K: Eq + std::hash::Hash, V> {
    map: HashMap<K, (V, usize)>,
    used_bytes: usize,
    budget_bytes: usize,
}

impl<K: Eq + std::hash::Hash, V> AdmissionBudgetCache<K, V> {
    pub(crate) fn new(budget_bytes: usize) -> Self {
        Self {
            map: HashMap::new(),
            used_bytes: 0,
            budget_bytes,
        }
    }

    pub(crate) fn get(&self, key: &K) -> Option<&V> {
        self.map.get(key).map(|(value, _)| value)
    }

    /// Whether [`Self::insert`] would store an entry of `bytes` under `key`.
    pub(crate) fn admits(&self, key: &K, bytes: usize) -> bool {
        let replaced = self.map.get(key).map_or(0, |(_, bytes)| *bytes);
        bytes <= self.budget_bytes - (self.used_bytes - replaced)
    }

    /// The bytes in use once an entry of `bytes` is stored under `key`.
    pub(crate) fn used_bytes_with(&self, key: &K, bytes: usize) -> usize {
        let replaced = self.map.get(key).map_or(0, |(_, bytes)| *bytes);
        self.used_bytes - replaced + bytes
    }

    /// Store `value` under `key`, replacing an existing entry, when the budget
    /// still fits it. Returns whether the value was stored; a value that does
    /// not fit leaves any existing entry in place.
    pub(crate) fn insert(&mut self, key: K, value: V, bytes: usize) -> bool {
        if !self.admits(&key, bytes) {
            return false;
        }
        let replaced = self.map.get(&key).map_or(0, |(_, bytes)| *bytes);
        self.used_bytes = self.used_bytes - replaced + bytes;
        self.map.insert(key, (value, bytes));
        true
    }

    pub(crate) fn clear(&mut self) {
        self.map.clear();
        self.used_bytes = 0;
    }

    #[cfg(test)]
    pub(crate) fn len(&self) -> usize {
        self.map.len()
    }

    #[cfg(test)]
    pub(crate) fn used_bytes(&self) -> usize {
        self.used_bytes
    }

    pub(crate) fn budget_bytes(&self) -> usize {
        self.budget_bytes
    }
}
