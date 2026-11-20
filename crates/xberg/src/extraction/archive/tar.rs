//! TAR archive extraction.
//!
//! Provides functions for extracting metadata and text content from TAR archives.
//! Supports plain TAR as well as compressed variants (TAR.GZ, TAR.BZ2).

use super::{ArchiveEntry, ArchiveMetadata, DecompressedBudget, MemberContents};
use crate::error::{Result, XbergError};
use crate::extractors::security::SecurityLimits;
#[cfg(test)]
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
            total_size = super::add_declared_size(total_size, size)?;
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

#[cfg(test)]
pub(crate) fn extract_tar_text_content(
    bytes: &[u8],
    limits: &SecurityLimits,
    budget: &mut DecompressedBudget<'_>,
) -> Result<AHashMap<String, String>> {
    extract_tar_members(bytes, limits, budget, None, Some(limits.max_content_size as u64)).map(|members| members.text)
}

#[cfg(test)]
pub(crate) fn extract_tar_file_bytes(
    bytes: &[u8],
    limits: &SecurityLimits,
    budget: &mut DecompressedBudget<'_>,
) -> Result<AHashMap<String, Vec<u8>>> {
    extract_tar_members(bytes, limits, budget, None, None).map(|members| members.bytes)
}

/// Read member bytes once, deriving text for eligible members when a text limit is supplied.
pub(crate) fn extract_tar_members(
    bytes: &[u8],
    limits: &SecurityLimits,
    budget: &mut DecompressedBudget<'_>,
    mut sparse_budget: Option<&mut DecompressedBudget<'_>>,
    text_limit: Option<u64>,
) -> Result<MemberContents> {
    let cursor = Cursor::new(bytes);
    let mut archive = TarArchive::new(cursor);

    let mut members = MemberContents::new(text_limit);
    let mut file_count = 0usize;

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

        if entry.header().entry_type().is_dir() {
            if let Err(e) = budget.read_member(&mut entry, &mut std::io::sink(), None, &path)? {
                tracing::warn!(member = %path, error = %e, "skipping TAR member: read failed");
            }
            continue;
        }

        let cap = members.text_limit(&path);
        if let Some(shared) = sparse_budget
            .as_deref_mut()
            .filter(|_| entry.header().entry_type().is_gnu_sparse())
        {
            let stored = sparse_stored_size(&mut entry)
                .map_err(|error| XbergError::parsing(format!("Failed to read size of TAR member '{path}': {error}")))?;
            charge_sparse_expansion(shared, stored, entry.size(), &path)?;
        }

        let mut content = Vec::new();
        match budget.read_member(&mut entry, &mut content, cap, &path)? {
            Ok(()) => {
                members.insert(path, content, "TAR")?;
            }
            Err(error) => tracing::warn!(member = %path, %error, "skipping TAR member: read failed"),
        }
    }

    Ok(members)
}

// Match the TAR parser's local PAX size override before using the raw header size. ~keep
fn sparse_stored_size<R: Read>(entry: &mut ::tar::Entry<'_, R>) -> std::io::Result<u64> {
    if let Some(extensions) = entry.pax_extensions()? {
        for extension in extensions {
            let Ok(extension) = extension else { break };
            if extension.key_bytes() == b"size" {
                if let Some(size) = extension.value().ok().and_then(|value| value.parse().ok()) {
                    return Ok(size);
                }
                break;
            }
        }
    }
    entry.header().entry_size()
}

fn charge_sparse_expansion(budget: &mut DecompressedBudget<'_>, stored: u64, expanded: u64, path: &str) -> Result<()> {
    let additional = expanded
        .checked_sub(stored)
        .ok_or_else(|| XbergError::parsing(format!("Stored size exceeds expanded size for TAR member '{path}'")))?;
    budget.charge(additional, path)
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::extractors::security::ArchiveBudget;

    #[test]
    fn sparse_expansion_charges_only_bytes_absent_from_the_inflated_tar() {
        let inflated_tar = 4096;
        let stored = 1024;
        let expanded = 8192;
        let expected_charge = 7168;

        for available in [expected_charge, expected_charge - 1] {
            let shared = ArchiveBudget::new(&SecurityLimits {
                max_archive_size: inflated_tar + available,
                ..SecurityLimits::default()
            });
            shared.charge_decompressed(inflated_tar).unwrap();
            let mut budget = DecompressedBudget::shared(&shared);
            let result = charge_sparse_expansion(&mut budget, stored, expanded, "sparse.bin");

            if available == expected_charge {
                result.unwrap();
                assert_eq!(shared.remaining_decompressed(), 0);
            } else {
                let error = result.unwrap_err();
                assert!(matches!(error, XbergError::Validation { .. }));
                assert_eq!(shared.remaining_decompressed(), available);
            }
        }
    }
}
