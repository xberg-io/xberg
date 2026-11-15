//! ZIP archive extraction.
//!
//! Provides functions for extracting metadata and text content from ZIP archives.

use super::{ArchiveEntry, ArchiveMetadata, DecompressedBudget, MemberContents};
use crate::error::{Result, XbergError};
use crate::extractors::security::SecurityLimits;
#[cfg(test)]
use ahash::AHashMap;
use std::io::Cursor;
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
        // Listing an entry needs neither its password nor its compression method, so an
        // encrypted member is listed like any other. ~keep
        let file = archive
            .by_index_raw(i)
            .map_err(|e| XbergError::parsing(format!("Failed to read ZIP entry: {}", e)))?;

        let path = file
            .name()
            .map_err(|error| XbergError::parsing(format!("Invalid ZIP entry name: {error}")))?
            .into_owned();
        let size = file.size();
        let is_dir = file.is_dir();

        if !is_dir {
            total_size = super::add_declared_size(total_size, size)?;
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

#[cfg(test)]
pub(crate) fn extract_zip_text_content(
    bytes: &[u8],
    limits: &SecurityLimits,
    budget: &mut DecompressedBudget<'_>,
) -> Result<AHashMap<String, String>> {
    extract_zip_members(bytes, limits, budget, Some(limits.max_content_size as u64)).map(|members| members.text)
}

#[cfg(test)]
pub(crate) fn extract_zip_file_bytes(
    bytes: &[u8],
    limits: &SecurityLimits,
    budget: &mut DecompressedBudget<'_>,
) -> Result<AHashMap<String, Vec<u8>>> {
    extract_zip_members(bytes, limits, budget, None).map(|members| members.bytes)
}

/// Read member bytes once, deriving text for eligible members when a text limit is supplied.
pub(crate) fn extract_zip_members(
    bytes: &[u8],
    limits: &SecurityLimits,
    budget: &mut DecompressedBudget<'_>,
    text_limit: Option<u64>,
) -> Result<MemberContents> {
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

    let mut members = MemberContents::new(text_limit);

    for i in 0..archive.len() {
        let mut file = match archive.by_index(i) {
            Ok(file) => file,
            Err(error) => {
                tracing::warn!(index = i, error = %error, "skipping ZIP member: open failed");
                continue;
            }
        };

        let path = file
            .name()
            .map_err(|error| XbergError::parsing(format!("Invalid ZIP entry name: {error}")))?
            .into_owned();
        if file.is_dir() {
            if let Err(e) = budget.read_member(&mut file, &mut std::io::sink(), None, &path)? {
                tracing::warn!(member = %path, error = %e, "skipping ZIP member: read failed");
            }
            continue;
        }

        let mut content = Vec::new();
        match budget.read_member(&mut file, &mut content, members.text_limit(&path), &path)? {
            Ok(()) => {
                members.insert(path, content, "ZIP")?;
            }
            Err(error) => tracing::warn!(member = %path, %error, "skipping ZIP member: read failed"),
        }
    }

    Ok(members)
}
