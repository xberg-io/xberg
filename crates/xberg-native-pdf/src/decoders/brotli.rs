//! BrotliDecode filter (PDF 2.0, ISO 32000-2:2020).

use crate::error::{Error, Result};
use std::io::Read;

use super::StreamDecoder;

/// Decoder for BrotliDecode filter (PDF 2.0).
pub struct BrotliDecoder;

impl StreamDecoder for BrotliDecoder {
    fn decode(&self, input: &[u8], max_output_bytes: usize) -> Result<Vec<u8>> {
        decode(input, max_output_bytes, false)
    }

    fn name(&self) -> &str {
        "BrotliDecode"
    }
}

pub(super) fn decode(input: &[u8], max_output_bytes: usize, complete: bool) -> Result<Vec<u8>> {
    let decode_error = |error| Error::Decode(format!("BrotliDecode: {error}"));
    let limit = if max_output_bytes == 0 {
        u64::MAX
    } else {
        (max_output_bytes as u64).saturating_add(1)
    };
    let mut output = Vec::new();
    let mut reader = brotli::Decompressor::new(input, 4096);
    reader
        .by_ref()
        .take(limit)
        .read_to_end(&mut output)
        .map_err(decode_error)?;
    super::check_output_cap("BrotliDecode", output.len(), max_output_bytes)?;
    // The reader checks for buffered surplus on the read after it first returns EOF.
    if complete && (reader.read(&mut [0]).map_err(decode_error)? != 0 || !reader.get_ref().is_empty()) {
        return Err(Error::Decode("BrotliDecode: data after end of stream".into()));
    }
    Ok(output)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_brotli_roundtrip() {
        let original = b"Hello, PDF 2.0 BrotliDecode filter! This is a test.";

        let mut compressed = Vec::new();
        {
            let mut writer = brotli::CompressorWriter::new(&mut compressed, 4096, 6, 22);
            std::io::Write::write_all(&mut writer, original).unwrap();
        }

        let decoder = BrotliDecoder;
        let decoded = decoder.decode(&compressed, 0).unwrap();
        assert_eq!(decoded, original);
    }

    #[test]
    fn complete_brotli_requires_a_finished_stream() {
        let mut compressed = Vec::new();
        {
            let mut writer = brotli::CompressorWriter::new(&mut compressed, 4096, 6, 22);
            std::io::Write::write_all(&mut writer, b"q Q").unwrap();
        }
        assert_eq!(decode(&compressed, 3, true).unwrap(), b"q Q");
        assert!(decode(&compressed[..compressed.len() - 1], 3, true).is_err());
        compressed.push(0);
        assert!(decode(&compressed, 3, true).is_err());
    }

    #[test]
    fn test_brotli_empty_input() {
        let mut compressed = Vec::new();
        {
            let mut writer = brotli::CompressorWriter::new(&mut compressed, 4096, 6, 22);
            std::io::Write::write_all(&mut writer, b"").unwrap();
        }
        let decoder = BrotliDecoder;
        let decoded = decoder.decode(&compressed, 0).unwrap();
        assert!(decoded.is_empty());
    }
}
