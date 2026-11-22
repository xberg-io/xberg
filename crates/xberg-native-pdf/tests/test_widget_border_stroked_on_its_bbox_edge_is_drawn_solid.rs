//! ISO 32000-1:2008 §12.5.5 — an annotation's appearance is scaled so that
//! its `/BBox` fills `/Rect`, and that mapping puts anything stroked *on* the
//! box boundary half outside it by construction.
//!
//! Form-field producers routinely draw the field border exactly there:
//! `1 w 0 G 0 0 149.9998 22 re S` inside a `/BBox [0 0 150 22]`. Table 95 says
//! a form's `/BBox` "shall be used to clip the form XObject", and clipping an
//! appearance to its own box halves that stroke — a one-unit black border
//! (`/MK /BC [0 0 0]`, Table 189; width 1 by the `/Border` default of Table
//! 164) came out pale and thin where every other renderer draws it solid.
//! The appearance is therefore fitted to `/Rect` but not clipped to `/BBox`.

use xberg_native_pdf::document::PdfDocument;
use xberg_native_pdf::rendering::{RenderOptions, render_page};

fn build_pdf(objects: &[Vec<u8>]) -> Vec<u8> {
    let mut out: Vec<u8> = Vec::new();
    out.extend_from_slice(b"%PDF-1.4\n");
    let mut offsets = Vec::new();
    for (i, body) in objects.iter().enumerate() {
        offsets.push(out.len());
        out.extend_from_slice(format!("{} 0 obj\n", i + 1).as_bytes());
        out.extend_from_slice(body);
        out.extend_from_slice(b"\nendobj\n");
    }
    let xref_pos = out.len();
    out.extend_from_slice(format!("xref\n0 {}\n", objects.len() + 1).as_bytes());
    out.extend_from_slice(b"0000000000 65535 f \n");
    for off in &offsets {
        out.extend_from_slice(format!("{off:010} 00000 n \n").as_bytes());
    }
    out.extend_from_slice(
        format!(
            "trailer\n<< /Size {} /Root 1 0 R >>\nstartxref\n{}\n%%EOF\n",
            objects.len() + 1,
            xref_pos
        )
        .as_bytes(),
    );
    out
}

fn obj(s: &str) -> Vec<u8> {
    s.as_bytes().to_vec()
}

fn stream_obj(dict: &str, data: &[u8]) -> Vec<u8> {
    let mut v = format!("<< {} /Length {} >>\nstream\n", dict, data.len()).into_bytes();
    v.extend_from_slice(data);
    v.extend_from_slice(b"\nendstream");
    v
}

/// One text field on a letter page, at the half-point offsets a real form
/// producer wrote (`/Rect [55.5 724.25 205.5 746.25]`), with the border
/// stroked on the boundary of a `/BBox` that is exactly the rectangle's size.
fn date_field_pdf() -> Vec<u8> {
    let ap = stream_obj(
        "/Type /XObject /Subtype /Form /BBox [0 0 150 22]",
        b"/Tx BMC q 1 w 0 G 0 0 149.9998 22 re S Q EMC",
    );
    let objects = vec![
        obj("<< /Type /Catalog /Pages 2 0 R /AcroForm << /Fields [5 0 R] >> >>"),
        obj("<< /Type /Pages /Kids [3 0 R] /Count 1 >>"),
        obj("<< /Type /Page /Parent 2 0 R /MediaBox [0 0 612 792] \
               /Contents 4 0 R /Annots [5 0 R] >>"),
        stream_obj("", b""),
        obj("<< /Type /Annot /Subtype /Widget /FT /Tx /T (date) /F 4 \
               /MK << /BC [0 0 0] >> /DA (/Helv 12 Tf 0 g) \
               /Rect [55.5 724.25 205.5 746.25] /AP << /N 6 0 R >> >>"),
        ap,
    ];
    build_pdf(&objects)
}

/// Luminance of every inked pixel (< 250), darkest first.
fn inked_lumas(pdf: &[u8], dpi: u32) -> Vec<u8> {
    let doc = PdfDocument::from_bytes(pdf.to_vec()).expect("synthetic PDF parses");
    let options = RenderOptions::with_dpi(dpi);
    let img = render_page(&doc, 0, &options).expect("page renders");
    let decoded = image::load_from_memory(&img.data).expect("PNG decodes").to_rgba8();
    let mut lumas: Vec<u8> = decoded
        .pixels()
        .filter(|p| p[3] > 0)
        .map(|p| ((p[0] as u32 + p[1] as u32 + p[2] as u32) / 3) as u8)
        .filter(|&l| l < 250)
        .collect();
    lumas.sort_unstable();
    lumas
}

#[test]
fn test_one_unit_border_on_the_bbox_edge_renders_black_at_72_dpi() {
    let lumas = inked_lumas(&date_field_pdf(), 72);
    assert!(!lumas.is_empty(), "the field border drew nothing at all");

    // A 1 pt stroke at 72 dpi is one device pixel wide. Along the vertical
    // edges (x = 55.5 and 205.5) it covers the whole of one pixel column, so
    // the darkest pixels are black; halved by a clip at the box edge, the
    // stroke covers half a column and the darkest value only reaches mid-grey.
    let darkest = lumas[0];
    assert!(darkest <= 32, "the border's darkest pixel is {darkest}, not black");

    // The border is a 150x22 pt ring: 2 * (150 + 22) = 344 pixel-lengths.
    // Solid, most of its pixels are dark; halved, almost none are.
    let dark = lumas.iter().filter(|&&l| l < 128).count();
    assert!(
        dark >= 300,
        "only {dark} of {} inked pixels are dark (< 128): the border is being clipped in half",
        lumas.len()
    );
}

#[test]
fn test_border_stays_solid_when_the_fit_scale_is_not_one() {
    // Rendering at 150 dpi scales the same stroke to ~2 pixels; a stroke that
    // is halved by a clip is still visibly half as heavy.
    let lumas = inked_lumas(&date_field_pdf(), 150);
    let dark = lumas.iter().filter(|&&l| l < 128).count();
    // Perimeter in device pixels at 150 dpi: 344 * 150 / 72 ≈ 717, and a
    // ~2 px stroke fills about twice that.
    assert!(
        dark >= 1000,
        "only {dark} of {} inked pixels are dark: the border is being clipped in half",
        lumas.len()
    );
}
