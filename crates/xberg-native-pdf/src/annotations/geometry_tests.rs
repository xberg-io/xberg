//! Geometry reading and polygon-writing tests for [`super`].
//!
//! Kept in a child module so the annotation source stays inside the file-size limit.

use super::*;

/// Build a one-page PDF and return its bytes plus the next free object id.
///
/// Objects 1-4 are the catalog, pages node, page and empty content stream;
/// callers append their own objects starting at id 5 and then call
/// [`finish_pdf`] with the offsets they recorded.
fn skeleton_pdf(extra_page_entries: &str) -> Vec<u8> {
    let mut pdf = b"%PDF-1.6\n".to_vec();
    let offsets = [
        (1, "<< /Type /Catalog /Pages 2 0 R >>"),
        (2, "<< /Type /Pages /Kids [3 0 R] /Count 1 >>"),
        (3, ""),
        (4, "<< /Length 0 >>\nstream\n\nendstream"),
    ];
    let mut written = Vec::new();
    for (id, body) in offsets {
        let offset = pdf.len();
        if id == 3 {
            pdf.extend_from_slice(
                format!(
                    "3 0 obj\n<< /Type /Page /Parent 2 0 R /MediaBox [0 0 612 792] /Resources << >> /Contents 4 0 R {extra_page_entries} >>\nendobj\n"
                )
                .as_bytes(),
            );
        } else {
            pdf.extend_from_slice(format!("{id} 0 obj\n{body}\nendobj\n").as_bytes());
        }
        written.push((id, offset));
    }
    finish_pdf(pdf, written)
}

fn finish_pdf(mut pdf: Vec<u8>, offsets: Vec<(u32, usize)>) -> Vec<u8> {
    let xref_offset = pdf.len();
    let size = offsets.iter().map(|(id, _)| *id).max().unwrap_or(0) + 1;
    pdf.extend_from_slice(format!("xref\n0 {size}\n").as_bytes());
    pdf.extend_from_slice(b"0000000000 65535 f \n");
    let mut by_id = offsets;
    by_id.sort_by_key(|(id, _)| *id);
    let mut next = 1;
    for (id, offset) in by_id {
        while next < id {
            pdf.extend_from_slice(b"0000000000 00000 f \n");
            next += 1;
        }
        pdf.extend_from_slice(format!("{offset:010} 00000 n \n").as_bytes());
        next = id + 1;
    }
    pdf.extend_from_slice(
        format!("trailer\n<< /Size {size} /Root 1 0 R >>\nstartxref\n{xref_offset}\n%%EOF\n").as_bytes(),
    );
    pdf
}

fn polygon_document() -> Vec<u8> {
    let mut pdf = b"%PDF-1.6\n".to_vec();
    let mut offsets = Vec::new();
    for (id, body) in [
        (1, "<< /Type /Catalog /Pages 2 0 R >>".to_string()),
        (2, "<< /Type /Pages /Kids [3 0 R] /Count 1 >>".to_string()),
        (
            3,
            "<< /Type /Page /Parent 2 0 R /MediaBox [0 0 612 792] /Contents 4 0 R \
             /VP [5 0 R] /Annots [6 0 R 7 0 R] >>"
                .to_string(),
        ),
        (4, "<< /Length 0 >>\nstream\n\nendstream".to_string()),
        (
            5,
            "<< /Type /Viewport /BBox [10 20 300 400] /Name (PLAN) \
             /Measure << /Type /Measure /Subtype /RL /R (3/8 in = 1 ft) \
             /X [1 1 0] /Y [2 3 4] >> >>"
                .to_string(),
        ),
        (
            6,
            "<< /Type /Annot /Subtype /Polygon /Rect [0 0 100 100] \
             /Vertices [0 0 100 0 100 50 0 50] /Contents (Room 101) \
             /Measure << /Type /Measure /Subtype /RL /R (1/4 in = 1 ft) /A [1 2] >> >>"
                .to_string(),
        ),
        (
            7,
            "<< /Type /Annot /Subtype /Ink /Rect [0 0 10 10] \
             /InkList [[1 1 2 2 3 1] [5 5 6 6]] >>"
                .to_string(),
        ),
    ] {
        let offset = pdf.len();
        pdf.extend_from_slice(format!("{id} 0 obj\n{body}\nendobj\n").as_bytes());
        offsets.push((id, offset));
    }
    finish_pdf(pdf, offsets)
}

#[test]
fn reads_polygon_vertices_measure_and_ink_list() {
    let bytes = polygon_document();
    let document = PdfDocument::from_bytes(bytes).expect("the document opens");
    let annotations = document.get_annotations(0).expect("annotations are read");

    let polygon = annotations
        .iter()
        .find(|annotation| annotation.subtype_enum == AnnotationSubtype::Polygon)
        .expect("the polygon is present");
    assert_eq!(
        polygon.vertices.as_deref(),
        Some([[0.0, 0.0], [100.0, 0.0], [100.0, 50.0], [0.0, 50.0]].as_slice())
    );
    assert_eq!(polygon.contents.as_deref(), Some("Room 101"));
    let measure = polygon.measure.as_ref().expect("the measure dictionary is read");
    assert_eq!(measure.subtype.as_deref(), Some("RL"));
    assert_eq!(measure.scale_ratio.as_deref(), Some("1/4 in = 1 ft"));
    assert_eq!(measure.area.as_deref(), Some([1.0, 2.0].as_slice()));

    let ink = annotations
        .iter()
        .find(|annotation| annotation.subtype_enum == AnnotationSubtype::Ink)
        .expect("the ink annotation is present");
    assert_eq!(
        ink.ink_list.as_deref(),
        Some([vec![[1.0, 1.0], [2.0, 2.0], [3.0, 1.0]], vec![[5.0, 5.0], [6.0, 6.0]],].as_slice())
    );
    assert!(ink.vertices.is_none(), "ink geometry lives in /InkList");
}

#[test]
fn reads_page_viewports_with_bounds_name_and_scale() {
    let bytes = polygon_document();
    let document = PdfDocument::from_bytes(bytes).expect("the document opens");
    let viewports = document.get_page_viewports(0).expect("viewports are read");
    assert_eq!(
        viewports,
        vec![PageViewport {
            bbox: [10.0, 20.0, 300.0, 400.0],
            name: Some("PLAN".to_string()),
            measure: Some(MeasureDictionary {
                subtype: Some("RL".to_string()),
                scale_ratio: Some("3/8 in = 1 ft".to_string()),
                x: Some(vec![1.0, 1.0, 0.0]),
                y: Some(vec![2.0, 3.0, 4.0]),
                ..MeasureDictionary::default()
            }),
        }]
    );
}

#[test]
fn appending_a_polygon_preserves_the_document_and_round_trips() {
    let original = skeleton_pdf("");
    let document = PdfDocument::from_bytes(original.clone()).expect("the document opens");
    assert!(document.get_annotations(0).expect("annotations are read").is_empty());

    let mut annotation = PolygonAnnotation::new(vec![[10.0, 10.0], [110.0, 10.0], [110.0, 60.0], [10.0, 60.0]]);
    annotation.contents = Some("Measured room".to_string());
    annotation.measure = Some(MeasureDictionary {
        subtype: Some("RL".to_string()),
        scale_ratio: Some("1/4 in = 1 ft".to_string()),
        area: Some(vec![1.0, 144.0]),
        ..MeasureDictionary::default()
    });

    let written = document
        .append_polygon_annotation(0, &annotation)
        .expect("the annotation is appended");
    assert!(written.starts_with(&original), "the original bytes must be preserved");

    let reopened = PdfDocument::from_bytes(written).expect("the written document opens");
    assert_eq!(reopened.page_count().expect("page count"), 1);
    let annotations = reopened.get_annotations(0).expect("annotations are read");
    assert_eq!(annotations.len(), 1);
    let polygon = &annotations[0];
    assert_eq!(polygon.subtype_enum, AnnotationSubtype::Polygon);
    assert_eq!(polygon.contents.as_deref(), Some("Measured room"));
    assert_eq!(
        polygon.vertices.as_deref(),
        Some([[10.0, 10.0], [110.0, 10.0], [110.0, 60.0], [10.0, 60.0]].as_slice())
    );
    assert_eq!(polygon.rect, Some([10.0, 10.0, 110.0, 60.0]));
    let measure = polygon.measure.as_ref().expect("the measure dictionary survives");
    assert_eq!(measure.scale_ratio.as_deref(), Some("1/4 in = 1 ft"));
    assert_eq!(measure.area.as_deref(), Some([1.0, 144.0].as_slice()));
}

#[test]
fn appending_to_a_document_that_already_has_annotations_keeps_them() {
    let document = PdfDocument::from_bytes(polygon_document()).expect("the document opens");
    let before = document.get_annotations(0).expect("annotations are read").len();

    let written = document
        .append_polygon_annotation(0, &PolygonAnnotation::new(vec![[1.0, 1.0], [2.0, 1.0], [2.0, 2.0]]))
        .expect("the annotation is appended");
    let reopened = PdfDocument::from_bytes(written).expect("the written document opens");
    let after = reopened.get_annotations(0).expect("annotations are read");
    assert_eq!(after.len(), before + 1, "existing annotations must survive");
    assert_eq!(reopened.get_page_viewports(0).expect("viewports survive").len(), 1);
}

#[test]
fn appending_rejects_a_polygon_with_too_few_vertices() {
    let document = PdfDocument::from_bytes(skeleton_pdf("")).expect("the document opens");
    let error = document
        .append_polygon_annotation(0, &PolygonAnnotation::new(vec![[0.0, 0.0], [1.0, 1.0]]))
        .expect_err("two vertices are not a polygon");
    assert!(error.to_string().contains("three vertices"), "got: {error}");
}

#[test]
fn measure_number_arrays_flatten_nested_entries() {
    let mut dict = std::collections::HashMap::new();
    dict.insert(
        "X".to_string(),
        Object::Array(vec![
            Object::Integer(1),
            Object::Array(vec![Object::Real(2.5), Object::Integer(3)]),
        ]),
    );
    let measure =
        PdfDocument::parse_measure_dictionary(Some(&Object::Dictionary(dict))).expect("the measure dictionary parses");
    assert_eq!(measure.x.as_deref(), Some([1.0, 2.5, 3.0].as_slice()));
}

#[test]
fn an_odd_coordinate_is_dropped_rather_than_invented() {
    let mut dict = std::collections::HashMap::new();
    dict.insert(
        "Vertices".to_string(),
        Object::Array(vec![Object::Integer(0), Object::Integer(0), Object::Integer(10)]),
    );
    let vertices = PdfDocument::parse_annotation_vertices(&dict, AnnotationSubtype::Polygon);
    assert_eq!(vertices.as_deref(), Some([[0.0, 0.0]].as_slice()));
}

/// External structural gate for the writer.
///
/// Ignored by default because it needs an external tool. Run it with
/// `XBERG_PDF_STRUCTURAL_CHECK_OUT=/tmp/append.pdf cargo test -p
/// xberg-native-pdf --lib appended_polygon -- --ignored`; it writes the
/// document, runs `qpdf --check` (or `mutool info` when qpdf is absent) and
/// fails when the tool reports an error.
#[test]
#[ignore = "runs an external qpdf/mutool structural check when explicitly requested"]
fn appended_polygon_passes_an_external_structural_check_when_available() {
    let Some(output) = std::env::var_os("XBERG_PDF_STRUCTURAL_CHECK_OUT") else {
        panic!("set XBERG_PDF_STRUCTURAL_CHECK_OUT to the path to write");
    };
    let document = PdfDocument::from_bytes(skeleton_pdf("")).expect("the document opens");
    let written = document
        .append_polygon_annotation(
            0,
            &PolygonAnnotation::new(vec![[10.0, 10.0], [100.0, 10.0], [100.0, 80.0], [10.0, 80.0]]),
        )
        .expect("the annotation is appended");
    std::fs::write(&output, &written).expect("the document is written");

    let qpdf = std::process::Command::new("qpdf").arg("--check").arg(&output).output();
    match qpdf {
        Ok(result) => {
            let stderr = String::from_utf8_lossy(&result.stderr);
            let stdout = String::from_utf8_lossy(&result.stdout);
            assert!(
                result.status.success(),
                "qpdf rejected the written document: {stdout}\n{stderr}"
            );
        }
        Err(_) => {
            let mutool = std::process::Command::new("mutool")
                .arg("info")
                .arg(&output)
                .output()
                .expect("qpdf is absent and mutool is not installed");
            assert!(
                mutool.status.success(),
                "mutool rejected the written document: {}",
                String::from_utf8_lossy(&mutool.stderr)
            );
        }
    }
}
