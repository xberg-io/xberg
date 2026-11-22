//! Page `/VP` viewports on the public page result.
//!
//! A construction sheet names its plans through viewport dictionaries. The
//! native reader turns them into `pages[].viewports`; this test goes through the
//! public extraction API so the wiring from the PDF dictionary to the result is
//! covered, not only the parser.

#![cfg(feature = "pdf")]

mod helpers;

use helpers::extract_bytes_document_blocking;
use xberg::ExtractionConfig;
use xberg::core::config::PageConfig;

const PDF_MIME: &str = "application/pdf";

/// One-page PDF with a viewport at (10, 20)-(300, 400), named `PLAN`, with a
/// rectilinear measure dictionary.
fn viewport_fixture() -> Vec<u8> {
    let mut pdf = b"%PDF-1.6\n".to_vec();
    let objects: [(u32, &[u8]); 5] = [
        (1, b"<< /Type /Catalog /Pages 2 0 R >>"),
        (2, b"<< /Type /Pages /Kids [3 0 R] /Count 1 >>"),
        (
            3,
            b"<< /Type /Page /Parent 2 0 R /MediaBox [0 0 612 792] /Resources << >> /Contents 4 0 R /VP [5 0 R] >>",
        ),
        (4, b"<< /Length 0 >>\nstream\n\nendstream"),
        (
            5,
            b"<< /Type /Viewport /BBox [10 20 300 400] /Name (PLAN) /Measure << /Type /Measure /Subtype /RL /R (3/8 in = 1 ft) /X [1 1 0] /Y [2 3 4] >> >>",
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
fn page_viewports_reach_the_public_page_result() {
    let config = ExtractionConfig {
        pages: Some(PageConfig {
            extract_pages: true,
            ..PageConfig::default()
        }),
        ..ExtractionConfig::default()
    };
    let document =
        extract_bytes_document_blocking(&viewport_fixture(), PDF_MIME, &config).expect("the fixture extracts");
    let pages = document.pages.expect("pages are tracked");
    let viewports = pages[0].viewports.as_ref().expect("the page defines a viewport");
    assert_eq!(viewports.len(), 1);
    let viewport = &viewports[0];
    assert_eq!(
        viewport.bounding_box,
        xberg::BoundingBox {
            x0: 10.0,
            y0: 20.0,
            x1: 300.0,
            y1: 400.0,
        }
    );
    assert_eq!(viewport.name.as_deref(), Some("PLAN"));
    let measure = viewport.measure.as_ref().expect("the measure is read");
    assert_eq!(measure.subtype.as_deref(), Some("RL"));
    assert_eq!(measure.scale_ratio.as_deref(), Some("3/8 in = 1 ft"));
    assert_eq!(measure.x.as_deref(), Some([1.0, 1.0, 0.0].as_slice()));
    assert_eq!(measure.y.as_deref(), Some([2.0, 3.0, 4.0].as_slice()));
}

#[test]
fn a_page_without_a_viewport_reports_none() {
    // The same document with no /VP: replace the page dictionary's /VP entry.
    let mut fixture = viewport_fixture();
    let needle = b" /VP [5 0 R]";
    let position = fixture
        .windows(needle.len())
        .position(|window| window == needle)
        .expect("the fixture carries the viewport marker");
    // Keep the byte length so the xref offsets stay valid.
    for byte in &mut fixture[position..position + needle.len()] {
        *byte = b' ';
    }
    let config = ExtractionConfig {
        pages: Some(PageConfig {
            extract_pages: true,
            ..PageConfig::default()
        }),
        ..ExtractionConfig::default()
    };
    let document = extract_bytes_document_blocking(&fixture, PDF_MIME, &config).expect("the fixture extracts");
    let pages = document.pages.expect("pages are tracked");
    assert!(pages[0].viewports.is_none());
}
