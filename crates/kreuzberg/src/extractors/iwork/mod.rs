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
use crate::text::utf8_validation;
use std::io::Cursor;
use std::io::Read;

/// Maximum size for an individual IWA file to guard against decompression bombs.
const MAX_IWA_DECOMPRESSED_SIZE: usize = 64 * 1024 * 1024; // 64 MiB

/// Collects all .iwa file paths from a ZIP archive.
///
/// Opens the ZIP from `content`, iterates every entry, and returns the names of
/// all entries whose path ends with `.iwa`. Entries that cannot be read are
/// silently skipped (consistent with the per-extractor `filter_map` pattern).
pub(crate) fn collect_iwa_paths(content: &[u8]) -> Result<Vec<String>> {
    let cursor = Cursor::new(content);
    let mut archive =
        zip::ZipArchive::new(cursor).map_err(|e| KreuzbergError::parsing(format!("Failed to open iWork ZIP: {e}")))?;

    let iwa_paths: Vec<String> = (0..archive.len())
        .filter_map(|i| {
            archive.by_index(i).ok().and_then(|f| {
                let name = f.name().to_string();
                if name.ends_with(".iwa") { Some(name) } else { None }
            })
        })
        .collect();

    Ok(iwa_paths)
}

/// Read and Snappy-decompress a single `.iwa` file from the ZIP archive.
///
/// Apple IWA files use a custom framing format:
/// Each block in the file is: `[type: u8][length: u24 LE][payload: length bytes]`
/// - type `0x00`: Snappy-compressed block → decompress payload with raw Snappy
/// - type `0x01`: Uncompressed block → use payload as-is
///
/// Multiple blocks are concatenated to form the decompressed IWA stream.
pub(crate) fn read_iwa_file(content: &[u8], path: &str) -> Result<Vec<u8>> {
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
pub(crate) fn decode_iwa_stream(data: &[u8]) -> std::result::Result<Vec<u8>, String> {
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
pub(crate) fn extract_text_from_proto(data: &[u8]) -> Vec<String> {
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
                if let Ok(s) = utf8_validation::from_utf8(payload) {
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

/// Extract metadata from an iWork ZIP archive.
///
/// Attempts to read `Metadata/Properties.plist` and
/// `Metadata/BuildVersionHistory.plist` from the ZIP. These files are XML plists
/// containing authorship and creation information. If the files cannot be read
/// or parsed, an empty `Metadata` is returned.
pub(crate) fn extract_metadata_from_zip(content: &[u8]) -> crate::types::metadata::Metadata {
    let cursor = Cursor::new(content);
    let Ok(mut archive) = zip::ZipArchive::new(cursor) else {
        return crate::types::metadata::Metadata::default();
    };

    let mut metadata = crate::types::metadata::Metadata::default();

    // Try to read Metadata/Properties.plist (XML plist with doc metadata)
    if let Ok(mut file) = archive.by_name("Metadata/Properties.plist") {
        let mut buf = Vec::new();
        if file.read_to_end(&mut buf).is_ok()
            && let Ok(text) = std::str::from_utf8(&buf)
        {
            parse_plist_metadata(text, &mut metadata);
        }
    }

    // Try to read Metadata/DocumentIdentifier from the ZIP
    // (some iWork files store the doc title here)
    if let Ok(mut file) = archive.by_name("Metadata/DocumentIdentifier") {
        let mut buf = Vec::new();
        if file.read_to_end(&mut buf).is_ok()
            && let Ok(text) = std::str::from_utf8(&buf)
        {
            let trimmed = text.trim();
            if !trimmed.is_empty() && metadata.title.is_none() {
                metadata.title = Some(trimmed.to_string());
            }
        }
    }

    metadata
}

/// Parse metadata fields from an XML plist string.
///
/// iWork plist metadata uses `<key>...</key><string>...</string>` pairs.
/// We extract known keys: title, author, keywords, language.
fn parse_plist_metadata(plist: &str, metadata: &mut crate::types::metadata::Metadata) {
    // Simple key/value extraction from XML plist without a full XML parser.
    // The format is: <key>NAME</key>\n<string>VALUE</string>
    let lines: Vec<&str> = plist.lines().map(|l| l.trim()).collect();
    let mut i = 0;
    while i < lines.len() {
        if let Some(key) = extract_plist_tag(lines[i], "key") {
            // Look at the next non-empty line for the value
            let mut j = i + 1;
            while j < lines.len() && lines[j].is_empty() {
                j += 1;
            }
            if j < lines.len()
                && let Some(value) = extract_plist_tag(lines[j], "string")
            {
                match key.as_str() {
                    "title" | "Title" if metadata.title.is_none() => {
                        metadata.title = Some(value);
                    }
                    "author" | "Author" | "creator" | "Creator" => {
                        let authors = metadata.authors.get_or_insert_with(Vec::new);
                        if !authors.contains(&value) {
                            authors.push(value);
                        }
                    }
                    "keywords" | "Keywords" => {
                        let kw = metadata.keywords.get_or_insert_with(Vec::new);
                        for word in value.split(',') {
                            let trimmed = word.trim().to_string();
                            if !trimmed.is_empty() && !kw.contains(&trimmed) {
                                kw.push(trimmed);
                            }
                        }
                    }
                    "language" | "Language" if metadata.language.is_none() => {
                        metadata.language = Some(value);
                    }
                    _ => {}
                }
                i = j + 1;
                continue;
            }
        }
        i += 1;
    }
}

/// Extract the text content of a simple XML tag, e.g. `<string>value</string>`.
fn extract_plist_tag(line: &str, tag: &str) -> Option<String> {
    let open = format!("<{tag}>");
    let close = format!("</{tag}>");
    if let Some(start) = line.find(&open)
        && let Some(end) = line.find(&close)
    {
        let content = &line[start + open.len()..end];
        return Some(content.to_string());
    }
    None
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

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_extract_text_from_proto_basic() {
        // Protobuf: field 3, wire type 2 (length-delimited) = tag 0x1A
        let text = b"Hello World from iWork";
        let mut proto = vec![0x1A, text.len() as u8];
        proto.extend_from_slice(text);

        let extracted = extract_text_from_proto(&proto);
        assert!(
            extracted.iter().any(|s| s.contains("Hello World")),
            "Should extract the embedded UTF-8 string: {:?}",
            extracted
        );
    }

    #[test]
    fn test_extract_text_from_proto_skips_binary() {
        // Craft a proto payload with binary blob (non-UTF-8)
        let binary: Vec<u8> = (0..20).map(|i| i * 7 + 3).collect();
        let mut proto = vec![0x1A, binary.len() as u8];
        proto.extend_from_slice(&binary);

        // Should not panic and should produce no valid text strings
        let extracted = extract_text_from_proto(&proto);
        // Binary data should not produce alphabetic strings
        for s in &extracted {
            assert!(
                !s.chars().all(|c| c.is_alphabetic()),
                "Binary blob should not produce clean alphabetic strings: {s}"
            );
        }
    }

    #[test]
    fn test_extract_text_from_proto_nested() {
        // Nested message: outer field 2 wrapping an inner field 3 with text
        let inner_text = b"Nested Content";
        let mut inner = vec![0x1A, inner_text.len() as u8];
        inner.extend_from_slice(inner_text);

        let mut outer = vec![0x12, inner.len() as u8]; // field 2, wire type 2
        outer.extend_from_slice(&inner);

        let extracted = extract_text_from_proto(&outer);
        assert!(
            extracted.iter().any(|s| s.contains("Nested Content")),
            "Should extract text from nested protobuf messages: {:?}",
            extracted
        );
    }

    #[test]
    fn test_collect_iwa_paths_returns_only_iwa() {
        use std::io::Write;

        // Build a minimal ZIP in memory with one .iwa and one .xml entry
        let mut buf = Vec::new();
        {
            let cursor = std::io::Cursor::new(&mut buf);
            let mut zip = zip::ZipWriter::new(cursor);
            let options = zip::write::FileOptions::<()>::default().compression_method(zip::CompressionMethod::Stored);
            zip.start_file("Index/Document.iwa", options).unwrap();
            zip.write_all(b"fake iwa content").unwrap();
            zip.start_file("metadata.xml", options).unwrap();
            zip.write_all(b"<xml/>").unwrap();
            zip.finish().unwrap();
        }

        let paths = collect_iwa_paths(&buf).expect("Should list IWA entries");
        assert_eq!(paths.len(), 1, "Should find exactly one .iwa entry");
        assert_eq!(paths[0], "Index/Document.iwa");
    }
}
