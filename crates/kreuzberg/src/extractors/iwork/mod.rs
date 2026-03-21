//! Apple iWork format extractors (.pages, .numbers, .key)
//!
//! Supports the modern iWork format (2013+):
//! - `.pages`   — Apple Pages word processor
//! - `.numbers` — Apple Numbers spreadsheet
//! - `.key`     — Apple Keynote presentation
//!
//! ## IWA Container Format
//!
//! Modern iWork files are ZIP archives containing `.iwa` (iWork Archive) files.
//! Each `.iwa` file is:
//! 1. Snappy-compressed using Apple's non-standard framing
//!    (no stream identifier chunk, no CRC-32C — raw Snappy blocks).
//! 2. The decompressed payload is a sequence of protobuf `TSP.ArchiveInfo`-framed
//!    messages from which text strings are extracted using raw wire parsing.

pub mod keynote;
pub mod numbers;
pub mod pages;

use crate::Result;
use crate::error::KreuzbergError;
use std::io::Cursor;
use std::io::Read;

/// Maximum size for an individual IWA file to guard against decompression bombs.
const MAX_IWA_DECOMPRESSED_SIZE: usize = 64 * 1024 * 1024; // 64 MiB

/// Open a ZIP archive from bytes and collect all `.iwa` entry names.
pub fn list_iwa_entries(content: &[u8]) -> Result<Vec<String>> {
    let cursor = Cursor::new(content);
    let mut archive =
        zip::ZipArchive::new(cursor).map_err(|e| KreuzbergError::parsing(format!("Failed to open iWork ZIP: {e}")))?;

    let mut names = Vec::new();
    for i in 0..archive.len() {
        let file = archive
            .by_index(i)
            .map_err(|e| KreuzbergError::parsing(format!("Failed to read ZIP entry {i}: {e}")))?;
        let name = file.name().to_string();
        if name.ends_with(".iwa") {
            names.push(name);
        }
    }
    Ok(names)
}

/// Read and Snappy-decompress a single `.iwa` file from the ZIP archive.
///
/// Apple IWA files use a custom framing format:
/// Each block in the file is: `[type: u8][length: u24 LE][payload: length bytes]`
/// - type `0x00`: Snappy-compressed block → decompress payload with raw Snappy
/// - type `0x01`: Uncompressed block → use payload as-is
///
/// Multiple blocks are concatenated to form the decompressed IWA stream.
pub fn read_iwa_file(content: &[u8], path: &str) -> Result<Vec<u8>> {
    use std::io::Read;

    let cursor = Cursor::new(content);
    let mut archive =
        zip::ZipArchive::new(cursor).map_err(|e| KreuzbergError::parsing(format!("Failed to open iWork ZIP: {e}")))?;

    let mut file = archive
        .by_name(path)
        .map_err(|_| KreuzbergError::parsing(format!("IWA file not found in archive: {path}")))?;

    let compressed_size = file.size() as usize;
    let mut raw = Vec::with_capacity(compressed_size.min(MAX_IWA_DECOMPRESSED_SIZE));
    file.read_to_end(&mut raw)
        .map_err(|e| KreuzbergError::parsing(format!("Failed to read IWA file {path}: {e}")))?;

    decode_iwa_stream(&raw).map_err(|e| KreuzbergError::parsing(format!("Failed to decode IWA {path}: {e}")))
}

/// Decode an Apple IWA byte stream into the raw protobuf payload.
///
/// IWA framing: each block = 1 byte type + 3 bytes LE length + N bytes payload
/// - type 0x00 → Snappy-compressed, decompress with `snap::raw::Decoder`
/// - type 0x01 → Uncompressed, use as-is
pub fn decode_iwa_stream(data: &[u8]) -> std::result::Result<Vec<u8>, String> {
    let mut decoder = snap::raw::Decoder::new();
    let mut output = Vec::new();
    let mut i = 0usize;

    while i + 4 <= data.len() {
        let chunk_type = data[i];
        // 24-bit little-endian length in bytes 1..4
        let chunk_len = (data[i + 1] as usize) | ((data[i + 2] as usize) << 8) | ((data[i + 3] as usize) << 16);
        i += 4;

        let end = i + chunk_len;
        if end > data.len() {
            return Err(format!(
                "IWA chunk out of bounds: offset={i}, chunk_len={chunk_len}, data_len={}",
                data.len()
            ));
        }

        let payload = &data[i..end];
        i = end;

        match chunk_type {
            0x00 => {
                // Snappy-compressed block
                let decompressed = decoder
                    .decompress_vec(payload)
                    .map_err(|e| format!("Snappy decompression failed: {e}"))?;

                if output.len() + decompressed.len() > MAX_IWA_DECOMPRESSED_SIZE {
                    return Err(format!(
                        "Decompressed IWA exceeds size limit ({MAX_IWA_DECOMPRESSED_SIZE} bytes)"
                    ));
                }
                output.extend_from_slice(&decompressed);
            }
            0x01 => {
                // Uncompressed block — use payload directly
                if output.len() + payload.len() > MAX_IWA_DECOMPRESSED_SIZE {
                    return Err(format!(
                        "Uncompressed IWA exceeds size limit ({MAX_IWA_DECOMPRESSED_SIZE} bytes)"
                    ));
                }
                output.extend_from_slice(payload);
            }
            _ => {
                // Unknown chunk type — skip to avoid corruption
                tracing::debug!("Unknown IWA chunk type: 0x{:02x}, len={chunk_len}", chunk_type);
            }
        }
    }

    Ok(output)
}

/// Extract all UTF-8 text strings from a raw protobuf byte slice.
///
/// This uses a simple wire-format scanner without a full schema:
/// - Field type 2 (length-delimited) with a valid UTF-8 payload of ≥3 bytes is
///   treated as a text string candidate.
/// - We skip binary blobs (non-UTF-8) and very short noise strings.
///
/// This approach avoids the need for `prost-build` and generated proto code while
/// still extracting human-readable text reliably from iWork documents.
pub fn extract_text_from_proto(data: &[u8]) -> Vec<String> {
    let mut texts: Vec<String> = Vec::new();
    let mut i = 0usize;

    while i < data.len() {
        // Read varint tag
        let (tag_varint, tag_len) = match read_varint(data, i) {
            Some(v) => v,
            None => break,
        };
        i += tag_len;

        let wire_type = tag_varint & 0x7;

        match wire_type {
            0 => {
                // Varint — skip
                match read_varint(data, i) {
                    Some((_, len)) => i += len,
                    None => break,
                }
            }
            1 => {
                // 64-bit — skip
                i += 8;
            }
            2 => {
                // Length-delimited — inspect for text
                let (length, len_bytes) = match read_varint(data, i) {
                    Some(v) => v,
                    None => break,
                };
                i += len_bytes;
                let end = i + length as usize;
                if end > data.len() {
                    break;
                }
                let payload = &data[i..end];
                i = end;

                // Attempt UTF-8 decode — only keep strings ≥ 3 chars of printable content
                if let Ok(s) = std::str::from_utf8(payload) {
                    let trimmed = s.trim();
                    if trimmed.len() >= 3 && trimmed.chars().any(|c| c.is_alphabetic() || c.is_numeric()) {
                        texts.push(trimmed.to_string());
                    }
                }

                // Also recurse into nested messages (they're also length-delimited)
                let nested = extract_text_from_proto(payload);
                texts.extend(nested);
            }
            5 => {
                // 32-bit — skip
                i += 4;
            }
            _ => {
                // Unknown wire type, stop parsing this message to avoid corruption
                break;
            }
        }
    }

    texts
}

/// Read a protobuf varint from `data` starting at byte `pos`.
///
/// Returns `(value, bytes_consumed)` or `None` if there aren't enough bytes.
fn read_varint(data: &[u8], pos: usize) -> Option<(u64, usize)> {
    let mut result: u64 = 0;
    let mut shift = 0u32;
    let mut i = pos;

    loop {
        if i >= data.len() {
            return None;
        }
        let byte = data[i] as u64;
        i += 1;
        result |= (byte & 0x7F) << shift;
        if byte & 0x80 == 0 {
            return Some((result, i - pos));
        }
        shift += 7;
        if shift >= 64 {
            return None;
        }
    }
}

/// Extract all text from an iWork ZIP archive by reading specified IWA entries.
///
/// `iwa_paths` should list the IWA file paths to read (e.g. `["Index/Document.iwa"]`).
/// Returns a flat joined string of all text found across all IWA files.
pub fn extract_text_from_iwa_files(content: &[u8], iwa_paths: &[&str]) -> Result<String> {
    let cursor = Cursor::new(content);
    let mut archive =
        zip::ZipArchive::new(cursor).map_err(|e| KreuzbergError::parsing(format!("Failed to open iWork ZIP: {e}")))?;

    let mut all_text: Vec<String> = Vec::new();

    for path in iwa_paths {
        // Some IWA files might not exist in all documents — skip missing ones gracefully
        match archive.by_name(path) {
            Ok(mut file) => {
                let compressed_size = file.size() as usize;
                let mut compressed = Vec::with_capacity(compressed_size.min(MAX_IWA_DECOMPRESSED_SIZE));

                if file.read_to_end(&mut compressed).is_err() {
                    continue;
                }

                // Apple uses raw Snappy without framing headers
                let mut decoder = snap::raw::Decoder::new();
                let Ok(decompressed) = decoder.decompress_vec(&compressed) else {
                    continue;
                };

                if decompressed.len() > MAX_IWA_DECOMPRESSED_SIZE {
                    continue;
                }

                let texts = extract_text_from_proto(&decompressed);
                all_text.extend(texts);
            }
            Err(_) => {
                // File not in archive — skip gracefully
                continue;
            }
        }
    }

    Ok(all_text.join("\n"))
}

/// Deduplicate a list of text strings while preserving order.
/// Adjacent duplicates and near-duplicates are removed.
pub fn dedup_text(texts: Vec<String>) -> Vec<String> {
    let mut seen = std::collections::HashSet::new();
    let mut result = Vec::new();
    for t in texts {
        if seen.insert(t.clone()) {
            result.push(t);
        }
    }
    result
}
