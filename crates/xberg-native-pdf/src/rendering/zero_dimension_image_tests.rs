use super::{ImageFormat, PageRenderer, RenderOptions};

/// Build a minimal single-page PDF whose only content is a `Do` of an
/// image XObject declaring `/Height 0`. The image decodes to an empty
/// buffer, so the blit dimensions computed by `render_image` are
/// degenerate.
fn build_zero_height_image_pdf() -> Vec<u8> {
    let mut pdf = Vec::new();
    let mut offsets: Vec<usize> = Vec::new();
    pdf.extend_from_slice(b"%PDF-1.4\n");

    offsets.push(pdf.len());
    pdf.extend_from_slice(b"1 0 obj\n<< /Type /Catalog /Pages 2 0 R >>\nendobj\n\n");

    offsets.push(pdf.len());
    pdf.extend_from_slice(b"2 0 obj\n<< /Type /Pages /Kids [3 0 R] /Count 1 >>\nendobj\n\n");

    offsets.push(pdf.len());
    pdf.extend_from_slice(
        b"3 0 obj\n\
          << /Type /Page /Parent 2 0 R /MediaBox [0 0 100 100]\n\
             /Contents 4 0 R\n\
             /Resources << /XObject << /Im0 5 0 R >> >>\n\
          >>\nendobj\n\n",
    );

    let content = b"q 100 0 0 100 0 0 cm /Im0 Do Q";
    offsets.push(pdf.len());
    let hdr = format!("4 0 obj\n<< /Length {} >>\nstream\n", content.len());
    pdf.extend_from_slice(hdr.as_bytes());
    pdf.extend_from_slice(content);
    pdf.extend_from_slice(b"\nendstream\nendobj\n\n");

    offsets.push(pdf.len());
    pdf.extend_from_slice(
        b"5 0 obj\n\
          << /Type /XObject /Subtype /Image /Width 4 /Height 0\n\
             /ColorSpace /DeviceGray /BitsPerComponent 8 /Length 0 >>\n\
          stream\n\nendstream\nendobj\n\n",
    );

    let xref_offset = pdf.len();
    let n_obj = offsets.len() + 1;
    let mut xref = format!("xref\n0 {}\n", n_obj);
    xref.push_str("0000000000 65535 f \n");
    for off in &offsets {
        xref.push_str(&format!("{:010} 00000 n \n", off));
    }
    pdf.extend_from_slice(xref.as_bytes());
    let trailer = format!(
        "trailer\n<< /Size {} /Root 1 0 R >>\nstartxref\n{}\n%%EOF\n",
        n_obj, xref_offset
    );
    pdf.extend_from_slice(trailer.as_bytes());
    pdf
}

/// `IntSize::from_wh` rejects a zero dimension, so a degenerate image must
/// take the same quiet-skip path as any other pixmap that cannot be built.
#[test]
fn zero_dimension_image_blit_is_skipped() {
    use crate::document::PdfDocument;

    let pdf = build_zero_height_image_pdf();
    let doc = PdfDocument::from_bytes(pdf).expect("parse degenerate-image PDF");

    let opts = RenderOptions {
        format: ImageFormat::RawRgba8,
        ..RenderOptions::with_dpi(72)
    };
    let mut renderer = PageRenderer::new(opts);
    let img = renderer.render_page(&doc, 0).expect("render page");

    assert_eq!(img.data.len(), (img.width * img.height * 4) as usize);
}
