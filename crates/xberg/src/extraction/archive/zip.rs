//! ZIP archive extraction.
//!
//! Provides functions for extracting metadata and text content from ZIP archives.

use super::{ArchiveEntry, ArchiveMetadata, TEXT_EXTENSIONS};
use crate::error::{Result, XbergError};
use crate::extractors::security::SecurityLimits;
use ahash::AHashMap;
use std::io::{Cursor, Read};
use zip::ZipArchive;

/// Extract metadata from a ZIP archive.
///
/// # Arguments
///
/// * `bytes` - The ZIP archive bytes
/// * `limits` - Security limits for archive extraction
///
/// # Returns
///
/// Returns `ArchiveMetadata` containing:
/// - Format: "ZIP"
/// - File list with paths, sizes, and directory flags
/// - Total file count
/// - Total uncompressed size
///
/// # Errors
///
/// Returns an error if the ZIP archive cannot be read or parsed,
/// or if security limits are exceeded.
pub(crate) fn extract_zip_metadata(bytes: &[u8], limits: &SecurityLimits) -> Result<ArchiveMetadata> {
    let cursor = Cursor::new(bytes);
    let mut archive =
        ZipArchive::new(cursor).map_err(|e| XbergError::parsing(format!("Failed to read ZIP archive: {}", e)))?;

    if archive.len() > limits.max_files_in_archive {
        return Err(XbergError::validation(format!(
            "ZIP archive has too many files: {} (max: {})",
            archive.len(),
            limits.max_files_in_archive
        )));
    }

    let mut file_list = Vec::with_capacity(archive.len());
    let mut total_size = 0u64;

    for i in 0..archive.len() {
        let file = archive
            .by_index(i)
            .map_err(|e| XbergError::parsing(format!("Failed to read ZIP entry: {}", e)))?;

        let path = file.name().to_string();
        let size = file.size();
        let is_dir = file.is_dir();

        if !is_dir {
            total_size += size;
        }

        if total_size > limits.max_archive_size as u64 {
            return Err(XbergError::validation(format!(
                "ZIP archive total uncompressed size exceeds limit: {} bytes (max: {} bytes)",
                total_size, limits.max_archive_size
            )));
        }

        file_list.push(ArchiveEntry { path, size, is_dir });
    }

    Ok(ArchiveMetadata {
        format: "ZIP".to_string(),
        file_list,
        file_count: archive.len(),
        total_size,
    })
}

/// Extract text content from files within a ZIP archive.
///
/// Only extracts files with common text extensions: .txt, .md, .json, .xml, .html, .csv, .log, .yaml, .toml
///
/// # Arguments
///
/// * `bytes` - The ZIP archive bytes
///
/// # Returns
///
/// Returns a `HashMap` mapping file paths to their text content.
/// Binary files and files with non-text extensions are excluded.
///
/// # Errors
///
/// Returns an error if the ZIP archive cannot be read or parsed.
pub(crate) fn extract_zip_text_content(bytes: &[u8], limits: &SecurityLimits) -> Result<AHashMap<String, String>> {
    let cursor = Cursor::new(bytes);
    let mut archive =
        ZipArchive::new(cursor).map_err(|e| XbergError::parsing(format!("Failed to read ZIP archive: {}", e)))?;

    if archive.len() > limits.max_files_in_archive {
        return Err(XbergError::validation(format!(
            "ZIP archive has too many files: {} (max: {})",
            archive.len(),
            limits.max_files_in_archive
        )));
    }

    let estimated_text_files = archive.len().saturating_mul(3).saturating_div(10).max(2);
    let mut contents = AHashMap::with_capacity(estimated_text_files);
    let mut total_content_size = 0usize;

    for i in 0..archive.len() {
        let mut file = archive
            .by_index(i)
            .map_err(|e| XbergError::parsing(format!("Failed to read ZIP entry: {}", e)))?;

        let path = file.name().to_string();

        if !file.is_dir() && TEXT_EXTENSIONS.iter().any(|ext| path.to_lowercase().ends_with(ext)) {
            let estimated_size = (file.size() as usize).min(10 * 1024 * 1024);
            // Read bytes then decode, so a non-UTF-8 member (legacy-encoded
            // .txt/.csv) is recovered instead of silently dropped
            // (xberg-io/xberg#1223). read_to_string would have failed and the
            // member would have vanished with no signal.
            let mut raw = Vec::with_capacity(estimated_size);
            if let Err(e) = file.read_to_end(&mut raw) {
                tracing::warn!(member = %path, error = %e, "skipping ZIP member: read failed");
                continue;
            }
            let content = super::decode_archive_text(&raw, &path);
            total_content_size = total_content_size.saturating_add(content.len());
            if total_content_size > limits.max_content_size {
                return Err(XbergError::validation(format!(
                    "ZIP archive text content exceeds limit: {} bytes (max: {} bytes)",
                    total_content_size, limits.max_content_size
                )));
            }
            contents.insert(path, content);
        }
    }

    Ok(contents)
}

/// Extract raw file bytes for all non-directory entries in a ZIP archive.
///
/// Returns a `HashMap` mapping file paths to their raw byte content.
/// Respects security limits for file count and total archive size.
///
/// # Arguments
///
/// * `bytes` - The ZIP archive bytes
/// * `limits` - Security limits for archive extraction
///
/// # Errors
///
/// Returns an error if the ZIP archive cannot be read or if security limits are exceeded.
pub(crate) fn extract_zip_file_bytes(bytes: &[u8], limits: &SecurityLimits) -> Result<AHashMap<String, Vec<u8>>> {
    let cursor = Cursor::new(bytes);
    let mut archive =
        ZipArchive::new(cursor).map_err(|e| XbergError::parsing(format!("Failed to read ZIP archive: {}", e)))?;

    if archive.len() > limits.max_files_in_archive {
        return Err(XbergError::validation(format!(
            "ZIP archive has too many files: {} (max: {})",
            archive.len(),
            limits.max_files_in_archive
        )));
    }

    let mut file_bytes = AHashMap::with_capacity(archive.len());
    let mut total_size = 0usize;

    for i in 0..archive.len() {
        let mut file = archive
            .by_index(i)
            .map_err(|e| XbergError::parsing(format!("Failed to read ZIP entry: {}", e)))?;

        if file.is_dir() {
            continue;
        }

        let path = file.name().to_string();
        let estimated_size = (file.size() as usize).min(limits.max_archive_size);
        let mut content = Vec::with_capacity(estimated_size);
        if file.read_to_end(&mut content).is_ok() {
            total_size = total_size.saturating_add(content.len());
            if total_size > limits.max_archive_size {
                return Err(XbergError::validation(format!(
                    "ZIP archive total extracted size exceeds limit: {} bytes (max: {} bytes)",
                    total_size, limits.max_archive_size
                )));
            }
            file_bytes.insert(path, content);
        }
    }

    Ok(file_bytes)
}
