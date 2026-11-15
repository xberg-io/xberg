//! ASCIIHexDecode implementation.
//!
//! Decodes hexadecimal-encoded data (e.g., "48656C6C6F" -> "Hello").
//! Whitespace is ignored, and odd-length input is padded with implicit '0'.

use crate::decoders::{StreamDecoder, check_output_cap};
use crate::error::{Error, Result};

/// ASCIIHexDecode filter implementation.
///
/// Decodes data encoded as pairs of hexadecimal digits.
pub struct AsciiHexDecoder;

impl StreamDecoder for AsciiHexDecoder {
    fn decode(&self, input: &[u8], max_output_bytes: usize) -> Result<Vec<u8>> {
        let mut output = Vec::new();
        let mut chars = input
            .iter()
            .filter(|&&c| !c.is_ascii_whitespace() && c != b'>')
            .peekable();

        while let Some(&high) = chars.next() {
            let low = chars.next().copied().unwrap_or(b'0');

            let high_nibble = hex_digit_to_value(high)
                .ok_or_else(|| Error::Decode(format!("ASCIIHexDecode: invalid hex digit '{}'", high as char)))?;

            let low_nibble = hex_digit_to_value(low)
                .ok_or_else(|| Error::Decode(format!("ASCIIHexDecode: invalid hex digit '{}'", low as char)))?;

            let byte = (high_nibble << 4) | low_nibble;
            check_output_cap(self.name(), output.len() + 1, max_output_bytes)?;
            output.push(byte);
        }

        Ok(output)
    }

    fn name(&self) -> &str {
        "ASCIIHexDecode"
    }
}

/// Convert a hexadecimal ASCII character to its numeric value.
fn hex_digit_to_value(digit: u8) -> Option<u8> {
    match digit {
        b'0'..=b'9' => Some(digit - b'0'),
        b'A'..=b'F' => Some(digit - b'A' + 10),
        b'a'..=b'f' => Some(digit - b'a' + 10),
        _ => None,
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_ascii_hex_decode_simple() {
        let decoder = AsciiHexDecoder;
        let input = b"48656C6C6F";
        let output = decoder.decode(input, 0).unwrap();
        assert_eq!(output, b"Hello");
    }

    #[test]
    fn test_ascii_hex_decode_with_whitespace() {
        let decoder = AsciiHexDecoder;
        let input = b"48 65 6C 6C 6F";
        let output = decoder.decode(input, 0).unwrap();
        assert_eq!(output, b"Hello");
    }

    #[test]
    fn test_ascii_hex_decode_odd_length() {
        let decoder = AsciiHexDecoder;
        let input = b"486";
        let output = decoder.decode(input, 0).unwrap();
        assert_eq!(output, b"H`");
    }

    #[test]
    fn test_ascii_hex_decode_with_end_marker() {
        let decoder = AsciiHexDecoder;
        let input = b"48656C6C6F>";
        let output = decoder.decode(input, 0).unwrap();
        assert_eq!(output, b"Hello");
    }

    #[test]
    fn test_ascii_hex_decode_lowercase() {
        let decoder = AsciiHexDecoder;
        let input = b"48656c6c6f";
        let output = decoder.decode(input, 0).unwrap();
        assert_eq!(output, b"Hello");
    }

    #[test]
    fn test_ascii_hex_decode_mixed_case() {
        let decoder = AsciiHexDecoder;
        let input = b"48656C6c6F";
        let output = decoder.decode(input, 0).unwrap();
        assert_eq!(output, b"Hello");
    }

    #[test]
    fn test_ascii_hex_decode_empty() {
        let decoder = AsciiHexDecoder;
        let input = b"";
        let output = decoder.decode(input, 0).unwrap();
        assert_eq!(output, b"");
    }

    #[test]
    fn test_ascii_hex_decode_invalid_digit() {
        let decoder = AsciiHexDecoder;
        let input = b"4G";
        let result = decoder.decode(input, 0);
        assert!(result.is_err());
    }

    #[test]
    fn test_ascii_hex_decoder_name() {
        let decoder = AsciiHexDecoder;
        assert_eq!(decoder.name(), "ASCIIHexDecode");
    }

    #[test]
    fn test_hex_digit_to_value() {
        assert_eq!(hex_digit_to_value(b'0'), Some(0));
        assert_eq!(hex_digit_to_value(b'9'), Some(9));
        assert_eq!(hex_digit_to_value(b'A'), Some(10));
        assert_eq!(hex_digit_to_value(b'F'), Some(15));
        assert_eq!(hex_digit_to_value(b'a'), Some(10));
        assert_eq!(hex_digit_to_value(b'f'), Some(15));
        assert_eq!(hex_digit_to_value(b'G'), None);
        assert_eq!(hex_digit_to_value(b'z'), None);
    }
}
