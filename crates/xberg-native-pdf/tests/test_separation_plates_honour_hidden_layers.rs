//! A layer the document hides must not be counted as ink on the plates.
//!
//! ISO 32000-1:2008 §8.11.4: the `/OCProperties /D` configuration
//! (`BaseState`, `/ON`, `/OFF`) sets the document's default visibility, and the
//! composite renderer already honours it — content inside a hidden `/OC`
//! marked-content scope is not drawn.
//!
//! The separation renderer had no marked-content arms at all, so the exclusion
//! never reached the ink plates: a layer omitted from the render was still
//! counted in the separations, and two renderers of one page gave contradictory
//! answers about the same ink.
//!
//! §8.11.3 also settles *how* to suppress: "the content shall not be drawn"
//! while "graphics state operations ... shall still be applied", so a painting
//! operator becomes the path-clearing `n` rather than being skipped outright.

use xberg_native_pdf::PdfDocument;
use xberg_native_pdf::rendering::{ImageFormat, RenderOptions, render_page, render_separations};

/// A page with two filled squares in a `/Separation` ink: one always visible,
/// one inside an `/OC` scope whose OCG is listed in `/OFF` when `hide` is set.
fn two_square_pdf(hide: bool) -> Vec<u8> {
    let d_config = if hide {
        "/D << /Order [6 0 R] /OFF [6 0 R] >>"
    } else {
        "/D << /Order [6 0 R] /ON [6 0 R] >>"
    };

    let content = b"/CS0 cs 1 scn\n\
        20 20 60 60 re f\n\
        /OC /MC0 BDC\n\
        120 120 60 60 re f\n\
        EMC\n";

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
        &format!("<< /Type /Catalog /Pages 2 0 R /OCProperties << /OCGs [6 0 R] {d_config} >> >>"),
    );
    obj(&mut buf, &mut off, 2, "<< /Type /Pages /Kids [3 0 R] /Count 1 >>");
    obj(
        &mut buf,
        &mut off,
        3,
        "<< /Type /Page /Parent 2 0 R /MediaBox [0 0 200 200] \
         /Resources << /ColorSpace << /CS0 5 0 R >> /Properties << /MC0 6 0 R >> >> \
         /Contents 4 0 R >>",
    );
    off[4] = buf.len();
    buf.extend_from_slice(format!("4 0 obj\n<< /Length {} >>\nstream\n", content.len()).as_bytes());
    buf.extend_from_slice(content);
    buf.extend_from_slice(b"\nendstream\nendobj\n");
    obj(
        &mut buf,
        &mut off,
        5,
        "[/Separation /Spot /DeviceGray << /FunctionType 2 /Domain [0 1] /N 1 \
         /C0 [1] /C1 [0] >>]",
    );
    obj(&mut buf, &mut off, 6, "<< /Type /OCG /Name (Hidden) >>");

    let xref = buf.len();
    buf.extend_from_slice(b"xref\n0 7\n0000000000 65535 f \n");
    for offset in &off[1..=6] {
        buf.extend_from_slice(format!("{:010} 00000 n \n", offset).as_bytes());
    }
    buf.extend_from_slice(b"trailer\n<< /Size 7 /Root 1 0 R >>\nstartxref\n");
    buf.extend_from_slice(format!("{xref}\n%%EOF\n").as_bytes());
    buf
}

/// Total ink across every plate.
fn plate_ink(pdf: Vec<u8>) -> u64 {
    let doc = PdfDocument::from_bytes(pdf).expect("parse");
    let plates = render_separations(&doc, 0, 72).expect("separations");
    plates
        .iter()
        .map(|p| p.data.iter().map(|&v| u64::from(v)).sum::<u64>())
        .sum()
}

/// Non-white pixels in the composite render.
fn composite_ink(pdf: Vec<u8>) -> u64 {
    let doc = PdfDocument::from_bytes(pdf).expect("parse");
    let mut opts = RenderOptions::default();
    opts.dpi = 72;
    opts.format = ImageFormat::RawRgba8;
    let img = render_page(&doc, 0, &opts).expect("render");
    img.data
        .as_chunks::<4>()
        .0
        .iter()
        .filter(|px| px[0] < 250 || px[1] < 250 || px[2] < 250)
        .count() as u64
}

/// The control: the composite renderer already honours the hidden layer, so
/// hiding it must reduce composite ink. If this fails the fixture is wrong.
#[test]
fn test_composite_render_already_honours_the_hidden_layer() {
    let visible = composite_ink(two_square_pdf(false));
    let hidden = composite_ink(two_square_pdf(true));
    assert!(
        hidden < visible,
        "fixture precondition: hiding the layer should reduce composite ink \
         ({hidden} vs {visible})"
    );
}

/// The defect: the plates counted the hidden layer's ink.
#[test]
fn test_plates_drop_the_hidden_layer_too() {
    let visible = plate_ink(two_square_pdf(false));
    let hidden = plate_ink(two_square_pdf(true));
    assert!(
        hidden < visible,
        "the hidden layer was still counted as ink on the plates \
         ({hidden} vs {visible})"
    );
}

/// The visible square must survive — suppression must be scoped to the `/OC`
/// section, not applied to the whole page.
#[test]
fn test_visible_square_still_inks_its_plate() {
    assert!(
        plate_ink(two_square_pdf(true)) > 0,
        "suppressing a hidden layer erased the visible content too"
    );
}
