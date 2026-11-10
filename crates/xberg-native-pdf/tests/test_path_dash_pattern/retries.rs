use super::{allocations, document_with_resource_objects, object_loads};
use xberg_native_pdf::document::PdfDocument;
use xberg_native_pdf::object::ObjectRef;

fn broken_object_stream_document(uses: usize, resources: &str) -> PdfDocument {
    let content = format!("{}0 700 m 300 700 l S", "/Missing gs ".repeat(uses));
    broken_object_stream_with_content(uses, resources, &content)
}

fn broken_object_stream_with_content(uses: usize, resources: &str, content: &str) -> PdfDocument {
    let (pdf, offsets) = broken_object_stream_body(uses, resources, content);
    finish_xref(pdf, compressed_references(&offsets))
}

fn broken_object_stream_body(uses: usize, resources: &str, content: &str) -> (Vec<u8>, [u32; 8]) {
    const DECODED_BYTES_PER_USE: usize = 64;
    let decoded_size = uses * DECODED_BYTES_PER_USE;
    let encoded = "20".repeat(decoded_size);
    let objects = [
        (1, "<< /Type /Catalog /Pages 2 0 R >>".to_owned()),
        (2, "<< /Type /Pages /Kids [3 0 R] /Count 1 >>".to_owned()),
        (
            3,
            format!(
                "<< /Type /Page /Parent 2 0 R /MediaBox [0 0 612 792] /Contents 4 0 R /Resources << {resources} >> >>"
            ),
        ),
        (
            4,
            format!("<< /Length {} >>\nstream\n{content}\nendstream", content.len()),
        ),
        (
            6,
            format!(
                "<< /Type /ObjStm /N 1 /First {} /Filter /ASCIIHexDecode /Length {} >>\nstream\n{encoded}\nendstream",
                decoded_size + 1,
                encoded.len()
            ),
        ),
    ];
    let mut pdf = b"%PDF-1.5\n".to_vec();
    let mut offsets = [0_u32; 8];
    for (id, object) in objects {
        offsets[id] = append_object(&mut pdf, id, &object);
    }
    (pdf, offsets)
}

fn append_object(pdf: &mut Vec<u8>, id: usize, object: &str) -> u32 {
    let offset = u32::try_from(pdf.len()).unwrap();
    pdf.extend_from_slice(format!("{id} 0 obj\n{object}\nendobj\n").as_bytes());
    offset
}

fn compressed_references(offsets: &[u32]) -> Vec<(u8, u32, u16)> {
    offsets
        .iter()
        .enumerate()
        .map(|(id, offset)| match id {
            0 => (0_u8, 0_u32, 65535_u16),
            5 => (2, 6, 0),
            _ => (1, *offset, 0),
        })
        .collect()
}

fn finish_xref(mut pdf: Vec<u8>, mut references: Vec<(u8, u32, u16)>) -> PdfDocument {
    let xref_offset = pdf.len();
    references[7] = (1, u32::try_from(xref_offset).unwrap(), 0);
    let mut xref = Vec::new();
    for (kind, value, index) in &references {
        xref.push(*kind);
        xref.extend_from_slice(&value.to_be_bytes());
        xref.extend_from_slice(&index.to_be_bytes());
    }
    pdf.extend_from_slice(
        format!(
            "7 0 obj\n<< /Type /XRef /Root 1 0 R /Size {} /W [1 4 2] /Length {} >>\nstream\n",
            references.len(),
            xref.len()
        )
        .as_bytes(),
    );
    pdf.extend_from_slice(&xref);
    pdf.extend_from_slice(format!("\nendstream\nendobj\nstartxref\n{xref_offset}\n%%EOF\n").as_bytes());
    PdfDocument::from_bytes(pdf).unwrap()
}

fn assert_bounded_failure_loads(resources: &str) {
    for uses in [128, 256] {
        let doc = broken_object_stream_document(uses, resources);
        let error = doc
            .load_object(ObjectRef::new(5, 0))
            .expect_err("broken ObjStm must fail after decoding");
        assert!(error.to_string().contains("data too short"), "{error}");
        let (paths, calls) = object_loads::count(5, || doc.extract_paths(0).unwrap());
        assert_eq!(paths.len(), 1);
        assert_eq!(calls, 1, "failed references may load once per recovery epoch");
    }
}

#[test]
fn broken_indirect_container_loads_are_bounded() {
    assert_bounded_failure_loads("/ExtGState 5 0 R");
}

#[test]
fn broken_indirect_named_state_loads_are_bounded() {
    assert_bounded_failure_loads("/ExtGState << /Missing 5 0 R >>");
}

#[test]
fn broken_indirect_dash_loads_are_bounded() {
    assert_bounded_failure_loads("/ExtGState << /Missing << /D 5 0 R >> >>");
}

#[test]
fn broken_object_stream_allocation_bytes_grow_linearly() {
    for resources in [
        "/ExtGState 5 0 R",
        "/ExtGState << /Missing 5 0 R >>",
        "/ExtGState << /Missing << /D 5 0 R >> >>",
    ] {
        let measurements = [128, 256].map(|uses| {
            let doc = broken_object_stream_document(uses, resources);
            let (paths, allocations, bytes) = allocations::measure(|| doc.extract_paths(0).unwrap());
            assert_eq!(paths.len(), 1);
            (allocations, bytes)
        });
        assert!(
            measurements[1].0 < measurements[0].0 * 3,
            "allocation growth: {measurements:?}"
        );
        assert!(
            measurements[1].1 < measurements[0].1 * 3,
            "quadratic failed-stream work: {measurements:?}"
        );
    }
}

#[test]
fn missing_name_in_a_valid_container_has_linear_cost_and_one_container_load() {
    let bytes = [128, 256].map(|uses| {
        let content = format!("{}0 700 m 300 700 l S", "/Missing gs ".repeat(uses));
        let entries = "/Other << /CA 1 >> ".repeat(uses);
        let dictionary = format!("<< {entries} >>");
        let doc = document_with_resource_objects(content.as_bytes(), "/ExtGState 5 0 R", &[&dictionary]);
        let (paths, calls) = object_loads::count(5, || doc.extract_paths(0).unwrap());
        assert_eq!(calls, 1);
        assert_eq!(paths.len(), 1);
        let (_, _, bytes) = allocations::measure(|| doc.extract_paths(0).unwrap());
        bytes
    });
    assert!(bytes[1] < bytes[0] * 3, "missing-name allocation growth: {bytes:?}");
}

#[test]
fn distinct_names_sharing_a_failed_reference_load_it_once() {
    for uses in [128, 256] {
        let entries = (0..uses).map(|id| format!("/S{id} 5 0 R ")).collect::<String>();
        let content = format!(
            "{}0 700 m 300 700 l S",
            (0..uses).map(|id| format!("/S{id} gs ")).collect::<String>()
        );
        let doc = broken_object_stream_with_content(uses, &format!("/ExtGState << {entries} >>"), &content);
        let (paths, calls) = object_loads::count(5, || doc.extract_paths(0).unwrap());
        assert_eq!(paths.len(), 1);
        assert_eq!(calls, 1, "failed-reference aliases share a bounded retry");
    }
}

#[test]
fn an_ordinary_form_resource_load_does_not_invalidate_failures() {
    let form = "0 50 m 100 50 l S";
    let form_object = format!(
        "<< /Type /XObject /Subtype /Form /BBox [0 0 612 792] /Resources 7 0 R /Length {} >>\nstream\n{form}\nendstream",
        form.len()
    );
    let doc = document_with_resource_objects(
        b"/Missing gs /F Do /After gs 0 700 m 300 700 l S",
        "/ExtGState 5 0 R /XObject << /F 6 0 R >>",
        &["]", &form_object, "<< >>"],
    );
    assert_eq!(
        doc.load_object(ObjectRef::new(5, 0)).unwrap(),
        xberg_native_pdf::object::Object::Null
    );
    let (paths, calls) = object_loads::count(5, || doc.extract_paths(0).unwrap());
    assert_eq!(paths.len(), 2, "actual Form and page visit");
    assert_eq!(calls, 1, "normal object loads do not advance recovery epoch");
}

#[test]
fn successful_indirect_container_is_loaded_once_for_distinct_names() {
    let entries = (0..128).map(|id| format!("/S{id} << /CA 1 >> ")).collect::<String>();
    let content = format!(
        "{}0 700 m 300 700 l S",
        (0..128).map(|id| format!("/S{id} gs ")).collect::<String>()
    );
    let doc = document_with_resource_objects(content.as_bytes(), "/ExtGState 5 0 R", &[&format!("<< {entries} >>")]);
    let (paths, calls) = object_loads::count(5, || doc.extract_paths(0).unwrap());
    assert_eq!(paths.len(), 1);
    assert_eq!(calls, 1, "successful container caching is required too");
}

#[test]
fn an_indirect_named_patch_is_not_reloaded_after_a_form() {
    let form = "/GS gs 0 50 m 100 50 l S";
    let form_object = format!(
        "<< /Type /XObject /Subtype /Form /BBox [0 0 612 792] /Resources << /ExtGState 7 0 R >> /Length {} >>\nstream\n{form}\nendstream",
        form.len()
    );
    let doc = document_with_resource_objects(
        b"/GS gs 0 700 m 300 700 l S /F Do /GS gs 0 690 m 300 690 l S",
        "/ExtGState 5 0 R /XObject << /F 6 0 R >>",
        &[
            "<< /GS 8 0 R >>",
            &form_object,
            "<< /GS << /CA 0 >> >>",
            "<< /CA 1 /D [[3 2] 1] >>",
        ],
    );
    let (paths, calls) = object_loads::count(8, || doc.extract_paths(0).unwrap());
    assert_eq!(paths.len(), 3);
    assert_eq!(paths[0].dash_pattern, Some((vec![3.0, 2.0], 1.0)));
    assert_eq!(paths[1].stroke_color, None);
    assert_eq!(paths[2].dash_pattern, Some((vec![3.0, 2.0], 1.0)));
    assert_eq!(calls, 1, "retain the page's parsed indirect patch across the Form");
}

fn interleaved_compressed_resources(forms: usize) -> PdfDocument {
    let names = (0..forms)
        .map(|id| format!("/F{id} {} 0 R ", 8 + id * 3))
        .collect::<String>();
    let content = format!(
        "{}/Missing gs 0 700 m 300 700 l S",
        (0..forms)
            .map(|id| format!("/Missing gs /F{id} Do "))
            .collect::<String>()
    );
    let resources = format!("/ExtGState 5 0 R /XObject << {names} >>");
    let (mut pdf, offsets) = broken_object_stream_body(forms, &resources, &content);
    let mut references = compressed_references(&offsets);
    references.resize(8 + forms * 3, (0, 0, 0));
    for index in 0..forms {
        let form = 8 + index * 3;
        let resource = form + 1;
        let stream = form + 2;
        let form_content = "0 50 m 100 50 l S";
        let form_object = format!(
            "<< /Type /XObject /Subtype /Form /BBox [0 0 612 792] /Resources {resource} 0 R /Length {} >>\nstream\n{form_content}\nendstream",
            form_content.len()
        );
        references[form] = (1, append_object(&mut pdf, form, &form_object), 0);
        let header = format!("{resource} 0 ");
        let body = format!("{header}<< >>");
        let object_stream = format!(
            "<< /Type /ObjStm /N 1 /First {} /Length {} >>\nstream\n{body}\nendstream",
            header.len(),
            body.len()
        );
        references[stream] = (1, append_object(&mut pdf, stream, &object_stream), 0);
        references[resource] = (2, u32::try_from(stream).unwrap(), 0);
    }
    finish_xref(pdf, references)
}

#[test]
fn unrelated_compressed_publications_do_not_retry_a_permanent_failure() {
    for forms in [128, 256] {
        let probe = interleaved_compressed_resources(forms);
        probe
            .load_object(ObjectRef::new(5, 0))
            .expect_err("permanent broken ObjStm");
        for resource in (9..8 + forms * 3).step_by(3) {
            assert!(
                probe
                    .load_object(ObjectRef::new(u32::try_from(resource).unwrap(), 0))
                    .unwrap()
                    .as_dict()
                    .is_some()
            );
        }
        let doc = interleaved_compressed_resources(forms);
        let (paths, calls) = object_loads::count(5, || doc.extract_paths(0).unwrap());
        assert_eq!(paths.len(), forms + 1, "every distinct Form is visited");
        assert_eq!(
            calls, 1,
            "unrelated object publication must not reopen a permanent failure"
        );
    }
}

#[test]
fn interleaved_unrelated_publication_allocation_growth_is_linear() {
    let measurements = [128, 256].map(|forms| {
        let doc = interleaved_compressed_resources(forms);
        let (paths, count, bytes) = allocations::measure(|| doc.extract_paths(0).unwrap());
        assert_eq!(paths.len(), forms + 1);
        (count, bytes)
    });
    assert!(
        measurements[1].0 < measurements[0].0 * 3,
        "interleaved allocations: {measurements:?}"
    );
    assert!(
        measurements[1].1 < measurements[0].1 * 3,
        "interleaved bytes: {measurements:?}"
    );
}
