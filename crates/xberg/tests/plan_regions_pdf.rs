//! Integration coverage for closed plan regions on a real PDF.
//!
//! The fixture draws one stroked rectangle and puts a label inside it. With
//! `pdf_options.plan_regions` on, the extraction returns that rectangle as one
//! face with its area and text. The raw paths are only reported when
//! `extract_vector_geometry` is also set.

#![cfg(all(feature = "pdf", feature = "plan-regions"))]

mod helpers;

use helpers::extract_bytes_document_blocking;
use xberg::ExtractionConfig;
use xberg::core::config::{PageConfig, PdfConfig, PlanRegionConfig};

const PDF_MIME: &str = "application/pdf";

/// One-page PDF: a stroked 100 x 50 rectangle from (10, 10) to (110, 60) and the
/// text `ROOM` at its centre.
fn plan_fixture() -> Vec<u8> {
    let mut pdf = b"%PDF-1.4\n".to_vec();
    let content = b"0 0 0 RG 2 w\n10 10 m 110 10 l 110 60 l 10 60 l h S\nBT /F1 12 Tf 45 30 Td (ROOM) Tj ET\n";
    let objects: [&[u8]; 5] = [
        b"<< /Type /Catalog /Pages 2 0 R >>",
        b"<< /Type /Pages /Kids [3 0 R] /Count 1 >>",
        b"<< /Type /Page /Parent 2 0 R /MediaBox [0 0 612 792] /Resources << /Font << /F1 5 0 R >> >> /Contents 4 0 R >>",
        b"<< /Type /Font /Subtype /Type1 /BaseFont /Helvetica >>",
        b"<< /Length 0 >>",
    ];
    let mut offsets = Vec::new();
    for (index, object) in objects.iter().enumerate() {
        let id = (index + 1) as u32;
        let offset = pdf.len();
        if id == 4 {
            pdf.extend_from_slice(format!("4 0 obj\n<< /Length {} >>\nstream\n", content.len()).as_bytes());
            pdf.extend_from_slice(content);
            pdf.extend_from_slice(b"\nendstream\nendobj\n");
        } else {
            pdf.extend_from_slice(format!("{id} 0 obj\n").as_bytes());
            pdf.extend_from_slice(object);
            pdf.extend_from_slice(b"\nendobj\n");
        }
        offsets.push((id, offset));
    }
    let xref = pdf.len();
    pdf.extend_from_slice(b"xref\n0 6\n0000000000 65535 f \n");
    for (_, offset) in offsets {
        pdf.extend_from_slice(format!("{offset:010} 00000 n \n").as_bytes());
    }
    pdf.extend_from_slice(format!("trailer\n<< /Size 6 /Root 1 0 R >>\nstartxref\n{xref}\n%%EOF\n").as_bytes());
    pdf
}

fn config(extract_vector_geometry: bool) -> ExtractionConfig {
    ExtractionConfig {
        pdf_options: Some(PdfConfig {
            extract_vector_geometry,
            plan_regions: Some(PlanRegionConfig {
                enabled: true,
                snap_tolerance: 0.5,
                gap_tolerance: 0.0,
                max_segments: 20_000,
            }),
            ..PdfConfig::default()
        }),
        pages: Some(PageConfig {
            extract_pages: true,
            ..PageConfig::default()
        }),
        ..ExtractionConfig::default()
    }
}

#[test]
fn a_rectangle_becomes_a_region_with_its_area_and_text() {
    let document =
        extract_bytes_document_blocking(&plan_fixture(), PDF_MIME, &config(false)).expect("the fixture extracts");
    let pages = document.pages.expect("pages are tracked");
    let geometry = pages[0].geometry.as_ref().expect("the region pass implies geometry");
    assert_eq!(geometry.regions.len(), 1, "one closed rectangle is one face");
    let region = &geometry.regions[0];
    assert!(
        (region.area - 5_000.0).abs() < 1.0,
        "area is 100 x 50, got {}",
        region.area
    );
    assert_eq!(region.text, "ROOM");
    assert!(
        geometry.paths.is_empty(),
        "raw paths must stay hidden unless extract_vector_geometry asked for them"
    );
}

#[test]
fn asking_for_geometry_keeps_the_paths_next_to_the_regions() {
    let document =
        extract_bytes_document_blocking(&plan_fixture(), PDF_MIME, &config(true)).expect("the fixture extracts");
    let pages = document.pages.expect("pages are tracked");
    let geometry = pages[0].geometry.as_ref().expect("geometry is present");
    assert!(!geometry.paths.is_empty(), "the rectangle path is reported");
    assert_eq!(geometry.regions.len(), 1);
    assert_eq!(geometry.regions[0].text, "ROOM");
}
