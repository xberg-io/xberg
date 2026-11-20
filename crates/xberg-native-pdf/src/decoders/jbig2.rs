//! JBIG2Decode implementation.
//!
//! JBIG2 (Joint Bi-level Image Experts Group 2) compression for monochrome
//! (bi-level) images. JBIG2 provides significantly better compression than
//! CCITT Fax for scanned documents and is optimized for text and halftones.
//!
//! ## Current Status
//!
//! This is a pass-through decoder for JBIG2 data. JBIG2 images are
//! binary image compression formats typically used for scanned documents.
//! For text extraction purposes, the data is kept in compressed format.
//!
//! ## Implementation Options
//!
//! Full JBIG2 decoding requires one of:
//! - **jbig2dec crate**: Rust bindings to C library (GPL-3.0 licensed)
//! - **pdfium-render**: BSD-licensed, but heavy dependency
//! - **Custom implementation**: Complex, no pure Rust MIT/Apache decoder exists
//!
//! For applications requiring actual JBIG2 image rendering:
//! - Use the rendering path, which rasterizes with tiny-skia
//! - Or use `pdfium-render` feature for full PDF rendering including JBIG2
//!
//! PDF Spec: ISO 32000-1:2008, Section 7.4.7 - JBIG2Decode Filter

use crate::decoders::StreamDecoder;
use crate::error::Result;

/// JBIG2Decode filter implementation.
///
/// Pass-through for JBIG2 data - no actual decoding performed.
/// JBIG2 images are kept in their compressed format for later extraction.
///
/// JBIG2 is a modern compression standard for bi-level (black and white)
/// images, offering much better compression ratios than older formats like
/// CCITT Fax. It's commonly used in PDF/A for archival documents.
pub struct Jbig2Decoder;

impl StreamDecoder for Jbig2Decoder {
    fn decode(&self, input: &[u8], max_output_bytes: usize) -> Result<Vec<u8>> {
        super::check_output_cap(self.name(), input.len(), max_output_bytes)?;
        Ok(input.to_vec())
    }

    fn name(&self) -> &str {
        "JBIG2Decode"
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_jbig2_decode_passthrough() {
        let decoder = Jbig2Decoder;
        let jbig2_data = b"\x97\x4A\x42\x32\x0D\x0A\x1A\x0A";
        let output = decoder.decode(jbig2_data, 0).unwrap();
        assert_eq!(output, jbig2_data);
    }

    #[test]
    fn test_jbig2_decode_empty() {
        let decoder = Jbig2Decoder;
        let input = b"";
        let output = decoder.decode(input, 0).unwrap();
        assert_eq!(output, b"");
    }

    #[test]
    fn test_jbig2_decoder_name() {
        let decoder = Jbig2Decoder;
        assert_eq!(decoder.name(), "JBIG2Decode");
    }
}
