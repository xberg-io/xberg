use xberg_native_pdf::PdfDocument;
use xberg_native_pdf::rendering::{ImageFormat, RenderOptions, render_page};

fn page(content: &str) -> Vec<u8> {
    let resources = "/ColorSpace << /CsK [/Separation /Black /DeviceCMYK \
        << /FunctionType 2 /Domain [0 1] /C0 [0 0 0 0] /C1 [0 0 0 1] /N 1 >>] >> \
        /ExtGState << /OP1 << /Type /ExtGState /OP true /op true /OPM 1 >> >> \
        /Font << /F1 << /Type /Font /Subtype /Type1 /BaseFont /Helvetica /Encoding /WinAnsiEncoding >> >>";
    let mut buf: Vec<u8> = b"%PDF-1.7\n%\xE2\xE3\xCF\xD3\n".to_vec();
    let mut offsets = Vec::new();
    let bodies = [
        "<< /Type /Catalog /Pages 2 0 R >>".to_string(),
        "<< /Type /Pages /Kids [3 0 R] /Count 1 >>".to_string(),
        format!("<< /Type /Page /Parent 2 0 R /MediaBox [0 0 100 100] /Resources << {resources} >> /Contents 4 0 R >>"),
        format!("<< /Length {} >>\nstream\n{content}\nendstream", content.len()),
    ];
    for (index, body) in bodies.iter().enumerate() {
        offsets.push(buf.len());
        buf.extend_from_slice(format!("{} 0 obj\n{body}\nendobj\n", index + 1).as_bytes());
    }
    let xref = buf.len();
    buf.extend_from_slice(b"xref\n0 5\n0000000000 65535 f \n");
    for offset in offsets {
        buf.extend_from_slice(format!("{offset:010} 00000 n \n").as_bytes());
    }
    buf.extend_from_slice(format!("trailer\n<< /Size 5 /Root 1 0 R >>\nstartxref\n{xref}\n%%EOF\n").as_bytes());
    buf
}

fn dark_pixels(content: &str) -> usize {
    let doc = PdfDocument::from_bytes(page(content)).expect("parse");
    let mut options = RenderOptions::with_dpi(72);
    options.format = ImageFormat::RawRgba8;
    let image = render_page(&doc, 0, &options).expect("render");
    image
        .data
        .chunks_exact(4)
        .filter(|px| px[..3].iter().all(|&v| v < 96))
        .count()
}

#[test]
fn an_overprinted_separation_black_text_paints_its_glyphs() {
    let text = "BT /F1 40 Tf 8 35 Td (HIH) Tj ET";
    let plain = dark_pixels(&format!("/CsK cs 1 scn {text}"));
    let overprinted = dark_pixels(&format!("/OP1 gs /CsK cs 1 scn {text}"));
    assert!(plain > 200, "the control text must render: {plain} dark pixels");
    assert!(overprinted * 10 >= plain * 9, "{overprinted} of {plain} dark pixels");
}
