//! A page with a `/CropBox` renders to the crop, not the media box.
//!
//! ISO 32000-1:2008 §14.11.2 and Table 30: the crop box defines "the region to
//! which the contents of the page shall be clipped (cropped) when displayed or
//! printed". Every viewer honours it, and `pdftoppm` needs `-cropbox` only
//! because its default is the other way round.

use xberg_native_pdf::PdfDocument;
use xberg_native_pdf::rendering::{RenderOptions, render_page};

fn crop_options() -> RenderOptions {
    let mut options = RenderOptions::with_dpi(72);
    options.honor_crop_box = true;
    options
}

/// `/MediaBox [0 0 612 792]` with `/CropBox [72 72 540 720]`, painting a black
/// square *outside* the crop (at the media-box corner) and a grey one inside.
fn cropped_page() -> Vec<u8> {
    let content: &[u8] = b"0 0 0 rg 0 0 40 40 re f\n0.5 0.5 0.5 rg 300 400 60 60 re f\n";
    let objects: Vec<Vec<u8>> = vec![
        b"<< /Type /Catalog /Pages 2 0 R >>".to_vec(),
        b"<< /Type /Pages /Kids [3 0 R] /Count 1 >>".to_vec(),
        b"<< /Type /Page /Parent 2 0 R /MediaBox [0 0 612 792] \
           /CropBox [72 72 540 720] /Contents 4 0 R /Resources << >> >>"
            .to_vec(),
        [
            format!("<< /Length {} >>\nstream\n", content.len()).into_bytes(),
            content.to_vec(),
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
    for off in &offsets {
        pdf.extend_from_slice(format!("{off:010} 00000 n \n").as_bytes());
    }
    pdf.extend_from_slice(format!("trailer\n<< /Size {n} /Root 1 0 R >>\nstartxref\n{xref}\n%%EOF\n").as_bytes());
    pdf
}

/// The crop is 468 x 648 pt against a 612 x 792 media box, so at 72 dpi the
/// pixmap must be the smaller of the two.
#[test]
fn test_pixmap_is_sized_to_the_crop_box() {
    let doc = PdfDocument::from_bytes(cropped_page()).expect("fixture parses");
    let img = render_page(&doc, 0, &crop_options()).expect("page renders");
    assert_eq!(
        (img.width, img.height),
        (468, 648),
        "a cropped page must render to its /CropBox (468x648), not its /MediaBox \
         (612x792) — §14.11.2 makes the crop box the region contents are clipped to"
    );
}

/// And the square painted outside the crop must not appear.
#[test]
fn content_outside_the_crop_box_is_not_shown() {
    let doc = PdfDocument::from_bytes(cropped_page()).expect("fixture parses");
    let img = render_page(&doc, 0, &crop_options()).expect("page renders");
    let px = image::load_from_memory(&img.data).expect("PNG decodes").to_rgba8();
    // The black square sits at media-space (0,0)-(40,40), entirely below-left of
    // the crop origin (72,72), so no pixel of it is inside the rendered region.
    let black = px
        .pixels()
        .filter(|p| p[0] < 40 && p[1] < 40 && p[2] < 40 && p[3] > 200)
        .count();
    assert_eq!(
        black, 0,
        "{black} black pixels survived; content outside the /CropBox must be clipped"
    );
}

/// The default does not honour the crop box: the raster stays the full media box.
#[test]
fn default_options_render_the_full_media_box() {
    let doc = PdfDocument::from_bytes(cropped_page()).expect("fixture parses");
    let img = render_page(&doc, 0, &RenderOptions::with_dpi(72)).expect("page renders");
    assert_eq!((img.width, img.height), (612, 792));
}
