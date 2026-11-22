//! Annotation geometry and measure data on the public result.
//!
//! The native reader parses `/Vertices` and `/Measure`; this test goes through
//! `pdf_options.extract_annotations` so the mapping into `PdfAnnotation` is
//! covered end to end.

#![cfg(feature = "pdf")]

mod helpers;

use helpers::extract_bytes_document_blocking;
use xberg::core::config::PdfConfig;
use xberg::{ExtractionConfig, PdfAnnotationType};

const PDF_MIME: &str = "application/pdf";

/// One-page PDF with a Polygon annotation carrying four vertices and a measure
/// dictionary.
fn measured_polygon_fixture() -> Vec<u8> {
    let mut pdf = b"%PDF-1.6\n".to_vec();
    let objects: [(u32, &[u8]); 5] = [
        (1, b"<< /Type /Catalog /Pages 2 0 R >>"),
        (2, b"<< /Type /Pages /Kids [3 0 R] /Count 1 >>"),
        (
            3,
            b"<< /Type /Page /Parent 2 0 R /MediaBox [0 0 612 792] /Resources << >> /Contents 4 0 R /Annots [5 0 R] >>",
        ),
        (4, b"<< /Length 0 >>\nstream\n\nendstream"),
        (
            5,
            b"<< /Type /Annot /Subtype /Polygon /Rect [10 10 110 60] /Vertices [10 10 110 10 110 60 10 60] /Contents (Room 101) /Measure << /Type /Measure /Subtype /RL /R (1/4 in = 1 ft) /A [1 144] >> >>",
        ),
    ];
    let mut offsets = Vec::new();
    for (id, body) in objects {
        let offset = pdf.len();
        pdf.extend_from_slice(format!("{id} 0 obj\n").as_bytes());
        pdf.extend_from_slice(body);
        pdf.extend_from_slice(b"\nendobj\n");
        offsets.push(offset);
    }
    let xref = pdf.len();
    pdf.extend_from_slice(b"xref\n0 6\n0000000000 65535 f \n");
    for offset in offsets {
        pdf.extend_from_slice(format!("{offset:010} 00000 n \n").as_bytes());
    }
    pdf.extend_from_slice(format!("trailer\n<< /Size 6 /Root 1 0 R >>\nstartxref\n{xref}\n%%EOF\n").as_bytes());
    pdf
}

#[test]
fn a_measured_polygon_keeps_its_vertices_and_scale() {
    let config = ExtractionConfig {
        pdf_options: Some(PdfConfig {
            extract_annotations: true,
            ..PdfConfig::default()
        }),
        ..ExtractionConfig::default()
    };
    let document =
        extract_bytes_document_blocking(&measured_polygon_fixture(), PDF_MIME, &config).expect("the fixture extracts");
    let annotations = document.annotations.expect("annotations are extracted");
    assert_eq!(annotations.len(), 1);
    let annotation = &annotations[0];
    assert_eq!(annotation.annotation_type, PdfAnnotationType::Polygon);
    assert_eq!(annotation.content.as_deref(), Some("Room 101"));
    let vertices = annotation.vertices.as_ref().expect("the vertices are read");
    assert_eq!(vertices.len(), 4);
    assert_eq!((vertices[0].x, vertices[0].y), (10.0, 10.0));
    assert_eq!((vertices[2].x, vertices[2].y), (110.0, 60.0));
    let measure = annotation.measure.as_ref().expect("the measure is read");
    assert_eq!(measure.scale_ratio.as_deref(), Some("1/4 in = 1 ft"));
    assert_eq!(measure.area.as_deref(), Some([1.0, 144.0].as_slice()));
}

/// One-page PDF with no annotations, for the writer round trip.
fn plain_fixture() -> Vec<u8> {
    let mut pdf = b"%PDF-1.4\n".to_vec();
    let objects: [(u32, &[u8]); 4] = [
        (1, b"<< /Type /Catalog /Pages 2 0 R >>"),
        (2, b"<< /Type /Pages /Kids [3 0 R] /Count 1 >>"),
        (
            3,
            b"<< /Type /Page /Parent 2 0 R /MediaBox [0 0 612 792] /Resources << >> /Contents 4 0 R >>",
        ),
        (4, b"<< /Length 0 >>\nstream\n\nendstream"),
    ];
    let mut offsets = Vec::new();
    for (id, body) in objects {
        let offset = pdf.len();
        pdf.extend_from_slice(format!("{id} 0 obj\n").as_bytes());
        pdf.extend_from_slice(body);
        pdf.extend_from_slice(b"\nendobj\n");
        offsets.push(offset);
    }
    let xref = pdf.len();
    pdf.extend_from_slice(b"xref\n0 5\n0000000000 65535 f \n");
    for offset in offsets {
        pdf.extend_from_slice(format!("{offset:010} 00000 n \n").as_bytes());
    }
    pdf.extend_from_slice(format!("trailer\n<< /Size 5 /Root 1 0 R >>\nstartxref\n{xref}\n%%EOF\n").as_bytes());
    pdf
}

#[test]
fn the_writer_appends_a_measured_polygon_that_extraction_reads_back() {
    use xberg::extraction::plan_geometry::write::{PolygonAnnotationInput, append_polygon_annotation};
    use xberg::{AnnotationPoint, MeasureData};

    let input = PolygonAnnotationInput {
        vertices: vec![
            AnnotationPoint { x: 10.0, y: 10.0 },
            AnnotationPoint { x: 110.0, y: 10.0 },
            AnnotationPoint { x: 110.0, y: 60.0 },
            AnnotationPoint { x: 10.0, y: 60.0 },
        ],
        contents: Some("Written room".to_string()),
        measure: Some(MeasureData {
            subtype: Some("RL".to_string()),
            scale_ratio: Some("1/4 in = 1 ft".to_string()),
            area: Some(vec![1.0, 144.0]),
            ..MeasureData::default()
        }),
        ..PolygonAnnotationInput::default()
    };
    let written = append_polygon_annotation(&plain_fixture(), 0, &input).expect("the annotation is written");

    let config = ExtractionConfig {
        pdf_options: Some(PdfConfig {
            extract_annotations: true,
            ..PdfConfig::default()
        }),
        ..ExtractionConfig::default()
    };
    let document = extract_bytes_document_blocking(&written, PDF_MIME, &config).expect("the written PDF extracts");
    let annotations = document.annotations.expect("annotations are extracted");
    assert_eq!(annotations.len(), 1);
    let annotation = &annotations[0];
    assert_eq!(annotation.content.as_deref(), Some("Written room"));
    assert_eq!(annotation.vertices.as_ref().map(Vec::len), Some(4));
    assert_eq!(
        annotation
            .measure
            .as_ref()
            .and_then(|measure| measure.scale_ratio.as_deref()),
        Some("1/4 in = 1 ft")
    );
}
