//! Character encoding utilities for RTF parsing.
//!
//! Provides hex byte parsing and legacy Windows codepage decoding for RTF byte escapes.

use encoding_rs::Encoding;

/// Convert a hex digit character to its numeric value.
///
/// Returns None if the character is not a valid hex digit.
#[inline]
pub fn hex_digit_to_u8(c: char) -> Option<u8> {
    match c {
        '0'..='9' => Some((c as u8) - b'0'),
        'a'..='f' => Some((c as u8) - b'a' + 10),
        'A'..='F' => Some((c as u8) - b'A' + 10),
        _ => None,
    }
}

/// Parse a hex-encoded byte from two characters.
///
/// Returns the decoded byte if both characters are valid hex digits.
#[inline]
pub fn parse_hex_byte(h1: char, h2: char) -> Option<u8> {
    let high = hex_digit_to_u8(h1)?;
    let low = hex_digit_to_u8(h2)?;
    Some((high << 4) | low)
}

/// Decode a byte using Windows-1252 encoding for the 0x80-0x9F range.
///
/// This function maps Windows-1252 bytes in the 0x80-0x9F range to their
/// corresponding Unicode characters. For other values, it returns the byte
/// as a character directly.
#[inline]
pub fn decode_windows_1252(byte: u8) -> char {
    match byte {
        0x80 => '\u{20AC}', // Euro sign
        0x81 => '?',
        0x82 => '\u{201A}', // Single low-9 quotation mark
        0x83 => '\u{0192}', // Latin small letter f with hook
        0x84 => '\u{201E}', // Double low-9 quotation mark
        0x85 => '\u{2026}', // Horizontal ellipsis
        0x86 => '\u{2020}', // Dagger
        0x87 => '\u{2021}', // Double dagger
        0x88 => '\u{02C6}', // Modifier letter circumflex accent
        0x89 => '\u{2030}', // Per mille sign
        0x8A => '\u{0160}', // Latin capital letter S with caron
        0x8B => '\u{2039}', // Single left-pointing angle quotation mark
        0x8C => '\u{0152}', // Latin capital ligature OE
        0x8D => '?',
        0x8E => '\u{017D}', // Latin capital letter Z with caron
        0x8F => '?',
        0x90 => '?',
        0x91 => '\u{2018}', // Left single quotation mark
        0x92 => '\u{2019}', // Right single quotation mark
        0x93 => '\u{201C}', // Left double quotation mark
        0x94 => '\u{201D}', // Right double quotation mark
        0x95 => '\u{2022}', // Bullet
        0x96 => '\u{2013}', // En dash
        0x97 => '\u{2014}', // Em dash
        0x98 => '\u{02DC}', // Small tilde
        0x99 => '\u{2122}', // Trade mark sign
        0x9A => '\u{0161}', // Latin small letter s with caron
        0x9B => '\u{203A}', // Single right-pointing angle quotation mark
        0x9C => '\u{0153}', // Latin small ligature oe
        0x9D => '?',
        0x9E => '\u{017E}', // Latin small letter z with caron
        0x9F => '\u{0178}', // Latin capital letter Y with diaeresis
        _ => byte as char,
    }
}

/// Map a Windows codepage number to an `encoding_rs` encoding.
///
/// Unknown values fall back to Windows-1252, the RTF default ANSI codepage.
#[inline]
pub(crate) fn encoding_for_windows_codepage(codepage: u32) -> &'static Encoding {
    let label: &[u8] = match codepage {
        65001 => b"utf-8",
        20127 => b"us-ascii",
        1250 => b"windows-1250",
        1251 => b"windows-1251",
        1252 => b"windows-1252",
        1253 => b"windows-1253",
        1254 => b"windows-1254",
        1255 => b"windows-1255",
        1256 => b"windows-1256",
        1257 => b"windows-1257",
        1258 => b"windows-1258",
        932 | 10001 => b"shift_jis",
        936 | 10008 => b"gbk",
        949 | 10003 => b"euc-kr",
        950 | 10002 => b"big5",
        28591 => b"iso-8859-1",
        28592 => b"iso-8859-2",
        28595 => b"iso-8859-5",
        28597 => b"iso-8859-7",
        28599 => b"iso-8859-9",
        _ => b"windows-1252",
    };
    Encoding::for_label(label).unwrap_or(encoding_rs::WINDOWS_1252)
}

/// Decode RTF hex escape bytes using the active ANSI codepage.
#[inline]
pub(crate) fn decode_ansi_bytes(bytes: &[u8], codepage: u32) -> String {
    if codepage == 1252 {
        return bytes.iter().map(|&byte| decode_windows_1252(byte)).collect();
    }

    let (decoded, _, _) = encoding_for_windows_codepage(codepage).decode(bytes);
    decoded.into_owned()
}

/// Parse an RTF control word and extract its value.
///
/// Returns a tuple of (control_word, optional_numeric_value).
pub fn parse_rtf_control_word(chars: &mut std::iter::Peekable<std::str::Chars>) -> (String, Option<i32>) {
    let mut word = String::new();
    let mut num_str = String::new();
    let mut is_negative = false;

    // Parse alphabetic control word
    while let Some(&c) = chars.peek() {
        if c.is_alphabetic() {
            word.push(c);
            chars.next();
        } else {
            break;
        }
    }

    // Check for negative sign
    if let Some(&c) = chars.peek()
        && c == '-'
    {
        is_negative = true;
        chars.next();
    }

    // Parse numeric parameter
    while let Some(&c) = chars.peek() {
        if c.is_ascii_digit() {
            num_str.push(c);
            chars.next();
        } else {
            break;
        }
    }

    let num_value = if !num_str.is_empty() {
        let val = num_str.parse::<i32>().unwrap_or(0);
        Some(if is_negative { -val } else { val })
    } else {
        None
    };

    // Per RTF spec, a space following a control word (with or without a
    // numeric parameter) is a delimiter and must be consumed. Without this,
    // font-encoding directives like `\loch\f31502 H` would emit a spurious
    // space before the text character.
    if let Some(&' ') = chars.peek() {
        chars.next();
    }

    (word, num_value)
}
