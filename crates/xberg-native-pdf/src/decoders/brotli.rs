//! BrotliDecode filter (PDF 2.0, ISO 32000-2:2020).

use crate::error::{Error, Result};
use std::io::Read;

use super::StreamDecoder;

/// Decoder for BrotliDecode filter (PDF 2.0).
pub struct BrotliDecoder;

impl StreamDecoder for BrotliDecoder {
    fn decode(&self, input: &[u8], max_output_bytes: usize) -> Result<Vec<u8>> {
        let limit = if max_output_bytes == 0 {
            u64::MAX
        } else {
            (max_output_bytes as u64).saturating_add(1)
        };
        let mut output = Vec::new();
        brotli::Decompressor::new(input, 4096)
            .take(limit)
            .read_to_end(&mut output)
            .map_err(|e| Error::Decode(format!("BrotliDecode: {}", e)))?;
        super::check_output_cap(self.name(), output.len(), max_output_bytes)?;
        Ok(output)
    }

    fn name(&self) -> &str {
        "BrotliDecode"
    }
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
