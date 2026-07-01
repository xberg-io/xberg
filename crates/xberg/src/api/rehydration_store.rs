//! In-memory store for encrypted rehydration map blobs.

use std::time::Duration;

use moka::sync::Cache;

const REHYDRATION_TTL: Duration = Duration::from_secs(24 * 60 * 60);
const MAX_CAPACITY: u64 = 10_000;

#[derive(Clone)]
pub struct RehydrationStore {
    blobs: Cache<String, Vec<u8>>,
}

impl Default for RehydrationStore {
    fn default() -> Self {
        Self::new()
    }
}

impl RehydrationStore {
    pub fn new() -> Self {
        let blobs = Cache::builder()
            .max_capacity(MAX_CAPACITY)
            .time_to_live(REHYDRATION_TTL)
            .build();
        Self { blobs }
    }

    pub fn store(&self, encrypted: Vec<u8>) -> String {
        let key = format!("reh_{}", uuid::Uuid::new_v4());
        self.blobs.insert(key.clone(), encrypted);
        key
    }

    pub fn get(&self, key: &str) -> Option<Vec<u8>> {
        self.blobs.get(key)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn store_then_get_round_trips() {
        let store = RehydrationStore::new();
        let key = store.store(vec![1, 2, 3]);
        assert!(key.starts_with("reh_"));
        assert_eq!(store.get(&key), Some(vec![1, 2, 3]));
    }

    #[test]
    fn get_missing_key_returns_none() {
        let store = RehydrationStore::new();
        assert_eq!(store.get("reh_nonexistent"), None);
    }

    #[test]
    fn each_store_call_gets_a_distinct_key() {
        let store = RehydrationStore::new();
        let a = store.store(vec![1]);
        let b = store.store(vec![1]);
        assert_ne!(a, b);
    }
}
