//! RunLengthDecode implementation.
//!
//! Decodes run-length encoded data according to PDF specification:
//! - Length byte 0-127: Copy next N+1 bytes literally
//! - Length byte 128: No-op (EOD marker)
//! - Length byte 129-255: Repeat next byte 257-N times

use crate::decoders::{StreamDecoder, check_output_cap};
use crate::error::{Error, Result};

/// RunLengthDecode filter implementation.
///
/// Decompresses run-length encoded data.
pub struct RunLengthDecoder;

impl StreamDecoder for RunLengthDecoder {
    fn decode(&self, input: &[u8], max_output_bytes: usize) -> Result<Vec<u8>> {
        decode(input, max_output_bytes, false)
    }

    fn name(&self) -> &str {
        "RunLengthDecode"
    }
}

pub(super) fn decode(input: &[u8], max_output_bytes: usize, complete: bool) -> Result<Vec<u8>> {
    let mut output = Vec::new();
    let mut i = 0;

    while i < input.len() {
        let length = input[i];
        i += 1;

        match length {
            0..=127 => {
                let count = length as usize + 1;

                if i + count > input.len() {
                    return Err(Error::Decode(format!(
                        "RunLengthDecode: not enough data for literal run (need {}, have {})",
                        count,
                        input.len() - i
                    )));
                }

                check_output_cap("RunLengthDecode", output.len() + count, max_output_bytes)?;
                output.extend_from_slice(&input[i..i + count]);
                i += count;
            }
            128 => {
                if complete && i != input.len() {
                    return Err(Error::Decode("RunLengthDecode: data after end-of-data marker".into()));
                }
                return Ok(output);
            }
            129..=255 => {
                let count = 257 - length as usize;

                if i >= input.len() {
                    return Err(Error::Decode("RunLengthDecode: missing byte for run".to_string()));
                }

                let byte = input[i];
                i += 1;
                check_output_cap("RunLengthDecode", output.len() + count, max_output_bytes)?;
                output.resize(output.len() + count, byte);
            }
        }
    }

    if complete {
        return Err(Error::Decode("RunLengthDecode: missing end-of-data marker".into()));
    }
    Ok(output)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_runlength_decode_literal() {
        let decoder = RunLengthDecoder;
        // Length 4 (copy 5 bytes), then "Hello" ~keep
        let input = vec![4, b'H', b'e', b'l', b'l', b'o'];
        let output = decoder.decode(&input, 0).unwrap();
        assert_eq!(output, b"Hello");
    }

    #[test]
    fn test_runlength_decode_run() {
        let decoder = RunLengthDecoder;
        // Repeat 'A' 5 times (257-252=5) ~keep
        let input = vec![252, b'A'];
        let output = decoder.decode(&input, 0).unwrap();
        assert_eq!(output, b"AAAAA");
    }

    #[test]
    fn test_runlength_decode_mixed() {
        let decoder = RunLengthDecoder;
        // Literal "Hi" (length 1 = 2 bytes), then repeat 'X' 3 times (257-254=3) ~keep
        let input = vec![1, b'H', b'i', 254, b'X'];
        let output = decoder.decode(&input, 0).unwrap();
        assert_eq!(output, b"HiXXX");
    }

    #[test]
    fn test_runlength_decode_eod_marker() {
        let decoder = RunLengthDecoder;
        // Literal "Hi", EOD marker (128), garbage after ~keep
        let input = vec![1, b'H', b'i', 128, 99, 99, 99];
        let output = decoder.decode(&input, 0).unwrap();
        assert_eq!(output, b"Hi");
    }

    #[test]
    fn test_runlength_decode_max_literal() {
        let decoder = RunLengthDecoder;
        // Max literal run: 127 -> copy 128 bytes ~keep
        let mut input = vec![127];
        input.extend_from_slice(&[b'A'; 128]);
        let output = decoder.decode(&input, 0).unwrap();
        assert_eq!(output.len(), 128);
        assert_eq!(output, vec![b'A'; 128]);
    }

    #[test]
    fn test_runlength_decode_max_run() {
        let decoder = RunLengthDecoder;
        // Max run: 129 -> repeat 128 times (257-129=128) ~keep
        let input = vec![129, b'B'];
        let output = decoder.decode(&input, 0).unwrap();
        assert_eq!(output.len(), 128);
        assert_eq!(output, vec![b'B'; 128]);
    }

    #[test]
    fn test_runlength_decode_empty() {
        let decoder = RunLengthDecoder;
        let input = vec![];
        let output = decoder.decode(&input, 0).unwrap();
        assert_eq!(output, b"");
    }

    #[test]
    fn test_runlength_decode_insufficient_data_literal() {
        let decoder = RunLengthDecoder;
        // Says copy 5 bytes but only provides 3 ~keep
        let input = vec![4, b'A', b'B', b'C'];
        let result = decoder.decode(&input, 0);
        assert!(result.is_err());
    }

    #[test]
    fn test_runlength_decode_missing_run_byte() {
        let decoder = RunLengthDecoder;
        // Says repeat but doesn't provide the byte to repeat ~keep
        let input = vec![252];
        let result = decoder.decode(&input, 0);
        assert!(result.is_err());
    }

    #[test]
    fn test_runlength_decoder_name() {
        let decoder = RunLengthDecoder;
        assert_eq!(decoder.name(), "RunLengthDecode");
    }

    /// GH#1764: `decode` must abort once output crosses `max_output_bytes`, not after
    /// building the stream's full expansion. `[129, byte]` is RunLength's densest
    /// encoding (2 input bytes -> 128 output bytes, the max unit this filter can ever
    /// emit in one step), so a bounded decoder can overshoot the cap by at most 127
    /// bytes. This input's full decode would be 2_000_000 * 128 = 256_000_000 bytes
    /// (256 MB); a decoder that actually built that before checking would need to
    /// process all 2,000,000 units. Proving `bytes_at_abort` stayed within one unit of
    /// a 1 MB cap proves it stopped after ~7,813 units, not 2,000,000. ~keep
    #[test]
    fn test_runlength_decode_aborts_before_building_full_expansion() {
        const MAX_RLE_UNIT_BYTES: usize = 128;
        let decoder = RunLengthDecoder;
        let unit_count = 2_000_000;
        let input = [129u8, b'A'].repeat(unit_count);
        let full_expansion = unit_count * MAX_RLE_UNIT_BYTES;
        let cap = 1_000_000;

        let start = std::time::Instant::now();
        let result = decoder.decode(&input, cap);
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
            reported_bytes < cap + MAX_RLE_UNIT_BYTES,
            "decode reported {reported_bytes} bytes at abort, which is more than one RLE unit \
             past the {cap} byte cap; the decoder built more output than the cap allows before checking"
        );
        assert!(
            reported_bytes < full_expansion / 10,
            "decode reported {reported_bytes} bytes, within an order of magnitude of the full \
             {full_expansion} byte expansion; the abort did not happen early"
        );
        assert!(
            elapsed < std::time::Duration::from_millis(500),
            "decode took {elapsed:?} to abort a 1 MB-capped stream; an unbounded decode of all \
             {unit_count} units would need to process 256x more data than this took"
        );
    }
}
