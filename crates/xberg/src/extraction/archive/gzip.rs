//! Gzip decompression and extraction.
//!
//! Provides functions for decompressing gzip files and extracting
//! metadata and text content from the compressed data.
//!
//! When a gzip file contains a TAR archive (e.g., .tar.gz files),
//! this module automatically detects the TAR format and delegates
//! to the TAR extraction functions.

use super::{ArchiveEntry, ArchiveMetadata, DecompressedBudget};
use crate::error::{Result, XbergError};
use crate::extractors::security::SecurityLimits;
use ahash::AHashMap;
use flate2::read::GzDecoder;

/// Check if data looks like a TAR archive (has "ustar" magic at offset 257).
///
/// The TAR format has a standard USTAR header starting at offset 257,
/// which helps identify TAR archives that have been gzip-compressed.
fn is_tar_archive(data: &[u8]) -> bool {
    data.len() > 262 && &data[257..262] == b"ustar"
}

/// Decompress gzip bytes with a size limit to prevent decompression bombs.
fn decompress_gzip_limited(bytes: &[u8], max_size: u64) -> Result<(Vec<u8>, String)> {
    let mut decoder = GzDecoder::new(bytes);
    let filename = decoder
        .header()
        .and_then(|header| header.filename())
        .and_then(|name| std::str::from_utf8(name).ok())
        .unwrap_or("compressed_content")
        .to_owned();
    let mut decompressed = Vec::new();
    DecompressedBudget::new(max_size)
        .read_member(&mut decoder, &mut decompressed, None, &filename)?
        .map_err(|e| XbergError::parsing(format!("Failed to decompress gzip: {e}")))?;
    Ok((decompressed, filename))
}

/// Decompress gzip bytes, returning the raw decompressed data.
#[cfg(test)]
pub(crate) fn decompress_gzip(bytes: &[u8], limits: &SecurityLimits) -> Result<Vec<u8>> {
    decompress_gzip_limited(bytes, limits.max_archive_size as u64).map(|(data, _)| data)
}

/// Extract both metadata and text content from gzip in a single decompression pass.
///
/// This avoids the overhead of decompressing the data multiple times when both
/// metadata and text content are needed.
///
/// If the decompressed data is a TAR archive, delegates to TAR extraction functions.
#[cfg(test)]
pub(crate) fn extract_gzip(
    bytes: &[u8],
    limits: &SecurityLimits,
) -> Result<(ArchiveMetadata, AHashMap<String, String>)> {
    let (decompressed, filename) = decompress_gzip_limited(bytes, limits.max_archive_size as u64)?;

    if is_tar_archive(&decompressed) {
        let mut metadata = super::tar::extract_tar_metadata(&decompressed, limits)?;
        metadata.format = "GZIP+TAR".to_string();
        let contents = super::tar::extract_tar_text_content(&decompressed, limits)?;
        return Ok((metadata, contents));
    }

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
    let (decompressed, filename) = decompress_gzip_limited(bytes, limits.max_archive_size as u64)?;

    if is_tar_archive(&decompressed) {
        let mut metadata = super::tar::extract_tar_metadata(&decompressed, limits)?;
        metadata.format = "GZIP+TAR".to_string();
        return Ok(metadata);
    }

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
    let (decompressed, filename) = decompress_gzip_limited(bytes, limits.max_archive_size as u64)?;

    if is_tar_archive(&decompressed) {
        return super::tar::extract_tar_text_content(&decompressed, limits);
    }

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
    let (decompressed, filename) = decompress_gzip_limited(bytes, limits.max_archive_size as u64)?;

    if is_tar_archive(&decompressed) {
        let mut metadata = super::tar::extract_tar_metadata(&decompressed, limits)?;
        metadata.format = "GZIP+TAR".to_string();
        let contents = super::tar::extract_tar_text_content(&decompressed, limits)?;
        let file_bytes = super::tar::extract_tar_file_bytes(&decompressed, limits)?;
        return Ok((metadata, contents, file_bytes));
    }

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
    if let Ok(text) = std::str::from_utf8(&decompressed) {
        contents.insert(filename.clone(), text.to_owned());
    }

    let mut file_bytes = AHashMap::new();
    file_bytes.insert(filename, decompressed);

    Ok((metadata, contents, file_bytes))
}
