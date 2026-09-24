//! LZWDecode implementation for PDF.
//!
//! Decompresses data using the Lempel-Ziv-Welch (LZW) algorithm as specified
//! in the PDF Reference (Section 7.4.4).
//!
//! PDF's LZW implementation:
//! - Uses MSB-first bit ordering
//! - Starts with 9-bit codes
//! - Increases code size when table fills up
//! - Uses EarlyChange=1 (change code size one code earlier than GIF/TIFF)
//! - Clear code is 256, EOD code is 257
//! - First available code is 258

use crate::decoders::{StreamDecoder, check_output_limit};
use crate::error::{Error, Result};

/// LZWDecode filter implementation.
///
/// Decompresses data using the LZW algorithm.
pub struct LzwDecoder;

impl StreamDecoder for LzwDecoder {
    fn decode(&self, input: &[u8]) -> Result<Vec<u8>> {
        self.decode_bounded(input, usize::MAX)
    }

    fn decode_bounded(&self, input: &[u8], max_output: usize) -> Result<Vec<u8>> {
        match decode_lzw_weezl(input, max_output)? {
            Some(data) => Ok(data),
            None => decode_lzw_custom(input, max_output),
        }
    }

    fn name(&self) -> &str {
        "LZWDecode"
    }
}

/// Decode using weezl crate (well-tested LZW implementation).
///
/// Returns `Ok(None)` when weezl cannot decode the stream, so the caller falls back to
/// the custom decoder. Crossing `max_output` is an error, not a reason to fall back.
fn decode_lzw_weezl(input: &[u8], max_output: usize) -> Result<Option<Vec<u8>>> {
    use weezl::{BitOrder, LzwError, LzwStatus, decode::Decoder as WeezlDecoder};

    const CHUNK_SIZE: usize = 1 << 12;

    // PDF uses MSB bit order, 8-bit minimum code size ~keep
    let mut decoder = WeezlDecoder::new(BitOrder::Msb, 8);
    let mut output = Vec::new();
    let mut remaining = input;

    let error = loop {
        let filled = output.len();
        output.resize(filled + CHUNK_SIZE, 0);
        let result = decoder.decode_bytes(remaining, &mut output[filled..]);
        output.truncate(filled + result.consumed_out);
        remaining = &remaining[result.consumed_in..];
        check_output_limit("LZWDecode", output.len(), max_output)?;

        match result.status {
            Ok(LzwStatus::Ok) => {}
            Ok(LzwStatus::Done) => return Ok(Some(output)),
            // Input ran out before the end code, which `Decoder::decode` also rejects ~keep
            Ok(LzwStatus::NoProgress) => break LzwError::InvalidCode,
            Err(e) => break e,
        }
    };

    tracing::warn!(filter = "LZWDecode", error = ?error, "weezl decode failed, falling back to custom decoder");
    Ok(None)
}

/// Custom LZW decoder for PDF (handles edge cases).
///
/// This implementation follows the PDF spec exactly, including EarlyChange behavior.
fn decode_lzw_custom(input: &[u8], max_output: usize) -> Result<Vec<u8>> {
    const CLEAR_CODE: u16 = 256;
    const EOD_CODE: u16 = 257;
    const FIRST_CODE: u16 = 258;
    const MAX_CODE_BITS: u8 = 12;

    let mut output = Vec::new();
    let mut table = init_lzw_table();
    let mut code_bits = 9;
    let mut next_code = FIRST_CODE;
    let mut bit_reader = BitReader::new(input);
    let mut prev_code: Option<u16> = None;

    loop {
        // EarlyChange=1: Check if we need to increase code size BEFORE reading
        // PDF's EarlyChange=1 means: increase code size when next_code == 2^code_bits - 1
        // This is "one code early" compared to standard LZW (which waits until 2^code_bits) ~keep
        if code_bits < MAX_CODE_BITS && next_code > 0 {
            let increase_at = (1 << code_bits) - 1;
            if next_code == increase_at {
                code_bits += 1;
            }
        }

        let code = match bit_reader.read_bits(code_bits) {
            Some(c) => c as u16,
            None => break,
        };

        if code == EOD_CODE {
            break;
        }

        if code == CLEAR_CODE {
            table = init_lzw_table();
            code_bits = 9;
            next_code = FIRST_CODE;
            prev_code = None;
            continue;
        }

        let string = if code < next_code {
            table
                .get(&code)
                .ok_or_else(|| Error::Decode(format!("Invalid LZW code: {} (table size: {})", code, table.len())))?
                .clone()
        } else if code == next_code && prev_code.is_some() {
            // Special case: code == next_code
            // String is prev_string + prev_string[0] ~keep
            let prev_string = table.get(&prev_code.unwrap()).unwrap();
            let mut s = prev_string.clone();
            s.push(prev_string[0]);
            s
        } else {
            return Err(Error::Decode(format!(
                "Invalid LZW code: {} (next_code={}, code_bits={})",
                code, next_code, code_bits
            )));
        };

        check_output_limit("LZWDecode", output.len() + string.len(), max_output)?;
        output.extend_from_slice(&string);

        if let Some(prev) = prev_code
            && next_code < 4096
        {
            let prev_string = table.get(&prev).unwrap();
            let mut new_string = prev_string.clone();
            new_string.push(string[0]);
            table.insert(next_code, new_string);
            next_code += 1;
        }

        prev_code = Some(code);
    }

    Ok(output)
}

/// Initialize the LZW string table with single-byte strings.
fn init_lzw_table() -> std::collections::HashMap<u16, Vec<u8>> {
    let mut table = std::collections::HashMap::new();
    for i in 0..=255u16 {
        table.insert(i, vec![i as u8]);
    }
    table
}

/// Bit reader for MSB-first bit ordering.
struct BitReader<'a> {
    data: &'a [u8],
    byte_pos: usize,
    bit_pos: u8,
}

impl<'a> BitReader<'a> {
    fn new(data: &'a [u8]) -> Self {
        Self {
            data,
            byte_pos: 0,
            bit_pos: 0,
        }
    }

    fn read_bits(&mut self, n: u8) -> Option<u32> {
        if n == 0 || n > 16 {
            return None;
        }

        let mut result = 0u32;
        let mut remaining = n;

        while remaining > 0 {
            if self.byte_pos >= self.data.len() {
                return None;
            }

            let bits_in_current_byte = 8 - self.bit_pos;
            let bits_to_read = remaining.min(bits_in_current_byte);

            let byte = self.data[self.byte_pos];
            let shift_amount = bits_in_current_byte - bits_to_read;
            let mask = if bits_to_read == 8 {
                0xFF
            } else {
                ((1u8 << bits_to_read) - 1) << shift_amount
            };
            let bits = (byte & mask) >> shift_amount;

            result = (result << bits_to_read) | (bits as u32);

            self.bit_pos += bits_to_read;
            if self.bit_pos >= 8 {
                self.byte_pos += 1;
                self.bit_pos = 0;
            }

            remaining -= bits_to_read;
        }

        Some(result)
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use weezl::{BitOrder, encode::Encoder as LzwEncoder};

    #[test]
    fn test_lzw_decode_simple() {
        let decoder = LzwDecoder;

        let original = b"ABCABCABCABC";
        let mut encoder = LzwEncoder::new(BitOrder::Msb, 8);
        let compressed = encoder.encode(original).unwrap();

        let decoded = decoder.decode(&compressed).unwrap();
        assert_eq!(decoded, original);
    }

    #[test]
    fn test_lzw_decode_empty() {
        let decoder = LzwDecoder;

        let original = b"";
        let mut encoder = LzwEncoder::new(BitOrder::Msb, 8);
        let compressed = encoder.encode(original).unwrap();

        let decoded = decoder.decode(&compressed).unwrap();
        assert_eq!(decoded, original);
    }

    #[test]
    fn test_lzw_decode_repeated_pattern() {
        let decoder = LzwDecoder;

        let original = b"The quick brown fox jumps over the lazy dog. ".repeat(10);
        let mut encoder = LzwEncoder::new(BitOrder::Msb, 8);
        let compressed = encoder.encode(&original).unwrap();

        let decoded = decoder.decode(&compressed).unwrap();
        assert_eq!(decoded, original);
    }

    #[test]
    fn test_lzw_decode_invalid_data() {
        let decoder = LzwDecoder;

        let invalid = b"This is not LZW compressed data";
        let result = decoder.decode(invalid);
        assert!(result.is_err());
    }

    #[test]
    fn test_lzw_decoder_name() {
        let decoder = LzwDecoder;
        assert_eq!(decoder.name(), "LZWDecode");
    }

    #[test]
    fn should_accept_output_exactly_at_the_limit() {
        let original = vec![b'A'; 10_000];
        let compressed = LzwEncoder::new(BitOrder::Msb, 8).encode(&original).unwrap();

        let decoded = LzwDecoder.decode_bounded(&compressed, original.len()).unwrap();
        assert_eq!(decoded, original);
    }

    #[test]
    fn should_reject_output_past_the_limit() {
        let original = vec![b'A'; 10_000];
        let compressed = LzwEncoder::new(BitOrder::Msb, 8).encode(&original).unwrap();

        let error = LzwDecoder.decode_bounded(&compressed, original.len() - 1).unwrap_err();
        assert!(error.to_string().contains("exceeds limit 9999 bytes"), "got: {error}");
    }

    #[test]
    fn should_bound_the_fallback_decoder_too() {
        let original = vec![b'A'; 10_000];
        let compressed = LzwEncoder::new(BitOrder::Msb, 8).encode(&original).unwrap();

        assert_eq!(decode_lzw_custom(&compressed, original.len()).unwrap(), original);
        let error = decode_lzw_custom(&compressed, 1_000).unwrap_err();
        assert!(error.to_string().contains("exceeds limit 1000 bytes"), "got: {error}");
    }
}
