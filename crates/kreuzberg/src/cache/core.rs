//! Core cache implementation with GenericCache struct.
//!
//! # Lock Poisoning Handling
//!
//! This module uses `Arc<Mutex<T>>` for thread-safe state management and implements
//! explicit lock poisoning recovery throughout all public methods.
//!
//! All `.lock()` calls use `.map_err()` to convert `PoisonError` into `KreuzbergError::LockPoisoned`.
//! The error propagates to callers via `Result` returns (never `.unwrap()` on locks).

use crate::error::{KreuzbergError, Result};
use serde::{Deserialize, Serialize};
use std::collections::HashSet;
use std::fs;
use std::path::{Path, PathBuf};
use std::sync::{Arc, Mutex};
use std::time::SystemTime;

use super::cleanup::smart_cleanup_cache;

/// Minimum seconds between automatic cleanup runs (5 minutes).
const CLEANUP_INTERVAL_SECS: u64 = 300;

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CacheStats {
    pub total_files: usize,
    pub total_size_mb: f64,
    pub available_space_mb: f64,
    pub oldest_file_age_days: f64,
    pub newest_file_age_days: f64,
}

#[derive(Debug, Clone)]
pub(super) struct CacheEntry {
    pub(super) path: PathBuf,
    pub(super) size: u64,
    pub(super) modified: SystemTime,
}

pub(super) struct CacheScanResult {
    pub(super) stats: CacheStats,
    pub(super) entries: Vec<CacheEntry>,
}

pub struct GenericCache {
    cache_dir: PathBuf,
    cache_type: String,
    max_age_days: f64,
    max_cache_size_mb: f64,
    min_free_space_mb: f64,
    processing_locks: Arc<Mutex<HashSet<String>>>,
    /// Tracks cache keys being deleted to prevent read-during-delete race conditions
    deleting_files: Arc<Mutex<HashSet<PathBuf>>>,
}

impl GenericCache {
    pub fn new(
        cache_type: String,
        cache_dir: Option<String>,
        max_age_days: f64,
        max_cache_size_mb: f64,
        min_free_space_mb: f64,
    ) -> Result<Self> {
        let cache_dir_path = if let Some(dir) = cache_dir {
            PathBuf::from(dir).join(&cache_type)
        } else if let Ok(env_path) = std::env::var("KREUZBERG_CACHE_DIR") {
            PathBuf::from(env_path).join(&cache_type)
        } else {
            std::env::current_dir()?.join(".kreuzberg").join(&cache_type)
        };

        fs::create_dir_all(&cache_dir_path)
            .map_err(|e| KreuzbergError::cache(format!("Failed to create cache directory: {}", e)))?;

        Ok(Self {
            cache_dir: cache_dir_path,
            cache_type,
            max_age_days,
            max_cache_size_mb,
            min_free_space_mb,
            processing_locks: Arc::new(Mutex::new(HashSet::new())),
            deleting_files: Arc::new(Mutex::new(HashSet::new())),
        })
    }

    /// Resolve the directory for a cache key, optionally within a namespace subdirectory.
    fn resolve_dir(&self, namespace: Option<&str>) -> PathBuf {
        match namespace {
            Some(ns) => self.cache_dir.join(ns),
            None => self.cache_dir.clone(),
        }
    }

    fn get_cache_path(&self, cache_key: &str, namespace: Option<&str>) -> PathBuf {
        self.resolve_dir(namespace).join(format!("{}.msgpack", cache_key))
    }

    fn get_metadata_path(&self, cache_key: &str, namespace: Option<&str>) -> PathBuf {
        self.resolve_dir(namespace).join(format!("{}.meta", cache_key))
    }

    fn is_valid(&self, cache_path: &Path, source_file: Option<&str>, ttl_override_secs: Option<u64>) -> bool {
        if !cache_path.exists() {
            return false;
        }

        if let Ok(metadata) = fs::metadata(cache_path)
            && let Ok(modified) = metadata.modified()
            && let Ok(elapsed) = SystemTime::now().duration_since(modified)
        {
            // Check TTL from .meta file first, then override, then global max_age_days
            let max_age_secs = if let Some(ttl) = ttl_override_secs {
                ttl as f64
            } else if let Some(meta_ttl) = self.read_meta_ttl(cache_path) {
                if meta_ttl > 0 {
                    meta_ttl as f64
                } else {
                    self.max_age_days * 86400.0
                }
            } else {
                self.max_age_days * 86400.0
            };

            if elapsed.as_secs_f64() > max_age_secs {
                return false;
            }
        }

        if let Some(source_path) = source_file {
            let Some(file_stem) = cache_path.file_stem().and_then(|s| s.to_str()) else {
                return false;
            };
            let namespace = self.infer_namespace(cache_path);
            let meta_path = self.get_metadata_path(file_stem, namespace.as_deref());

            if meta_path.exists() {
                if let Ok(cached_meta_bytes) = fs::read(&meta_path)
                    && cached_meta_bytes.len() >= 16
                {
                    let cached_size = u64::from_le_bytes(cached_meta_bytes[0..8].try_into().unwrap());
                    let cached_mtime = u64::from_le_bytes(cached_meta_bytes[8..16].try_into().unwrap());

                    if let Ok(source_metadata) = fs::metadata(source_path) {
                        let current_size = source_metadata.len();
                        let Some(current_mtime) = source_metadata
                            .modified()
                            .ok()
                            .and_then(|t| t.duration_since(std::time::UNIX_EPOCH).ok())
                            .map(|d| d.as_secs())
                        else {
                            return false;
                        };

                        return cached_size == current_size && cached_mtime == current_mtime;
                    }
                }
                return false;
            }
        }

        true
    }

    /// Read TTL from .meta file (bytes 16-23). Returns None if not present.
    fn read_meta_ttl(&self, cache_path: &Path) -> Option<u64> {
        let file_stem = cache_path.file_stem()?.to_str()?;
        let namespace = self.infer_namespace(cache_path);
        let meta_path = self.get_metadata_path(file_stem, namespace.as_deref());
        let bytes = fs::read(&meta_path).ok()?;
        if bytes.len() >= 24 {
            Some(u64::from_le_bytes(bytes[16..24].try_into().unwrap()))
        } else {
            None // Old-format 16-byte .meta, no TTL stored
        }
    }

    /// Infer namespace from a cache path by checking if it's in a subdirectory.
    fn infer_namespace(&self, cache_path: &Path) -> Option<String> {
        let parent = cache_path.parent()?;
        if parent == self.cache_dir {
            None
        } else {
            parent.file_name()?.to_str().map(|s| s.to_string())
        }
    }

    fn save_metadata(
        &self,
        cache_key: &str,
        source_file: Option<&str>,
        namespace: Option<&str>,
        ttl_secs: Option<u64>,
    ) {
        let meta_path = self.get_metadata_path(cache_key, namespace);

        let mut bytes = Vec::with_capacity(24);

        if let Some(source_path) = source_file
            && let Ok(metadata) = fs::metadata(source_path)
        {
            let size = metadata.len();
            let mtime = metadata
                .modified()
                .ok()
                .and_then(|t| t.duration_since(std::time::UNIX_EPOCH).ok())
                .map(|d| d.as_secs())
                .unwrap_or(0);

            bytes.extend_from_slice(&size.to_le_bytes());
            bytes.extend_from_slice(&mtime.to_le_bytes());
        } else {
            bytes.extend_from_slice(&0u64.to_le_bytes());
            bytes.extend_from_slice(&0u64.to_le_bytes());
        }

        // TTL in seconds (0 = use global default)
        bytes.extend_from_slice(&ttl_secs.unwrap_or(0).to_le_bytes());

        let _ = fs::write(meta_path, bytes);
    }

    #[cfg_attr(feature = "otel", tracing::instrument(
        skip(self),
        fields(
            cache.hit = tracing::field::Empty,
            cache.key = %cache_key,
        )
    ))]
    pub fn get(
        &self,
        cache_key: &str,
        source_file: Option<&str>,
        namespace: Option<&str>,
        ttl_override_secs: Option<u64>,
    ) -> Result<Option<Vec<u8>>> {
        let cache_path = self.get_cache_path(cache_key, namespace);

        {
            let deleting = self
                .deleting_files
                .lock()
                .map_err(|e| KreuzbergError::LockPoisoned(format!("Deleting files mutex poisoned: {}", e)))?;
            if deleting.contains(&cache_path) {
                #[cfg(feature = "otel")]
                tracing::Span::current().record("cache.hit", false);
                return Ok(None);
            }
        }

        if !self.is_valid(&cache_path, source_file, ttl_override_secs) {
            #[cfg(feature = "otel")]
            tracing::Span::current().record("cache.hit", false);
            return Ok(None);
        }

        match fs::read(&cache_path) {
            Ok(content) => {
                #[cfg(feature = "otel")]
                tracing::Span::current().record("cache.hit", true);
                Ok(Some(content))
            }
            Err(_) => {
                if let Err(e) = fs::remove_file(&cache_path) {
                    tracing::debug!("Failed to remove corrupted cache file: {}", e);
                }
                let meta_path = self.get_metadata_path(cache_key, namespace);
                if let Err(e) = fs::remove_file(meta_path) {
                    tracing::debug!("Failed to remove corrupted metadata file: {}", e);
                }
                #[cfg(feature = "otel")]
                tracing::Span::current().record("cache.hit", false);
                Ok(None)
            }
        }
    }

    /// Backward-compatible get without namespace/TTL.
    pub fn get_simple(&self, cache_key: &str, source_file: Option<&str>) -> Result<Option<Vec<u8>>> {
        self.get(cache_key, source_file, None, None)
    }

    #[cfg_attr(feature = "otel", tracing::instrument(
        skip(self, data),
        fields(
            cache.key = %cache_key,
            cache.size_bytes = data.len(),
        )
    ))]
    pub fn set(
        &self,
        cache_key: &str,
        data: Vec<u8>,
        source_file: Option<&str>,
        namespace: Option<&str>,
        ttl_secs: Option<u64>,
    ) -> Result<()> {
        // create_dir_all is idempotent — safe for concurrent multi-worker calls
        let dir = self.resolve_dir(namespace);
        fs::create_dir_all(&dir)
            .map_err(|e| KreuzbergError::cache(format!("Failed to create cache namespace dir: {}", e)))?;

        let cache_path = self.get_cache_path(cache_key, namespace);

        fs::write(&cache_path, &data)
            .map_err(|e| KreuzbergError::cache(format!("Failed to write cache file: {}", e)))?;

        self.save_metadata(cache_key, source_file, namespace, ttl_secs);

        if self.should_run_cleanup() {
            if let Some(cache_path_str) = self.cache_dir.to_str() {
                let _ = smart_cleanup_cache(
                    cache_path_str,
                    self.max_age_days,
                    self.max_cache_size_mb,
                    self.min_free_space_mb,
                );
            }
            self.touch_cleanup_marker();
        }

        Ok(())
    }

    /// Backward-compatible set without namespace/TTL.
    pub fn set_simple(&self, cache_key: &str, data: Vec<u8>, source_file: Option<&str>) -> Result<()> {
        self.set(cache_key, data, source_file, None, None)
    }

    /// Check if cleanup should run based on filesystem marker timestamp.
    ///
    /// Multi-worker safe: uses filesystem mtime instead of in-memory counter.
    fn should_run_cleanup(&self) -> bool {
        let marker = self.cache_dir.join(".last_cleanup");
        match fs::metadata(&marker) {
            Ok(meta) => {
                if let Ok(modified) = meta.modified() {
                    let age = SystemTime::now().duration_since(modified).unwrap_or_default();
                    age.as_secs() > CLEANUP_INTERVAL_SECS
                } else {
                    true
                }
            }
            Err(_) => true,
        }
    }

    /// Touch the cleanup marker file to record last cleanup time.
    fn touch_cleanup_marker(&self) {
        let marker = self.cache_dir.join(".last_cleanup");
        let _ = fs::write(&marker, []);
    }

    pub fn is_processing(&self, cache_key: &str) -> Result<bool> {
        let locks = self
            .processing_locks
            .lock()
            .map_err(|e| KreuzbergError::LockPoisoned(format!("Processing locks mutex poisoned: {}", e)))?;
        Ok(locks.contains(cache_key))
    }

    pub fn mark_processing(&self, cache_key: String) -> Result<()> {
        let mut locks = self
            .processing_locks
            .lock()
            .map_err(|e| KreuzbergError::LockPoisoned(format!("Processing locks mutex poisoned: {}", e)))?;
        locks.insert(cache_key);
        Ok(())
    }

    pub fn mark_complete(&self, cache_key: &str) -> Result<()> {
        let mut locks = self
            .processing_locks
            .lock()
            .map_err(|e| KreuzbergError::LockPoisoned(format!("Processing locks mutex poisoned: {}", e)))?;
        locks.remove(cache_key);
        Ok(())
    }

    fn mark_for_deletion(&self, path: &Path) -> Result<()> {
        let mut deleting = self
            .deleting_files
            .lock()
            .map_err(|e| KreuzbergError::LockPoisoned(format!("Deleting files mutex poisoned: {}", e)))?;
        deleting.insert(path.to_path_buf());
        Ok(())
    }

    fn unmark_deletion(&self, path: &Path) -> Result<()> {
        let mut deleting = self
            .deleting_files
            .lock()
            .map_err(|e| KreuzbergError::LockPoisoned(format!("Deleting files mutex poisoned: {}", e)))?;
        deleting.remove(&path.to_path_buf());
        Ok(())
    }

    pub fn clear(&self) -> Result<(usize, f64)> {
        let dir_path = &self.cache_dir;

        if !dir_path.exists() {
            return Ok((0, 0.0));
        }

        let mut removed_count = 0;
        let mut removed_size = 0.0;

        let read_dir = fs::read_dir(dir_path)
            .map_err(|e| KreuzbergError::cache(format!("Failed to read cache directory: {}", e)))?;

        for entry in read_dir {
            let entry = match entry {
                Ok(e) => e,
                Err(e) => {
                    tracing::debug!("Error reading entry: {}", e);
                    continue;
                }
            };

            let path = entry.path();

            // Skip the cleanup marker file
            if path.file_name().and_then(|n| n.to_str()) == Some(".last_cleanup") {
                continue;
            }

            let metadata = match entry.metadata() {
                Ok(m) => m,
                Err(_) => continue,
            };

            // Recursively clear namespace subdirectories
            if metadata.is_dir() {
                let (ns_removed, ns_freed) = self.delete_namespace_inner(&path)?;
                removed_count += ns_removed;
                removed_size += ns_freed;
                continue;
            }

            if !metadata.is_file() {
                continue;
            }

            let ext = path.extension().and_then(|s| s.to_str());
            if ext != Some("msgpack") && ext != Some("meta") {
                continue;
            }

            let size_mb = metadata.len() as f64 / (1024.0 * 1024.0);

            if let Err(e) = self.mark_for_deletion(&path) {
                tracing::debug!("Failed to mark file for deletion: {} (continuing anyway)", e);
            }

            match fs::remove_file(&path) {
                Ok(_) => {
                    removed_count += 1;
                    removed_size += size_mb;
                    if let Err(e) = self.unmark_deletion(&path) {
                        tracing::debug!("Failed to unmark deleted file: {} (non-critical)", e);
                    }
                }
                Err(e) => {
                    tracing::debug!("Failed to remove {:?}: {}", path, e);
                    if let Err(e) = self.unmark_deletion(&path) {
                        tracing::debug!("Failed to unmark file after deletion error: {} (non-critical)", e);
                    }
                }
            }
        }

        Ok((removed_count, removed_size))
    }

    /// Delete all cache entries under a namespace.
    ///
    /// Removes the namespace subdirectory and all its contents.
    /// Returns (files_removed, mb_freed).
    pub fn delete_namespace(&self, namespace: &str) -> Result<(usize, f64)> {
        let ns_dir = self.cache_dir.join(namespace);
        self.delete_namespace_inner(&ns_dir)
    }

    /// Inner implementation: remove a directory and count its contents.
    fn delete_namespace_inner(&self, dir: &Path) -> Result<(usize, f64)> {
        if !dir.exists() {
            return Ok((0, 0.0));
        }

        let mut removed_count = 0;
        let mut removed_size = 0.0;

        // Count files before removal
        if let Ok(read_dir) = fs::read_dir(dir) {
            for entry in read_dir.flatten() {
                if let Ok(meta) = entry.metadata()
                    && meta.is_file()
                {
                    removed_size += meta.len() as f64 / (1024.0 * 1024.0);
                    removed_count += 1;
                }
            }
        }

        fs::remove_dir_all(dir)
            .map_err(|e| KreuzbergError::cache(format!("Failed to remove directory {}: {}", dir.display(), e)))?;

        Ok((removed_count, removed_size))
    }

    pub fn get_stats(&self) -> Result<CacheStats> {
        self.get_stats_filtered(None)
    }

    /// Get cache stats, optionally filtered to a specific namespace.
    pub fn get_stats_filtered(&self, namespace: Option<&str>) -> Result<CacheStats> {
        let dir = self.resolve_dir(namespace);
        let dir_str = dir
            .to_str()
            .ok_or_else(|| KreuzbergError::validation("Cache directory path contains invalid UTF-8".to_string()))?;
        super::cleanup::get_cache_metadata(dir_str)
    }

    pub fn cache_dir(&self) -> &Path {
        &self.cache_dir
    }

    pub fn cache_type(&self) -> &str {
        &self.cache_type
    }
}
