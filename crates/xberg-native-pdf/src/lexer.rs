//! PDF lexer (tokenizer).
//!
//! This module provides low-level tokenization of PDF byte streams.
//! It recognizes all PDF token types including numbers, strings, names,
//! keywords, and delimiters.
//!
//! # PDF Syntax Overview
//!
//! PDF uses a PostScript-like syntax with the following token types:
//! - Numbers: integers (42, -123) and reals (3.14, -2.5)
//! - Strings: literal ((Hello)) and hexadecimal (<48656C6C6F>)
//! - Names: identifiers starting with / (/Type, /Pages)
//! - Keywords: true, false, null
//! - Delimiters: `[`, `]`, `<<`, `>>`, `obj`, `endobj`, `stream`, `endstream`
//! - References: indirect object references (10 0 R)
//!
//! Whitespace (space, \t, \r, \n, \0, \f) and comments (% to EOL) are skipped.

use nom::{
    IResult, Parser,
    branch::alt,
    bytes::complete::{take_till, take_while},
    character::complete::{char, digit1},
    combinator::{map, opt, value},
    sequence::{delimited, preceded},
};

/// Token types recognized by the PDF lexer.
///
/// Tokens are the atomic units of PDF syntax. The parser combines tokens
/// into higher-level objects (dictionaries, arrays, etc.).
#[derive(Debug, PartialEq, Clone)]
pub enum Token<'a> {
    /// Integer number (e.g., 42, -123)
    Integer(i64),

    /// Real (floating-point) number (e.g., 3.14, -2.5, .5)
    Real(f64),

    /// Literal string bytes (e.g., content of "(Hello)")
    /// Note: Escape sequences are NOT decoded at lexer level
    LiteralString(&'a [u8]),

    /// Hexadecimal string bytes (e.g., content of "<48656C6C6F>")
    /// Whitespace is preserved; decoding happens at parser level
    HexString(&'a [u8]),

    /// Name (e.g., "Type" from "/Type")
    /// Note: # escape sequences ARE decoded at lexer level per PDF spec
    Name(String),

    /// Boolean true keyword
    True,

    /// Boolean false keyword
    False,

    /// Null keyword
    Null,

    /// Array start delimiter [
    ArrayStart,

    /// Array end delimiter ]
    ArrayEnd,

    /// Dictionary start delimiter <<
    DictStart,

    /// Dictionary end delimiter >>
    DictEnd,

    /// Indirect object start keyword "obj"
    ObjStart,

    /// Indirect object end keyword "endobj"
    ObjEnd,

    /// Stream start keyword "stream"
    StreamStart,

    /// Stream end keyword "endstream"
    StreamEnd,

    /// Reference keyword "R" (used in "10 0 R")
    R,
}

/// Parse whitespace characters (PDF Ref 1.7, Table 3.1).
///
/// PDF whitespace: space (0x20), tab (0x09), CR (0x0D), LF (0x0A),
/// null (0x00), form feed (0x0C).
///
/// Returns an error if no whitespace is found (requires at least one whitespace char).
fn whitespace(input: &[u8]) -> IResult<&[u8], ()> {
    let (remaining, ws) = take_while(|c| matches!(c, b' ' | b'\t' | b'\r' | b'\n' | 0x00 | 0x0C)).parse(input)?;

    if ws.is_empty() {
        return Err(nom::Err::Error(nom::error::Error::new(
            input,
            nom::error::ErrorKind::Space,
        )));
    }

    Ok((remaining, ()))
}

/// Parse a comment (% to end of line).
///
/// Comments start with % and continue until CR or LF (PDF Ref 1.7, Section 3.1.2).
fn comment(input: &[u8]) -> IResult<&[u8], ()> {
    value((), preceded(char('%'), take_till(|c| c == b'\r' || c == b'\n'))).parse(input)
}

/// Skip all whitespace and comments.
///
/// This is used before parsing each token to handle arbitrary
/// amounts of whitespace and comments between tokens.
pub(crate) fn skip_ws(input: &[u8]) -> IResult<&[u8], &[u8]> {
    let mut remaining = input;

    loop {
        if let Ok((rest, _)) = whitespace(remaining) {
            remaining = rest;
            continue;
        }

        if let Ok((rest, _)) = comment(remaining) {
            remaining = rest;
            continue;
        }

        break;
    }

    Ok((remaining, input))
}

/// Parse an integer or real number.
///
/// PDF numbers can be:
/// - Integers: 42, -123, +17
/// - Reals: 3.14, -2.5, .5, 0., -.002
///
/// Note: PDF allows leading +/- signs and numbers starting with decimal point.
fn parse_number(input: &[u8]) -> IResult<&[u8], Token<'_>> {
    // Repeated signs collapse to one (`--5` is -5, as pdf.js and Adobe Reader read it). ~keep
    let sign_len = input.iter().take_while(|b| matches!(**b, b'+' | b'-')).count();
    let negative = input[..sign_len].contains(&b'-');
    let input = &input[sign_len..];

    let (mut input, int_part) = opt(digit1).parse(input)?;
    // A minus inside a digit run is ignored (`4-5` reads as 45, as pdf.js does). ~keep
    let mut int_part: Option<std::borrow::Cow<[u8]>> = int_part.map(std::borrow::Cow::Borrowed);
    while let Some(digits) = int_part.as_mut()
        && input.first() == Some(&b'-')
        && input.get(1).is_some_and(u8::is_ascii_digit)
    {
        let (rest, more) = digit1.parse(&input[1..])?;
        digits.to_mut().extend_from_slice(more);
        input = rest;
    }

    let (input, frac_part) = opt(preceded(char('.'), opt(digit1))).parse(input)?;

    if int_part.is_none() && frac_part.is_none() {
        if sign_len > 0 {
            // Bare sign (e.g., `-` or `+`) without digits — treat as integer 0.
            // This occurs in malformed TJ arrays like `(v)-(e)` where `-` is a bare offset. ~keep
            return Ok((input, Token::Integer(0)));
        }
        return Err(nom::Err::Error(nom::error::Error::new(
            input,
            nom::error::ErrorKind::Digit,
        )));
    }

    if frac_part.is_some() {
        let mut num_str = String::new();
        if negative {
            num_str.push('-');
        }
        if let Some(int) = &int_part {
            num_str.push_str(
                std::str::from_utf8(int)
                    .map_err(|_| nom::Err::Error(nom::error::Error::new(input, nom::error::ErrorKind::Digit)))?,
            );
        } else {
            num_str.push('0');
        }
        num_str.push('.');
        if let Some(Some(frac)) = frac_part {
            num_str.push_str(
                std::str::from_utf8(frac)
                    .map_err(|_| nom::Err::Error(nom::error::Error::new(input, nom::error::ErrorKind::Digit)))?,
            );
        } else {
            num_str.push('0');
        }

        let mut num: f64 = num_str
            .parse()
            .map_err(|_| nom::Err::Error(nom::error::Error::new(input, nom::error::ErrorKind::Digit)))?;
        // PDF 32000-1:2008 Annex C.2, Table C.1: real values are bounded to
        // approximately ±3.403×10^38. `f64::from_str` saturates an
        // oversized all-digit literal to `f64::INFINITY` rather than
        // erroring, and that Infinity can poison downstream arithmetic
        // into NaN (e.g. combined with a degenerate CTM component,
        // `0.0 * Infinity`). Clamp to the spec's implementation limit
        // instead of propagating an out-of-spec magnitude. ~keep
        if num.is_infinite() {
            num = f64::from(f32::MAX).copysign(num);
        }
        Ok((input, Token::Real(num)))
    } else {
        let int_bytes = int_part
            .as_deref()
            .ok_or_else(|| nom::Err::Error(nom::error::Error::new(input, nom::error::ErrorKind::Digit)))?;
        let int_str = std::str::from_utf8(int_bytes)
            .map_err(|_| nom::Err::Error(nom::error::Error::new(input, nom::error::ErrorKind::Digit)))?;
        // A literal beyond i64 clamps rather than failing the token, so one absurd operand
        // cannot abort the stream. ~keep
        let magnitude: i64 = int_str.parse().unwrap_or(i64::MAX);
        Ok((input, Token::Integer(if negative { -magnitude } else { magnitude })))
    }
}

/// Parse a literal string enclosed in parentheses.
///
/// Literal strings are enclosed in ( and ) (PDF Ref 1.7, Section 3.2.3).
/// They can contain balanced nested parentheses and escape sequences.
///
/// This implementation handles:
/// - Balanced nested parentheses: (Hello (World))
/// - Escape sequences: \n, \r, \t, \b, \f, \\, \(, \), \ddd (octal)
/// - Line continuation: backslash at end of line
///
/// Note: We return the raw bytes including escape sequences.
/// Decoding happens at the parser level.
/// Position just past the backslash escape that starts at `pos`.
///
/// ISO 32000-1 §7.3.4.2: a backslash introduces either up to three octal
/// digits or one escaped byte. A backslash at end-of-input consumes nothing
/// further. Scanning only — decoding happens at the parser level.
fn skip_escape_sequence(bytes: &[u8], pos: usize) -> usize {
    let mut pos = pos + 1;
    if pos >= bytes.len() {
        return pos;
    }
    if !bytes[pos].is_ascii_digit() {
        return pos + 1;
    }
    pos += 1;
    for _ in 0..2 {
        if pos < bytes.len() && bytes[pos].is_ascii_digit() {
            pos += 1;
        }
    }
    pos
}

fn parse_literal_string(input: &[u8]) -> IResult<&[u8], Token<'_>> {
    let (mut remaining, _) = char('(')(input)?;
    let mut depth = 1;
    let mut pos = 0;

    while depth > 0 && pos < remaining.len() {
        match remaining[pos] {
            b'\\' => pos = skip_escape_sequence(remaining, pos),
            b'(' => {
                depth += 1;
                pos += 1;
            }
            b')' => {
                depth -= 1;
                pos += 1;
            }
            _ => {
                pos += 1;
            }
        }
    }

    if depth != 0 {
        return Err(nom::Err::Error(nom::error::Error::new(
            input,
            nom::error::ErrorKind::Tag,
        )));
    }

    let content = &remaining[..pos - 1];
    remaining = &remaining[pos..];

    Ok((remaining, Token::LiteralString(content)))
}

/// Parse a hexadecimal string enclosed in angle brackets.
///
/// Hex strings are enclosed in < and > (PDF Ref 1.7, Section 3.2.3).
/// They contain pairs of hex digits representing bytes.
/// Whitespace is ignored. Odd number of digits is padded with 0.
///
/// Examples: <48656C6C6F> = "Hello", <901FA3> = bytes [0x90, 0x1F, 0xA3]
fn parse_hex_string(input: &[u8]) -> IResult<&[u8], Token<'_>> {
    if input.len() >= 2 && input[0] == b'<' && input[1] == b'<' {
        return Err(nom::Err::Error(nom::error::Error::new(
            input,
            nom::error::ErrorKind::Tag,
        )));
    }

    delimited(
        char('<'),
        map(take_while(|c: u8| c != b'>'), Token::HexString),
        char('>'),
    )
    .parse(input)
}

/// Decode #XX escape sequences in PDF names.
///
/// PDF Spec: ISO 32000-1:2008, Section 7.3.5 - Name Objects
///
/// Name objects can contain any characters encoded as #XX where XX is a
/// two-digit hexadecimal code. For example, /A#20B becomes "A B".
///
/// `raw` is the bytes of the name token after the `/`. A malformed escape is
/// preserved: `A#` and `A#ZZ` decode unchanged.
///
/// ISO 32000-1 §7.3.5: a name is a byte sequence and `#XX` stands for the byte XX, so `/A#42`
/// and `/AB` are the same name. Escapes are resolved to bytes first, then the bytes become UTF-8
/// text when valid UTF-8 and one char per byte (U+0000–U+00FF) otherwise, so the raw and the
/// escaped spelling of a name decode alike and distinct byte names stay distinct. ~keep
pub fn decode_name_escapes(raw: &[u8]) -> String {
    let unescaped;
    let bytes = if memchr::memchr(b'#', raw).is_none() {
        raw
    } else {
        unescaped = unescape_name(raw);
        unescaped.as_slice()
    };
    match std::str::from_utf8(bytes) {
        Ok(name) => name.to_owned(),
        Err(_) => bytes.iter().map(|&byte| char::from(byte)).collect(),
    }
}

/// Resolve `#XX` escapes to their bytes; a malformed escape is kept verbatim.
fn unescape_name(raw: &[u8]) -> Vec<u8> {
    let mut out = Vec::with_capacity(raw.len());
    let mut i = 0;
    while i < raw.len() {
        let byte = raw[i];
        i += 1;
        if byte != b'#' {
            out.push(byte);
            continue;
        }
        match (raw.get(i).copied(), raw.get(i + 1).copied()) {
            (Some(h1), Some(h2)) => {
                i += 2;
                match hex_escape_byte(h1, h2) {
                    Some(value) => out.push(value),
                    None => out.extend_from_slice(&[b'#', h1, h2]),
                }
            }
            (Some(h1), None) => {
                i += 1;
                out.extend_from_slice(&[b'#', h1]);
            }
            _ => out.push(b'#'),
        }
    }
    out
}

/// The byte a `#XX` name escape encodes. Matches `u8::from_str_radix(XX, 16)`,
/// which this replaces, including its acceptance of a leading `+` (`#+F` is
/// 0x0F). ~keep
fn hex_escape_byte(h1: u8, h2: u8) -> Option<u8> {
    let low = char::from(h2).to_digit(16)?;
    let high = if h1 == b'+' { 0 } else { char::from(h1).to_digit(16)? };
    u8::try_from(high * 16 + low).ok()
}

/// Parse a name starting with /.
///
/// Names are identifiers starting with / (PDF Ref 1.7, Section 3.2.4).
/// They can contain any characters except whitespace and delimiters.
/// Special characters can be encoded as #XX where XX is hex.
///
/// Examples: /Type, /FontName, /A;Name_With-Various***Characters, /A#20B (A B)
///
/// Note: # escape sequences ARE decoded at lexer level per PDF spec.
fn parse_name(input: &[u8]) -> IResult<&[u8], Token<'_>> {
    preceded(
        char('/'),
        map(
            take_while(|c: u8| {
                !matches!(
                    c,
                    b' ' | b'\t'
                        | b'\r'
                        | b'\n'
                        | 0x00
                        | 0x0C
                        | b'/'
                        | b'%'
                        | b'('
                        | b')'
                        | b'<'
                        | b'>'
                        | b'['
                        | b']'
                        | b'{'
                        | b'}'
                )
            }),
            |bytes| {
                // SPEC COMPLIANCE: PDF Spec ISO 32000-1:2008, Section 7.3.5
                // requires decoding #XX escape sequences in names.
                //
                // Empty names (e.g., "/ ") are technically invalid per spec but
                // we allow them in lenient mode for compatibility with malformed PDFs. ~keep
                Token::Name(decode_name_escapes(bytes))
            },
        ),
    )
    .parse(input)
}

/// Parse PDF keywords and delimiters.
///
/// Keywords are reserved words in PDF syntax:
/// - Boolean: true, false
/// - Null: null
/// - Object markers: obj, endobj, stream, endstream
/// - Delimiters: [, ], <<, >>
/// - Reference marker: R
fn parse_keyword(input: &[u8]) -> IResult<&[u8], Token<'_>> {
    match input {
        [b'f', b'a', b'l', b's', b'e', rest @ ..] => Ok((rest, Token::False)),
        [b't', b'r', b'u', b'e', rest @ ..] => Ok((rest, Token::True)),
        [b'n', b'u', b'l', b'l', rest @ ..] => Ok((rest, Token::Null)),
        [b'o', b'b', b'j', rest @ ..] => Ok((rest, Token::ObjStart)),
        [b'e', b'n', b'd', b'o', b'b', b'j', rest @ ..] => Ok((rest, Token::ObjEnd)),
        [b'e', b'n', b'd', b's', b't', b'r', b'e', b'a', b'm', rest @ ..] => Ok((rest, Token::StreamEnd)),
        [b's', b't', b'r', b'e', b'a', b'm', rest @ ..] => Ok((rest, Token::StreamStart)),
        [b'<', b'<', rest @ ..] => Ok((rest, Token::DictStart)),
        [b'>', b'>', rest @ ..] => Ok((rest, Token::DictEnd)),
        [b'[', rest @ ..] => Ok((rest, Token::ArrayStart)),
        [b']', rest @ ..] => Ok((rest, Token::ArrayEnd)),
        // Alphabetic continuations are operators such as RG, Re, and RI. ~keep
        [b'R', rest @ ..] if !rest.first().is_some_and(u8::is_ascii_alphabetic) => Ok((rest, Token::R)),
        _ => Err(nom::Err::Error(nom::error::Error::new(
            input,
            nom::error::ErrorKind::Tag,
        ))),
    }
}

/// Parse a bare identifier as a lenient fallback.
///
/// Some malformed PDFs use identifiers without a leading `/`, for example
/// `OBJR` instead of the spec-correct `/OBJR`. The PDF spec does not define
/// such bare words, but real-world parsers (poppler, pdfjs) treat them as
/// Name tokens to avoid failing on otherwise-recoverable documents. We do
/// the same as a last resort, consuming alphanumeric chars and `_-+*@.!^~`.
fn parse_bare_identifier(input: &[u8]) -> IResult<&[u8], Token<'_>> {
    let (rest, word) = take_while(|c: u8| {
        c.is_ascii_alphanumeric() || matches!(c, b'_' | b'-' | b'+' | b'*' | b'@' | b'.' | b'!' | b'^' | b'~')
    })(input)?;
    if word.is_empty() {
        return Err(nom::Err::Error(nom::error::Error::new(
            input,
            nom::error::ErrorKind::TakeWhile1,
        )));
    }
    let name = std::str::from_utf8(word).unwrap_or("");
    Ok((rest, Token::Name(name.to_string())))
}

/// Parse a single PDF token (strict mode).
///
/// This is the main entry point for the lexer. It skips whitespace/comments
/// and then tries to parse any valid PDF token type.
///
/// # Parsing Order
///
/// The order of alternatives matters:
/// 1. Keywords (true/false/null/obj/etc.) - must check before names
/// 2. Names (/Type) - must check before numbers (/ could start a name)
/// 3. Numbers (integers and reals)
/// 4. Strings (literal and hex)
///
/// # Errors
///
/// Returns `Err` if the input doesn't start with a valid token after
/// skipping whitespace.
pub fn token(input: &[u8]) -> IResult<&[u8], Token<'_>> {
    let (input, _) = skip_ws(input)?;

    alt((
        parse_keyword,
        parse_name,
        parse_number,
        parse_literal_string,
        parse_hex_string,
    ))
    .parse(input)
}

/// Parse a single PDF token with lenient bare-identifier fallback.
///
/// Like `token()`, but also accepts bare alphanumeric words (e.g. `OBJR`
/// without the required `/` prefix). Only use this for object-body parsing
/// where malformed PDFs may omit the `/` — **never** in content-stream
/// operator scanning, where bare words are operators, not names.
pub(crate) fn token_lenient(input: &[u8]) -> IResult<&[u8], Token<'_>> {
    let (input, _) = skip_ws(input)?;

    alt((
        parse_keyword,
        parse_name,
        parse_number,
        parse_literal_string,
        parse_hex_string,
        parse_bare_identifier,
    ))
    .parse(input)
}

#[cfg(test)]
mod tests {
    use super::*;

    // 3.14 is a common PDF test value, not trying to use PI constant ~keep
    #[allow(clippy::approx_constant)]
    fn _allow_approx_const() {}

    /// A content stream is tokenized from one slice whose remainder shrinks token by token; each
    /// token must cost its own length, not the length of everything after it (#3040). Doubling the
    /// stream doubles the time when linear and quadruples it when not, so the ratio is asserted
    /// instead of an absolute time that depends on the machine. An optimized build runs both sizes
    /// in milliseconds, where timer noise outweighs the ratio, so the test runs unoptimized only.
    #[cfg(debug_assertions)]
    #[test]
    fn tokenizing_a_large_stream_costs_time_linear_in_its_length() {
        const TOKENS: usize = 150_000;
        const MAX_RATIO: f64 = 2.8;
        let time_tokens = |count: usize| {
            let stream = b"12 ".repeat(count);
            (0..3)
                .map(|_| {
                    let started = std::time::Instant::now();
                    let mut remaining = &stream[..];
                    let mut tokens = 0_usize;
                    while let Ok((rest, _)) = token(remaining) {
                        remaining = rest;
                        tokens += 1;
                    }
                    let took = started.elapsed().as_secs_f64();
                    assert_eq!(tokens, count);
                    took
                })
                .fold(f64::INFINITY, f64::min)
        };
        let single = time_tokens(TOKENS);
        let double = time_tokens(2 * TOKENS);
        let ratio = double / single;
        assert!(
            ratio < MAX_RATIO,
            "doubling the stream multiplied the tokenizing time by {ratio:.2} ({single:.3}s -> {double:.3}s): the cost is not linear"
        );
    }

    #[test]
    fn repeated_minus_signs_read_as_one_negative() {
        assert_eq!(token(b"--5 0 Td"), Ok((&b" 0 Td"[..], Token::Integer(-5))));
        assert_eq!(token(b"--1.5"), Ok((&b""[..], Token::Real(-1.5))));
        assert_eq!(token(b"+-7"), Ok((&b""[..], Token::Integer(-7))));
    }

    #[test]
    fn minus_inside_digits_is_ignored_and_second_dot_ends_the_number() {
        assert_eq!(token(b"4-5"), Ok((&b""[..], Token::Integer(45))));
        assert_eq!(token(b"1.2.3"), Ok((&b".3"[..], Token::Real(1.2))));
        assert_eq!(token(b"4 -5"), Ok((&b" -5"[..], Token::Integer(4))));
    }

    #[test]
    fn oversized_integer_literal_clamps_instead_of_failing() {
        assert_eq!(
            token(b"99999999999999999999999"),
            Ok((&b""[..], Token::Integer(i64::MAX)))
        );
        assert_eq!(
            token(b"-99999999999999999999999"),
            Ok((&b""[..], Token::Integer(-i64::MAX)))
        );
    }

    #[test]
    fn test_parse_positive_integer() {
        let result = token(b"42");
        assert_eq!(result, Ok((&b""[..], Token::Integer(42))));
    }

    #[test]
    fn test_parse_negative_integer() {
        let result = token(b"-123");
        assert_eq!(result, Ok((&b""[..], Token::Integer(-123))));
    }

    #[test]
    fn test_parse_zero() {
        let result = token(b"0");
        assert_eq!(result, Ok((&b""[..], Token::Integer(0))));
    }

    #[test]
    #[allow(clippy::approx_constant)]
    fn test_parse_positive_real() {
        let result = token(b"3.14");
        assert_eq!(result, Ok((&b""[..], Token::Real(3.14))));
    }

    #[test]
    fn test_parse_negative_real() {
        let result = token(b"-2.5");
        assert_eq!(result, Ok((&b""[..], Token::Real(-2.5))));
    }

    #[test]
    fn test_parse_real_starting_with_dot() {
        let result = token(b".5");
        assert_eq!(result, Ok((&b""[..], Token::Real(0.5))));
    }

    #[test]
    fn test_parse_real_ending_with_dot() {
        let result = token(b"5.");
        assert_eq!(result, Ok((&b""[..], Token::Real(5.0))));
    }

    #[test]
    fn test_parse_negative_real_starting_with_dot() {
        let result = token(b"-.002");
        assert_eq!(result, Ok((&b""[..], Token::Real(-0.002))));
    }

    /// A real literal with enough digits to overflow `f64` (well past the
    /// PDF 32000-1 Annex C.2 implementation limit of ±3.403×10^38) must
    /// clamp to a finite value instead of becoming `Infinity` — an infinite
    /// coordinate can poison downstream arithmetic into NaN.
    #[test]
    fn test_parse_oversized_real_clamps_to_finite() {
        let huge = "9".repeat(400) + ".0";
        let (rest, tok) = token(huge.as_bytes()).unwrap();
        assert!(rest.is_empty());
        match tok {
            Token::Real(n) => assert!(n.is_finite(), "expected a finite clamp, got {n}"),
            other => panic!("expected Token::Real, got {other:?}"),
        }
    }

    #[test]
    fn test_parse_oversized_negative_real_clamps_to_finite_negative() {
        let huge = "-".to_string() + &"9".repeat(400) + ".0";
        let (rest, tok) = token(huge.as_bytes()).unwrap();
        assert!(rest.is_empty());
        match tok {
            Token::Real(n) => {
                assert!(n.is_finite() && n < 0.0, "expected a finite negative clamp, got {n}")
            }
            other => panic!("expected Token::Real, got {other:?}"),
        }
    }

    #[test]
    fn test_parse_literal_string() {
        let result = token(b"(Hello)");
        assert_eq!(result, Ok((&b""[..], Token::LiteralString(b"Hello"))));
    }

    #[test]
    fn test_parse_literal_string_with_spaces() {
        let result = token(b"(Hello World)");
        assert_eq!(result, Ok((&b""[..], Token::LiteralString(b"Hello World"))));
    }

    #[test]
    fn test_parse_literal_string_with_nested_parens() {
        let result = token(b"(Hello (nested) World)");
        assert_eq!(result, Ok((&b""[..], Token::LiteralString(b"Hello (nested) World"))));
    }

    #[test]
    fn test_parse_literal_string_with_escape() {
        let result = token(b"(Line1\\nLine2)");
        assert_eq!(result, Ok((&b""[..], Token::LiteralString(b"Line1\\nLine2"))));
    }

    #[test]
    fn test_parse_literal_string_with_escaped_paren() {
        let result = token(b"(Open \\( Close \\))");
        assert_eq!(result, Ok((&b""[..], Token::LiteralString(b"Open \\( Close \\)"))));
    }

    #[test]
    fn test_parse_empty_literal_string() {
        let result = token(b"()");
        assert_eq!(result, Ok((&b""[..], Token::LiteralString(b""))));
    }

    #[test]
    fn test_parse_hex_string() {
        let result = token(b"<48656C6C6F>");
        assert_eq!(result, Ok((&b""[..], Token::HexString(b"48656C6C6F"))));
    }

    #[test]
    fn test_parse_hex_string_with_whitespace() {
        let result = token(b"<48 65 6C 6C 6F>");
        assert_eq!(result, Ok((&b""[..], Token::HexString(b"48 65 6C 6C 6F"))));
    }

    #[test]
    fn test_parse_empty_hex_string() {
        let result = token(b"<>");
        assert_eq!(result, Ok((&b""[..], Token::HexString(b""))));
    }

    #[test]
    fn test_parse_name() {
        let result = token(b"/Type");
        assert_eq!(result, Ok((&b""[..], Token::Name("Type".to_string()))));
    }

    #[test]
    fn test_parse_name_with_special_chars() {
        let result = token(b"/A;Name_With-Various***Characters");
        assert_eq!(
            result,
            Ok((&b""[..], Token::Name("A;Name_With-Various***Characters".to_string())))
        );
    }

    #[test]
    fn test_parse_empty_name() {
        let result = token(b"/ ");
        assert_eq!(result, Ok((&b" "[..], Token::Name("".to_string()))));
    }

    #[test]
    fn test_parse_name_with_hex_escape() {
        let result = token(b"/A#20B");
        assert_eq!(result, Ok((&b""[..], Token::Name("A B".to_string()))));
    }

    #[test]
    fn test_parse_name_with_multiple_hex_escapes() {
        let result = token(b"/A#20B#23C");
        assert_eq!(result, Ok((&b""[..], Token::Name("A B#C".to_string()))));
    }

    #[test]
    fn test_parse_name_with_invalid_hex_escape() {
        let result = token(b"/A#ZZ");
        assert_eq!(result, Ok((&b""[..], Token::Name("A#ZZ".to_string()))));
    }

    #[test]
    fn test_decode_name_escapes_directly() {
        assert_eq!(decode_name_escapes("Type".as_bytes()), "Type");
        assert_eq!(decode_name_escapes("A#20B".as_bytes()), "A B");
        assert_eq!(decode_name_escapes("A#20B#23C".as_bytes()), "A B#C");
        assert_eq!(decode_name_escapes("A#".as_bytes()), "A#");
        assert_eq!(decode_name_escapes("A#2".as_bytes()), "A#2");
        assert_eq!(decode_name_escapes("A#ZZ".as_bytes()), "A#ZZ");
        assert_eq!(decode_name_escapes("A#ff#Fa".as_bytes()), "A\u{ff}\u{fa}");
        assert_eq!(decode_name_escapes("A#+F".as_bytes()), "A\u{f}");
        assert_eq!(decode_name_escapes("A#F+".as_bytes()), "A#F+");
        assert_eq!(decode_name_escapes("A#é1".as_bytes()), "A#é1");
    }

    #[test]
    fn escaped_and_raw_utf8_name_bytes_are_the_same_name() {
        let escaped = token(b"/F#C3#A9");
        let raw = token("/Fé".as_bytes());
        assert_eq!(escaped, Ok((&b""[..], Token::Name("Fé".to_string()))));
        assert_eq!(raw, escaped);
    }

    #[test]
    fn escaped_and_raw_non_utf8_name_bytes_are_the_same_name() {
        let escaped = token(b"/A#E9#82");
        let raw = token(b"/A\xE9\x82");
        assert_eq!(escaped, Ok((&b""[..], Token::Name("A\u{e9}\u{82}".to_string()))));
        assert_eq!(raw, escaped);
    }

    #[test]
    fn distinct_non_utf8_names_stay_distinct() {
        assert_ne!(token(b"/\xE9"), token(b"/\xE8"));
    }

    #[test]
    fn test_parse_true() {
        let result = token(b"true");
        assert_eq!(result, Ok((&b""[..], Token::True)));
    }

    #[test]
    fn test_parse_false() {
        let result = token(b"false");
        assert_eq!(result, Ok((&b""[..], Token::False)));
    }

    #[test]
    fn test_parse_null() {
        let result = token(b"null");
        assert_eq!(result, Ok((&b""[..], Token::Null)));
    }

    #[test]
    fn test_parse_array_start() {
        let result = token(b"[");
        assert_eq!(result, Ok((&b""[..], Token::ArrayStart)));
    }

    #[test]
    fn test_parse_array_end() {
        let result = token(b"]");
        assert_eq!(result, Ok((&b""[..], Token::ArrayEnd)));
    }

    #[test]
    fn test_parse_dict_start() {
        let result = token(b"<<");
        assert_eq!(result, Ok((&b""[..], Token::DictStart)));
    }

    #[test]
    fn test_parse_dict_end() {
        let result = token(b">>");
        assert_eq!(result, Ok((&b""[..], Token::DictEnd)));
    }

    #[test]
    fn test_parse_obj_start() {
        let result = token(b"obj");
        assert_eq!(result, Ok((&b""[..], Token::ObjStart)));
    }

    #[test]
    fn test_parse_obj_end() {
        let result = token(b"endobj");
        assert_eq!(result, Ok((&b""[..], Token::ObjEnd)));
    }

    #[test]
    fn test_parse_stream_start() {
        let result = token(b"stream");
        assert_eq!(result, Ok((&b""[..], Token::StreamStart)));
    }

    #[test]
    fn test_parse_stream_end() {
        let result = token(b"endstream");
        assert_eq!(result, Ok((&b""[..], Token::StreamEnd)));
    }

    #[test]
    fn test_parse_reference_marker() {
        let result = token(b"R");
        assert_eq!(result, Ok((&b""[..], Token::R)));
    }

    #[test]
    fn keyword_prefixes_preserve_adjacent_input() {
        for (input, rest, expected) in [
            (&b"falsehood"[..], &b"hood"[..], Token::False),
            (b"true/Name", b"/Name", Token::True),
            (b"null]", b"]", Token::Null),
            (b"obj<<", b"<<", Token::ObjStart),
            (b"endobj%", b"%", Token::ObjEnd),
            (b"stream\r\n", b"\r\n", Token::StreamStart),
            (b"endstreamendobj", b"endobj", Token::StreamEnd),
            (b"<< /Type", b" /Type", Token::DictStart),
            (b">>endobj", b"endobj", Token::DictEnd),
            (b"[1", b"1", Token::ArrayStart),
            (b"]>>", b">>", Token::ArrayEnd),
            (b"R2", b"2", Token::R),
        ] {
            assert_eq!(token(input), Ok((rest, expected.clone())));
            assert_eq!(token_lenient(input), Ok((rest, expected)));
        }
    }

    #[test]
    fn incomplete_keywords_and_operator_names_keep_lenient_fallback() {
        for input in [b"RG".as_slice(), b"RI", b"Re", b"endstr", b"fal", b"n"] {
            assert!(token(input).is_err(), "{input:?}");
            assert_eq!(
                token_lenient(input),
                Ok((&b""[..], Token::Name(std::str::from_utf8(input).unwrap().to_owned())))
            );
        }
    }

    #[test]
    fn test_skip_leading_whitespace() {
        let result = token(b"  \n\t42");
        assert_eq!(result, Ok((&b""[..], Token::Integer(42))));
    }

    #[test]
    fn test_skip_comment() {
        let result = token(b"% This is a comment\n42");
        assert_eq!(result, Ok((&b""[..], Token::Integer(42))));
    }

    #[test]
    fn test_skip_multiple_comments() {
        let result = token(b"% Comment 1\n% Comment 2\n42");
        assert_eq!(result, Ok((&b""[..], Token::Integer(42))));
    }

    #[test]
    fn test_skip_mixed_whitespace_and_comments() {
        let result = token(b"  % Comment\n  \t% Another\n  42");
        assert_eq!(result, Ok((&b""[..], Token::Integer(42))));
    }

    #[test]
    fn test_multiple_tokens() {
        let input = b"42 /Type (Hello) true";
        let (input, tok1) = token(input).unwrap();
        assert_eq!(tok1, Token::Integer(42));

        let (input, tok2) = token(input).unwrap();
        assert_eq!(tok2, Token::Name("Type".to_string()));

        let (input, tok3) = token(input).unwrap();
        assert_eq!(tok3, Token::LiteralString(b"Hello"));

        let (input, tok4) = token(input).unwrap();
        assert_eq!(tok4, Token::True);
        assert_eq!(input, &b""[..]);
    }

    #[test]
    fn test_dict_vs_hex_string() {
        let result = token(b"<<");
        assert_eq!(result, Ok((&b""[..], Token::DictStart)));

        let result = token(b"<ABC>");
        assert_eq!(result, Ok((&b""[..], Token::HexString(b"ABC"))));
    }

    #[test]
    fn test_complex_pdf_snippet() {
        let input = b"1 0 obj\n<< /Type /Catalog /Pages 2 0 R >>\nendobj";
        let (input, tok1) = token(input).unwrap();
        assert_eq!(tok1, Token::Integer(1));

        let (input, tok2) = token(input).unwrap();
        assert_eq!(tok2, Token::Integer(0));

        let (input, tok3) = token(input).unwrap();
        assert_eq!(tok3, Token::ObjStart);

        let (input, tok4) = token(input).unwrap();
        assert_eq!(tok4, Token::DictStart);

        let (input, tok5) = token(input).unwrap();
        assert_eq!(tok5, Token::Name("Type".to_string()));

        let (input, tok6) = token(input).unwrap();
        assert_eq!(tok6, Token::Name("Catalog".to_string()));

        let (input, tok7) = token(input).unwrap();
        assert_eq!(tok7, Token::Name("Pages".to_string()));

        let (input, tok8) = token(input).unwrap();
        assert_eq!(tok8, Token::Integer(2));

        let (input, tok9) = token(input).unwrap();
        assert_eq!(tok9, Token::Integer(0));

        let (input, tok10) = token(input).unwrap();
        assert_eq!(tok10, Token::R);

        let (input, tok11) = token(input).unwrap();
        assert_eq!(tok11, Token::DictEnd);

        let (input, tok12) = token(input).unwrap();
        assert_eq!(tok12, Token::ObjEnd);

        assert_eq!(input, &b""[..]);
    }

    #[test]
    fn test_real_vs_integer_distinction() {
        assert!(matches!(token(b"0").unwrap().1, Token::Integer(0)));
        assert!(matches!(token(b"42").unwrap().1, Token::Integer(42)));
        assert!(matches!(token(b"-123").unwrap().1, Token::Integer(-123)));

        assert!(matches!(token(b"0.0").unwrap().1, Token::Real(_)));
        assert!(matches!(token(b"3.14").unwrap().1, Token::Real(_)));
        assert!(matches!(token(b".5").unwrap().1, Token::Real(_)));
        assert!(matches!(token(b"5.").unwrap().1, Token::Real(_)));
    }
}
