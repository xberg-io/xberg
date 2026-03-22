//! Generic cache implementation with lock poisoning recovery.
//!
//! This module provides a thread-safe caching system with automatic cleanup,
//! processing locks, and validation capabilities.

mod cleanup;
mod core;
mod utilities;

// Re-export all public types and functions for backward compatibility
pub use cleanup::{
    batch_cleanup_caches, cleanup_cache, clear_cache_directory, get_cache_metadata, is_cache_valid, smart_cleanup_cache,
};
pub use core::{CacheStats, GenericCache};
pub use utilities::{
    blake3_hash_bytes, blake3_hash_file, fast_hash, filter_old_cache_entries, generate_cache_key,
    get_available_disk_space, sanitize_namespace, sort_cache_by_access_time, validate_cache_key,
};

#[cfg(test)]
mod tests {
    use super::*;
    use std::fs::File;
    use tempfile::tempdir;

    #[test]
    fn test_generate_cache_key_empty() {
        let result = generate_cache_key(&[]);
        assert_eq!(result, "empty");
    }

    #[test]
    fn test_generate_cache_key_consistent() {
        let parts = [("key1", "value1"), ("key2", "value2")];
        let key1 = generate_cache_key(&parts);
        let key2 = generate_cache_key(&parts);
        assert_eq!(key1, key2);
        assert_eq!(key1.len(), 32);
    }

    #[test]
    fn test_validate_cache_key() {
        assert!(validate_cache_key("0123456789abcdef0123456789abcdef"));
        assert!(!validate_cache_key("invalid_key"));
        assert!(!validate_cache_key("0123456789abcdef"));
        assert!(!validate_cache_key("0123456789abcdef0123456789abcdef0"));
    }

    #[test]
    fn test_fast_hash() {
        let data1 = b"test data";
        let data2 = b"test data";
        let data3 = b"different data";

        assert_eq!(fast_hash(data1), fast_hash(data2));
        assert_ne!(fast_hash(data1), fast_hash(data3));
    }

    #[test]
    fn test_filter_old_cache_entries() {
        let cache_times = vec![100.0, 200.0, 300.0, 400.0];
        let current_time = 500.0;
        let max_age = 200.0;

        let old_indices = filter_old_cache_entries(&cache_times, current_time, max_age);
        assert_eq!(old_indices, vec![0, 1]);
    }

    #[test]
    fn test_sort_cache_by_access_time() {
        let entries = vec![
            ("key3".to_string(), 300.0),
            ("key1".to_string(), 100.0),
            ("key2".to_string(), 200.0),
        ];

        let sorted = sort_cache_by_access_time(entries);
        assert_eq!(sorted, vec!["key1", "key2", "key3"]);
    }

    #[test]
    fn test_sort_cache_with_nan() {
        let entries = vec![
            ("key1".to_string(), 100.0),
            ("key2".to_string(), f64::NAN),
            ("key3".to_string(), 200.0),
        ];

        let sorted = sort_cache_by_access_time(entries);
        assert_eq!(sorted.len(), 3);
    }

    #[test]
    fn test_cache_metadata() {
        let temp_dir = tempdir().unwrap();
        let cache_dir = temp_dir.path().to_str().unwrap();

        let file1 = temp_dir.path().join("test1.msgpack");
        let file2 = temp_dir.path().join("test2.msgpack");
        File::create(&file1).unwrap();
        File::create(&file2).unwrap();

        let stats = get_cache_metadata(cache_dir).unwrap();
        assert_eq!(stats.total_files, 2);
        assert!(stats.available_space_mb > 0.0);
    }

    #[test]
    fn test_cleanup_cache() {
        use std::io::Write;

        let temp_dir = tempdir().unwrap();
        let cache_dir = temp_dir.path().to_str().unwrap();

        let file1 = temp_dir.path().join("old.msgpack");
        let mut f = File::create(&file1).unwrap();
        f.write_all(b"test data for cleanup").unwrap();
        drop(f);

        let (removed_count, _) = cleanup_cache(cache_dir, 1000.0, 0.000001, 0.8).unwrap();
        assert_eq!(removed_count, 1);
        assert!(!file1.exists());
    }

    #[test]
    fn test_is_cache_valid() {
        let temp_dir = tempdir().unwrap();
        let file_path = temp_dir.path().join("test.msgpack");
        File::create(&file_path).unwrap();

        let path_str = file_path.to_str().unwrap();

        assert!(is_cache_valid(path_str, 1.0));

        assert!(!is_cache_valid("/nonexistent/path", 1.0));
    }

    #[test]
    fn test_generic_cache_new() {
        let temp_dir = tempdir().unwrap();
        let cache = GenericCache::new(
            "test".to_string(),
            Some(temp_dir.path().to_str().unwrap().to_string()),
            30.0,
            500.0,
            1000.0,
        )
        .unwrap();

        assert_eq!(cache.cache_type(), "test");
        assert!(cache.cache_dir().exists());
    }

    #[test]
    fn test_generic_cache_get_set() {
        let temp_dir = tempdir().unwrap();
        let cache = GenericCache::new(
            "test".to_string(),
            Some(temp_dir.path().to_str().unwrap().to_string()),
            30.0,
            500.0,
            1000.0,
        )
        .unwrap();

        let cache_key = "test_key";
        let data = b"test data".to_vec();

        cache.set_simple(cache_key, data.clone(), None).unwrap();

        let result = cache.get_simple(cache_key, None).unwrap();
        assert_eq!(result, Some(data));
    }

    #[test]
    fn test_generic_cache_get_miss() {
        let temp_dir = tempdir().unwrap();
        let cache = GenericCache::new(
            "test".to_string(),
            Some(temp_dir.path().to_str().unwrap().to_string()),
            30.0,
            500.0,
            1000.0,
        )
        .unwrap();

        let result = cache.get_simple("nonexistent", None).unwrap();
        assert_eq!(result, None);
    }

    #[test]
    fn test_generic_cache_source_file_invalidation() {
        use std::io::Write;
        use std::thread::sleep;
        use std::time::Duration;

        let temp_dir = tempdir().unwrap();
        let cache = GenericCache::new(
            "test".to_string(),
            Some(temp_dir.path().to_str().unwrap().to_string()),
            30.0,
            500.0,
            1000.0,
        )
        .unwrap();

        let source_file = temp_dir.path().join("source.txt");
        let mut f = File::create(&source_file).unwrap();
        f.write_all(b"original content").unwrap();
        drop(f);

        let cache_key = "test_key";
        let data = b"cached data".to_vec();

        cache
            .set_simple(cache_key, data.clone(), Some(source_file.to_str().unwrap()))
            .unwrap();

        let result = cache
            .get_simple(cache_key, Some(source_file.to_str().unwrap()))
            .unwrap();
        assert_eq!(result, Some(data.clone()));

        sleep(Duration::from_millis(10));
        use std::fs;
        let mut f = fs::OpenOptions::new()
            .write(true)
            .truncate(true)
            .open(&source_file)
            .unwrap();
        f.write_all(b"modified content with different size").unwrap();
        drop(f);

        let result = cache
            .get_simple(cache_key, Some(source_file.to_str().unwrap()))
            .unwrap();
        assert_eq!(result, None);
    }

    #[test]
    fn test_generic_cache_processing_locks() {
        let temp_dir = tempdir().unwrap();
        let cache = GenericCache::new(
            "test".to_string(),
            Some(temp_dir.path().to_str().unwrap().to_string()),
            30.0,
            500.0,
            1000.0,
        )
        .unwrap();

        let cache_key = "test_key";

        assert!(!cache.is_processing(cache_key).unwrap());

        cache.mark_processing(cache_key.to_string()).unwrap();
        assert!(cache.is_processing(cache_key).unwrap());

        cache.mark_complete(cache_key).unwrap();
        assert!(!cache.is_processing(cache_key).unwrap());
    }

    #[test]
    fn test_generic_cache_clear() {
        let temp_dir = tempdir().unwrap();
        let cache = GenericCache::new(
            "test".to_string(),
            Some(temp_dir.path().to_str().unwrap().to_string()),
            30.0,
            500.0,
            1000.0,
        )
        .unwrap();

        cache.set_simple("key1", b"data1".to_vec(), None).unwrap();
        cache.set_simple("key2", b"data2".to_vec(), None).unwrap();

        let (removed, _freed) = cache.clear().unwrap();
        assert!(removed >= 2, "Should remove at least 2 cache entries (got {})", removed);

        assert_eq!(cache.get_simple("key1", None).unwrap(), None);
        assert_eq!(cache.get_simple("key2", None).unwrap(), None);
    }

    #[test]
    fn test_generic_cache_stats() {
        let temp_dir = tempdir().unwrap();
        let cache = GenericCache::new(
            "test".to_string(),
            Some(temp_dir.path().to_str().unwrap().to_string()),
            30.0,
            500.0,
            1000.0,
        )
        .unwrap();

        cache.set_simple("key1", b"test data 1".to_vec(), None).unwrap();
        cache.set_simple("key2", b"test data 2".to_vec(), None).unwrap();

        let stats = cache.get_stats().unwrap();
        assert_eq!(stats.total_files, 2);
        assert!(stats.total_size_mb > 0.0);
        assert!(stats.available_space_mb > 0.0);
    }

    #[test]
    fn test_generic_cache_expired_entry() {
        use std::io::Write;

        let temp_dir = tempdir().unwrap();
        let cache = GenericCache::new(
            "test".to_string(),
            Some(temp_dir.path().to_str().unwrap().to_string()),
            0.000001,
            500.0,
            1000.0,
        )
        .unwrap();

        let cache_key = "test_key";

        let cache_path = cache.cache_dir().join(format!("{}.msgpack", cache_key));
        let mut f = File::create(&cache_path).unwrap();
        f.write_all(b"test data").unwrap();
        drop(f);

        let old_time = std::time::SystemTime::now() - std::time::Duration::from_secs(60);
        filetime::set_file_mtime(&cache_path, filetime::FileTime::from_system_time(old_time)).unwrap();

        let result = cache.get_simple(cache_key, None).unwrap();
        assert_eq!(result, None);
    }

    #[test]
    fn test_generic_cache_properties() {
        let temp_dir = tempdir().unwrap();
        let cache = GenericCache::new(
            "test".to_string(),
            Some(temp_dir.path().to_str().unwrap().to_string()),
            30.0,
            500.0,
            1000.0,
        )
        .unwrap();

        assert_eq!(cache.cache_type(), "test");
        assert!(cache.cache_dir().to_string_lossy().contains("test"));
    }
}
