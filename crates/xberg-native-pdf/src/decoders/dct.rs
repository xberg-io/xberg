//! DCTDecode (JPEG) filter implementation.
//!
//! Pass-through decoder for JPEG data. JPEG images are already in compressed
//! format and are returned unchanged for later image extraction.

use crate::decoders::StreamDecoder;
use crate::error::Result;

/// DCTDecode filter implementation.
///
/// Pass-through for JPEG data - no actual decoding performed.
/// JPEG images are kept in their compressed format for later extraction.
pub struct DctDecoder;

impl StreamDecoder for DctDecoder {
    fn decode(&self, input: &[u8], max_output_bytes: usize) -> Result<Vec<u8>> {
        super::check_output_cap(self.name(), input.len(), max_output_bytes)?;
        Ok(input.to_vec())
    }

    fn name(&self) -> &str {
        "DCTDecode"
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_dct_decode_passthrough() {
        let decoder = DctDecoder;
        let jpeg_data = b"\xFF\xD8\xFF\xE0\x00\x10JFIF";
        let output = decoder.decode(jpeg_data, 0).unwrap();
        assert_eq!(output, jpeg_data);
    }

    #[test]
    fn test_dct_decode_empty() {
        let decoder = DctDecoder;
        let input = b"";
        let output = decoder.decode(input, 0).unwrap();
        assert_eq!(output, b"");
    }

    #[test]
    fn test_dct_decoder_name() {
        let decoder = DctDecoder;
        assert_eq!(decoder.name(), "DCTDecode");
    }
}
