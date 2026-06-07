//! Cache cleanup operations for managing cache size and age.

use crate::error::{KreuzbergError, Result};
use std::fs;
use std::path::Path;
use std::time::{SystemTime, UNIX_EPOCH};

use super::core::{CacheEntry, CacheScanResult, CacheStats};
use super::utilities::get_available_disk_space;

pub(super) fn scan_cache_directory(cache_dir: &str) -> Result<CacheScanResult> {
    let dir_path = Path::new(cache_dir);

    if !dir_path.exists() {
        return Ok(CacheScanResult {
            stats: CacheStats {
                total_files: 0,
                total_size_mb: 0.0,
                available_space_mb: get_available_disk_space(cache_dir)?,
                oldest_file_age_days: 0.0,
                newest_file_age_days: 0.0,
            },
            entries: Vec::new(),
        });
    }

    let current_time = SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .unwrap_or_default()
        .as_secs() as f64;

    let read_dir =
        fs::read_dir(dir_path).map_err(|e| KreuzbergError::cache(format!("Failed to read cache directory: {}", e)))?;

    let mut total_size = 0u64;
    let mut oldest_age = 0.0f64;
    let mut newest_age = f64::INFINITY;
    let mut entries = Vec::new();

    for entry in read_dir {
        let entry = match entry {
            Ok(e) => e,
            Err(e) => {
                tracing::debug!("Error reading cache entry: {}", e);
                continue;
            }
        };

        let metadata = match entry.metadata() {
            Ok(m) if m.is_file() => m,
            _ => continue,
        };

        let path = entry.path();
        if path.extension().and_then(|s| s.to_str()) != Some("msgpack") {
            continue;
        }

        let modified = match metadata.modified() {
            Ok(m) => m,
            Err(e) => {
                tracing::debug!("Error getting modification time for {:?}: {}", path, e);
                continue;
            }
        };

        let size = metadata.len();
        total_size += size;

        if let Ok(duration) = modified.duration_since(UNIX_EPOCH) {
            let age_days = (current_time - duration.as_secs() as f64) / (24.0 * 3600.0);
            oldest_age = oldest_age.max(age_days);
            newest_age = newest_age.min(age_days);
        }

        entries.push(CacheEntry { path, size, modified });
    }

    if entries.is_empty() {
        oldest_age = 0.0;
        newest_age = 0.0;
    }

    Ok(CacheScanResult {
        stats: CacheStats {
            total_files: entries.len(),
            total_size_mb: total_size as f64 / (1024.0 * 1024.0),
            available_space_mb: get_available_disk_space(cache_dir)?,
            oldest_file_age_days: oldest_age,
            newest_file_age_days: newest_age,
        },
        entries,
    })
}

/// Return aggregate statistics for the cache directory at `cache_dir`.
///
/// # Errors
///
/// Returns an error if the directory cannot be read or its metadata cannot be queried.
#[cfg_attr(alef, alef(skip))]
pub fn get_cache_metadata(cache_dir: &str) -> Result<CacheStats> {
    let scan_result = scan_cache_directory(cache_dir)?;
    Ok(scan_result.stats)
}

pub(crate) fn cleanup_cache(
    cache_dir: &str,
    max_age_days: f64,
    max_size_mb: f64,
    target_size_ratio: f64,
) -> Result<(usize, f64)> {
    let scan_result = scan_cache_directory(cache_dir)?;

    if scan_result.entries.is_empty() {
        return Ok((0, 0.0));
    }

    let current_time = SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .unwrap_or_default()
        .as_secs() as f64;
    let max_age_seconds = max_age_days * 24.0 * 3600.0;

    let mut removed_count = 0;
    let mut removed_size = 0.0;
    let mut remaining_entries = Vec::new();
    let mut total_remaining_size = 0u64;

    for entry in scan_result.entries {
        if let Ok(age) = entry.modified.duration_since(UNIX_EPOCH) {
            let age_seconds = current_time - age.as_secs() as f64;
            if age_seconds > max_age_seconds {
                match fs::remove_file(&entry.path) {
                    Ok(_) => {
                        removed_count += 1;
                        removed_size += entry.size as f64 / (1024.0 * 1024.0);
                    }
                    Err(e) => {
                        tracing::debug!("Failed to remove {:?}: {}", entry.path, e);
                    }
                }
            } else {
                total_remaining_size += entry.size;
                remaining_entries.push(entry);
            }
        }
    }

    let mut total_size_mb = total_remaining_size as f64 / (1024.0 * 1024.0);

    if total_size_mb > max_size_mb {
        remaining_entries.sort_by_key(|e| e.modified);

        let target_size = max_size_mb * target_size_ratio;

        for entry in remaining_entries {
            if total_size_mb <= target_size {
                break;
            }

            match fs::remove_file(&entry.path) {
                Ok(_) => {
                    let size_mb = entry.size as f64 / (1024.0 * 1024.0);
                    removed_count += 1;
                    removed_size += size_mb;
                    total_size_mb -= size_mb;
                }
                Err(e) => {
                    tracing::debug!("Failed to remove {:?}: {}", entry.path, e);
                }
            }
        }
    }

    Ok((removed_count, removed_size))
}

pub(crate) fn smart_cleanup_cache(
    cache_dir: &str,
    max_age_days: f64,
    max_size_mb: f64,
    min_free_space_mb: f64,
) -> Result<(usize, f64)> {
    let stats = get_cache_metadata(cache_dir)?;

    let needs_cleanup = stats.available_space_mb < min_free_space_mb
        || stats.total_size_mb > max_size_mb
        || stats.oldest_file_age_days > max_age_days;

    if !needs_cleanup {
        return Ok((0, 0.0));
    }

    let target_ratio = if stats.available_space_mb < min_free_space_mb {
        0.5
    } else {
        0.8
    };

    cleanup_cache(cache_dir, max_age_days, max_size_mb, target_ratio)
}

/// Delete every file in `cache_dir`, returning the count and total size (MB) removed.
///
/// Returns `(0, 0.0)` if the directory does not exist. Files that cannot be removed
/// are skipped with a debug log; errors do not abort the clearing operation.
///
/// # Errors
///
/// Returns an error if the directory cannot be read.
#[cfg_attr(alef, alef(skip))]
pub fn clear_cache_directory(cache_dir: &str) -> Result<(usize, f64)> {
    let dir_path = Path::new(cache_dir);

    if !dir_path.exists() {
        return Ok((0, 0.0));
    }

    let mut removed_count = 0;
    let mut removed_size = 0.0;

    let read_dir =
        fs::read_dir(dir_path).map_err(|e| KreuzbergError::cache(format!("Failed to read cache directory: {}", e)))?;

    for entry in read_dir {
        let entry = match entry {
            Ok(e) => e,
            Err(e) => {
                tracing::debug!("Error reading entry: {}", e);
                continue;
            }
        };

        let metadata = match entry.metadata() {
            Ok(m) if m.is_file() => m,
            _ => continue,
        };

        let path = entry.path();
        if path.extension().and_then(|s| s.to_str()) != Some("msgpack") {
            continue;
        }

        let size_mb = metadata.len() as f64 / (1024.0 * 1024.0);
        match fs::remove_file(&path) {
            Ok(_) => {
                removed_count += 1;
                removed_size += size_mb;
            }
            Err(e) => {
                tracing::debug!("Failed to remove {:?}: {}", path, e);
            }
        }
    }

    Ok((removed_count, removed_size))
}
