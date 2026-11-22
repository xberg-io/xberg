use super::super::*;
use super::common::*;
use super::pdf_fixtures::*;
use tracing::Level;

#[test]
fn test_from_bytes_minimal_pdf() {
    let pdf = build_minimal_pdf(b"");
    let doc = PdfDocument::from_bytes(pdf).unwrap();
    assert_eq!(doc.version(), (1, 4));
    assert!(doc.trailer().as_dict().is_some());
}

// catalog() must fall back to scanning indirect objects for
// `/Type /Catalog` when the trailer omits /Root. The public open path
// can't reach this — a /Root-less parsed trailer fails root validation
// and xref reconstruction synthesizes a /Root-bearing trailer before
// catalog() ever runs — so cover find_catalog_by_scan() directly: open a
// valid PDF, then strip /Root from the in-memory trailer and confirm
// catalog() still resolves the Catalog by object scan. ~keep
#[test]
fn test_catalog_recovers_when_trailer_omits_root() {
    let mut doc = PdfDocument::from_bytes(build_minimal_pdf(b"")).unwrap();
    assert!(doc.catalog().is_ok());

    // Drop /Root so only the indirect-object scan can find the Catalog. ~keep
    match doc.trailer {
        Object::Dictionary(ref mut d) => {
            d.remove("Root");
            assert!(d.get("Root").is_none());
        }
        _ => panic!("trailer is not a dictionary"),
    }

    let catalog = doc
        .catalog()
        .expect("catalog() must recover the /Type /Catalog object by scan when /Root is absent");
    assert_eq!(
        catalog.as_dict().and_then(|d| d.get("Type")).and_then(|t| t.as_name()),
        Some("Catalog"),
        "find_catalog_by_scan must return the actual Catalog object"
    );
}

#[test]
fn test_catalog_returns_dictionary() {
    let pdf = build_minimal_pdf(b"");
    let doc = PdfDocument::from_bytes(pdf).unwrap();
    let catalog = doc.catalog().unwrap();
    let dict = catalog.as_dict().unwrap();
    assert_eq!(dict.get("Type").unwrap().as_name(), Some("Catalog"));
}

#[test]
fn test_mark_info_untagged_pdf() {
    let pdf = build_minimal_pdf(b"");
    let doc = PdfDocument::from_bytes(pdf).unwrap();
    let mark_info = doc.mark_info().unwrap();
    assert!(!mark_info.marked);
    assert!(!mark_info.suspects);
}

#[test]
fn test_structure_tree_untagged_pdf() {
    let pdf = build_minimal_pdf(b"");
    let doc = PdfDocument::from_bytes(pdf).unwrap();
    let tree = doc.structure_tree().unwrap();
    assert!(tree.is_none());
}

#[test]
fn test_mark_info_tagged_pdf() {
    let mut pdf = b"%PDF-1.4\n".to_vec();

    let off1 = pdf.len();
    pdf.extend_from_slice(
        b"1 0 obj\n<< /Type /Catalog /Pages 2 0 R /MarkInfo << /Marked true /Suspects false >> >>\nendobj\n",
    );

    let off2 = pdf.len();
    pdf.extend_from_slice(b"2 0 obj\n<< /Type /Pages /Kids [] /Count 0 >>\nendobj\n");

    let xref_off = pdf.len();
    pdf.extend_from_slice(b"xref\n0 3\n");
    pdf.extend_from_slice(b"0000000000 65535 f \n");
    pdf.extend_from_slice(format!("{:010} 00000 n \n", off1).as_bytes());
    pdf.extend_from_slice(format!("{:010} 00000 n \n", off2).as_bytes());
    pdf.extend_from_slice(format!("trailer\n<< /Size 3 /Root 1 0 R >>\nstartxref\n{}\n%%EOF\n", xref_off).as_bytes());

    let doc = PdfDocument::from_bytes(pdf).unwrap();
    let mark_info = doc.mark_info().unwrap();
    assert!(mark_info.marked);
    assert!(!mark_info.suspects);
}

#[test]
fn test_get_page_ref_valid() {
    let pdf = build_minimal_pdf(b"");
    let doc = PdfDocument::from_bytes(pdf).unwrap();
    let page_ref = doc.get_page_ref(0).unwrap();
    // Page should be object 3 (catalog=1, pages=2, page=3) ~keep
    assert_eq!(page_ref.id, 3);
}

#[test]
fn test_mark_info_with_suspects() {
    let mut pdf = b"%PDF-1.4\n".to_vec();
    let off1 = pdf.len();
    pdf.extend_from_slice(
            b"1 0 obj\n<< /Type /Catalog /Pages 2 0 R /MarkInfo << /Marked true /Suspects true /UserProperties true >> >>\nendobj\n",
        );
    let off2 = pdf.len();
    pdf.extend_from_slice(b"2 0 obj\n<< /Type /Pages /Kids [] /Count 0 >>\nendobj\n");
    let xref_off = pdf.len();
    pdf.extend_from_slice(b"xref\n0 3\n");
    pdf.extend_from_slice(b"0000000000 65535 f \n");
    pdf.extend_from_slice(format!("{:010} 00000 n \n", off1).as_bytes());
    pdf.extend_from_slice(format!("{:010} 00000 n \n", off2).as_bytes());
    pdf.extend_from_slice(format!("trailer\n<< /Size 3 /Root 1 0 R >>\nstartxref\n{}\n%%EOF\n", xref_off).as_bytes());
    let doc = PdfDocument::from_bytes(pdf).unwrap();
    let mi = doc.mark_info().unwrap();
    assert!(mi.marked);
    assert!(mi.suspects);
    assert!(mi.user_properties);
}

#[test]
fn output_intent_and_page_tree_warnings_hide_attacker_controlled_details() {
    const SECRET_FILTER: &str = "CONFIDENTIAL_OUTPUT_FILTER_7b91";
    const SECRET_PROFILE: &str = "CONFIDENTIAL_OUTPUT_PROFILE_34c0";
    const SECRET_PAGE_TYPE: &str = "CONFIDENTIAL_PAGE_TYPE_a29e";
    let pages = b"<< /Type /Pages /Kids [] /Count 0 >>";
    let filtered_profile_body = format!("<< /N 4 /Filter /{SECRET_FILTER} /Length 1 >>\nstream\nx\nendstream");
    let undecodable_profile = PdfDocument::from_bytes(build_catalog_test_pdf(
        b"<< /Type /Catalog /Pages 2 0 R /OutputIntents [<< /DestOutputProfile 3 0 R >>] >>",
        pages,
        &[(3, filtered_profile_body.as_bytes())],
    ))
    .unwrap();
    let invalid_profile_body = format!(
        "<< /N 4 /Length {} >>\nstream\n{SECRET_PROFILE}\nendstream",
        SECRET_PROFILE.len()
    );
    let invalid_profile = PdfDocument::from_bytes(build_catalog_test_pdf(
        b"<< /Type /Catalog /Pages 2 0 R /OutputIntents [<< /DestOutputProfile 3 0 R >>] >>",
        pages,
        &[(3, invalid_profile_body.as_bytes())],
    ))
    .unwrap();
    let malformed_indirect_body = format!("<< /{SECRET_PROFILE}");
    let malformed_entry = PdfDocument::from_bytes(corrupt_third_object_header(build_catalog_test_pdf(
        b"<< /Type /Catalog /Pages 2 0 R /OutputIntents [3 0 R] >>",
        pages,
        &[(3, malformed_indirect_body.as_bytes())],
    )))
    .unwrap();
    let malformed_profile = PdfDocument::from_bytes(corrupt_third_object_header(build_catalog_test_pdf(
        b"<< /Type /Catalog /Pages 2 0 R /OutputIntents [<< /DestOutputProfile 3 0 R >>] >>",
        pages,
        &[(3, malformed_indirect_body.as_bytes())],
    )))
    .unwrap();
    let unknown_page_catalog = b"<< /Type /Catalog /Pages 2 0 R >>";
    let unknown_page_body = format!("<< /Type /{SECRET_PAGE_TYPE} /Kids [] /Count 0 >>");
    let unknown_page = PdfDocument::from_bytes(build_catalog_test_pdf(
        unknown_page_catalog,
        unknown_page_body.as_bytes(),
        &[],
    ))
    .unwrap();

    let (_, events) = capture_events(|| {
        assert!(malformed_entry.output_intent_cmyk_profile().is_none());
        assert!(malformed_profile.output_intent_cmyk_profile().is_none());
        assert!(undecodable_profile.output_intent_cmyk_profile().is_none());
        assert!(invalid_profile.output_intent_cmyk_profile().is_none());
        assert_eq!(
            unknown_page
                .count_pages_recursive(ObjectRef::new(2, 0), 0, &mut std::collections::HashSet::new())
                .unwrap(),
            0
        );
    });
    let warnings: Vec<_> = events
        .iter()
        .filter(|event| event.level == Level::WARN && event.target == crate::LOG_TARGET_ROOT)
        .collect();
    for (operation, error_code) in [
        ("load_output_intent_entry", "parse_error"),
        ("load_output_intent_profile", "parse_error"),
        ("decode_output_intent_profile", "unsupported_filter"),
        ("parse_output_intent_profile", "invalid_icc_profile"),
        ("traverse_page_tree", "unknown_node_type"),
    ] {
        assert_eq!(
            warnings
                .iter()
                .filter(|event| {
                    event.fields.get("operation").map(String::as_str) == Some(operation)
                        && event.fields.get("error_code").map(String::as_str) == Some(error_code)
                })
                .count(),
            1,
            "missing exact {operation}/{error_code} warning: {events:#?}"
        );
    }
    let rendered = format!("{events:?}");
    assert!(!rendered.contains(SECRET_FILTER));
    assert!(!rendered.contains(SECRET_PROFILE));
    assert!(!rendered.contains(SECRET_PAGE_TYPE));
}

#[test]
fn test_multiline_object_header() {
    let mut pdf = b"%PDF-1.4\n".to_vec();
    let off1 = pdf.len();
    pdf.extend_from_slice(b"1\n0\nobj\n<< /Type /Catalog /Pages 2 0 R >>\nendobj\n");
    let off2 = pdf.len();
    pdf.extend_from_slice(b"2 0 obj\n<< /Type /Pages /Kids [] /Count 0 >>\nendobj\n");
    let xref_off = pdf.len();
    pdf.extend_from_slice(b"xref\n0 3\n");
    pdf.extend_from_slice(b"0000000000 65535 f \n");
    pdf.extend_from_slice(format!("{:010} 00000 n \n", off1).as_bytes());
    pdf.extend_from_slice(format!("{:010} 00000 n \n", off2).as_bytes());
    pdf.extend_from_slice(format!("trailer\n<< /Size 3 /Root 1 0 R >>\nstartxref\n{}\n%%EOF\n", xref_off).as_bytes());
    let doc = PdfDocument::from_bytes(pdf).unwrap();
    assert!(doc.catalog().unwrap().as_dict().is_some());
}

#[test]
fn test_object_content_on_same_line() {
    let mut pdf = b"%PDF-1.4\n".to_vec();
    let off1 = pdf.len();
    pdf.extend_from_slice(b"1 0 obj<< /Type /Catalog /Pages 2 0 R >>\nendobj\n");
    let off2 = pdf.len();
    pdf.extend_from_slice(b"2 0 obj\n<< /Type /Pages /Kids [] /Count 0 >>\nendobj\n");
    let xref_off = pdf.len();
    pdf.extend_from_slice(b"xref\n0 3\n");
    pdf.extend_from_slice(b"0000000000 65535 f \n");
    pdf.extend_from_slice(format!("{:010} 00000 n \n", off1).as_bytes());
    pdf.extend_from_slice(format!("{:010} 00000 n \n", off2).as_bytes());
    pdf.extend_from_slice(format!("trailer\n<< /Size 3 /Root 1 0 R >>\nstartxref\n{}\n%%EOF\n", xref_off).as_bytes());
    let doc = PdfDocument::from_bytes(pdf).unwrap();
    assert!(doc.catalog().unwrap().as_dict().is_some());
}
