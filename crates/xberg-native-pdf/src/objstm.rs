//! Object stream parsing (PDF 1.5+).
//!
//! Object streams (/Type /ObjStm) allow multiple objects to be compressed together
//! in a single stream for better compression ratios. This module handles parsing
//! these streams and extracting individual objects.
//!
//! # Format
//!
//! An object stream has this structure:
//! ```text
//! N 0 obj
//! << /Type /ObjStm
//!    /N 5              % Number of objects in stream
//!    /First 30         % Byte offset to first object's data
//!    /Filter /FlateDecode
//! >>
//! stream
//! 10 0 11 15 12 28 13 42 14 55    % Pairs: (obj_num, offset)
//! <dict>                           % Object 10 at offset 0
//! <array>                          % Object 11 at offset 15
//! ...
//! endstream
//! endobj
//! ```
//!
//! The first part contains N pairs of integers (object number, byte offset relative
//! to /First). The second part contains the actual object data.

use crate::error::{Error, Result};
use crate::object::Object;
use crate::parser::parse_object;
use std::collections::HashMap;

const MAX_OBJECT_STREAM_OBJECTS: i64 = 1_000_000;
const MAX_OBJECT_STREAM_FIRST_OFFSET: i64 = 10_000_000;

pub(crate) struct ObjectStreamParseOutcome {
    pub(crate) objects: HashMap<u32, Object>,
    recovery: Option<ObjectStreamRecovery>,
}

struct ObjectStreamRecovery {
    invalid_offset_count: usize,
    parse_failure_count: usize,
}

impl ObjectStreamParseOutcome {
    pub(crate) fn has_recovery(&self) -> bool {
        self.recovery.is_some()
    }

    pub(crate) fn trace_recovery(&self) {
        let Some(recovery) = &self.recovery else {
            return;
        };
        let skipped_count = recovery.invalid_offset_count + recovery.parse_failure_count;
        tracing::warn!(
            target: crate::LOG_TARGET_ROOT,
            operation = "parse_object_stream",
            error_code = "invalid_embedded_object",
            skipped_count,
            parse_failure_count = recovery.parse_failure_count,
            invalid_offset_count = recovery.invalid_offset_count,
            "object stream entries were skipped"
        );
    }
}

/// Parse an object stream and extract all objects.
///
/// This is a convenience method that calls `parse_object_stream_with_decryption`
/// with no encryption parameters.
///
/// # Arguments
///
/// * `stream_obj` - The object stream object (must be a Stream with /Type /ObjStm)
///
/// # Returns
///
/// A HashMap mapping object numbers to their parsed objects.
///
/// # Example
///
/// ```ignore
/// use xberg_native_pdf::objstm::parse_object_stream;
/// use xberg_native_pdf::object::Object;
///
/// // Assuming we have loaded an object stream
/// let objects = parse_object_stream(&stream_obj)?;
/// let obj_10 = objects.get(&10).unwrap();
/// # Ok::<(), xberg_native_pdf::error::Error>(())
/// ```
pub fn parse_object_stream(stream_obj: &Object) -> Result<HashMap<u32, Object>> {
    parse_object_stream_with_decryption(stream_obj, None, 0, 0)
}

/// Parse an object stream with optional decryption.
///
/// PDF Spec: Object streams (PDF 1.5+) can be encrypted like any other stream.
/// The encryption must be applied before decompression.
///
/// # Arguments
///
/// * `stream_obj` - The object stream object (must be a Stream with /Type /ObjStm)
/// * `decryption_fn` - Optional decryption function (from EncryptionHandler)
/// * `obj_num` - Object number (for encryption key derivation)
/// * `gen_num` - Generation number (for encryption key derivation)
///
/// # Returns
///
/// A HashMap mapping object numbers to their parsed objects.
///
/// # Errors
///
/// Returns an error if:
/// - The object is not a stream
/// - The stream is not a valid object stream (/Type /ObjStm)
/// - Required dictionary entries (/N, /First) are missing
/// - Stream decoding/decryption fails
/// - Object parsing fails
pub fn parse_object_stream_with_decryption(
    stream_obj: &Object,
    decryption_fn: Option<&dyn Fn(&[u8]) -> Result<Vec<u8>>>,
    obj_num: u32,
    gen_num: u32,
) -> Result<HashMap<u32, Object>> {
    let outcome = parse_object_stream_with_decryption_outcome(stream_obj, decryption_fn, obj_num, gen_num)?;
    outcome.trace_recovery();
    Ok(outcome.objects)
}

pub(crate) fn parse_object_stream_with_decryption_outcome(
    stream_obj: &Object,
    decryption_fn: Option<&dyn Fn(&[u8]) -> Result<Vec<u8>>>,
    obj_num: u32,
    gen_num: u32,
) -> Result<ObjectStreamParseOutcome> {
    let dict = match stream_obj {
        Object::Stream { dict, .. } => dict,
        _ => return Err(Error::InvalidPdf("object stream is not a Stream object".to_string())),
    };

    if let Some(type_obj) = dict.get("Type")
        && let Some(type_name) = type_obj.as_name()
        && type_name != "ObjStm"
    {
        return Err(Error::InvalidPdf(format!(
            "expected /Type /ObjStm, got /Type /{}",
            type_name
        )));
    }

    let n = dict
        .get("N")
        .and_then(|o| o.as_integer())
        .ok_or_else(|| Error::InvalidPdf("object stream missing /N entry".to_string()))?;

    let first = dict
        .get("First")
        .and_then(|o| o.as_integer())
        .ok_or_else(|| Error::InvalidPdf("object stream missing /First entry".to_string()))?;

    if !(0..=MAX_OBJECT_STREAM_OBJECTS).contains(&n) {
        return Err(Error::InvalidPdf(format!("invalid object stream /N value: {}", n)));
    }

    if !(0..=MAX_OBJECT_STREAM_FIRST_OFFSET).contains(&first) {
        return Err(Error::InvalidPdf(format!(
            "invalid object stream /First value: {}",
            first
        )));
    }

    let n = n as usize;
    let first = first as usize;

    let decoded_data = stream_obj.decode_stream_data_with_decryption(decryption_fn, obj_num, gen_num)?;

    if decoded_data.len() < first {
        return Err(Error::InvalidPdf(format!(
            "object stream data too short: {} bytes, expected at least {}",
            decoded_data.len(),
            first
        )));
    }

    let pairs_data = &decoded_data[..first];
    let pairs = parse_object_number_pairs(pairs_data, n)?;

    Ok(parse_embedded_objects(&decoded_data[first..], pairs))
}

fn parse_embedded_objects(objects_data: &[u8], pairs: Vec<(u32, usize)>) -> ObjectStreamParseOutcome {
    let mut result = HashMap::new();
    let mut invalid_offset_count = 0usize;
    let mut parse_failure_count = 0usize;

    for (obj_num, offset_in_data) in pairs {
        // The offset is relative to the start of objects_data ~keep
        if offset_in_data >= objects_data.len() {
            invalid_offset_count += 1;
            continue;
        }

        let obj_data = &objects_data[offset_in_data..];
        match parse_object(obj_data) {
            Ok((_remaining, obj)) => {
                result.insert(obj_num, obj);
            }
            Err(_) => {
                parse_failure_count += 1;
                // Continue parsing other objects even if one fails ~keep
                continue;
            }
        }
    }

    let recovery = (invalid_offset_count + parse_failure_count > 0).then_some(ObjectStreamRecovery {
        invalid_offset_count,
        parse_failure_count,
    });
    ObjectStreamParseOutcome {
        objects: result,
        recovery,
    }
}

/// Parse the pairs section of an object stream.
///
/// The pairs section contains N pairs of integers: (object_number, offset).
/// The offset is relative to the start of the objects data section.
///
/// # Arguments
///
/// * `data` - The pairs section data (before /First offset)
/// * `count` - Expected number of pairs (from /N)
///
/// # Returns
///
/// A vector of (object_number, offset) tuples.
fn parse_object_number_pairs(data: &[u8], count: usize) -> Result<Vec<(u32, usize)>> {
    let mut pairs = Vec::with_capacity(count);
    let mut remaining = data;

    for i in 0..count {
        remaining = skip_whitespace(remaining);

        let (rest, obj_num_str) = read_integer_string(remaining).ok_or_else(|| Error::ParseError {
            offset: 0,
            reason: format!("failed to parse object number for pair {}", i),
        })?;

        let obj_num: u32 = obj_num_str.parse().map_err(|_| Error::ParseError {
            offset: 0,
            reason: format!("invalid object number: {}", obj_num_str),
        })?;

        remaining = skip_whitespace(rest);

        let (rest, offset_str) = read_integer_string(remaining).ok_or_else(|| Error::ParseError {
            offset: 0,
            reason: format!("failed to parse offset for pair {}", i),
        })?;

        let offset: usize = offset_str.parse().map_err(|_| Error::ParseError {
            offset: 0,
            reason: format!("invalid offset: {}", offset_str),
        })?;

        pairs.push((obj_num, offset));
        remaining = rest;
    }

    Ok(pairs)
}

/// Skip PDF whitespace characters.
///
/// PDF whitespace: null (0), tab (9), LF (10), FF (12), CR (13), space (32)
fn skip_whitespace(data: &[u8]) -> &[u8] {
    let mut i = 0;
    while i < data.len() {
        match data[i] {
            0 | 9 | 10 | 12 | 13 | 32 => i += 1,
            _ => break,
        }
    }
    &data[i..]
}

/// Read an integer string from the input.
///
/// Reads consecutive digit characters (with optional leading sign).
/// Returns the remaining input and the integer string.
fn read_integer_string(data: &[u8]) -> Option<(&[u8], &str)> {
    if data.is_empty() {
        return None;
    }

    let mut i = 0;

    if data[i] == b'+' || data[i] == b'-' {
        i += 1;
    }

    let start = i;
    while i < data.len() && data[i].is_ascii_digit() {
        i += 1;
    }

    if i == start {
        return None;
    }

    let int_str = std::str::from_utf8(&data[..i]).ok()?;
    Some((&data[i..], int_str))
}

#[cfg(test)]
mod tests {
    use super::*;
    use bytes::Bytes;
    use std::collections::{BTreeMap, HashMap};
    use std::sync::{Arc, Mutex};
    use tracing_subscriber::Layer;
    use tracing_subscriber::layer::SubscriberExt as _;

    #[derive(Clone, Debug)]
    struct CapturedEvent {
        level: tracing::Level,
        target: String,
        fields: BTreeMap<String, String>,
    }

    #[derive(Clone, Default)]
    struct EventCapture(Arc<Mutex<Vec<CapturedEvent>>>);

    impl<S> Layer<S> for EventCapture
    where
        S: tracing::Subscriber,
    {
        fn on_event(&self, event: &tracing::Event<'_>, _context: tracing_subscriber::layer::Context<'_, S>) {
            let mut visitor = FieldCapture::default();
            event.record(&mut visitor);
            self.0.lock().unwrap().push(CapturedEvent {
                level: *event.metadata().level(),
                target: event.metadata().target().to_string(),
                fields: visitor.0,
            });
        }
    }

    #[derive(Default)]
    struct FieldCapture(BTreeMap<String, String>);

    impl tracing::field::Visit for FieldCapture {
        fn record_debug(&mut self, field: &tracing::field::Field, value: &dyn std::fmt::Debug) {
            self.0.insert(field.name().to_string(), format!("{value:?}"));
        }

        fn record_u64(&mut self, field: &tracing::field::Field, value: u64) {
            self.0.insert(field.name().to_string(), value.to_string());
        }

        fn record_str(&mut self, field: &tracing::field::Field, value: &str) {
            self.0.insert(field.name().to_string(), value.to_string());
        }
    }

    fn capture_events<T>(operation: impl FnOnce() -> T) -> (T, Vec<CapturedEvent>) {
        let capture = EventCapture::default();
        let subscriber = tracing_subscriber::registry().with(capture.clone());
        let result = tracing::subscriber::with_default(subscriber, operation);
        let events = capture.0.lock().unwrap().clone();
        (result, events)
    }

    #[test]
    fn test_skip_whitespace() {
        assert_eq!(skip_whitespace(b"   hello"), b"hello");
        assert_eq!(skip_whitespace(b"\t\n\r hello"), b"hello");
        assert_eq!(skip_whitespace(b"hello"), b"hello");
        assert_eq!(skip_whitespace(b""), b"");
    }

    #[test]
    fn test_read_integer_string() {
        assert_eq!(read_integer_string(b"123 rest"), Some((&b" rest"[..], "123")));
        assert_eq!(read_integer_string(b"-456 rest"), Some((&b" rest"[..], "-456")));
        assert_eq!(read_integer_string(b"+789"), Some((&b""[..], "+789")));
        assert_eq!(read_integer_string(b"notanumber"), None);
        assert_eq!(read_integer_string(b""), None);
    }

    #[test]
    fn object_stream_header_preserves_signed_tokens_and_pdf_whitespace() {
        assert_eq!(
            parse_object_number_pairs(b"+17\x00+42\t0023\x0c000", 2).unwrap(),
            [(17, 42), (23, 0)]
        );
        for data in [b"-17 42".as_slice(), b"17 -42", b"4294967296 0", b"17 +"] {
            assert!(parse_object_number_pairs(data, 1).is_err());
        }
    }

    #[test]
    fn test_parse_object_number_pairs() {
        let data = b"10 0 11 15 12 28";
        let pairs = parse_object_number_pairs(data, 3).unwrap();

        assert_eq!(pairs.len(), 3);
        assert_eq!(pairs[0], (10, 0));
        assert_eq!(pairs[1], (11, 15));
        assert_eq!(pairs[2], (12, 28));
    }

    #[test]
    fn test_parse_object_number_pairs_with_whitespace() {
        let data = b"  10   0   11  15  12   28  ";
        let pairs = parse_object_number_pairs(data, 3).unwrap();

        assert_eq!(pairs.len(), 3);
        assert_eq!(pairs[0], (10, 0));
        assert_eq!(pairs[1], (11, 15));
        assert_eq!(pairs[2], (12, 28));
    }

    #[test]
    fn test_parse_object_stream_basic() {
        let pairs_data = b"10 0 11 3";
        let objects_data = b"42 /Test";

        let mut combined = Vec::new();
        combined.extend_from_slice(pairs_data);
        combined.push(b' ');
        combined.extend_from_slice(objects_data);

        let mut dict = HashMap::new();
        dict.insert("Type".to_string(), Object::Name("ObjStm".to_string()));
        dict.insert("N".to_string(), Object::Integer(2));
        dict.insert("First".to_string(), Object::Integer(9));
        dict.insert("Length".to_string(), Object::Integer(combined.len() as i64));

        let stream = Object::Stream {
            dict,
            data: Bytes::from(combined),
        };

        let objects = parse_object_stream(&stream).unwrap();
        assert_eq!(objects.len(), 2);
        assert_eq!(objects.get(&10).unwrap().as_integer(), Some(42));
        assert_eq!(objects.get(&11).unwrap().as_name(), Some("Test"));
    }

    #[test]
    fn test_parse_object_stream_not_stream() {
        let obj = Object::Integer(42);
        let result = parse_object_stream(&obj);
        assert!(result.is_err());
    }

    #[test]
    fn test_parse_object_stream_missing_type() {
        let mut dict = HashMap::new();
        dict.insert("N".to_string(), Object::Integer(1));
        dict.insert("First".to_string(), Object::Integer(5));

        let stream = Object::Stream {
            dict,
            data: Bytes::from(b"1 0 42".to_vec()),
        };

        let result = parse_object_stream(&stream);
        assert!(result.is_ok());
    }

    #[test]
    fn test_parse_object_stream_missing_n() {
        let mut dict = HashMap::new();
        dict.insert("Type".to_string(), Object::Name("ObjStm".to_string()));
        dict.insert("First".to_string(), Object::Integer(5));

        let stream = Object::Stream {
            dict,
            data: Bytes::from(b"1 0 42".to_vec()),
        };

        let result = parse_object_stream(&stream);
        assert!(result.is_err());
    }

    #[test]
    fn test_parse_object_stream_missing_first() {
        let mut dict = HashMap::new();
        dict.insert("Type".to_string(), Object::Name("ObjStm".to_string()));
        dict.insert("N".to_string(), Object::Integer(1));

        let stream = Object::Stream {
            dict,
            data: Bytes::from(b"1 0 42".to_vec()),
        };

        let result = parse_object_stream(&stream);
        assert!(result.is_err());
    }

    #[test]
    fn test_parse_object_stream_invalid_n() {
        let mut dict = HashMap::new();
        dict.insert("Type".to_string(), Object::Name("ObjStm".to_string()));
        dict.insert("N".to_string(), Object::Integer(-1));
        dict.insert("First".to_string(), Object::Integer(5));

        let stream = Object::Stream {
            dict,
            data: Bytes::from(b"1 0 42".to_vec()),
        };

        let result = parse_object_stream(&stream);
        assert!(result.is_err());
    }

    #[test]
    fn test_parse_object_stream_data_too_short() {
        let mut dict = HashMap::new();
        dict.insert("Type".to_string(), Object::Name("ObjStm".to_string()));
        dict.insert("N".to_string(), Object::Integer(1));
        dict.insert("First".to_string(), Object::Integer(100));

        let stream = Object::Stream {
            dict,
            data: Bytes::from(b"1 0 42".to_vec()),
        };

        let result = parse_object_stream(&stream);
        assert!(result.is_err());
    }

    #[test]
    fn malformed_entries_emit_one_bounded_summary_without_parser_input() {
        const CONFIDENTIAL_MARKER: &str = "CONFIDENTIAL_OBJSTM_PAYLOAD_944d";
        let pairs_data = b"10 0 11 1 12 2 13 999";
        let mut combined = pairs_data.to_vec();
        combined.push(b' ');
        combined.extend_from_slice(CONFIDENTIAL_MARKER.as_bytes());

        let mut dict = HashMap::new();
        dict.insert("Type".to_string(), Object::Name("ObjStm".to_string()));
        dict.insert("N".to_string(), Object::Integer(4));
        dict.insert("First".to_string(), Object::Integer((pairs_data.len() + 1) as i64));
        let stream = Object::Stream {
            dict,
            data: Bytes::from(combined),
        };

        let (result, events) = capture_events(|| parse_object_stream(&stream));

        assert!(result.is_ok());
        assert_eq!(
            events.len(),
            1,
            "one object stream must emit at most one recovery event"
        );
        assert_eq!(events[0].level, tracing::Level::WARN);
        assert_eq!(events[0].target, crate::LOG_TARGET_ROOT);
        assert_eq!(
            events[0].fields.get("operation").map(String::as_str),
            Some("parse_object_stream")
        );
        assert_eq!(
            events[0].fields.get("error_code").map(String::as_str),
            Some("invalid_embedded_object")
        );
        assert_eq!(events[0].fields.get("skipped_count").map(String::as_str), Some("4"));
        assert_eq!(
            events[0].fields.get("parse_failure_count").map(String::as_str),
            Some("3")
        );
        assert_eq!(
            events[0].fields.get("invalid_offset_count").map(String::as_str),
            Some("1")
        );
        assert!(
            !format!("{events:?}").contains(CONFIDENTIAL_MARKER),
            "object-stream telemetry exposed parser input: {events:?}"
        );
    }
}
