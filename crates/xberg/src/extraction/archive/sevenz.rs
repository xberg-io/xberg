//! 7Z archive extraction.
//!
//! Provides functions for extracting metadata and text content from 7Z archives.

use super::{ArchiveEntry, ArchiveMetadata, TEXT_EXTENSIONS};
use crate::error::{Result, XbergError};
use crate::extractors::security::SecurityLimits;
use ahash::AHashMap;
use sevenz_rust2::{ArchiveReader, Password};
use std::io::Cursor;

/// Extract metadata from a 7z archive.
///
/// # Arguments
///
/// * `bytes` - The 7z archive bytes
/// * `limits` - Security limits for archive extraction
///
/// # Returns
///
/// Returns `ArchiveMetadata` containing:
/// - Format: "7Z"
/// - File list with paths, sizes, and directory flags
/// - Total file count
/// - Total uncompressed size
///
/// # Errors
///
/// Returns an error if the 7z archive cannot be read or parsed,
/// or if security limits are exceeded.
pub(crate) fn extract_7z_metadata(bytes: &[u8], limits: &SecurityLimits) -> Result<ArchiveMetadata> {
    let cursor = Cursor::new(bytes);
    let archive = ArchiveReader::new(cursor, Password::empty())
        .map_err(|e| XbergError::parsing(format!("Failed to read 7z archive: {}", e)))?;

    let mut file_list = Vec::new();
    let mut total_size = 0u64;

    let files = &archive.archive().files;
    if files.len() > limits.max_files_in_archive {
        return Err(XbergError::validation(format!(
            "7z archive has too many files: {} (max: {})",
            files.len(),
            limits.max_files_in_archive
        )));
    }

    for entry in files {
        let path = entry.name().to_string();
        let size = entry.size();
        let is_dir = entry.is_directory();

        if !is_dir {
            total_size += size;
        }

        if total_size > limits.max_archive_size as u64 {
            return Err(XbergError::validation(format!(
                "7z archive total uncompressed size exceeds limit: {} bytes (max: {} bytes)",
                total_size, limits.max_archive_size
            )));
        }

        file_list.push(ArchiveEntry { path, size, is_dir });
    }

    let file_count = file_list.len();

    Ok(ArchiveMetadata {
        format: "7Z".to_string(),
        file_list,
        file_count,
        total_size,
    })
}

/// Extract text content from files within a 7z archive.
///
/// Only extracts files with common text extensions: .txt, .md, .json, .xml, .html, .csv, .log, .yaml, .toml
///
/// # Arguments
///
/// * `bytes` - The 7z archive bytes
///
/// # Returns
///
/// Returns a `HashMap` mapping file paths to their text content.
/// Binary files and files with non-text extensions are excluded.
///
/// # Errors
///
/// Returns an error if the 7z archive cannot be read or parsed.
pub(crate) fn extract_7z_text_content(bytes: &[u8], limits: &SecurityLimits) -> Result<AHashMap<String, String>> {
    let cursor = Cursor::new(bytes);
    let mut archive = ArchiveReader::new(cursor, Password::empty())
        .map_err(|e| XbergError::parsing(format!("Failed to read 7z archive: {}", e)))?;

    let file_count = archive.archive().files.len();
    if file_count > limits.max_files_in_archive {
        return Err(XbergError::validation(format!(
            "7z archive has too many files: {} (max: {})",
            file_count, limits.max_files_in_archive
        )));
    }

    let mut contents = AHashMap::new();
    let max_content_size = limits.max_content_size;
    let mut total_content_size = 0usize;

    archive
        .for_each_entries(|entry, reader| {
            let path = entry.name().to_string();

            if !entry.is_directory() && TEXT_EXTENSIONS.iter().any(|ext| path.to_lowercase().ends_with(ext)) {
                let mut content = Vec::new();
                if reader.read_to_end(&mut content).is_ok() {
                    // Decode with charset detection so a non-UTF-8 member is
                    // recovered rather than silently dropped (xberg-io/xberg#1223).
                    let text = super::decode_archive_text(&content, &path);
                    total_content_size = total_content_size.saturating_add(text.len());
                    if total_content_size > max_content_size {
                        return Ok(false);
                    }
                    contents.insert(path, text);
                }
            }
            Ok(true)
        })
        .map_err(|e| XbergError::parsing(format!("Failed to read 7z entries: {}", e)))?;

    if total_content_size > max_content_size {
        return Err(XbergError::validation(format!(
            "7z archive text content exceeds limit: {} bytes (max: {} bytes)",
            total_content_size, max_content_size
        )));
    }

    Ok(contents)
}

/// Extract raw file bytes for all non-directory entries in a 7z archive.
///
/// Returns a `HashMap` mapping file paths to their raw byte content.
/// Respects security limits for file count and total archive size.
///
/// # Arguments
///
/// * `bytes` - The 7z archive bytes
/// * `limits` - Security limits for archive extraction
///
/// # Errors
///
/// Returns an error if the 7z archive cannot be read or if security limits are exceeded.
pub(crate) fn extract_7z_file_bytes(bytes: &[u8], limits: &SecurityLimits) -> Result<AHashMap<String, Vec<u8>>> {
    let cursor = Cursor::new(bytes);
    let mut archive = ArchiveReader::new(cursor, Password::empty())
        .map_err(|e| XbergError::parsing(format!("Failed to read 7z archive: {}", e)))?;

    let file_count = archive.archive().files.len();
    if file_count > limits.max_files_in_archive {
        return Err(XbergError::validation(format!(
            "7z archive has too many files: {} (max: {})",
            file_count, limits.max_files_in_archive
        )));
    }

    let mut file_bytes = AHashMap::new();
    let max_size = limits.max_archive_size;
    let mut total_size = 0usize;

    archive
        .for_each_entries(|entry, reader| {
            let path = entry.name().to_string();

            if !entry.is_directory() {
                let mut content = Vec::new();
                if reader.read_to_end(&mut content).is_ok() {
                    total_size = total_size.saturating_add(content.len());
                    if total_size > max_size {
                        return Ok(false);
                    }
                    file_bytes.insert(path, content);
                }
            }
            Ok(true)
        })
        .map_err(|e| XbergError::parsing(format!("Failed to read 7z entries: {}", e)))?;

    if total_size > max_size {
        return Err(XbergError::validation(format!(
            "7z archive total extracted size exceeds limit: {} bytes (max: {} bytes)",
            total_size, max_size
        )));
    }

    Ok(file_bytes)
}
