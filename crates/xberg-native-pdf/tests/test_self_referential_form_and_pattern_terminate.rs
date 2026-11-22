//! A Form XObject that paints itself, and a tiling pattern that fills with
//! itself, both terminate instead of overflowing the stack.
//!
//! The existing caps bound *size* — the offscreen cell raster and the tile
//! count. Neither bounds *depth*, which is a different hazard: a content
//! stream that names the very resource painting it recurses until the stack
//! goes. `MAX_FORM_DEPTH` and `MAX_PATTERN_DEPTH`
//! (`src/rendering/page_renderer.rs`) close that, the same way the Type 3
//! glyph and soft-mask chains already were.
//!
//! Self-reference is legal to *write* and a reader must survive it, so the
//! contract is "renders and returns", not "rejects the file".

use xberg_native_pdf::PdfDocument;
use xberg_native_pdf::rendering::{RenderOptions, render_page};

fn build(objects: Vec<Vec<u8>>) -> Vec<u8> {
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
    for off in &offsets {
        pdf.extend_from_slice(format!("{off:010} 00000 n \n").as_bytes());
    }
    pdf.extend_from_slice(format!("trailer\n<< /Size {n} /Root 1 0 R >>\nstartxref\n{xref}\n%%EOF\n").as_bytes());
    pdf
}

fn stream(dict: &str, data: &[u8]) -> Vec<u8> {
    [
        format!("<< {dict} /Length {} >>\nstream\n", data.len()).into_bytes(),
        data.to_vec(),
        b"\nendstream".to_vec(),
    ]
    .concat()
}

/// `/Fm0` is a form whose own content stream invokes `/Fm0`.
fn self_referential_form() -> Vec<u8> {
    let page = b"q 1 0 0 1 0 0 cm /Fm0 Do Q\n";
    let form = b"0 0 1 rg 10 10 80 80 re f\n/Fm0 Do\n";
    build(vec![
        b"<< /Type /Catalog /Pages 2 0 R >>".to_vec(),
        b"<< /Type /Pages /Kids [3 0 R] /Count 1 >>".to_vec(),
        b"<< /Type /Page /Parent 2 0 R /MediaBox [0 0 200 200] /Contents 4 0 R \
           /Resources << /XObject << /Fm0 5 0 R >> >> >>"
            .to_vec(),
        stream("", page),
        stream(
            "/Type /XObject /Subtype /Form /BBox [0 0 200 200] \
             /Resources << /XObject << /Fm0 5 0 R >> >>",
            form,
        ),
    ])
}

/// `/P0` is a tiling pattern whose cell fills using `/P0`.
fn self_referential_tiling_pattern() -> Vec<u8> {
    let page = b"/Pattern cs /P0 scn 0 0 200 200 re f\n";
    let cell = b"/Pattern cs /P0 scn 0 0 20 20 re f\n";
    build(vec![
        b"<< /Type /Catalog /Pages 2 0 R >>".to_vec(),
        b"<< /Type /Pages /Kids [3 0 R] /Count 1 >>".to_vec(),
        b"<< /Type /Page /Parent 2 0 R /MediaBox [0 0 200 200] /Contents 4 0 R \
           /Resources << /Pattern << /P0 5 0 R >> >> >>"
            .to_vec(),
        stream("", page),
        stream(
            "/Type /Pattern /PatternType 1 /PaintType 1 /TilingType 1 \
             /BBox [0 0 20 20] /XStep 20 /YStep 20 \
             /Resources << /Pattern << /P0 5 0 R >> >>",
            cell,
        ),
    ])
}

#[test]
fn test_form_that_paints_itself_terminates() {
    let doc = PdfDocument::from_bytes(self_referential_form()).expect("fixture parses");
    let img = render_page(&doc, 0, &RenderOptions::default()).expect("page renders");
    assert!(img.width > 0 && img.height > 0, "the page must still produce a pixmap");
}

#[test]
fn test_tiling_pattern_that_fills_with_itself_terminates() {
    let doc = PdfDocument::from_bytes(self_referential_tiling_pattern()).expect("fixture parses");
    let img = render_page(&doc, 0, &RenderOptions::default()).expect("page renders");
    assert!(img.width > 0 && img.height > 0, "the page must still produce a pixmap");
}
