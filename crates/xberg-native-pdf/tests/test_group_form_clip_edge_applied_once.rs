//! A transparency-group form invoked under a clip is bounded by that clip once.
//!
//! The group paints into its own surface and is composited through the caller's clip. Painting
//! under the clip as well squares an anti-aliased edge's coverage: a pixel the clip covers by
//! half would keep a quarter of the ink.

use xberg_native_pdf::PdfDocument;
use xberg_native_pdf::rendering::{RenderOptions, render_page};

fn page(form_group: &str) -> Vec<u8> {
    let content = b"q 20.5 20 30 30 re W n /Fm0 Do Q\n";
    let form = b"0 g 0 0 100 100 re f\n";
    let objects: Vec<Vec<u8>> = vec![
        b"<< /Type /Catalog /Pages 2 0 R >>".to_vec(),
        b"<< /Type /Pages /Kids [3 0 R] /Count 1 >>".to_vec(),
        b"<< /Type /Page /Parent 2 0 R /MediaBox [0 0 100 100] /Contents 4 0 R \
           /Resources << /XObject << /Fm0 5 0 R >> >> >>"
            .to_vec(),
        [
            format!("<< /Length {} >>\nstream\n", content.len()).into_bytes(),
            content.to_vec(),
            b"\nendstream".to_vec(),
        ]
        .concat(),
        [
            format!(
                "<< /Type /XObject /Subtype /Form /BBox [0 0 100 100] {form_group} /Length {} >>\nstream\n",
                form.len()
            )
            .into_bytes(),
            form.to_vec(),
            b"\nendstream".to_vec(),
        ]
        .concat(),
    ];
    let mut pdf = b"%PDF-1.7\n".to_vec();
    let mut offsets = Vec::new();
    for (i, body) in objects.iter().enumerate() {
        offsets.push(pdf.len());
        pdf.extend_from_slice(format!("{} 0 obj\n", i + 1).as_bytes());
        pdf.extend_from_slice(body);
        pdf.extend_from_slice(b"\nendobj\n");
    }
    let xref = pdf.len();
    let n = objects.len() + 1;
    pdf.extend_from_slice(format!("xref\n0 {n}\n0000000000 65535 f \n").as_bytes());
    for offset in &offsets {
        pdf.extend_from_slice(format!("{offset:010} 00000 n \n").as_bytes());
    }
    pdf.extend_from_slice(format!("trailer\n<< /Size {n} /Root 1 0 R >>\nstartxref\n{xref}\n%%EOF\n").as_bytes());
    pdf
}

fn edge_pixel(form_group: &str) -> u8 {
    let doc = PdfDocument::from_bytes(page(form_group)).expect("parse");
    let img = render_page(&doc, 0, &RenderOptions::with_dpi(72)).expect("render");
    let decoded = image::load_from_memory(&img.data).expect("PNG decodes").to_rgba8();
    // The clip's left edge at x = 20.5 covers half of pixel column 20.
    decoded.get_pixel(20, 50)[0]
}

#[test]
fn test_isolated_group_edge_pixel_keeps_the_clip_coverage_once() {
    let value = edge_pixel("/Group << /S /Transparency /I true >>");
    assert!(
        (110..=145).contains(&value),
        "half-covered edge pixel over white must be mid-grey, got {value} (a squared coverage gives ~191)"
    );
}

#[test]
fn test_plain_form_edge_pixel_is_the_control() {
    let value = edge_pixel("");
    assert!((110..=145).contains(&value), "got {value}");
}
