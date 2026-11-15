//! 7Z archive extraction.
//!
//! Provides functions for extracting metadata and text content from 7Z archives.

use super::{ArchiveEntry, ArchiveMetadata, DecompressedBudget, TEXT_EXTENSIONS};
use crate::error::{Result, XbergError};
use crate::extractors::security::SecurityLimits;
use ahash::AHashMap;
use sevenz_rust2::{ArchiveReader, EntryDecision, Password};
use std::io::{self, Cursor};

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
            total_size = super::add_declared_size(total_size, size)?;
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
pub(crate) fn extract_7z_text_content(
    bytes: &[u8],
    limits: &SecurityLimits,
    budget: &mut DecompressedBudget<'_>,
) -> Result<AHashMap<String, String>> {
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
    let mut total_content_size = 0u64;

    let mut member_error = None;
    let entries = archive.for_each_entries_with_decision_allowing_surplus(|entry, reader| {
        let path = entry.name();
        if entry.is_directory() || !TEXT_EXTENSIONS.iter().any(|ext| path.to_lowercase().ends_with(ext)) {
            read_member(budget, reader, &mut io::sink(), None, path, &mut member_error)?;
            return Ok(EntryDecision::Continue);
        }

        let mut content = Vec::new();
        let cap = limits.max_content_size as u64;
        read_member(budget, reader, &mut content, Some(cap), path, &mut member_error)?;
        let text = super::decode_archive_text(&content, path);
        total_content_size = total_content_size.saturating_add(text.len() as u64);
        if total_content_size > cap {
            return Ok(EntryDecision::Stop);
        }
        contents.insert(path.to_owned(), text);
        Ok(EntryDecision::Continue)
    });
    if let Some(error) = member_error {
        return Err(error);
    }
    entries.map_err(|e| XbergError::parsing(format!("Failed to read 7z entries: {e}")))?;

    if total_content_size > limits.max_content_size as u64 {
        return Err(XbergError::validation(format!(
            "7z archive text content exceeds limit: {} bytes (max: {} bytes)",
            total_content_size, limits.max_content_size
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
pub(crate) fn extract_7z_file_bytes(
    bytes: &[u8],
    limits: &SecurityLimits,
    budget: &mut DecompressedBudget<'_>,
) -> Result<AHashMap<String, Vec<u8>>> {
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

    let mut member_error = None;
    let entries = archive.for_each_entries_with_decision_allowing_surplus(|entry, reader| {
        let path = entry.name();
        if entry.is_directory() {
            read_member(budget, reader, &mut io::sink(), None, path, &mut member_error)?;
        } else {
            let mut content = Vec::new();
            read_member(budget, reader, &mut content, None, path, &mut member_error)?;
            file_bytes.insert(path.to_owned(), content);
        }
        Ok(EntryDecision::Continue)
    });
    if let Some(error) = member_error {
        return Err(error);
    }
    entries.map_err(|e| XbergError::parsing(format!("Failed to read 7z entries: {e}")))?;

    Ok(file_bytes)
}

fn read_member(
    budget: &mut DecompressedBudget<'_>,
    reader: impl io::Read,
    output: &mut impl io::Write,
    cap: Option<u64>,
    path: &str,
    member_error: &mut Option<XbergError>,
) -> std::result::Result<(), sevenz_rust2::Error> {
    let error = match budget.read_member(reader, output, cap, path) {
        Ok(Ok(())) => return Ok(()),
        Ok(Err(error)) => XbergError::Io(io::Error::new(
            error.kind(),
            format!("Failed to read 7z member '{path}': {error}"),
        )),
        Err(error) => error,
    };
    // The decoder callback has its own error type; return the original error after it aborts. ~keep
    *member_error = Some(error);
    Err(sevenz_rust2::Error::Other("7z member read failed".into()))
}

#[cfg(test)]
mod tests {
    use super::*;
    use sevenz_rust2::{ArchiveEntry as SevenzEntry, ArchiveWriter};

    /// Covers per-member rejection and error naming for an oversized 7z member's text
    /// content -- NOT memory-boundedness. `extract_7z_text_content` takes `bytes: &[u8]` and
    /// `for_each_entries` hands the closure a concrete sevenz-rust2 block reader; there is no
    /// seam here to substitute a counting/instrumented `Read` for it, so the actual property
    /// the `.take()` exists for (the reader is never asked for more than `cap + 1` bytes)
    /// cannot be observed from this test. It is covered by inspection only. What this test does
    /// prove: a member whose decoded content dwarfs `max_content_size` is rejected by a
    /// *member-scoped* error that names the member, rather than surfacing only from the
    /// aggregate `total_content_size` check (which, once several members have been summed, can
    /// no longer report which one was responsible).
    ///
    /// Neutralisation that must break this test: replace `.take(cap.saturating_add(1))` with
    /// `.take(u64::MAX)` in `extract_7z_text_content`. That neutralisation does NOT break this test on its
    /// own -- the post-read `content.len() as u64 > cap` check a few lines below still fires
    /// and still names "huge.txt", so the assertion still passes. Only removing that length
    /// check too (or renaming the error's member field) would fail it, which is exactly the
    /// point: this test cannot distinguish a bounded reader from an unbounded one.
    /// `--features archives`.
    #[test]
    fn test_7z_text_content_names_offending_member_when_it_exceeds_content_cap() {
        let cursor = {
            let cursor = Cursor::new(Vec::new());
            let mut sz = ArchiveWriter::new(cursor).unwrap();
            sz.push_archive_entry(
                SevenzEntry::new_file("huge.txt"),
                Some(Cursor::new(vec![b'A'; 200_000])),
            )
            .unwrap();
            sz.finish().unwrap()
        };
        let bytes = cursor.into_inner();
        let limits = SecurityLimits {
            max_content_size: 1_000,
            ..SecurityLimits::default()
        };

        let result = extract_7z_text_content(
            &bytes,
            &limits,
            &mut DecompressedBudget::new(limits.max_archive_size as u64),
        );

        let error = result.expect_err("a member whose decoded size dwarfs max_content_size must be rejected");
        assert!(matches!(error, XbergError::Validation { .. }));
        let message = error.to_string();
        assert!(
            message.contains("huge.txt"),
            "the rejection must name the offending member instead of only reporting an \
             aggregate total that cannot identify one: {message}"
        );
    }

    /// Same defect family, different call: `extract_7z_file_bytes`. See the doc comment on
    /// `test_7z_text_content_names_offending_member_when_it_exceeds_content_cap` for why this
    /// covers per-member rejection and naming, not memory-boundedness.
    ///
    /// Neutralisation that must break this test: replace `.take(cap.saturating_add(1))` with
    /// `.take(u64::MAX)` in `extract_7z_file_bytes` -- and, as above, that alone does not break it, since
    /// the `content.len() as u64 > cap` check still fires and still names "huge.bin".
    /// `--features archives`.
    #[test]
    fn test_7z_file_bytes_names_offending_member_when_it_exceeds_archive_cap() {
        let cursor = {
            let cursor = Cursor::new(Vec::new());
            let mut sz = ArchiveWriter::new(cursor).unwrap();
            sz.push_archive_entry(
                SevenzEntry::new_file("huge.bin"),
                Some(Cursor::new(vec![0xABu8; 200_000])),
            )
            .unwrap();
            sz.finish().unwrap()
        };
        let bytes = cursor.into_inner();
        let limits = SecurityLimits {
            max_archive_size: 1_000,
            ..SecurityLimits::default()
        };

        let result = extract_7z_file_bytes(
            &bytes,
            &limits,
            &mut DecompressedBudget::new(limits.max_archive_size as u64),
        );

        let error = result.expect_err("a member whose decoded size dwarfs max_archive_size must be rejected");
        let message = error.to_string();
        assert!(
            message.contains("huge.bin"),
            "the rejection must name the offending member instead of only reporting an \
             aggregate total that cannot identify one: {message}"
        );
    }
}
