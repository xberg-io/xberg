//! A genuine container load failure must leave current graphics fields unchanged.
mod common;

use xberg_native_pdf::document::PdfDocument;
use xberg_native_pdf::layout::Color;
use xberg_native_pdf::object::ObjectRef;

#[test]
fn mismatched_indirect_container_keeps_inherited_fields_across_distinct_names() {
    let content = concat!(
        "[3 2] 1 d /A gs 0 10 m 100 10 l S ",
        "q [7 1] 2 d /B gs 0 20 m 100 20 l S Q ",
        "/C gs 0 30 m 100 30 l S"
    );
    let objects = [
        "<< /Type /Catalog /Pages 2 0 R >>".to_owned(),
        "<< /Type /Pages /Kids [3 0 R] /Count 1 >>".to_owned(),
        "<< /Type /Page /Parent 2 0 R /MediaBox [0 0 612 792] /Contents 4 0 R /Resources << /ExtGState 5 0 R >> >>"
            .to_owned(),
        format!("<< /Length {} >>\nstream\n{content}\nendstream", content.len()),
        "]".to_owned(),
    ];
    let mut bytes = b"%PDF-1.4\n".to_vec();
    let mut offsets = vec![0];
    for (index, object) in objects.iter().enumerate() {
        offsets.push(bytes.len());
        let object_id = if index == 4 { 6 } else { index + 1 };
        bytes.extend_from_slice(format!("{object_id} 0 obj\n{object}\nendobj\n").as_bytes());
    }
    let doc = PdfDocument::from_bytes(common::finalize_pdf(bytes, &offsets)).unwrap();
    let error = doc
        .load_object(ObjectRef::new(5, 0))
        .expect_err("actual header failure, not missing-object Ok(Null)");
    assert!(error.to_string().contains("Object reference mismatch"), "{error}");
    let paths = doc.extract_paths(0).unwrap();
    assert_eq!(paths.len(), 3);
    for (index, (lengths, phase)) in [(vec![3.0, 2.0], 1.0), (vec![7.0, 1.0], 2.0), (vec![3.0, 2.0], 1.0)]
        .into_iter()
        .enumerate()
    {
        assert_eq!(paths[index].dash_pattern, Some((lengths, phase)));
        assert_eq!(paths[index].stroke_color, Some(Color::black()));
    }
}

fn later_recovered_container_pdf(missing_entry: bool) -> Vec<u8> {
    let content = "/Before gs 0 10 m 100 10 l S /F Do /After gs 0 30 m 100 30 l S";
    recovered_object_pdf(
        missing_entry,
        content,
        "/ExtGState 5 0 R",
        "<< /Before << /CA 1 >> /After << /CA 0 >> >>",
    )
}

fn recovered_object_pdf(missing_entry: bool, content: &str, resources: &str, container: &str) -> Vec<u8> {
    let form = "0 20 m 100 20 l S";
    let index = format!("5 0 7 {} ", container.len() + 1);
    let body = format!("{index}{container} << >>");
    let objects = [
        (1, "<< /Type /Catalog /Pages 2 0 R >>".to_owned()),
        (2, "<< /Type /Pages /Kids [3 0 R] /Count 1 >>".to_owned()),
        (
            3,
            format!(
                "<< /Type /Page /Parent 2 0 R /MediaBox [0 0 612 792] /Contents 4 0 R /Resources << {resources} /XObject << /F 6 0 R >> >> >>"
            ),
        ),
        (
            4,
            format!("<< /Length {} >>\nstream\n{content}\nendstream", content.len()),
        ),
        (
            6,
            format!(
                "<< /Type /XObject /Subtype /Form /BBox [0 0 612 792] /Resources 7 0 R /Length {} >>\nstream\n{form}\nendstream",
                form.len()
            ),
        ),
        (
            8,
            format!(
                "<< /Type /ObjStm /N 2 /First {} /Length {} >>\nstream\n{body}\nendstream",
                index.len(),
                body.len()
            ),
        ),
    ];
    let mut bytes = b"%PDF-1.5\n".to_vec();
    let mut offsets = [0; 9];
    for (id, object) in objects {
        offsets[id] = bytes.len();
        bytes.extend_from_slice(format!("{id} 0 obj\n{object}\nendobj\n").as_bytes());
    }
    let xref = bytes.len();
    let header = if missing_entry { "xref\n0 5\n" } else { "xref\n0 9\n" };
    bytes.extend_from_slice(header.as_bytes());
    bytes.extend_from_slice(b"0000000000 65535 f\r\n");
    for (id, offset) in offsets.iter().enumerate().skip(1) {
        if missing_entry && id == 5 {
            bytes.extend_from_slice(b"6 3\n");
            continue;
        }
        match id {
            5 => bytes.extend_from_slice(b"9999999999 00000 n\r\n"),
            7 => bytes.extend_from_slice(b"0000000000 00000 f\r\n"),
            _ => bytes.extend_from_slice(format!("{offset:010} 00000 n\r\n").as_bytes()),
        }
    }
    bytes.extend_from_slice(format!("trailer\n<< /Size 9 /Root 1 0 R >>\nstartxref\n{xref}\n%%EOF\n").as_bytes());
    bytes
}

#[test]
fn a_later_name_can_use_a_container_recovered_while_loading_form_resources() {
    let bytes = later_recovered_container_pdf(false);
    let probe = PdfDocument::from_bytes(bytes.clone()).unwrap();
    let error = probe
        .load_object(ObjectRef::new(5, 0))
        .expect_err("initial bad offset must fail");
    assert!(
        matches!(error, xberg_native_pdf::error::Error::UnexpectedEof),
        "{error}"
    );
    assert!(probe.load_object(ObjectRef::new(7, 0)).unwrap().as_dict().is_some());
    assert!(probe.load_object(ObjectRef::new(5, 0)).unwrap().as_dict().is_some());
    let doc = PdfDocument::from_bytes(bytes).unwrap();
    let paths = doc.extract_paths(0).unwrap();
    assert_eq!(paths.len(), 3, "page, actual Form visit, page");
    assert_eq!(paths[0].stroke_color, Some(Color::black()));
    assert_eq!(paths[1].stroke_color, Some(Color::black()));
    assert_eq!(
        paths[2].stroke_color, None,
        "recovered /After /CA 0 must hide the final stroke"
    );
}

#[test]
fn a_later_name_can_use_a_null_container_recovered_from_an_object_stream() {
    let bytes = later_recovered_container_pdf(true);
    let probe = PdfDocument::from_bytes(bytes.clone()).unwrap();
    assert_eq!(
        probe.load_object(ObjectRef::new(5, 0)).unwrap(),
        xberg_native_pdf::object::Object::Null
    );
    assert!(probe.load_object(ObjectRef::new(7, 0)).unwrap().as_dict().is_some());
    assert!(probe.load_object(ObjectRef::new(5, 0)).unwrap().as_dict().is_some());
    let doc = PdfDocument::from_bytes(bytes).unwrap();
    let paths = doc.extract_paths(0).unwrap();
    assert_eq!(paths.len(), 3, "page, actual Form visit, page");
    assert_eq!(paths[0].stroke_color, Some(Color::black()));
    assert_eq!(paths[1].stroke_color, Some(Color::black()));
    assert_eq!(
        paths[2].stroke_color, None,
        "a recovered Null placeholder must not hide /After /CA 0"
    );
}

#[test]
fn the_same_name_can_use_a_container_recovered_while_loading_form_resources() {
    for missing_entry in [false, true] {
        let bytes = later_recovered_container_pdf(missing_entry);
        let source = String::from_utf8(bytes).unwrap();
        let modified = source.replace("/Before gs", "/After gs ");
        assert_eq!(source.len(), modified.len(), "preserve stream lengths and xref offsets");
        let doc = PdfDocument::from_bytes(modified.into_bytes()).unwrap();
        let paths = doc.extract_paths(0).unwrap();
        assert_eq!(paths.len(), 3, "page, actual Form visit, page");
        assert_eq!(paths[0].stroke_color, Some(Color::black()));
        assert_eq!(paths[1].stroke_color, Some(Color::black()));
        assert_eq!(
            paths[2].stroke_color, None,
            "same-name recovery, missing_entry={missing_entry}"
        );
    }
}

#[test]
fn nested_indirect_dash_values_retry_after_actual_object_recovery() {
    for missing_entry in [false, true] {
        for (dash, recovered) in [
            ("5 0 R", "[[7 2] 1]"),
            ("[5 0 R 1]", "[7 2]"),
            ("[[7 2] 5 0 R]", "1"),
            ("[[5 0 R 2] 1]", "7"),
        ] {
            let resources = format!("/ExtGState << /GS << /D {dash} /CA 1 >> >>");
            let bytes = recovered_object_pdf(
                missing_entry,
                "/GS gs 0 10 m 100 10 l S /F Do /GS gs 0 30 m 100 30 l S",
                &resources,
                recovered,
            );
            let doc = PdfDocument::from_bytes(bytes).unwrap();
            let paths = doc.extract_paths(0).unwrap();
            assert_eq!(paths.len(), 3, "page, actual Form visit, page");
            assert_eq!(paths[0].dash_pattern, None);
            assert_eq!(paths[1].dash_pattern, None);
            assert_eq!(
                paths[2].dash_pattern,
                Some((vec![7.0, 2.0], 1.0)),
                "{dash}, missing={missing_entry}"
            );
        }
    }
}

#[test]
fn nested_indirect_opacity_retries_after_actual_object_recovery() {
    for missing_entry in [false, true] {
        let bytes = recovered_object_pdf(
            missing_entry,
            "/GS gs 0 10 m 100 10 l S /F Do /GS gs 0 30 m 100 30 l S",
            "/ExtGState << /GS << /CA 5 0 R >> >>",
            "0",
        );
        let paths = PdfDocument::from_bytes(bytes).unwrap().extract_paths(0).unwrap();
        assert_eq!(paths.len(), 3);
        assert_eq!(paths[0].stroke_color, Some(Color::black()));
        assert_eq!(paths[1].stroke_color, Some(Color::black()));
        assert_eq!(paths[2].stroke_color, None, "missing={missing_entry}");
    }
}

fn compressed_resource_recovery_pdf() -> Vec<u8> {
    let original = later_recovered_container_pdf(true);
    let source = String::from_utf8(original).unwrap();
    let (prefix, _) = source.rsplit_once("xref\n0 5\n").expect("traditional xref fixture");
    let mut entries = Vec::new();
    for id in [0, 1, 2, 3, 4, 6, 7, 8, 9] {
        let (kind, offset, index) = match id {
            0 => (0_u8, 0_u32, 65535_u16),
            7 => (2, 8, 1),
            9 => (1, u32::try_from(prefix.len()).unwrap(), 0),
            _ => (
                1,
                u32::try_from(prefix.find(&format!("{id} 0 obj\n")).expect("object header")).unwrap(),
                0,
            ),
        };
        entries.push(kind);
        entries.extend_from_slice(&offset.to_be_bytes());
        entries.extend_from_slice(&index.to_be_bytes());
    }
    let mut bytes = prefix.as_bytes().to_vec();
    bytes.extend_from_slice(
        format!(
            "9 0 obj\n<< /Type /XRef /Root 1 0 R /Size 10 /Index [0 5 6 4] /W [1 4 2] /Length {} >>\nstream\n",
            entries.len()
        )
        .as_bytes(),
    );
    bytes.extend_from_slice(&entries);
    bytes.extend_from_slice(format!("\nendstream\nendobj\nstartxref\n{}\n%%EOF\n", prefix.len()).as_bytes());
    bytes
}

#[test]
fn a_normal_compressed_resource_load_also_invalidates_failed_page_lookups() {
    let bytes = compressed_resource_recovery_pdf();
    let probe = PdfDocument::from_bytes(bytes.clone()).unwrap();
    assert_eq!(
        probe.load_object(ObjectRef::new(5, 0)).unwrap(),
        xberg_native_pdf::object::Object::Null
    );
    assert!(probe.load_object(ObjectRef::new(7, 0)).unwrap().as_dict().is_some());
    assert!(probe.load_object(ObjectRef::new(5, 0)).unwrap().as_dict().is_some());
    let paths = PdfDocument::from_bytes(bytes).unwrap().extract_paths(0).unwrap();
    assert_eq!(paths.len(), 3);
    assert_eq!(paths[0].stroke_color, Some(Color::black()));
    assert_eq!(paths[1].stroke_color, Some(Color::black()));
    assert_eq!(
        paths[2].stroke_color, None,
        "normal compressed loading must publish a recovery epoch"
    );
}
