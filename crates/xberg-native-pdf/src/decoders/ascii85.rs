//! ASCII85Decode (Base85) implementation.
//!
//! Decodes ASCII85/Base85 encoded data. This encoding represents 4 bytes
//! as 5 ASCII characters in the range '!' to 'u'.
//! Special case: 'z' represents 4 zero bytes (00000000).

use crate::decoders::{StreamDecoder, check_output_cap};
use crate::error::{Error, Result};

/// ASCII85Decode filter implementation.
///
/// Decodes data encoded using the ASCII85/Base85 algorithm.
pub struct Ascii85Decoder;

impl StreamDecoder for Ascii85Decoder {
    fn decode(&self, input: &[u8], max_output_bytes: usize) -> Result<Vec<u8>> {
        let mut output = Vec::new();
        let mut acc: u32 = 0;
        let mut count = 0;

        for &byte in input {
            match byte {
                b'~' => break,
                b'z' => {
                    if count != 0 {
                        return Err(Error::Decode(
                            "ASCII85Decode: 'z' must not appear in the middle of a group".to_string(),
                        ));
                    }
                    check_output_cap(self.name(), output.len() + 4, max_output_bytes)?;
                    output.extend_from_slice(&[0, 0, 0, 0]);
                }
                b'!'..=b'u' => {
                    acc = acc
                        .checked_mul(85)
                        .and_then(|v| v.checked_add((byte - b'!') as u32))
                        .ok_or_else(|| Error::Decode("ASCII85Decode: overflow in decoding".to_string()))?;
                    count += 1;

                    if count == 5 {
                        check_output_cap(self.name(), output.len() + 4, max_output_bytes)?;
                        output.extend_from_slice(&acc.to_be_bytes());
                        acc = 0;
                        count = 0;
                    }
                }
                _ if byte.is_ascii_whitespace() => {}
                _ => {
                    return Err(Error::Decode(format!(
                        "ASCII85Decode: invalid character '{}'",
                        byte as char
                    )));
                }
            }
        }

        if count > 0 {
            if count == 1 {
                return Err(Error::Decode(
                    "ASCII85Decode: incomplete group (need at least 2 characters)".to_string(),
                ));
            }

            // Pad with 'u' (84 = 117 - 33) to complete the group ~keep
            for _ in count..5 {
                acc = acc
                    .checked_mul(85)
                    .and_then(|v| v.checked_add(84))
                    .ok_or_else(|| Error::Decode("ASCII85Decode: overflow in padding".to_string()))?;
            }

            // Output count-1 bytes (first N-1 bytes are valid) ~keep
            let bytes = acc.to_be_bytes();
            check_output_cap(self.name(), output.len() + count - 1, max_output_bytes)?;
            output.extend_from_slice(&bytes[..count - 1]);
        }

        Ok(output)
    }

    fn name(&self) -> &str {
        "ASCII85Decode"
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_ascii85_decode_simple() {
        let decoder = Ascii85Decoder;
        // "Test" encoded in ASCII85 (4 bytes = 1 complete group) ~keep
        let input = b"<+U,m";
        let output = decoder.decode(input, 0).unwrap();
        assert_eq!(output, b"Test");
    }

    #[test]
    fn test_ascii85_decode_z_special_case() {
        let decoder = Ascii85Decoder;
        let input = b"z";
        let output = decoder.decode(input, 0).unwrap();
        assert_eq!(output, b"\x00\x00\x00\x00");
    }

    #[test]
    fn test_ascii85_decode_multiple_z() {
        let decoder = Ascii85Decoder;
        let input = b"zz";
        let output = decoder.decode(input, 0).unwrap();
        assert_eq!(output, b"\x00\x00\x00\x00\x00\x00\x00\x00");
    }

    #[test]
    fn test_ascii85_decode_with_whitespace() {
        let decoder = Ascii85Decoder;
        let input = b"<+U ,m";
        let output = decoder.decode(input, 0).unwrap();
        assert_eq!(output, b"Test");
    }

    #[test]
    fn test_ascii85_decode_with_end_marker() {
        let decoder = Ascii85Decoder;
        let input = b"<+U,m~>";
        let output = decoder.decode(input, 0).unwrap();
        assert_eq!(output, b"Test");
    }

    #[test]
    fn test_ascii85_decode_empty() {
        let decoder = Ascii85Decoder;
        let input = b"";
        let output = decoder.decode(input, 0).unwrap();
        assert_eq!(output, b"");
    }

    #[test]
    fn test_ascii85_decode_padding() {
        let decoder = Ascii85Decoder;
        let input = b"!!";
        let output = decoder.decode(input, 0).unwrap();
        assert!(!output.is_empty());
    }

    #[test]
    fn test_ascii85_decode_invalid_character() {
        let decoder = Ascii85Decoder;
        let input = b"Hello\x00";
        let result = decoder.decode(input, 0);
        assert!(result.is_err());
    }

    #[test]
    fn test_ascii85_decode_z_in_middle() {
        let decoder = Ascii85Decoder;
        // 'z' in the middle of a group is invalid ~keep
        let input = b"!z";
        let result = decoder.decode(input, 0);
        assert!(result.is_err());
    }

    #[test]
    fn test_ascii85_decode_single_char() {
        let decoder = Ascii85Decoder;
        // Single character (not 'z') is invalid ~keep
        let input = b"!";
        let result = decoder.decode(input, 0);
        assert!(result.is_err());
    }

    #[test]
    fn test_ascii85_decoder_name() {
        let decoder = Ascii85Decoder;
        assert_eq!(decoder.name(), "ASCII85Decode");
    }
}
