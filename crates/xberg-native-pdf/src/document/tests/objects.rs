use super::super::*;
use super::common::*;
use std::collections::BTreeMap;
use tracing::Level;

fn build_shifted_xref_pdf(first_object: u8) -> Vec<u8> {
    let stream = b"BT /F1 12 Tf 72 720 Td (Synthetic xref reproduction) Tj ET\n";
    let objects = [
        format!(
            "<< /Length {} >>\nstream\n{}endstream",
            stream.len(),
            String::from_utf8_lossy(stream)
        ),
        "<< /Type /Font /Subtype /Type1 /BaseFont /Helvetica >>".to_string(),
        "<< /Type /Page /Parent 4 0 R /MediaBox [0 0 595 842] /Resources << /Font << /F1 2 0 R >> >> /Contents 1 0 R >>"
            .to_string(),
        "<< /Type /Pages /Kids [3 0 R] /Count 1 >>".to_string(),
        "<< /Type /Catalog /Pages 4 0 R >>".to_string(),
        "<< /Producer (Synthetic reproducer) >>".to_string(),
    ];

    let mut pdf = b"%PDF-1.4\n".to_vec();
    let mut offsets = Vec::with_capacity(objects.len());
    for (index, body) in objects.iter().enumerate() {
        offsets.push(pdf.len());
        pdf.extend_from_slice(format!("{} 0 obj\n{}\nendobj\n", index + 1, body).as_bytes());
    }

    let xref_offset = pdf.len();
    pdf.extend_from_slice(format!("xref\n{first_object} 7\n").as_bytes());
    pdf.extend_from_slice(b"0000000000 65535 f \n");
    for offset in offsets {
        pdf.extend_from_slice(format!("{offset:010} 00000 n \n").as_bytes());
    }
    pdf.extend_from_slice(format!("trailer\n<< /Size 7 /Root 5 0 R >>\nstartxref\n{xref_offset}\n%%EOF\n").as_bytes());
    pdf
}

// A corrupt/zero startxref forces full-file xref reconstruction.
// Because reconstruction already scans the whole file for every
// uncompressed object, the document must pre-seed its object-scan cache
// from the reconstructed table — so the first object miss is O(1) instead
// of triggering a SECOND full-file scan (the heavy "first extract_text"
// cost on corrupt-xref polyglot PDFs). ~keep
#[test]
fn test_reconstructed_xref_preseeds_scan_cache() {
    let pdf = b"%PDF-1.4\n\
            1 0 obj\n<< /Type /Catalog /Pages 2 0 R >>\nendobj\n\
            2 0 obj\n<< /Type /Pages /Count 0 /Kids [] >>\nendobj\n\
            trailer\n<< /Root 1 0 R /Size 3 >>\n\
            startxref\n0\n%%EOF";
    let doc = PdfDocument::from_bytes(pdf.to_vec()).expect("open corrupt-xref pdf");

    let cache = doc.scanned_object_offsets.lock_or_recover();
    let offsets = cache
        .as_ref()
        .expect("reconstructed xref must pre-seed the scan-offset cache");
    assert!(
        offsets.contains_key(&1) && offsets.contains_key(&2),
        "pre-seeded cache should hold the reconstructed object offsets, got {offsets:?}"
    );
}

#[test]
fn test_version_accessor() {
    let pdf = build_minimal_pdf(b"");
    let doc = PdfDocument::from_bytes(pdf).unwrap();
    let (major, minor) = doc.version();
    assert_eq!(major, 1);
    assert_eq!(minor, 4);
}

#[test]
fn test_trailer_accessor() {
    let pdf = build_minimal_pdf(b"");
    let doc = PdfDocument::from_bytes(pdf).unwrap();
    let trailer = doc.trailer();
    let dict = trailer.as_dict().unwrap();
    assert!(dict.contains_key("Root"));
    assert!(dict.contains_key("Size"));
}

#[test]
fn test_debug_impl() {
    let pdf = build_minimal_pdf(b"");
    let doc = PdfDocument::from_bytes(pdf).unwrap();
    let debug_str = format!("{:?}", doc);
    assert!(debug_str.contains("PdfDocument"));
    assert!(debug_str.contains("version"));
    assert!(debug_str.contains("(1, 4)"));
}

#[test]
fn test_page_count_zero_pages() {
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

    let doc = PdfDocument::from_bytes(pdf).unwrap();
    assert_eq!(doc.page_count().unwrap(), 0);
}

#[test]
fn test_load_object_from_cache() {
    let pdf = build_minimal_pdf(b"");
    let doc = PdfDocument::from_bytes(pdf).unwrap();

    let obj_ref = ObjectRef::new(1, 0);
    let obj1 = doc.load_object(obj_ref).unwrap();
    let obj2 = doc.load_object(obj_ref).unwrap();
    assert_eq!(obj1.as_dict().unwrap().get("Type").unwrap().as_name(), Some("Catalog"));
    assert_eq!(obj2.as_dict().unwrap().get("Type").unwrap().as_name(), Some("Catalog"));
}

#[test]
fn warm_object_reads_share_the_cache_and_owned_loads_stay_independent() {
    let document = PdfDocument::from_bytes(build_minimal_pdf(b"")).unwrap();
    let reference = ObjectRef::new(1, 0);
    document.object_cache.lock_or_recover().clear();
    let super::super::objects::LoadedObject::Fresh(cold) = document.load_object_for_read(reference).unwrap() else {
        panic!("a cold read that fills the cache must share the cached allocation");
    };

    let super::super::objects::LoadedObject::Cached(first) = document.load_object_for_read(reference).unwrap() else {
        panic!("the warm object must share the cached allocation");
    };
    assert!(
        std::sync::Arc::ptr_eq(&cold, &first),
        "the cold read must not copy the object it cached"
    );
    let super::super::objects::LoadedObject::Cached(second) = document.load_object_for_read(reference).unwrap() else {
        panic!("a second warm read must share the cached allocation");
    };
    assert!(std::sync::Arc::ptr_eq(&first, &second));

    let count = std::sync::Arc::strong_count(&first);
    let owned = document.load_object(reference).unwrap();
    assert_eq!(std::sync::Arc::strong_count(&first), count);
    assert_eq!(owned, *first);
}

#[test]
fn warm_object_reads_keep_depth_and_cycle_guards() {
    let document = PdfDocument::from_bytes(build_minimal_pdf(b"")).unwrap();
    let reference = ObjectRef::new(1, 0);
    document.load_object(reference).unwrap();

    let depth_error = RECURSION_DEPTH.with(|depth| {
        let saved = depth.replace(MAX_RECURSION_DEPTH);
        let result = document.load_object_for_read(reference);
        depth.replace(saved);
        result
    });
    assert!(matches!(
        depth_error,
        Err(Error::RecursionLimitExceeded(MAX_RECURSION_DEPTH))
    ));

    RESOLVING_STACK.with(|stack| {
        assert!(stack.borrow_mut().insert(reference));
        let result = document.load_object_for_read(reference);
        stack.borrow_mut().remove(&reference);
        assert!(matches!(result, Err(Error::CircularReference(object)) if object == reference));
    });
    assert!(document.load_object_for_read(reference).is_ok());
}

#[test]
fn load_fonts_borrows_cached_resources_and_font_dictionaries() {
    let document = PdfDocument::from_bytes(build_minimal_pdf(b"")).unwrap();
    let resources_ref = ObjectRef::new(900, 0);
    let fonts_ref = ObjectRef::new(901, 0);
    {
        let mut cache = document.object_cache.lock_or_recover();
        cache.insert(
            resources_ref,
            Object::Dictionary(std::collections::HashMap::from([(
                "Font".to_string(),
                Object::Reference(fonts_ref),
            )])),
        );
        cache.insert(fonts_ref, Object::Dictionary(std::collections::HashMap::new()));
    }

    let resources = Object::Reference(resources_ref);
    for _ in 0..2 {
        let mut extractor = crate::extractors::TextExtractor::new();
        document.load_fonts(&resources, &mut extractor).unwrap();
    }
    assert_eq!(document.cached_object_clone_count(), 0);

    document.load_object(fonts_ref).unwrap();
    assert_eq!(document.cached_object_clone_count(), 1);
}

#[test]
fn shifted_xref_subsection_recovers_requested_objects_by_header() {
    let valid = build_shifted_xref_pdf(0);
    let malformed = build_shifted_xref_pdf(1);
    assert_eq!(valid.len(), malformed.len());
    assert_eq!(
        valid
            .iter()
            .zip(&malformed)
            .filter(|(left, right)| left != right)
            .count(),
        1
    );
    assert_eq!(PdfDocument::from_bytes(valid).unwrap().page_count().unwrap(), 1);

    let document = PdfDocument::from_bytes(malformed).unwrap();
    assert_eq!(document.page_count().unwrap(), 1);
    assert_eq!(
        document
            .load_object(ObjectRef::new(5, 0))
            .unwrap()
            .as_dict()
            .and_then(|dict| dict.get("Type"))
            .and_then(Object::as_name),
        Some("Catalog")
    );
    assert!(
        document
            .extract_all_text()
            .unwrap()
            .contains("Synthetic xref reproduction")
    );
}

#[test]
fn mismatched_xref_without_requested_header_errors_without_caching() {
    use crate::xref::XRefEntry;

    let mut document = PdfDocument::from_bytes(build_minimal_pdf(b"")).unwrap();
    let wrong_offset = document.xref.get(1).unwrap().offset;
    let requested = ObjectRef::new(99, 0);
    document
        .xref
        .add_entry(requested.id, XRefEntry::uncompressed(wrong_offset, 0));

    let error = document.load_object(requested).unwrap_err();
    assert_eq!(
        error.to_string(),
        format!(
            "Failed to parse object at byte {wrong_offset}: Object reference mismatch: expected 99 0 obj, found 1 0 obj"
        )
    );
    assert!(document.object_cache.lock_or_recover().get(&requested).is_none());
}

#[test]
fn mismatched_generation_after_scan_errors_without_caching() {
    use crate::xref::XRefEntry;

    let mut pdf = b"%PDF-1.4\n".to_vec();
    let catalog_offset = pdf.len();
    pdf.extend_from_slice(b"1 0 obj\n<< /Type /Catalog /Pages 2 0 R >>\nendobj\n");
    let pages_offset = pdf.len();
    pdf.extend_from_slice(b"2 0 obj\n<< /Type /Pages /Kids [] /Count 0 >>\nendobj\n");
    let mismatched_generation_offset = pdf.len();
    pdf.extend_from_slice(b"9 1 obj\n<< /Type /Metadata >>\nendobj\n");
    let xref_offset = pdf.len();
    pdf.extend_from_slice(b"xref\n0 3\n0000000000 65535 f \n");
    pdf.extend_from_slice(format!("{catalog_offset:010} 00000 n \n").as_bytes());
    pdf.extend_from_slice(format!("{pages_offset:010} 00000 n \n").as_bytes());
    pdf.extend_from_slice(format!("trailer\n<< /Size 10 /Root 1 0 R >>\nstartxref\n{xref_offset}\n%%EOF\n").as_bytes());

    let mut document = PdfDocument::from_bytes(pdf).unwrap();
    let requested = ObjectRef::new(9, 0);
    document
        .xref
        .add_entry(requested.id, XRefEntry::uncompressed(catalog_offset as u64, 0));

    let error = document.load_object(requested).unwrap_err();
    assert_eq!(
        error.to_string(),
        format!(
            "Failed to parse object at byte {mismatched_generation_offset}: Object reference mismatch: expected 9 0 obj, found 9 1 obj"
        )
    );
    assert!(document.object_cache.lock_or_recover().get(&requested).is_none());
}

#[test]
fn genuine_catalog_without_pages_still_errors() {
    let mut pdf = b"%PDF-1.4\n".to_vec();
    let catalog_offset = pdf.len();
    pdf.extend_from_slice(b"1 0 obj\n<< /Type /Catalog >>\nendobj\n");
    let xref_offset = pdf.len();
    pdf.extend_from_slice(b"xref\n0 2\n0000000000 65535 f \n");
    pdf.extend_from_slice(format!("{catalog_offset:010} 00000 n \n").as_bytes());
    pdf.extend_from_slice(format!("trailer\n<< /Size 2 /Root 1 0 R >>\nstartxref\n{xref_offset}\n%%EOF\n").as_bytes());

    let error = PdfDocument::from_bytes(pdf).unwrap().page_count().unwrap_err();
    assert_eq!(error.to_string(), "Invalid PDF: Catalog missing /Pages entry");
}

#[test]
fn test_load_object_missing_returns_null() {
    let pdf = build_minimal_pdf(b"");
    let doc = PdfDocument::from_bytes(pdf).unwrap();

    let obj_ref = ObjectRef::new(999, 0);
    let obj = doc.load_object(obj_ref).unwrap();
    // Per PDF Spec 7.3.10: missing objects treated as Null ~keep
    assert!(matches!(obj, Object::Null));
}

#[test]
fn test_resolve_references_integer() {
    let pdf = build_minimal_pdf(b"");
    let doc = PdfDocument::from_bytes(pdf).unwrap();

    let obj = Object::Integer(42);
    let resolved = doc.resolve_references(&obj, 3).unwrap();
    assert_eq!(resolved.as_integer(), Some(42));
}

#[test]
fn test_resolve_references_null() {
    let pdf = build_minimal_pdf(b"");
    let doc = PdfDocument::from_bytes(pdf).unwrap();

    let obj = Object::Null;
    let resolved = doc.resolve_references(&obj, 3).unwrap();
    assert!(matches!(resolved, Object::Null));
}

#[test]
fn test_resolve_references_max_depth_zero() {
    let pdf = build_minimal_pdf(b"");
    let doc = PdfDocument::from_bytes(pdf).unwrap();

    let obj = Object::Reference(ObjectRef::new(1, 0));
    let resolved = doc.resolve_references(&obj, 0).unwrap();
    assert!(resolved.as_reference().is_some());
}

#[test]
fn test_resolve_references_reference() {
    let pdf = build_minimal_pdf(b"");
    let doc = PdfDocument::from_bytes(pdf).unwrap();

    let obj = Object::Reference(ObjectRef::new(1, 0));
    let resolved = doc.resolve_references(&obj, 3).unwrap();
    assert!(resolved.as_dict().is_some());
}

#[test]
fn test_resolve_references_array() {
    let pdf = build_minimal_pdf(b"");
    let doc = PdfDocument::from_bytes(pdf).unwrap();

    let arr = Object::Array(vec![Object::Integer(1), Object::Integer(2)]);
    let resolved = doc.resolve_references(&arr, 3).unwrap();
    let resolved_arr = resolved.as_array().unwrap();
    assert_eq!(resolved_arr.len(), 2);
}

#[test]
fn test_resolve_references_dictionary() {
    let pdf = build_minimal_pdf(b"");
    let doc = PdfDocument::from_bytes(pdf).unwrap();

    let mut dict = std::collections::HashMap::new();
    dict.insert("Key".to_string(), Object::Integer(42));
    let obj = Object::Dictionary(dict);
    let resolved = doc.resolve_references(&obj, 3).unwrap();
    let resolved_dict = resolved.as_dict().unwrap();
    assert_eq!(resolved_dict.get("Key").unwrap().as_integer(), Some(42));
}

#[test]
fn test_resolve_references_bad_reference() {
    let pdf = build_minimal_pdf(b"");
    let doc = PdfDocument::from_bytes(pdf).unwrap();

    let obj = Object::Reference(ObjectRef::new(999, 0));
    let resolved = doc.resolve_references(&obj, 3).unwrap();
    assert!(matches!(resolved, Object::Null));
}

#[test]
fn test_is_form_xobject_nonexistent_ref() {
    let pdf = build_minimal_pdf(b"");
    let doc = PdfDocument::from_bytes(pdf).unwrap();
    // Non-existent object should return true (conservative) ~keep
    let result = doc.is_form_xobject(ObjectRef::new(999, 0));
    assert!(result);
}

#[test]
fn test_is_form_xobject_catalog_not_form() {
    let pdf = build_minimal_pdf(b"");
    let doc = PdfDocument::from_bytes(pdf).unwrap();
    let _ = doc.load_object(ObjectRef::new(1, 0));
    let result = doc.is_form_xobject(ObjectRef::new(1, 0));
    assert!(!result);
}

#[test]
fn test_from_bytes_with_v2_header() {
    let mut pdf = b"%PDF-2.0\n".to_vec();

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

    let doc = PdfDocument::from_bytes(pdf).unwrap();
    assert_eq!(doc.version(), (2, 0));
}

#[test]
fn test_nested_page_tree() {
    let mut pdf = b"%PDF-1.4\n".to_vec();

    let off1 = pdf.len();
    pdf.extend_from_slice(b"1 0 obj\n<< /Type /Catalog /Pages 2 0 R >>\nendobj\n");

    let off2 = pdf.len();
    pdf.extend_from_slice(b"2 0 obj\n<< /Type /Pages /Kids [3 0 R] /Count 2 >>\nendobj\n");

    let off3 = pdf.len();
    pdf.extend_from_slice(b"3 0 obj\n<< /Type /Pages /Kids [4 0 R 5 0 R] /Count 2 /Parent 2 0 R >>\nendobj\n");

    let off4 = pdf.len();
    pdf.extend_from_slice(b"4 0 obj\n<< /Type /Page /Parent 3 0 R /MediaBox [0 0 612 792] >>\nendobj\n");

    let off5 = pdf.len();
    pdf.extend_from_slice(b"5 0 obj\n<< /Type /Page /Parent 3 0 R /MediaBox [0 0 612 792] >>\nendobj\n");

    let xref_off = pdf.len();
    pdf.extend_from_slice(b"xref\n0 6\n");
    pdf.extend_from_slice(b"0000000000 65535 f \n");
    pdf.extend_from_slice(format!("{:010} 00000 n \n", off1).as_bytes());
    pdf.extend_from_slice(format!("{:010} 00000 n \n", off2).as_bytes());
    pdf.extend_from_slice(format!("{:010} 00000 n \n", off3).as_bytes());
    pdf.extend_from_slice(format!("{:010} 00000 n \n", off4).as_bytes());
    pdf.extend_from_slice(format!("{:010} 00000 n \n", off5).as_bytes());
    pdf.extend_from_slice(format!("trailer\n<< /Size 6 /Root 1 0 R >>\nstartxref\n{}\n%%EOF\n", xref_off).as_bytes());

    let doc = PdfDocument::from_bytes(pdf).unwrap();
    assert_eq!(doc.page_count().unwrap(), 2);
}

#[test]
fn test_resolve_references_boolean() {
    let pdf = build_minimal_pdf(b"");
    let doc = PdfDocument::from_bytes(pdf).unwrap();
    let resolved = doc.resolve_references(&Object::Boolean(true), 5).unwrap();
    assert!(matches!(resolved, Object::Boolean(true)));
}

#[test]
fn test_resolve_references_nested_dict_with_refs() {
    let pdf = build_minimal_pdf(b"");
    let doc = PdfDocument::from_bytes(pdf).unwrap();
    let mut dict = std::collections::HashMap::new();
    dict.insert("CatalogRef".to_string(), Object::Reference(ObjectRef::new(1, 0)));
    dict.insert("Direct".to_string(), Object::Integer(42));
    let resolved = doc.resolve_references(&Object::Dictionary(dict), 3).unwrap();
    let rd = resolved.as_dict().unwrap();
    assert!(rd.get("CatalogRef").unwrap().as_dict().is_some());
    assert_eq!(rd.get("Direct").unwrap().as_integer(), Some(42));
}

#[test]
fn test_resolve_references_array_with_refs() {
    let pdf = build_minimal_pdf(b"");
    let doc = PdfDocument::from_bytes(pdf).unwrap();
    let arr = Object::Array(vec![Object::Reference(ObjectRef::new(1, 0)), Object::Integer(99)]);
    let resolved = doc.resolve_references(&arr, 3).unwrap();
    let ra = resolved.as_array().unwrap();
    assert!(ra[0].as_dict().is_some());
    assert_eq!(ra[1].as_integer(), Some(99));
}

#[test]
fn test_extract_all_text_zero_pages() {
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
    let doc = PdfDocument::from_bytes(pdf).unwrap();
    assert!(doc.extract_all_text().unwrap().is_empty());
}

#[test]
fn test_page_count_exceeds_objects() {
    let mut pdf = b"%PDF-1.4\n".to_vec();
    let off1 = pdf.len();
    pdf.extend_from_slice(b"1 0 obj\n<< /Type /Catalog /Pages 2 0 R >>\nendobj\n");
    let off2 = pdf.len();
    pdf.extend_from_slice(b"2 0 obj\n<< /Type /Pages /Kids [3 0 R] /Count 999 >>\nendobj\n");
    let off3 = pdf.len();
    pdf.extend_from_slice(b"3 0 obj\n<< /Type /Page /Parent 2 0 R /MediaBox [0 0 612 792] >>\nendobj\n");
    let xref_off = pdf.len();
    pdf.extend_from_slice(b"xref\n0 4\n");
    pdf.extend_from_slice(b"0000000000 65535 f \n");
    pdf.extend_from_slice(format!("{:010} 00000 n \n", off1).as_bytes());
    pdf.extend_from_slice(format!("{:010} 00000 n \n", off2).as_bytes());
    pdf.extend_from_slice(format!("{:010} 00000 n \n", off3).as_bytes());
    pdf.extend_from_slice(format!("trailer\n<< /Size 4 /Root 1 0 R >>\nstartxref\n{}\n%%EOF\n", xref_off).as_bytes());
    let doc = PdfDocument::from_bytes(pdf).unwrap();
    assert_eq!(doc.page_count().unwrap(), 1);
}

#[test]
fn xref_recovery_tracing_does_not_expose_document_bytes() {
    const CONFIDENTIAL_MARKER: &str = "CONFIDENTIAL_ENTERPRISE_PAYLOAD_7f40c6";
    let confidential_content = CONFIDENTIAL_MARKER.repeat(4096);
    let mut pdf = b"%PDF-1.4\n".to_vec();
    let object_offset = pdf.len();
    pdf.extend_from_slice(b"1 0 obj\n<< /Type /Catalog >>\nendobj\n");
    let xref_offset = pdf.len();
    pdf.extend_from_slice(b"xref\n0 2\n0000000000 65535 f \n");
    pdf.extend_from_slice(format!("{object_offset:010} 00000 n \n").as_bytes());
    pdf.extend_from_slice(format!("trailer\n<< /Root ${confidential_content} >>\n").as_bytes());
    pdf.extend_from_slice(format!("startxref\n{xref_offset}\n%%EOF\n").as_bytes());

    let (result, events) = capture_events(|| PdfDocument::from_bytes(pdf));
    assert!(result.is_ok(), "reconstruction should recover the malformed trailer");
    let parse_failures: Vec<_> = events
        .iter()
        .filter(|event| {
            event.level == Level::WARN
                && event.target == format!("{}::document", crate::LOG_TARGET_ROOT)
                && event.fields.get("error_code").map(String::as_str) == Some("parse_error")
                && event.fields.get("message").map(String::as_str)
                    == Some("regular xref parsing failed; attempting reconstruction")
        })
        .collect();
    assert_eq!(
        parse_failures.len(),
        1,
        "expected exactly one regular-xref recovery warning: {events:#?}"
    );
    let captured = format!("{events:?}");
    let confidential_bytes = CONFIDENTIAL_MARKER
        .as_bytes()
        .iter()
        .map(u8::to_string)
        .collect::<Vec<_>>()
        .join(", ");

    assert!(
        !captured.contains(CONFIDENTIAL_MARKER) && !captured.contains(&confidential_bytes),
        "recovery telemetry exposed attacker-controlled document bytes: {captured}"
    );
    assert!(
        events
            .iter()
            .flat_map(|event| event.fields.values())
            .all(|value| value.len() <= 256),
        "recovery telemetry emitted an unbounded field: {events:#?}"
    );
}

#[test]
fn malformed_object_header_warning_does_not_expose_header_bytes() {
    const CONFIDENTIAL_MARKER: &str = "CONFIDENTIAL_OBJECT_HEADER_182f13";
    let mut pdf = build_minimal_pdf(b"");
    let marker_offset = pdf.len();
    pdf.extend_from_slice(CONFIDENTIAL_MARKER.as_bytes());
    let document = PdfDocument::from_bytes(pdf).unwrap();

    let (result, events) =
        capture_events(|| document.load_uncompressed_object_impl(ObjectRef::new(999, 0), marker_offset as u64, true));

    assert!(result.is_err());
    let warnings: Vec<_> = events.iter().filter(|event| event.level == Level::WARN).collect();
    assert_eq!(
        warnings.len(),
        1,
        "expected exactly one malformed-header warning: {events:#?}"
    );
    assert_eq!(warnings[0].target, format!("{}::document", crate::LOG_TARGET_ROOT));
    assert_eq!(
        warnings[0].fields.get("operation").map(String::as_str),
        Some("load_uncompressed_object")
    );
    assert_eq!(
        warnings[0].fields.get("error_code").map(String::as_str),
        Some("malformed_object_header")
    );
    assert!(
        !format!("{events:?}").contains(CONFIDENTIAL_MARKER),
        "object-header telemetry exposed attacker-controlled bytes: {events:#?}"
    );
}

#[test]
fn malformed_object_stream_is_parsed_once_for_multiple_missing_references() {
    use crate::xref::XRefEntry;

    let mut pdf = b"%PDF-1.5\n".to_vec();
    let catalog_offset = pdf.len();
    pdf.extend_from_slice(b"1 0 obj\n<< /Type /Catalog /Pages 2 0 R >>\nendobj\n");
    let pages_offset = pdf.len();
    pdf.extend_from_slice(b"2 0 obj\n<< /Type /Pages /Kids [] /Count 0 >>\nendobj\n");
    let object_stream_offset = pdf.len();
    pdf.extend_from_slice(
        b"10 0 obj\n<< /Type /ObjStm /N 2 /First 10 /Length 14 >>\nstream\n30 0 31 3 42 $\nendstream\nendobj\n",
    );
    let xref_offset = pdf.len();
    pdf.extend_from_slice(b"xref\n0 11\n0000000000 65535 f \n");
    for object_id in 1..=10 {
        let offset = match object_id {
            1 => catalog_offset,
            2 => pages_offset,
            10 => object_stream_offset,
            _ => 0,
        };
        let state = if offset == 0 { 'f' } else { 'n' };
        pdf.extend_from_slice(format!("{offset:010} 00000 {state} \n").as_bytes());
    }
    pdf.extend_from_slice(format!("trailer\n<< /Size 22 /Root 1 0 R >>\nstartxref\n{xref_offset}\n%%EOF\n").as_bytes());

    let mut document = PdfDocument::from_bytes(pdf).unwrap();
    document.xref.add_entry(20, XRefEntry::compressed(10, 0));
    document.xref.add_entry(21, XRefEntry::compressed(10, 1));
    document.object_stream_cache = Mutex::new(BoundedObjectStreamCache::new(1));

    let ((first, second), events) = capture_events(|| {
        (
            document.load_compressed_object(ObjectRef::new(20, 0), 10, 0),
            document.load_compressed_object(ObjectRef::new(21, 0), 10, 1),
        )
    });
    assert_eq!(first.unwrap().into_object(), Object::Null);
    assert_eq!(second.unwrap().into_object(), Object::Null);
    let warnings: Vec<_> = events
        .iter()
        .filter(|event| {
            event.level == Level::WARN
                && event.target == crate::LOG_TARGET_ROOT
                && event.fields.get("operation").map(String::as_str) == Some("parse_object_stream")
        })
        .collect();
    assert_eq!(warnings.len(), 1, "object stream must be parsed once: {events:#?}");
    assert_eq!(
        warnings[0].fields,
        BTreeMap::from([
            ("error_code".to_string(), "invalid_embedded_object".to_string()),
            ("invalid_offset_count".to_string(), "0".to_string()),
            ("message".to_string(), "object stream entries were skipped".to_string()),
            ("operation".to_string(), "parse_object_stream".to_string()),
            ("parse_failure_count".to_string(), "1".to_string()),
            ("skipped_count".to_string(), "1".to_string()),
        ])
    );
    assert_eq!(document.object_stream_cache.lock_or_recover().map.len(), 0);
    assert_eq!(document.object_stream_telemetry_seen.lock_or_recover().seen.len(), 1);
}

#[test]
fn clean_object_stream_does_not_consume_recovery_marker_budget() {
    let document = PdfDocument::from_bytes(build_minimal_pdf(b"")).unwrap();
    let stream = Object::Stream {
        dict: HashMap::from([
            ("Type".to_string(), Object::Name("ObjStm".to_string())),
            ("N".to_string(), Object::Integer(1)),
            ("First".to_string(), Object::Integer(5)),
        ]),
        data: bytes::Bytes::from_static(b"30 0 42"),
    };
    let outcome = crate::objstm::parse_object_stream_with_decryption_outcome(&stream, None, 0, 0).unwrap();

    document.trace_object_stream_recovery_once(10, &outcome);

    let marker = document.object_stream_telemetry_seen.lock_or_recover();
    assert_eq!(marker.seen.len(), 0);
    assert!(!marker.saturated);
}

#[test]
fn test_scan_for_object_finds_missing() {
    let mut pdf = b"%PDF-1.4\n".to_vec();
    let off1 = pdf.len();
    pdf.extend_from_slice(b"1 0 obj\n<< /Type /Catalog /Pages 2 0 R >>\nendobj\n");
    let off2 = pdf.len();
    pdf.extend_from_slice(b"2 0 obj\n<< /Type /Pages /Kids [3 0 R] /Count 1 >>\nendobj\n");
    let off3 = pdf.len();
    pdf.extend_from_slice(b"3 0 obj\n<< /Type /Page /Parent 2 0 R /MediaBox [0 0 612 792] >>\nendobj\n");
    let _off5 = pdf.len();
    pdf.extend_from_slice(b"5 0 obj\n<< /Type /Metadata /Subtype /XML >>\nendobj\n");
    let xref_off = pdf.len();
    pdf.extend_from_slice(b"xref\n0 4\n");
    pdf.extend_from_slice(b"0000000000 65535 f \n");
    pdf.extend_from_slice(format!("{:010} 00000 n \n", off1).as_bytes());
    pdf.extend_from_slice(format!("{:010} 00000 n \n", off2).as_bytes());
    pdf.extend_from_slice(format!("{:010} 00000 n \n", off3).as_bytes());
    pdf.extend_from_slice(format!("trailer\n<< /Size 4 /Root 1 0 R >>\nstartxref\n{}\n%%EOF\n", xref_off).as_bytes());
    let doc = PdfDocument::from_bytes(pdf).unwrap();
    let obj = doc.load_object(ObjectRef::new(5, 0)).unwrap();
    assert!(obj.as_dict().is_some());
}

#[test]
fn test_load_object_missing_returns_null_simple() {
    let pdf = build_minimal_pdf(b"");
    let doc = PdfDocument::from_bytes(pdf).unwrap();
    assert!(matches!(doc.load_object(ObjectRef::new(999, 0)).unwrap(), Object::Null));
}

#[test]
fn test_is_form_xobject_from_cache() {
    let pdf = build_minimal_pdf(b"");
    let doc = PdfDocument::from_bytes(pdf).unwrap();
    let _ = doc.load_object(ObjectRef::new(1, 0)).unwrap();
    assert!(!doc.is_form_xobject(ObjectRef::new(1, 0)));
}

#[test]
fn test_open_pdf_version_2_0() {
    let mut pdf = b"%PDF-2.0\n".to_vec();
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
    assert_eq!(PdfDocument::from_bytes(pdf).unwrap().version(), (2, 0));
}

#[test]
fn a_loaded_stream_object_borrows_its_payload_from_the_source_allocation() {
    let doc = PdfDocument::from_bytes(build_nested_xobject_do_with_orphaned_operands_pdf()).expect("parse pdf");
    let Object::Stream { data, .. } = doc.load_object(ObjectRef::new(4, 0)).expect("load stream object") else {
        panic!("object 4 is a content stream");
    };

    assert!(!data.is_empty());
    let source = doc.source_bytes.as_ptr() as usize..doc.source_bytes.as_ptr() as usize + doc.source_bytes.len();
    let payload = data.as_ptr() as usize..data.as_ptr() as usize + data.len();
    assert!(source.start <= payload.start && payload.end <= source.end);
}

#[test]
fn cold_owned_load_is_independent_of_the_cached_object() {
    let document = PdfDocument::from_bytes(build_minimal_pdf(b"")).unwrap();
    let reference = ObjectRef::new(1, 0);
    document.object_cache.lock_or_recover().clear();
    let clones_before = document.cached_object_clone_count();

    let mut owned = document.load_object(reference).unwrap();
    assert_eq!(
        document.cached_object_clone_count(),
        clones_before,
        "a miss is not a cache-hit clone"
    );
    let cached = document.load_object_shared(reference).unwrap();
    assert_eq!(owned, *cached);

    owned = Object::Null;
    assert_ne!(owned, *cached, "mutating the owned copy leaves the cached object alone");
}

/// Catalog and empty page tree as objects 1 and 2, then `bodies` as objects 3, 4, ...
fn pdf_with_trailing_objects(bodies: &[&[u8]]) -> Vec<u8> {
    let mut pdf = b"%PDF-1.4\n".to_vec();
    let mut offsets = vec![pdf.len()];
    pdf.extend_from_slice(b"1 0 obj\n<< /Type /Catalog /Pages 2 0 R >>\nendobj\n");
    offsets.push(pdf.len());
    pdf.extend_from_slice(b"2 0 obj\n<< /Type /Pages /Kids [] /Count 0 >>\nendobj\n");
    for (index, body) in bodies.iter().enumerate() {
        offsets.push(pdf.len());
        pdf.extend_from_slice(format!("{} 0 obj\n", index + 3).as_bytes());
        pdf.extend_from_slice(body);
        pdf.extend_from_slice(b"\nendobj\n");
    }
    let xref_offset = pdf.len();
    pdf.extend_from_slice(format!("xref\n0 {}\n0000000000 65535 f \n", offsets.len() + 1).as_bytes());
    for offset in &offsets {
        pdf.extend_from_slice(format!("{offset:010} 00000 n \n").as_bytes());
    }
    pdf.extend_from_slice(
        format!(
            "trailer\n<< /Size {} /Root 1 0 R >>\nstartxref\n{xref_offset}\n%%EOF\n",
            offsets.len() + 1
        )
        .as_bytes(),
    );
    pdf
}

fn stream_payload(doc: &PdfDocument, id: u32) -> Vec<u8> {
    match doc.load_object(ObjectRef::new(id, 0)).expect("load stream object") {
        Object::Stream { data, .. } => data.to_vec(),
        other => panic!("object {id} should be a stream, got {other:?}"),
    }
}

#[test]
fn stream_with_a_declared_length_is_stepped_over_to_its_endobj() {
    let pdf = pdf_with_trailing_objects(&[
        b"<< /Length 22 >>\nstream\nfirst line\nsecond line\nendstream",
        b"<< /Length 5 >>\r\nstream\r\nCRLF!\r\nendstream",
        b"<< /Type /Font /Subtype /Type1 /BaseFont /Helvetica >>",
    ]);
    let doc = PdfDocument::from_bytes(pdf).expect("open pdf");

    assert_eq!(stream_payload(&doc, 3), b"first line\nsecond line");
    assert_eq!(stream_payload(&doc, 4), b"CRLF!");
    let font = doc
        .load_object(ObjectRef::new(5, 0))
        .expect("load object after the streams");
    assert_eq!(
        font.as_dict()
            .and_then(|dict| dict.get("BaseFont"))
            .and_then(Object::as_name),
        Some("Helvetica")
    );
}

#[test]
fn stream_payload_holding_endobj_is_read_whole_when_its_length_is_declared() {
    let pdf = pdf_with_trailing_objects(&[b"<< /Length 19 >>\nstream\n1 0 obj\nendobj\nxyz\nendstream"]);
    let doc = PdfDocument::from_bytes(pdf).expect("open pdf");

    assert_eq!(stream_payload(&doc, 3), b"1 0 obj\nendobj\nxyz\n");
}

#[test]
fn stream_with_a_wrong_or_indirect_length_still_ends_at_endstream() {
    let pdf = pdf_with_trailing_objects(&[
        b"<< /Length 30 >>\nstream\nhello world\nendstream",
        b"<< /Length 5 0 R >>\nstream\nindirect\nendstream",
        b"11",
    ]);
    let doc = PdfDocument::from_bytes(pdf).expect("open pdf");

    assert_eq!(stream_payload(&doc, 3), b"hello world");
    assert_eq!(stream_payload(&doc, 4), b"indirect");
}

fn object_headers_were_indexed(document: &PdfDocument) -> bool {
    document.scanned_object_offsets.lock_or_recover().is_some()
}

fn object_streams_were_swept(document: &PdfDocument) -> bool {
    *document.objstm_recovery_done.lock_or_recover()
}

/// Objects 1 to 10, so the body holds `10 0 obj`, whose tail reads `0 0 obj`.
fn document_with_ten_objects() -> PdfDocument {
    let body: &[u8] = b"<< /Marker 1 >>";
    PdfDocument::from_bytes(pdf_with_trailing_objects(&[body; 8])).expect("open pdf")
}

#[test]
fn reference_to_free_object_zero_is_null_without_indexing_object_headers() {
    use crate::xref::XRefEntry;

    let mut document = document_with_ten_objects();
    document.xref.add_entry(0, XRefEntry::free(0, 65535));
    for generation in [0, 65535] {
        assert_eq!(
            document.load_object(ObjectRef::new(0, generation)).unwrap(),
            Object::Null
        );
    }
    assert!(!object_headers_were_indexed(&document));
    assert!(
        object_streams_were_swept(&document),
        "the sweep repairs later lookups of compressed objects the xref omits"
    );

    let mut control = document_with_ten_objects();
    control.xref.add_entry(11, XRefEntry::free(0, 1));
    assert_eq!(control.load_object(ObjectRef::new(11, 0)).unwrap(), Object::Null);
    assert!(
        object_headers_were_indexed(&control) && object_streams_were_swept(&control),
        "a free entry for any other object keeps both recovery paths"
    );
}

#[test]
fn reference_to_object_zero_missing_from_the_xref_is_null_without_scanning_the_file() {
    let mut document = document_with_ten_objects();
    document.xref.entries.remove(&0);
    assert_eq!(document.load_object(ObjectRef::new(0, 0)).unwrap(), Object::Null);
    assert!(!object_headers_were_indexed(&document));

    assert_eq!(document.load_object(ObjectRef::new(99, 0)).unwrap(), Object::Null);
    assert!(
        object_headers_were_indexed(&document),
        "a missing entry for any other object still scans the file"
    );
}

#[test]
fn object_zero_with_a_header_in_the_file_is_still_recovered() {
    use crate::xref::XRefEntry;

    let mut pdf = b"%PDF-1.4\n0 0 obj\n<< /Marker 7 >>\nendobj\n".to_vec();
    let catalog_offset = pdf.len();
    pdf.extend_from_slice(b"1 0 obj\n<< /Type /Catalog /Pages 2 0 R >>\nendobj\n");
    let pages_offset = pdf.len();
    pdf.extend_from_slice(b"2 0 obj\n<< /Type /Pages /Kids [] /Count 0 >>\nendobj\n");
    let xref_offset = pdf.len();
    pdf.extend_from_slice(b"xref\n0 3\n0000000000 65535 f \n");
    pdf.extend_from_slice(format!("{catalog_offset:010} 00000 n \n{pages_offset:010} 00000 n \n").as_bytes());
    pdf.extend_from_slice(format!("trailer\n<< /Size 3 /Root 1 0 R >>\nstartxref\n{xref_offset}\n%%EOF\n").as_bytes());

    for listed_as_free in [true, false] {
        let mut document = PdfDocument::from_bytes(pdf.clone()).expect("open pdf");
        if listed_as_free {
            document.xref.add_entry(0, XRefEntry::free(0, 65535));
        } else {
            document.xref.entries.remove(&0);
        }
        let recovered = document.load_object(ObjectRef::new(0, 0)).unwrap();
        assert_eq!(
            recovered
                .as_dict()
                .and_then(|dict| dict.get("Marker"))
                .and_then(Object::as_integer),
            Some(7),
            "listed_as_free={listed_as_free}"
        );
    }
}
