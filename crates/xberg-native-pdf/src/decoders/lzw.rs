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

use crate::decoders::{StreamDecoder, check_output_cap};
use crate::error::{Error, Result};

/// LZWDecode filter implementation.
///
/// Decompresses data using the LZW algorithm.
pub struct LzwDecoder;

impl StreamDecoder for LzwDecoder {
    fn decode(&self, input: &[u8], max_output_bytes: usize) -> Result<Vec<u8>> {
        match decode_lzw_weezl(input, max_output_bytes, None) {
            WeezlAttempt::Recovered(data) => Ok(data),
            // A cap violation is not a "this decoder can't handle it" failure the custom
            // fallback might succeed at differently — it is a real property of the stream's
            // expansion, so re-decoding the same input through an independent implementation
            // would only repeat the same bounded allocation for no benefit. ~keep
            WeezlAttempt::CapExceeded(err) => Err(err),
            WeezlAttempt::Failed => decode_lzw_custom(input, max_output_bytes),
        }
    }

    fn name(&self) -> &str {
        "LZWDecode"
    }
}

pub(super) fn decode_complete(input: &[u8], limit: usize, early_change: bool) -> Result<Vec<u8>> {
    match decode_lzw_weezl(input, limit, Some(early_change)) {
        WeezlAttempt::Recovered(data) => Ok(data),
        WeezlAttempt::CapExceeded(error) => Err(error),
        WeezlAttempt::Failed => Err(Error::Decode("LZWDecode: incomplete code stream".into())),
    }
}

/// Bytes decoded per `weezl` chunk before the output is re-checked against
/// `max_output_bytes`. Bounds how far a bounded decode can overshoot the cap. ~keep
const WEEZL_CHUNK_BYTES: usize = 8 * 1024;

/// Outcome of a weezl decode attempt: a clean decode, a decode that exceeded
/// `max_output_bytes`, or a genuine decode failure worth retrying with the custom decoder.
/// `Failed` carries no payload: the failure is already logged via `tracing::warn!` at the
/// point it occurs, and the caller retries with an independent decoder rather than
/// reporting this error, so keeping it around would just be dead weight. ~keep
enum WeezlAttempt {
    Recovered(Vec<u8>),
    CapExceeded(Error),
    Failed,
}

/// Decode using the weezl crate (well-tested LZW implementation), decoding in bounded
/// chunks via `decode_bytes` rather than `Decoder::decode`'s single unbounded call so a
/// dense LZW stream aborts mid-decode instead of materialising its full expansion before
/// anything checks it (GH#1764).
fn decode_lzw_weezl(input: &[u8], max_output_bytes: usize, complete_early_change: Option<bool>) -> WeezlAttempt {
    use weezl::{BitOrder, LzwStatus, decode::Decoder as WeezlDecoder};

    // PDF uses MSB bit order, 8-bit minimum code size ~keep
    let complete = complete_early_change.is_some();
    let mut decoder = if complete_early_change == Some(true) {
        WeezlDecoder::with_tiff_size_switch(BitOrder::Msb, 8)
    } else {
        WeezlDecoder::new(BitOrder::Msb, 8)
    };
    let mut output = Vec::new();
    let mut chunk = vec![0u8; WEEZL_CHUNK_BYTES];
    let mut remaining = input;

    loop {
        let chunk_len = if complete {
            chunk.len().min((max_output_bytes - output.len()).saturating_add(1))
        } else {
            chunk.len()
        };
        let result = decoder.decode_bytes(remaining, &mut chunk[..chunk_len]);
        output.extend_from_slice(&chunk[..result.consumed_out]);
        remaining = &remaining[result.consumed_in..];

        if let Err(err) = check_output_cap("LZWDecode", output.len(), max_output_bytes) {
            return WeezlAttempt::CapExceeded(err);
        }

        match result.status {
            Ok(LzwStatus::Done) if !complete || remaining.is_empty() => return WeezlAttempt::Recovered(output),
            Ok(LzwStatus::Done) => return WeezlAttempt::Failed,
            Ok(LzwStatus::Ok) => continue,
            Ok(LzwStatus::NoProgress) | Err(_) if complete => return WeezlAttempt::Failed,
            Ok(LzwStatus::NoProgress) => {
                tracing::warn!(
                    filter = "LZWDecode",
                    "weezl decode stalled before an end-of-data marker, falling back to custom decoder"
                );
                return WeezlAttempt::Failed;
            }
            Err(e) => {
                tracing::warn!(filter = "LZWDecode", error = ?e, "weezl decode failed, falling back to custom decoder");
                return WeezlAttempt::Failed;
            }
        }
    }
}

/// EarlyChange=1: increase the code size one code earlier than GIF/TIFF would (before
/// reading, when `next_code` has just reached `2^code_bits - 1`), per the PDF spec.
fn maybe_increase_code_size(code_bits: &mut u8, next_code: u16, max_code_bits: u8) {
    if *code_bits < max_code_bits && next_code > 0 {
        let increase_at = (1 << *code_bits) - 1;
        if next_code == increase_at {
            *code_bits += 1;
        }
    }
}

/// Custom LZW decoder for PDF (handles edge cases).
///
/// This implementation follows the PDF spec exactly, including EarlyChange behavior.
fn decode_lzw_custom(input: &[u8], max_output_bytes: usize) -> Result<Vec<u8>> {
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
        maybe_increase_code_size(&mut code_bits, next_code, MAX_CODE_BITS);

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

        output.extend_from_slice(&string);

        // GH#1764: a table entry can be as long as the whole 4096-code table allows, so
        // checking after every emitted code (rather than only at the end) bounds a bounded
        // decode's overshoot to one entry's length instead of the stream's full expansion. ~keep
        check_output_cap("LZWDecode", output.len(), max_output_bytes)?;

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

        let decoded = decoder.decode(&compressed, 0).unwrap();
        assert_eq!(decoded, original);
    }

    #[test]
    fn test_lzw_decode_empty() {
        let decoder = LzwDecoder;

        let original = b"";
        let mut encoder = LzwEncoder::new(BitOrder::Msb, 8);
        let compressed = encoder.encode(original).unwrap();

        let decoded = decoder.decode(&compressed, 0).unwrap();
        assert_eq!(decoded, original);
    }

    #[test]
    fn test_lzw_decode_repeated_pattern() {
        let decoder = LzwDecoder;

        let original = b"The quick brown fox jumps over the lazy dog. ".repeat(10);
        let mut encoder = LzwEncoder::new(BitOrder::Msb, 8);
        let compressed = encoder.encode(&original).unwrap();

        let decoded = decoder.decode(&compressed, 0).unwrap();
        assert_eq!(decoded, original);
    }

    #[test]
    fn test_lzw_decode_invalid_data() {
        let decoder = LzwDecoder;

        let invalid = b"This is not LZW compressed data";
        let result = decoder.decode(invalid, 0);
        assert!(result.is_err());
    }

    #[test]
    fn test_lzw_decoder_name() {
        let decoder = LzwDecoder;
        assert_eq!(decoder.name(), "LZWDecode");
    }

    /// GH#1764: `decode` must abort once output crosses `max_output_bytes`, not after
    /// building the stream's full expansion. A 10,000,000 byte input built from a
    /// two-byte repeat compresses extremely well under LZW, so decoding it fully needs
    /// far more work and memory than a 100,000 byte cap should ever allow the decoder
    /// to reach. Proving `bytes_at_abort` stayed within a small multiple of the cap
    /// (and well under the full expansion) proves the decoder stopped early rather than
    /// building the whole 10 MB before the caller's post-hoc check would have rejected it.
    #[test]
    fn test_lzw_decode_aborts_before_building_full_expansion() {
        let decoder = LzwDecoder;
        let original = b"AB".repeat(5_000_000);
        let full_expansion = original.len();
        let mut encoder = LzwEncoder::new(BitOrder::Msb, 8);
        let compressed = encoder.encode(&original).unwrap();
        let cap = 100_000;

        let start = std::time::Instant::now();
        let result = decoder.decode(&compressed, cap);
        let elapsed = start.elapsed();

        let err = result.expect_err("decode must reject output exceeding the cap");
        let message = err.to_string();
        let reported_bytes: usize = message
            .split("output size ")
            .nth(1)
            .and_then(|rest| rest.split(' ').next())
            .and_then(|n| n.parse().ok())
            .unwrap_or_else(|| panic!("error message did not report an observed size: {message}"));

        assert!(
            reported_bytes < cap + WEEZL_CHUNK_BYTES,
            "decode reported {reported_bytes} bytes at abort, more than one weezl chunk past \
             the {cap} byte cap; the decoder built more output than the cap allows before checking"
        );
        assert!(
            reported_bytes < full_expansion / 10,
            "decode reported {reported_bytes} bytes, within an order of magnitude of the full \
             {full_expansion} byte expansion; the abort did not happen early"
        );
        assert!(
            elapsed < std::time::Duration::from_millis(500),
            "decode took {elapsed:?} to abort a 100 KB-capped stream whose full decode is \
             {full_expansion} bytes; that is too slow for an early abort"
        );
    }
}
