use super::super::*;
use super::common::*;
use std::io::Cursor;
use tracing::Level;

#[test]
fn test_parse_valid_header_1_7() {
    let mut cursor = Cursor::new(b"%PDF-1.7\n");
    let (major, minor, offset) = parse_header(&mut cursor, false).unwrap();
    assert_eq!((major, minor, offset), (1, 7, 0));
}

#[test]
fn test_parse_valid_header_1_4() {
    let mut cursor = Cursor::new(b"%PDF-1.4");
    let (major, minor, offset) = parse_header(&mut cursor, false).unwrap();
    assert_eq!((major, minor, offset), (1, 4, 0));
}

#[test]
fn test_parse_valid_header_1_0() {
    let mut cursor = Cursor::new(b"%PDF-1.0");
    let (major, minor, offset) = parse_header(&mut cursor, false).unwrap();
    assert_eq!((major, minor, offset), (1, 0, 0));
}

#[test]
fn test_parse_valid_header_2_0() {
    let mut cursor = Cursor::new(b"%PDF-2.0");
    let (major, minor, offset) = parse_header(&mut cursor, false).unwrap();
    assert_eq!((major, minor, offset), (2, 0, 0));
}

#[test]
fn test_parse_invalid_header_wrong_magic_strict() {
    let mut cursor = Cursor::new(b"NotAPDF\n");
    let result = parse_header(&mut cursor, false);
    assert!(result.is_err());
    assert!(matches!(result.unwrap_err(), Error::InvalidHeader(_)));
}

#[test]
fn test_parse_invalid_header_unsupported_version() {
    let mut cursor = Cursor::new(b"%PDF-3.0");
    let result = parse_header(&mut cursor, false);
    assert!(result.is_err());
    assert!(matches!(result.unwrap_err(), Error::UnsupportedVersion(_)));
}

#[test]
fn test_parse_invalid_header_version_0_0() {
    let mut cursor = Cursor::new(b"%PDF-0.0");
    let result = parse_header(&mut cursor, false);
    assert!(result.is_err());
}

#[test]
fn test_parse_invalid_header_no_dot() {
    let mut cursor = Cursor::new(b"%PDF-17\n");
    let result = parse_header(&mut cursor, false);
    assert!(result.is_err());
    assert!(matches!(result.unwrap_err(), Error::InvalidHeader(_)));
}

#[test]
fn test_parse_invalid_header_too_short() {
    let mut cursor = Cursor::new(b"%PDF");
    let result = parse_header(&mut cursor, false);
    assert!(result.is_err());
}

#[test]
fn test_parse_invalid_header_non_digit_version() {
    let mut cursor = Cursor::new(b"%PDF-X.Y");
    let result = parse_header(&mut cursor, false);
    assert!(result.is_err());
    assert!(matches!(result.unwrap_err(), Error::InvalidHeader(_)));
}

#[test]
fn test_parse_header_with_bom_prefix() {
    let data = b"\xEF\xBB\xBF%PDF-1.7\n";
    let mut cursor = Cursor::new(data);
    let (major, minor, offset) = parse_header(&mut cursor, true).unwrap();
    assert_eq!((major, minor, offset), (1, 7, 3));
}

#[test]
fn test_parse_header_with_binary_prefix() {
    let mut data = vec![0x1b, 0x96, 0x5f];
    data.extend_from_slice(b"%PDF-1.4\n");
    let mut cursor = Cursor::new(data);
    let (major, minor, offset) = parse_header(&mut cursor, true).unwrap();
    assert_eq!((major, minor, offset), (1, 4, 3));
}

#[test]
fn test_parse_header_at_boundary() {
    // Header starting at byte 1016 (within 1024-byte window, with 8 bytes for full header)
    // ~keep
    let mut data = vec![0u8; 1016];
    data.extend_from_slice(b"%PDF-1.5");
    let mut cursor = Cursor::new(data);
    let (major, minor, offset) = parse_header(&mut cursor, true).unwrap();
    assert_eq!((major, minor, offset), (1, 5, 1016));
}

#[test]
fn test_parse_header_not_found_lenient() {
    let data = vec![0u8; 1024];
    let mut cursor = Cursor::new(data);
    let (major, minor, offset) = parse_header(&mut cursor, true).unwrap();
    assert_eq!((major, minor), (1, 4));
    assert_eq!(offset, 0);
}

#[test]
fn test_parse_header_strict_rejects_offset() {
    let mut data = vec![0x1b, 0x96, 0x5f];
    data.extend_from_slice(b"%PDF-1.4\n");
    let mut cursor = Cursor::new(data);
    let result = parse_header(&mut cursor, false);
    assert!(result.is_err());
    assert!(matches!(result.unwrap_err(), Error::InvalidHeader(_)));
}

#[test]
fn test_parse_trailer_basic() {
    let data = b"trailer\n<< /Size 6 /Root 1 0 R >>\nstartxref\n";
    let mut cursor = Cursor::new(data);
    let trailer = parse_trailer(&mut cursor).unwrap();

    let dict = trailer.as_dict().unwrap();
    assert_eq!(dict.get("Size").unwrap().as_integer(), Some(6));
    assert!(dict.get("Root").unwrap().as_reference().is_some());
}

#[test]
fn test_parse_trailer_missing_keyword() {
    let data = b"<< /Size 6 >>\nstartxref\n";
    let mut cursor = Cursor::new(data);
    let result = parse_trailer(&mut cursor);
    assert!(result.is_err());
}

#[test]
fn test_parse_trailer_not_dictionary() {
    let data = b"trailer\n[ 1 2 3 ]\nstartxref\n";
    let mut cursor = Cursor::new(data);
    let result = parse_trailer(&mut cursor);
    assert!(result.is_err());
}

#[test]
fn test_circular_reference_detection() {
    // This test ensures that the cycle detection mechanism works
    // We can't easily create a circular PDF in a unit test, but we can
    // verify that the error types exist and are properly defined ~keep
    use crate::object::ObjectRef;

    let obj_ref = ObjectRef::new(1, 0);
    let err = Error::CircularReference(obj_ref);
    let msg = format!("{}", err);
    assert!(msg.contains("Circular reference"));
    assert!(msg.contains("object 1 0 R"));
}

#[test]
fn test_recursion_limit_error() {
    let err = Error::RecursionLimitExceeded(100);
    let msg = format!("{}", err);
    assert!(msg.contains("Recursion depth limit exceeded"));
    assert!(msg.contains("100"));
}

#[test]
fn test_from_bytes_invalid_data() {
    let result = PdfDocument::from_bytes(b"not a pdf".to_vec());

    // `parse_header` (lenient mode) never fails on missing `%PDF-` -- it defaults to
    // version 1.4 (line ~19772) -- so the actual rejection happens one step later:
    // `find_xref_offset` (crates/xberg-native-pdf/src/xref.rs) finds no `startxref`
    // keyword anywhere in the 9-byte input and returns `Error::InvalidXref`.
    // `open_from_reader` then tries `reconstruct_xref` as a fallback, which also
    // fails (`RE_OBJ_PATTERN` matches no `N G obj` header at all), so it re-returns
    // the *original* `InvalidXref` error rather than the reconstruction one.
    let error = result.expect_err("9 bytes with no PDF structure at all must be rejected");
    assert!(
        matches!(error, Error::InvalidXref),
        "expected Error::InvalidXref, got: {error:?}"
    );
}

#[test]
fn test_find_substring_found() {
    assert_eq!(find_substring(b"Hello World", b"World"), Some(6));
}

#[test]
fn test_find_substring_not_found() {
    assert_eq!(find_substring(b"Hello World", b"xyz"), None);
}

#[test]
fn test_find_substring_empty_needle() {
    assert_eq!(find_substring(b"Hello", b""), Some(0));
}

#[test]
fn test_find_substring_at_start() {
    assert_eq!(find_substring(b"Hello", b"Hello"), Some(0));
}

#[test]
fn test_find_substring_at_end() {
    assert_eq!(find_substring(b"Hello", b"lo"), Some(3));
}

#[test]
fn test_find_substring_empty_haystack() {
    assert_eq!(find_substring(b"", b"Hello"), None);
}

#[test]
fn test_parse_version_from_header_strict_valid() {
    let header = *b"%PDF-1.7";
    let (major, minor) = parse_version_from_header(&header, false).unwrap();
    assert_eq!((major, minor), (1, 7));
}

#[test]
fn test_parse_version_from_header_strict_invalid_dot() {
    let header = *b"%PDF-1X7";
    let result = parse_version_from_header(&header, false);
    assert!(result.is_err());
}

#[test]
fn test_parse_version_from_header_lenient_invalid_dot() {
    let header = *b"%PDF-1X7";
    let (major, minor) = parse_version_from_header(&header, true).unwrap();
    assert_eq!((major, minor), (1, 4));
}

#[test]
fn lenient_version_warning_has_stable_identity() {
    let header = *b"%PDF-1X7";
    let (result, events) = capture_events(|| parse_version_from_header(&header, true));

    assert_eq!(result.unwrap(), (1, 4));
    let warnings: Vec<_> = events.iter().filter(|event| event.level == Level::WARN).collect();
    assert_eq!(warnings.len(), 1, "expected exactly one recovery warning: {events:#?}");
    assert_eq!(warnings[0].target, format!("{}::document", crate::LOG_TARGET_ROOT));
    assert_eq!(
        warnings[0].fields.get("operation").map(String::as_str),
        Some("parse_pdf_version")
    );
    assert_eq!(
        warnings[0].fields.get("reason").map(String::as_str),
        Some("invalid_version_separator")
    );
}

#[test]
fn test_parse_version_from_header_strict_non_digit() {
    let header = *b"%PDF-X.Y";
    let result = parse_version_from_header(&header, false);
    assert!(result.is_err());
}

#[test]
fn test_parse_version_from_header_lenient_non_digit() {
    let header = *b"%PDF-X.Y";
    let (major, minor) = parse_version_from_header(&header, true).unwrap();
    assert_eq!((major, minor), (1, 4));
}

#[test]
fn test_parse_version_from_header_strict_too_high() {
    let header = *b"%PDF-3.0";
    let result = parse_version_from_header(&header, false);
    assert!(result.is_err());
}

#[test]
fn test_parse_version_from_header_lenient_too_high() {
    let header = *b"%PDF-3.0";
    let (major, minor) = parse_version_from_header(&header, true).unwrap();
    assert_eq!((major, minor), (1, 4));
}

#[test]
fn test_parse_version_from_header_wrong_magic() {
    let header = *b"NotPDF17";
    let result = parse_version_from_header(&header, false);
    assert!(result.is_err());
}

#[test]
fn test_parse_header_empty_file_strict() {
    let mut cursor = Cursor::new(b"");
    let result = parse_header(&mut cursor, false);
    assert!(result.is_err());
}

#[test]
fn test_parse_header_empty_file_lenient() {
    let mut cursor = Cursor::new(b"");
    let result = parse_header(&mut cursor, true);
    assert!(result.is_err());
}

#[test]
fn test_parse_header_very_short_lenient() {
    let mut cursor = Cursor::new(b"AB");
    let result = parse_header(&mut cursor, true);
    let (major, minor, _) = result.unwrap();
    assert_eq!((major, minor), (1, 4));
}

#[test]
fn test_parse_header_header_near_end_of_buffer() {
    // Header at position 8100 (within 8192 byte search window) ~keep
    let mut data = vec![0u8; 8100];
    data.extend_from_slice(b"%PDF-1.6");
    data.extend_from_slice(b"\nrest of file data here");
    let mut cursor = Cursor::new(data);
    let (major, minor, offset) = parse_header(&mut cursor, true).unwrap();
    assert_eq!((major, minor, offset), (1, 6, 8100));
}

#[test]
fn test_parse_trailer_with_extra_data() {
    let data = b"some xref data\ntrailer\n<< /Size 10 /Root 1 0 R /Info 2 0 R >>\nstartxref\n100\n";
    let mut cursor = Cursor::new(data);
    let trailer = parse_trailer(&mut cursor).unwrap();
    let dict = trailer.as_dict().unwrap();
    assert_eq!(dict.get("Size").unwrap().as_integer(), Some(10));
}

#[test]
fn test_parse_trailer_empty_after_keyword() {
    let data = b"trailer";
    let mut cursor = Cursor::new(data);
    let result = parse_trailer(&mut cursor);
    assert!(result.is_err());
}

#[test]
fn encryption_recovery_warning_does_not_expose_parser_error_text() {
    const CONFIDENTIAL_MARKER: &str = "CONFIDENTIAL_ENCRYPTION_ENTRY_43fb0d";
    let error = Error::InvalidPdf(CONFIDENTIAL_MARKER.repeat(4096));

    let (_, events) = capture_events(|| trace_recoverable_pdf_error("initialize_encryption", &error));

    let warnings: Vec<_> = events.iter().filter(|event| event.level == Level::WARN).collect();
    assert_eq!(
        warnings.len(),
        1,
        "expected exactly one encryption warning: {events:#?}"
    );
    assert_eq!(warnings[0].target, crate::LOG_TARGET_ROOT);
    assert_eq!(
        warnings[0].fields.get("operation").map(String::as_str),
        Some("initialize_encryption")
    );
    assert_eq!(
        warnings[0].fields.get("error_code").map(String::as_str),
        Some("invalid_pdf")
    );
    assert!(
        !format!("{events:?}").contains(CONFIDENTIAL_MARKER),
        "encryption telemetry exposed attacker-controlled error text: {events:#?}"
    );
}

#[test]
fn unresolved_encryption_references_emit_one_counted_warning() {
    let encryption_dictionary = HashMap::from([
        ("V".to_string(), Object::Reference(ObjectRef::new(900, 0))),
        ("R".to_string(), Object::Reference(ObjectRef::new(901, 0))),
    ]);

    let (_, events) = capture_events(|| {
        resolve_encrypt_dictionary_references(&encryption_dictionary, |_| {
            Err(Error::InvalidPdf("CONFIDENTIAL_ENCRYPT_REFERENCE".repeat(4096)))
        })
    });

    let warnings: Vec<_> = events
        .iter()
        .filter(|event| {
            event.level == Level::WARN
                && event.target == crate::LOG_TARGET_ROOT
                && event.fields.get("operation").map(String::as_str) == Some("resolve_encrypt_reference")
        })
        .collect();
    assert_eq!(
        warnings.len(),
        1,
        "expected one aggregate encryption warning: {events:#?}"
    );
    assert_eq!(
        warnings[0].fields.get("error_code").map(String::as_str),
        Some("unresolved_reference")
    );
    assert_eq!(warnings[0].fields.get("skipped_count").map(String::as_str), Some("2"));
    assert!(!warnings[0].fields.contains_key("error"));
    assert!(!format!("{events:?}").contains("CONFIDENTIAL_ENCRYPT_REFERENCE"));
}

#[test]
fn test_find_substring_middle() {
    assert_eq!(find_substring(b"Hello World", b"lo W"), Some(3));
}

#[test]
fn test_find_substring_full_match() {
    assert_eq!(find_substring(b"ABC", b"ABC"), Some(0));
}

#[test]
fn test_find_substring_needle_longer() {
    assert_eq!(find_substring(b"AB", b"ABCD"), None);
}

#[test]
fn test_parse_header_lenient_no_header() {
    let mut cursor = Cursor::new(vec![0xABu8; 100]);
    let (major, minor, _) = parse_header(&mut cursor, true).unwrap();
    assert_eq!((major, minor), (1, 4));
}

#[test]
fn test_parse_version_lenient_version_0_0() {
    let header = *b"%PDF-0.0";
    assert_eq!(parse_version_from_header(&header, true).unwrap(), (1, 4));
}

#[test]
fn test_parse_trailer_empty_input() {
    assert!(parse_trailer(&mut Cursor::new(b"")).is_err());
}

#[test]
#[allow(deprecated)]
fn test_page_count_u32_zero_pages() {
    let mut pdf = b"%PDF-1.4\n".to_vec();
    let off1 = pdf.len();
    pdf.extend_from_slice(b"1 0 obj\n<< /Type /Catalog /Pages 2 0 R >>\nendobj\n");
    let off2 = pdf.len();
    pdf.extend_from_slice(b"2 0 obj\n<< /Type /Pages /Kids [] /Count 0 >>\nendobj\n");
    let xref_off = pdf.len();
    pdf.extend_from_slice(b"xref\n0 3\n");
    pdf.extend_from_slice(b"0000000000 65535 f \n");
    pdf.extend_from_slice(format!("{:010} 00000 n \n", off1).as_bytes());
    pdf.extend_from_slice(format!("{:010} 00000 n \n", off2).as_bytes());
    pdf.extend_from_slice(format!("trailer\n<< /Size 3 /Root 1 0 R >>\nstartxref\n{}\n%%EOF\n", xref_off).as_bytes());
    assert_eq!(PdfDocument::from_bytes(pdf).unwrap().page_count_u32(), 0);
}

/// Regression test: validate_object_at_offset must return true for
/// compressed (type 2) xref entries. Previously, it treated the object
/// stream number as a byte offset, sought to a random location,
/// returned false — triggering a full-file xref reconstruction that took
/// 35+ seconds on large PDFs.
#[test]
fn test_validate_compressed_xref_entry() {
    use crate::xref::{CrossRefTable, XRefEntry, XRefEntryType};

    let mut xref = CrossRefTable::new();
    // Add a compressed entry: object 5 lives inside object stream 10, at index 3 ~keep
    xref.entries.insert(
        5,
        XRefEntry {
            entry_type: XRefEntryType::Compressed,
            offset: 10,
            generation: 3,
            in_use: true,
        },
    );

    let data = b"%PDF-1.7\n%%EOF\n";
    let mut cursor = Cursor::new(data.to_vec());
    let obj_ref = ObjectRef { id: 5, generation: 0 };

    // Must return true — compressed objects are valid by virtue of being in the xref ~keep
    assert!(validate_object_at_offset(&mut cursor, &xref, obj_ref));
}

#[test]
fn test_lock_or_recover_on_poisoned_mutex() {
    use std::sync::Mutex;
    let m = Mutex::new(42);
    let _ = std::panic::catch_unwind(std::panic::AssertUnwindSafe(|| {
        let _guard = m.lock().unwrap();
        panic!("intentional");
    }));
    assert!(m.lock().is_err(), "Mutex should be poisoned");
    let val = *m.lock_or_recover();
    assert_eq!(val, 42);
}

#[test]
fn test_bounded_entry_cache_lru_eviction_order() {
    let mut c = BoundedEntryCache::new(3);
    c.insert(1u32, "a");
    c.insert(2, "b");
    c.insert(3, "c");
    assert_eq!(c.get(&1), Some(&"a"));
    // Insert key 4 — should evict 2 (oldest untouched), not 1 ~keep
    c.insert(4, "d");
    assert_eq!(c.get(&1), Some(&"a"), "LRU-promoted key should survive");
    assert!(c.get(&2).is_none(), "Oldest untouched key should be evicted");
    assert_eq!(c.get(&3), Some(&"c"));
    assert_eq!(c.get(&4), Some(&"d"));
}

#[test]
fn test_bounded_entry_cache_reinsert_no_eviction() {
    let mut c = BoundedEntryCache::new(1);
    c.insert(1u32, "a");
    // Re-insert same key — should NOT evict, just replace ~keep
    c.insert(1, "b");
    assert_eq!(c.len(), 1);
    assert_eq!(c.get(&1), Some(&"b"));
}

#[test]
fn test_bounded_entry_cache_fifo_eviction_without_get() {
    let mut c = BoundedEntryCache::new(2);
    c.insert(1u32, "a");
    c.insert(2, "b");
    // No get() calls — pure insertion order ~keep
    c.insert(3, "c");
    assert!(c.get(&1).is_none(), "First inserted should be evicted");
    assert_eq!(c.get(&2), Some(&"b"));
    assert_eq!(c.get(&3), Some(&"c"));
}

#[test]
fn test_bounded_object_cache_oversized_rejection() {
    let mut c = BoundedObjectCache::new(100);
    let big = Object::String(vec![0u8; 200]);
    c.insert(ObjectRef::new(1, 0), big);
    assert_eq!(c.len(), 0, "Oversized object should be rejected");
}

#[test]
fn test_bounded_object_cache_byte_budget_eviction() {
    // Use a budget that fits ~2 small objects but not 3 ~keep
    let small = Object::Integer(1);
    let budget = 80;
    let mut c = BoundedObjectCache::new(budget);
    c.insert(ObjectRef::new(1, 0), small.clone());
    c.insert(ObjectRef::new(2, 0), small.clone());
    assert_eq!(c.len(), 2);
    c.insert(ObjectRef::new(3, 0), small.clone());
    assert!(c.get(&ObjectRef::new(1, 0)).is_none(), "Oldest should be evicted");
    assert!(c.get(&ObjectRef::new(3, 0)).is_some());
    assert!(c.current_bytes <= budget);
}

#[test]
fn object_stream_cache_rejects_oversized_entries_and_evicts_to_budget() {
    let first = Arc::new(HashMap::from([(1, Object::String(vec![0; 64]))]));
    let second = Arc::new(HashMap::from([(2, Object::String(vec![0; 64]))]));
    let third = Arc::new(HashMap::from([(3, Object::String(vec![0; 64]))]));
    let entry_bytes = BoundedObjectStreamCache::estimate_size(&first).unwrap();
    let budget = entry_bytes * 2;
    let mut cache = BoundedObjectStreamCache::new(budget);

    assert!(cache.insert(ObjectRef::new(10, 0), Arc::clone(&first)));
    assert!(cache.insert(ObjectRef::new(11, 0), Arc::clone(&second)));
    assert!(cache.insert(ObjectRef::new(12, 0), Arc::clone(&third)));
    assert!(
        cache.get(&ObjectRef::new(10, 0)).is_none(),
        "oldest stream must be evicted"
    );
    assert!(cache.get(&ObjectRef::new(12, 0)).is_some());
    assert!(cache.current_bytes <= budget);

    let oversized = Arc::new(HashMap::from([(4, Object::String(vec![0; budget]))]));
    assert!(!cache.insert(ObjectRef::new(13, 0), oversized));
    assert!(cache.get(&ObjectRef::new(13, 0)).is_none());
    assert!(cache.current_bytes <= budget);
}

#[test]
fn object_stream_cache_rejects_deeply_nested_oversized_string() {
    let mut nested = Object::String(vec![0; 1024 * 1024]);
    for _ in 0..9 {
        nested = Object::Array(vec![nested]);
    }
    let mut cache = BoundedObjectStreamCache::new(4 * 1024);

    assert!(!cache.insert(ObjectRef::new(10, 0), Arc::new(HashMap::from([(20, nested)]))));
    assert_eq!(cache.map.len(), 0);
    assert_eq!(cache.current_bytes, 0);
}

#[test]
fn object_stream_cache_accounts_for_nested_stream_bytes() {
    let nested = Object::Array(vec![Object::Dictionary(HashMap::from([(
        "payload".to_string(),
        Object::Stream {
            dict: HashMap::new(),
            data: bytes::Bytes::from(vec![0; 1024 * 1024]),
        },
    )]))]);
    let mut cache = BoundedObjectStreamCache::new(4 * 1024);

    assert!(!cache.insert(ObjectRef::new(10, 0), Arc::new(HashMap::from([(20, nested)]))));
    assert_eq!(cache.map.len(), 0);
    assert_eq!(cache.current_bytes, 0);
}

#[test]
fn object_stream_cache_rejects_accounting_overflow() {
    assert_eq!(BoundedObjectStreamCache::checked_capacity_bytes(usize::MAX, 2), None);
}

#[test]
fn object_stream_recovery_marker_is_bounded_and_fail_closed() {
    let mut marker = BoundedRecoveryTelemetry::new(1);

    assert!(marker.should_emit(10));
    assert!(!marker.should_emit(10));
    assert!(!marker.should_emit(11));
    assert!(marker.saturated);
    assert_eq!(marker.seen.len(), 0);
    assert!(!marker.should_emit(10));
}

#[test]
fn test_estimate_size_depth_bottoms_out() {
    // Deeply nested array — should not stack overflow ~keep
    let mut obj = Object::Integer(1);
    for _ in 0..100 {
        obj = Object::Array(vec![obj]);
    }
    // Should return a finite value without panicking ~keep
    let size = BoundedObjectCache::estimate_size(&obj);
    assert!(size > 0);
}

#[test]
fn pinned_cache_entries_survive_eviction_pressure() {
    let mut cache = BoundedObjectCache::new(4096);
    let synthetic = ObjectRef::new(9999, 0);
    cache.insert_pinned(synthetic, Object::Integer(42));

    for i in 0..500u32 {
        cache.insert(ObjectRef::new(i, 0), Object::String(vec![b'x'; 256]));
    }

    assert!(
        cache.get(&synthetic).is_some(),
        "the pinned synthetic object was evicted and is now unreachable"
    );
    assert!(
        cache.len() < 500,
        "eviction should still be reclaiming ordinary entries"
    );
}

#[test]
fn test_unpinned_entry_is_evicted_under_the_same_pressure() {
    let mut cache = BoundedObjectCache::new(4096);
    let ordinary = ObjectRef::new(9999, 0);
    cache.insert(ordinary, Object::Integer(42));

    for i in 0..500u32 {
        cache.insert(ObjectRef::new(i, 0), Object::String(vec![b'x'; 256]));
    }

    assert!(cache.get(&ordinary).is_none());
}

#[test]
fn clearing_the_cache_keeps_pinned_entries() {
    let mut cache = BoundedObjectCache::new(1 << 20);
    let synthetic = ObjectRef::new(9999, 0);
    cache.insert_pinned(synthetic, Object::Integer(7));
    cache.insert(ObjectRef::new(1, 0), Object::Integer(1));

    cache.clear();

    assert!(cache.get(&synthetic).is_some());
    assert!(cache.get(&ObjectRef::new(1, 0)).is_none());
}

#[test]
fn test_all_pinned_cache_does_not_spin() {
    let mut cache = BoundedObjectCache::new(512);
    for i in 0..8u32 {
        cache.insert_pinned(ObjectRef::new(i, 0), Object::String(vec![b'y'; 256]));
    }
    assert_eq!(cache.len(), 8);
}
