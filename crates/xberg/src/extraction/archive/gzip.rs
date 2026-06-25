//! Gzip decompression and extraction.
//!
//! Provides functions for decompressing gzip files and extracting
//! metadata and text content from the compressed data.
//!
//! When a gzip file contains a TAR archive (e.g., .tar.gz files),
//! this module automatically detects the TAR format and delegates
//! to the TAR extraction functions.

use super::{ArchiveEntry, ArchiveMetadata};
use crate::error::{Result, XbergError};
use crate::extractors::security::SecurityLimits;
use ahash::AHashMap;
use flate2::read::GzDecoder;
use std::io::Read;

/// Check if data looks like a TAR archive (has "ustar" magic at offset 257).
///
/// The TAR format has a standard USTAR header starting at offset 257,
/// which helps identify TAR archives that have been gzip-compressed.
fn is_tar_archive(data: &[u8]) -> bool {
    data.len() > 262 && &data[257..262] == b"ustar"
}

/// Decompress gzip bytes with a size limit to prevent decompression bombs.
fn decompress_gzip_limited(bytes: &[u8], max_size: u64) -> Result<Vec<u8>> {
    let decoder = GzDecoder::new(bytes);
    let mut limited = decoder.take(max_size + 1);
    let mut decompressed = Vec::new();
    limited
        .read_to_end(&mut decompressed)
        .map_err(|e| XbergError::parsing(format!("Failed to decompress gzip: {}", e)))?;

    if decompressed.len() as u64 > max_size {
        return Err(XbergError::validation(format!(
            "Gzip decompressed size exceeds {} byte limit",
            max_size
        )));
    }

    Ok(decompressed)
}

/// Decompress gzip bytes, returning the raw decompressed data.
#[cfg(test)]
pub(crate) fn decompress_gzip(bytes: &[u8], limits: &SecurityLimits) -> Result<Vec<u8>> {
    decompress_gzip_limited(bytes, limits.max_archive_size as u64)
}

/// Extract both metadata and text content from gzip in a single decompression pass.
///
/// This avoids the overhead of decompressing the data multiple times when both
/// metadata and text content are needed.
///
/// If the decompressed data is a TAR archive, delegates to TAR extraction functions.
pub(crate) fn extract_gzip(
    bytes: &[u8],
    limits: &SecurityLimits,
) -> Result<(ArchiveMetadata, AHashMap<String, String>)> {
    let decompressed = decompress_gzip_limited(bytes, limits.max_archive_size as u64)?;

    // Check if the decompressed data is a TAR archive
    if is_tar_archive(&decompressed) {
        let mut metadata = super::tar::extract_tar_metadata(&decompressed, limits)?;
        metadata.format = "GZIP+TAR".to_string();
        let contents = super::tar::extract_tar_text_content(&decompressed, limits)?;
        return Ok((metadata, contents));
    }

    // Re-read header for filename (lightweight - no decompression)
    let mut decoder = GzDecoder::new(bytes);
    let mut _discard = [0u8; 1];
    let _ = decoder.read(&mut _discard); // trigger header read
    let filename = decoder
        .header()
        .and_then(|h| h.filename())
        .and_then(|f| std::str::from_utf8(f).ok())
        .unwrap_or("compressed_content")
        .to_string();

    let size = decompressed.len() as u64;

    let metadata = ArchiveMetadata {
        format: "GZIP".to_string(),
        file_list: vec![ArchiveEntry {
            path: filename.clone(),
            size,
            is_dir: false,
        }],
        file_count: 1,
        total_size: size,
    };

    let mut contents = AHashMap::new();
    if let Ok(text) = String::from_utf8(decompressed) {
        contents.insert(filename, text);
    }

    Ok((metadata, contents))
}

/// Extract metadata from a gzip-compressed file.
///
/// Gzip wraps a single stream, so the metadata contains one entry
/// with the original filename (from gzip header) and decompressed size.
///
/// If the decompressed data is a TAR archive, delegates to TAR extraction.
#[cfg(test)]
pub(crate) fn extract_gzip_metadata(bytes: &[u8], limits: &SecurityLimits) -> Result<ArchiveMetadata> {
    let decompressed = decompress_gzip_limited(bytes, limits.max_archive_size as u64)?;

    // Check if the decompressed data is a TAR archive
    if is_tar_archive(&decompressed) {
        let mut metadata = super::tar::extract_tar_metadata(&decompressed, limits)?;
        metadata.format = "GZIP+TAR".to_string();
        return Ok(metadata);
    }

    let mut decoder = GzDecoder::new(bytes);
    let mut _discard = [0u8; 1];
    let _ = decoder.read(&mut _discard);
    let filename = decoder
        .header()
        .and_then(|h| h.filename())
        .and_then(|f| std::str::from_utf8(f).ok())
        .unwrap_or("compressed_content")
        .to_string();

    let size = decompressed.len() as u64;

    Ok(ArchiveMetadata {
        format: "GZIP".to_string(),
        file_list: vec![ArchiveEntry {
            path: filename,
            size,
            is_dir: false,
        }],
        file_count: 1,
        total_size: size,
    })
}

/// Extract text content from a gzip-compressed file.
///
/// Decompresses and attempts to read the result as UTF-8 text.
///
/// If the decompressed data is a TAR archive, delegates to TAR extraction.
#[cfg(test)]
pub(crate) fn extract_gzip_text_content(bytes: &[u8], limits: &SecurityLimits) -> Result<AHashMap<String, String>> {
    let decompressed = decompress_gzip_limited(bytes, limits.max_archive_size as u64)?;

    // Check if the decompressed data is a TAR archive
    if is_tar_archive(&decompressed) {
        return super::tar::extract_tar_text_content(&decompressed, limits);
    }

    let mut decoder = GzDecoder::new(bytes);
    let mut _discard = [0u8; 1];
    let _ = decoder.read(&mut _discard);
    let filename = decoder
        .header()
        .and_then(|h| h.filename())
        .and_then(|f| std::str::from_utf8(f).ok())
        .unwrap_or("compressed_content")
        .to_string();

    let mut contents = AHashMap::new();
    if let Ok(text) = String::from_utf8(decompressed) {
        contents.insert(filename, text);
    }

    Ok(contents)
}

/// Return type for `extract_gzip_with_bytes`: metadata, text content map, and raw file bytes map.
type GzipWithBytesResult = (ArchiveMetadata, AHashMap<String, String>, AHashMap<String, Vec<u8>>);

/// Extract metadata, text content, and raw file bytes from gzip in a single pass.
///
/// Similar to `extract_gzip` but also returns the raw file bytes for recursive extraction.
/// For TAR-within-GZIP, delegates to TAR file bytes extraction.
pub(crate) fn extract_gzip_with_bytes(bytes: &[u8], limits: &SecurityLimits) -> Result<GzipWithBytesResult> {
    let decompressed = decompress_gzip_limited(bytes, limits.max_archive_size as u64)?;

    // Check if the decompressed data is a TAR archive
    if is_tar_archive(&decompressed) {
        let mut metadata = super::tar::extract_tar_metadata(&decompressed, limits)?;
        metadata.format = "GZIP+TAR".to_string();
        let contents = super::tar::extract_tar_text_content(&decompressed, limits)?;
        let file_bytes = super::tar::extract_tar_file_bytes(&decompressed, limits)?;
        return Ok((metadata, contents, file_bytes));
    }

    // Re-read header for filename (lightweight - no decompression)
    let mut decoder = GzDecoder::new(bytes);
    let mut _discard = [0u8; 1];
    let _ = decoder.read(&mut _discard);
    let filename = decoder
        .header()
        .and_then(|h| h.filename())
        .and_then(|f| std::str::from_utf8(f).ok())
        .unwrap_or("compressed_content")
        .to_string();

    let size = decompressed.len() as u64;

    let metadata = ArchiveMetadata {
        format: "GZIP".to_string(),
        file_list: vec![ArchiveEntry {
            path: filename.clone(),
            size,
            is_dir: false,
        }],
        file_count: 1,
        total_size: size,
    };

    let mut file_bytes = AHashMap::new();
    file_bytes.insert(filename.clone(), decompressed.clone());

    let mut contents = AHashMap::new();
    if let Ok(text) = String::from_utf8(decompressed) {
        contents.insert(filename, text);
    }

    Ok((metadata, contents, file_bytes))
}
