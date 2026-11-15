//! TAR archive extraction.
//!
//! Provides functions for extracting metadata and text content from TAR archives.
//! Supports plain TAR as well as compressed variants (TAR.GZ, TAR.BZ2).

use super::{ArchiveEntry, ArchiveMetadata, TEXT_EXTENSIONS};
use crate::error::{Result, XbergError};
use crate::extractors::security::SecurityLimits;
use ahash::AHashMap;
use std::io::{Cursor, Read};
use tar::Archive as TarArchive;

/// Extract metadata from a TAR archive.
///
/// # Arguments
///
/// * `bytes` - The TAR archive bytes (can be compressed with gzip or bzip2)
/// * `limits` - Security limits for archive extraction
///
/// # Returns
///
/// Returns `ArchiveMetadata` containing:
/// - Format: "TAR"
/// - File list with paths, sizes, and directory flags
/// - Total file count
/// - Total uncompressed size
///
/// # Errors
///
/// Returns an error if the TAR archive cannot be read or parsed,
/// or if security limits are exceeded.
pub(crate) fn extract_tar_metadata(bytes: &[u8], limits: &SecurityLimits) -> Result<ArchiveMetadata> {
    let cursor = Cursor::new(bytes);
    let mut archive = TarArchive::new(cursor);

    let estimated_entries = bytes.len().saturating_div(512).max(16);
    let mut file_list = Vec::with_capacity(estimated_entries);
    let mut total_size = 0u64;
    let mut file_count = 0;

    let entries = archive
        .entries()
        .map_err(|e| XbergError::parsing(format!("Failed to read TAR archive: {}", e)))?;

    for entry_result in entries {
        let entry = entry_result.map_err(|e| XbergError::parsing(format!("Failed to read TAR entry: {}", e)))?;

        let path = entry
            .path()
            .map_err(|e| XbergError::parsing(format!("Failed to read TAR entry path: {}", e)))?
            .to_string_lossy()
            .to_string();

        let size = entry.size();
        let is_dir = entry.header().entry_type().is_dir();

        if !is_dir {
            total_size = super::add_declared_size(total_size, size);
        }

        file_count += 1;

        if file_count > limits.max_files_in_archive {
            return Err(XbergError::validation(format!(
                "TAR archive has too many files: {} (max: {})",
                file_count, limits.max_files_in_archive
            )));
        }

        if total_size > limits.max_archive_size as u64 {
            return Err(XbergError::validation(format!(
                "TAR archive total uncompressed size exceeds limit: {} bytes (max: {} bytes)",
                total_size, limits.max_archive_size
            )));
        }

        file_list.push(ArchiveEntry { path, size, is_dir });
    }

    Ok(ArchiveMetadata {
        format: "TAR".to_string(),
        file_list,
        file_count,
        total_size,
    })
}

/// Extract text content from files within a TAR archive.
///
/// Only extracts files with common text extensions: .txt, .md, .json, .xml, .html, .csv, .log, .yaml, .toml
///
/// # Arguments
///
/// * `bytes` - The TAR archive bytes (can be compressed with gzip or bzip2)
///
/// # Returns
///
/// Returns a `HashMap` mapping file paths to their text content.
/// Binary files and files with non-text extensions are excluded.
///
/// # Errors
///
/// Returns an error if the TAR archive cannot be read or parsed.
pub(crate) fn extract_tar_text_content(bytes: &[u8], limits: &SecurityLimits) -> Result<AHashMap<String, String>> {
    let cursor = Cursor::new(bytes);
    let mut archive = TarArchive::new(cursor);

    let estimated_text_files = bytes.len().saturating_div(1024 * 10).min(100);
    let mut contents = AHashMap::with_capacity(estimated_text_files.max(2));
    let mut file_count = 0usize;
    let mut total_content_size = 0usize;

    let entries = archive
        .entries()
        .map_err(|e| XbergError::parsing(format!("Failed to read TAR archive: {}", e)))?;

    for entry_result in entries {
        let mut entry = entry_result.map_err(|e| XbergError::parsing(format!("Failed to read TAR entry: {}", e)))?;

        file_count += 1;
        if file_count > limits.max_files_in_archive {
            return Err(XbergError::validation(format!(
                "TAR archive has too many files: {} (max: {})",
                file_count, limits.max_files_in_archive
            )));
        }

        let path = entry
            .path()
            .map_err(|e| XbergError::parsing(format!("Failed to read TAR entry path: {}", e)))?
            .to_string_lossy()
            .to_string();

        if !entry.header().entry_type().is_dir() && TEXT_EXTENSIONS.iter().any(|ext| path.to_lowercase().ends_with(ext))
        {
            // Unlike ZIP, tar-rs's `Entry` reader is itself bounded to the header's declared
            // `size()` (TAR stores members uncompressed, contiguous, at exactly that length), so
            // this is not a decompression-ratio bomb. The bug was that `entry.size()` was only
            // used as a `Vec::with_capacity` *hint* (`.min(10 * 1024 * 1024)`) -- the subsequent
            // `read_to_end` was never actually bounded by it, so a header declaring an
            // enormous member (nothing stops an attacker writing one) is read into memory in
            // full before any limit is checked. Bound the read at `max_content_size`: that is
            // the aggregate budget this member's decoded text is checked against a few lines
            // down, so no single member can legitimately need more. `+1` distinguishes "exactly
            // at the limit" from "over" so a hostile member is rejected rather than silently
            // truncated.
            let cap = limits.max_content_size as u64;
            let mut raw = Vec::new();
            let mut limited = (&mut entry).take(cap.saturating_add(1));
            if let Err(e) = limited.read_to_end(&mut raw) {
                tracing::warn!(member = %path, error = %e, "skipping TAR member: read failed");
                continue;
            }
            if raw.len() as u64 > cap {
                // Reject rather than silently truncate or skip: the pre-existing code already
                // surfaced a hard error once the *decoded* total crossed `max_content_size`
                // (below), so capping the read must not become a quieter failure mode than the
                // one it replaces.
                return Err(XbergError::validation(format!(
                    "TAR archive member '{}' exceeds max_content_size while reading (limit: {} bytes)",
                    path, limits.max_content_size
                )));
            }
            let content = super::decode_archive_text(&raw, &path);
            total_content_size = total_content_size.saturating_add(content.len());
            if total_content_size > limits.max_content_size {
                return Err(XbergError::validation(format!(
                    "TAR archive text content exceeds limit: {} bytes (max: {} bytes)",
                    total_content_size, limits.max_content_size
                )));
            }
            contents.insert(path, content);
        }
    }

    Ok(contents)
}

/// Extract raw file bytes for all non-directory entries in a TAR archive.
///
/// Returns a `HashMap` mapping file paths to their raw byte content.
/// Respects security limits for file count and total archive size.
///
/// # Arguments
///
/// * `bytes` - The TAR archive bytes
/// * `limits` - Security limits for archive extraction
///
/// # Errors
///
/// Returns an error if the TAR archive cannot be read or if security limits are exceeded.
pub(crate) fn extract_tar_file_bytes(bytes: &[u8], limits: &SecurityLimits) -> Result<AHashMap<String, Vec<u8>>> {
    let cursor = Cursor::new(bytes);
    let mut archive = TarArchive::new(cursor);

    let mut file_bytes = AHashMap::new();
    let mut file_count = 0usize;
    let mut total_size = 0usize;

    let entries = archive
        .entries()
        .map_err(|e| XbergError::parsing(format!("Failed to read TAR archive: {}", e)))?;

    for entry_result in entries {
        let mut entry = entry_result.map_err(|e| XbergError::parsing(format!("Failed to read TAR entry: {}", e)))?;

        file_count += 1;
        if file_count > limits.max_files_in_archive {
            return Err(XbergError::validation(format!(
                "TAR archive has too many files: {} (max: {})",
                file_count, limits.max_files_in_archive
            )));
        }

        if entry.header().entry_type().is_dir() {
            continue;
        }

        let path = entry
            .path()
            .map_err(|e| XbergError::parsing(format!("Failed to read TAR entry path: {}", e)))?
            .to_string_lossy()
            .to_string();

        // See the comment in `extract_tar_text_content`: `entry.size()` was only ever used as a
        // `Vec::with_capacity` hint, never as an actual bound on `read_to_end`, so a header
        // declaring an oversized member is read into memory in full before any check runs. Cap
        // it ourselves at `max_archive_size` -- the same budget `total_size` is checked against
        // below. `+1` distinguishes "exactly at the limit" from "over".
        let cap = limits.max_archive_size as u64;
        let mut content = Vec::new();
        let mut limited = (&mut entry).take(cap.saturating_add(1));
        if limited.read_to_end(&mut content).is_ok() {
            if content.len() as u64 > cap {
                // Reject rather than silently truncate: the pre-existing code already surfaced
                // a hard error once the running `total_size` crossed `max_archive_size` (below),
                // so capping the read must not become a quieter failure mode than that.
                return Err(XbergError::validation(format!(
                    "TAR archive member '{}' exceeds max_archive_size while reading (limit: {} bytes)",
                    path, limits.max_archive_size
                )));
            }
            total_size = total_size.saturating_add(content.len());
            if total_size > limits.max_archive_size {
                return Err(XbergError::validation(format!(
                    "TAR archive total extracted size exceeds limit: {} bytes (max: {} bytes)",
                    total_size, limits.max_archive_size
                )));
            }
            file_bytes.insert(path, content);
        }
    }

    Ok(file_bytes)
}
