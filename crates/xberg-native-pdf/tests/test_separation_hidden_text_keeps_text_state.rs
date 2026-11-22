//! Hidden text is not drawn but still advances the text matrix.
//!
//! ISO 32000-1:2008 §8.11.3: content in a hidden optional-content scope shall
//! not be drawn while graphics-state operations still apply. Text shown in
//! such a scope therefore still moves the text position, so visible text that
//! follows it in the same text object lands where it would have anyway.

use xberg_native_pdf::PdfDocument;
use xberg_native_pdf::rendering::render_separations;

/// A page showing "AA", a middle run and "VV" in one text object with no
/// positioning operator between them. The middle run is hidden either by an
/// excluded `/OC` scope or, as the control, by text render mode 3.
fn text_pdf(middle: &str) -> Vec<u8> {
    let content = format!("/CS0 cs 1 scn BT /F1 20 Tf 10 50 Td (AA) Tj {middle} (VV) Tj ET\n");
    let mut buf: Vec<u8> = Vec::new();
    let mut off = vec![0usize; 8];
    let obj = |buf: &mut Vec<u8>, off: &mut Vec<usize>, id: usize, body: &str| {
        off[id] = buf.len();
        buf.extend_from_slice(format!("{id} 0 obj\n{body}\nendobj\n").as_bytes());
    };
    buf.extend_from_slice(b"%PDF-1.7\n%\xE2\xE3\xCF\xD3\n");
    obj(
        &mut buf,
        &mut off,
        1,
        "<< /Type /Catalog /Pages 2 0 R /OCProperties << /OCGs [6 0 R] /D << /Order [6 0 R] /OFF [6 0 R] >> >> >>",
    );
    obj(&mut buf, &mut off, 2, "<< /Type /Pages /Kids [3 0 R] /Count 1 >>");
    obj(
        &mut buf,
        &mut off,
        3,
        "<< /Type /Page /Parent 2 0 R /MediaBox [0 0 200 100] \
         /Resources << /ColorSpace << /CS0 5 0 R >> /Properties << /MC0 6 0 R >> \
         /Font << /F1 7 0 R >> >> /Contents 4 0 R >>",
    );
    off[4] = buf.len();
    buf.extend_from_slice(format!("4 0 obj\n<< /Length {} >>\nstream\n", content.len()).as_bytes());
    buf.extend_from_slice(content.as_bytes());
    buf.extend_from_slice(b"\nendstream\nendobj\n");
    obj(
        &mut buf,
        &mut off,
        5,
        "[/Separation /Spot /DeviceGray << /FunctionType 2 /Domain [0 1] /N 1 /C0 [1] /C1 [0] >>]",
    );
    obj(&mut buf, &mut off, 6, "<< /Type /OCG /Name (Hidden) >>");
    obj(
        &mut buf,
        &mut off,
        7,
        "<< /Type /Font /Subtype /Type1 /BaseFont /Helvetica >>",
    );
    let xref = buf.len();
    buf.extend_from_slice(b"xref\n0 8\n0000000000 65535 f \n");
    for offset in &off[1..=7] {
        buf.extend_from_slice(format!("{:010} 00000 n \n", offset).as_bytes());
    }
    buf.extend_from_slice(b"trailer\n<< /Size 8 /Root 1 0 R >>\nstartxref\n");
    buf.extend_from_slice(format!("{xref}\n%%EOF\n").as_bytes());
    buf
}

fn plates(pdf: Vec<u8>) -> Vec<Vec<u8>> {
    let doc = PdfDocument::from_bytes(pdf).expect("parse");
    render_separations(&doc, 0, 72)
        .expect("separations")
        .into_iter()
        .map(|p| p.data)
        .collect()
}

#[test]
fn test_text_after_hidden_text_lands_where_it_would_without_hiding() {
    let hidden_by_layer = plates(text_pdf("/OC /MC0 BDC (HHHHHHHH) Tj EMC"));
    let hidden_by_render_mode = plates(text_pdf("3 Tr (HHHHHHHH) Tj 0 Tr"));

    let ink = |p: &[Vec<u8>]| p.iter().flatten().filter(|&&v| v != 0).count();
    assert!(
        ink(&hidden_by_render_mode) > 0,
        "fixture precondition: the control draws visible text"
    );
    assert!(
        hidden_by_layer == hidden_by_render_mode,
        "visible text after a hidden-layer run moved: the hidden text did not advance the text matrix"
    );
}
