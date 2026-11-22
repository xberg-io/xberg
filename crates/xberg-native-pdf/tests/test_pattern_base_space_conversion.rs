//! An uncoloured tiling pattern's colour operands belong to the pattern's
//! underlying colour space, so a calibrated base must be converted, not read as
//! device RGB (ISO 32000-1:2008 §8.7.3.3, §8.6.5.3).

use xberg_native_pdf::document::PdfDocument;
use xberg_native_pdf::rendering::{RenderOptions, render_page};

/// A page filled with an uncoloured tiling pattern whose base is `/CalRGB` with
/// linear gamma and the sRGB primaries, painted with operands 0.5 0.5 0.5.
fn calrgb_pattern_pdf() -> Vec<u8> {
    let page_content = b"/Cs1 cs 0.5 0.5 0.5 /P0 scn 0 0 100 100 re f";
    let tile = b"0 0 10 10 re f";
    let cal = "[/Pattern [/CalRGB << /WhitePoint [0.9505 1 1.089] /Gamma [1 1 1] \
               /Matrix [0.4124 0.2126 0.0193 0.3576 0.7152 0.1192 0.1805 0.0722 0.9505] >>]]";

    let mut pdf: Vec<u8> = Vec::new();
    let mut off = [0usize; 7];
    macro_rules! push {
        ($s:expr) => {
            pdf.extend_from_slice($s.as_bytes())
        };
    }
    push!("%PDF-1.4\n");
    off[1] = pdf.len();
    push!("1 0 obj\n<< /Type /Catalog /Pages 2 0 R >>\nendobj\n");
    off[2] = pdf.len();
    push!("2 0 obj\n<< /Type /Pages /Kids [3 0 R] /Count 1 >>\nendobj\n");
    off[3] = pdf.len();
    push!(format!(
        "3 0 obj\n<< /Type /Page /Parent 2 0 R /MediaBox [0 0 100 100] \
         /Contents 4 0 R /Resources << /ColorSpace << /Cs1 {cal} >> \
         /Pattern << /P0 5 0 R >> >> >>\nendobj\n"
    ));
    off[4] = pdf.len();
    push!(format!("4 0 obj\n<< /Length {} >>\nstream\n", page_content.len()));
    pdf.extend_from_slice(page_content);
    push!("\nendstream\nendobj\n");
    off[5] = pdf.len();
    push!(format!(
        "5 0 obj\n<< /Type /Pattern /PatternType 1 /PaintType 2 /TilingType 1 \
         /BBox [0 0 10 10] /XStep 10 /YStep 10 /Resources << >> /Length {} >>\nstream\n",
        tile.len()
    ));
    pdf.extend_from_slice(tile);
    push!("\nendstream\nendobj\n");

    let xref = pdf.len();
    push!("xref\n0 6\n0000000000 65535 f \r\n");
    for offset in &off[1..=5] {
        push!(format!("{:010} 00000 n \r\n", offset));
    }
    push!(format!(
        "trailer\n<< /Size 6 /Root 1 0 R >>\nstartxref\n{xref}\n%%EOF\n"
    ));
    pdf
}

#[test]
fn test_calrgb_pattern_base_applies_its_gamma_and_matrix() {
    let doc = PdfDocument::from_bytes(calrgb_pattern_pdf()).expect("synthetic PDF parses");
    let img = render_page(&doc, 0, &RenderOptions::default()).expect("page renders");
    let px = image::load_from_memory(&img.data).expect("PNG decodes").to_rgba8();
    let p = px.get_pixel(px.width() / 2, px.height() / 2);
    // Linear 0.5 grey encodes to sRGB ~0.735 (188); reading the operands as
    // device RGB gave 128.
    assert!(
        (180..=196).contains(&p[0]) && (180..=196).contains(&p[1]) && (180..=196).contains(&p[2]),
        "linear CalRGB 0.5 should render about 188, got {:?}",
        (p[0], p[1], p[2])
    );
}
