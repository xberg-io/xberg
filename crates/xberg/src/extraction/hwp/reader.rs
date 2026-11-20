/// CFB stream reading and decompression utilities.
///
/// Consolidated from hwpers reader/cfb.rs, reader/stream.rs, and utils/compression.rs.
use super::error::{HwpError, Result};
use crate::extractors::security::SecurityError;
use cfb::CompoundFile;
use flate2::read::{DeflateDecoder, ZlibDecoder};
use std::io::{Cursor, Read, Seek};

// ---------------------------------------------------------------------------

/// Thin wrapper around a CFB `CompoundFile` that reads named streams from HWP documents.
#[cfg_attr(alef, alef(skip))]
pub struct CfbReader<F> {
    cfb: CompoundFile<F>,
}

impl CfbReader<Cursor<Vec<u8>>> {
    /// Open a CFB compound file from raw bytes.
    pub(crate) fn from_bytes(bytes: &[u8]) -> Result<Self> {
        let cursor = Cursor::new(bytes.to_vec());
        let cfb = CompoundFile::open(cursor).map_err(|e| HwpError::Cfb(format!("Failed to open CFB: {e}")))?;
        Ok(Self { cfb })
    }
}

impl<F: Read + Seek> CfbReader<F> {
    /// Read a named stream into a `Vec<u8>`.
    pub(crate) fn read_stream(&mut self, path: &str) -> Result<Vec<u8>> {
        let mut stream = self
            .cfb
            .open_stream(path)
            .map_err(|e| HwpError::NotFound(format!("Stream '{path}' not found: {e}")))?;
        let mut buf = Vec::new();
        stream.read_to_end(&mut buf)?;
        Ok(buf)
    }

    /// Return `true` if the named stream exists in the compound file.
    pub(crate) fn stream_exists(&self, path: &str) -> bool {
        self.cfb.exists(path)
    }

    /// List all streams in the compound file, as root-relative paths.
    ///
    /// `cfb` reports entry paths absolutely (`/BodyText/Section0`), while every caller
    /// here — and [`Self::read_stream`], which resolves relative to the root — speaks in
    /// relative names (`BodyText/Section0`). Returning the absolute form made every
    /// `starts_with("BodyText/Section")` and `starts_with("BinData/")` test fail, so no
    /// section and no image was ever found in a real HWP file.
    pub(crate) fn list_streams(&self) -> Vec<String> {
        self.cfb
            .walk()
            .filter_map(|entry| {
                if entry.is_stream() {
                    let path = entry.path().to_string_lossy().into_owned();
                    Some(path.trim_start_matches('/').to_string())
                } else {
                    None
                }
            })
            .collect()
    }
}

/// Cursor-backed little-endian binary reader for HWP stream payloads.
#[cfg_attr(alef, alef(skip))]
pub struct StreamReader {
    cursor: Cursor<Vec<u8>>,
}

impl StreamReader {
    pub(crate) fn new(data: Vec<u8>) -> Self {
        Self {
            cursor: Cursor::new(data),
        }
    }

    #[cfg_attr(alef, alef(skip))]
    pub(crate) fn read_u16(&mut self) -> Result<u16> {
        let mut buf = [0u8; 2];
        self.cursor.read_exact(&mut buf)?;
        Ok(u16::from_le_bytes(buf))
    }

    #[cfg_attr(alef, alef(skip))]
    pub(crate) fn read_u32(&mut self) -> Result<u32> {
        let mut buf = [0u8; 4];
        self.cursor.read_exact(&mut buf)?;
        Ok(u32::from_le_bytes(buf))
    }

    #[cfg_attr(alef, alef(skip))]
    pub(crate) fn read_bytes(&mut self, len: usize) -> Result<Vec<u8>> {
        let mut buf = vec![0u8; len];
        self.cursor.read_exact(&mut buf)?;
        Ok(buf)
    }

    /// Number of bytes remaining from the current position to the end.
    pub(crate) fn remaining(&self) -> usize {
        let pos = self.cursor.position() as usize;
        let len = self.cursor.get_ref().len();
        len.saturating_sub(pos)
    }
}

/// Decompress a raw-deflate stream from an HWP section, producing at most `max_output` bytes.
///
/// HWP 5.0 compresses sections with raw deflate (no zlib header). Falls back
/// to zlib if raw deflate fails, and returns the data as-is if both fail.
///
/// The output is bounded by the read itself (`take(max_output + 1)`), not by any size
/// the file declares, so a deflate bomb is rejected after at most `max_output + 1`
/// bytes have been allocated.
pub(crate) fn decompress_stream(data: &[u8], max_output: usize) -> Result<Vec<u8>> {
    if data.is_empty() {
        return Ok(Vec::new());
    }

    if let Some(out) = inflate_bounded(DeflateDecoder::new(data), max_output)? {
        return Ok(out);
    }
    if let Some(out) = inflate_bounded(ZlibDecoder::new(data), max_output)? {
        return Ok(out);
    }

    if data.len() > max_output {
        return Err(HwpError::Security(SecurityError::ContentTooLarge {
            size: data.len(),
            max: max_output,
        }));
    }
    Ok(data.to_vec())
}

/// Read `decoder` to the end, `Ok(None)` if it fails, `Err` if it yields more than `max_output` bytes.
fn inflate_bounded<R: Read>(decoder: R, max_output: usize) -> Result<Option<Vec<u8>>> {
    let mut out = Vec::new();
    let cap = (max_output as u64).saturating_add(1);
    if decoder.take(cap).read_to_end(&mut out).is_err() {
        return Ok(None);
    }
    if out.len() > max_output {
        return Err(HwpError::Security(SecurityError::ContentTooLarge {
            size: out.len(),
            max: max_output,
        }));
    }
    Ok(Some(out))
}

#[cfg(test)]
mod tests {
    use super::*;
    use flate2::Compression;
    use flate2::write::DeflateEncoder;
    use std::io::Write;

    fn deflate_zeros(len: usize) -> Vec<u8> {
        let mut enc = DeflateEncoder::new(Vec::new(), Compression::best());
        enc.write_all(&vec![0u8; len]).expect("write zeros");
        enc.finish().expect("finish deflate")
    }

    #[test]
    fn decompress_stream_rejects_output_over_limit() {
        let bomb = deflate_zeros(4 * 1024 * 1024);
        assert!(
            bomb.len() < 64 * 1024,
            "fixture must be a small, highly compressed stream"
        );
        let err = decompress_stream(&bomb, 1024).expect_err("4 MiB of zeros must exceed a 1 KiB limit");
        assert!(
            matches!(
                err,
                HwpError::Security(SecurityError::ContentTooLarge { max: 1024, .. })
            ),
            "got {err:?}"
        );
    }

    #[test]
    fn decompress_stream_accepts_output_within_limit() {
        let data = deflate_zeros(4 * 1024 * 1024);
        let out = decompress_stream(&data, 8 * 1024 * 1024).expect("within limit");
        assert_eq!(out.len(), 4 * 1024 * 1024);
    }

    #[test]
    fn decompress_stream_accepts_output_exactly_at_limit() {
        let data = deflate_zeros(4096);
        assert_eq!(decompress_stream(&data, 4096).expect("at limit").len(), 4096);
        assert!(decompress_stream(&data, 4095).is_err());
    }
}
