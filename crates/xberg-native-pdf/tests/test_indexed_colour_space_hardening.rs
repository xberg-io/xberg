//! `/Indexed` colour spaces built from hostile or non-RGB bases.
//!
//! ISO 32000-1:2008 §8.6.6.3: an Indexed space maps an index in `0..=hival` to
//! a palette entry whose bytes are components of the base space, scaled over
//! that base's component ranges (§8.6.5.4 for `/Lab`).
//!
//! - An Indexed space whose base is a Separation whose alternate is that same
//!   Indexed space resolved forever and overflowed the stack.
//! - A `/Lab` base had its palette bytes read as RGB fractions and then painted
//!   as first-component grey.

use xberg_native_pdf::PdfDocument;
use xberg_native_pdf::rendering::{RenderOptions, render_page};

/// One-page 100x100 PDF filling the page with `content`; `/Cs1` is object 5,
/// further objects follow from 6.
fn page_pdf(content: &str, objects: &[&str]) -> Vec<u8> {
    let mut pdf = Vec::new();
    let mut off = Vec::new();
    pdf.extend_from_slice(b"%PDF-1.7\n");
    let mut push_obj = |pdf: &mut Vec<u8>, id: usize, body: &str| {
        off.push((id, pdf.len()));
        pdf.extend_from_slice(format!("{id} 0 obj\n{body}\nendobj\n").as_bytes());
    };
    push_obj(&mut pdf, 1, "<< /Type /Catalog /Pages 2 0 R >>");
    push_obj(&mut pdf, 2, "<< /Type /Pages /Kids [3 0 R] /Count 1 >>");
    push_obj(
        &mut pdf,
        3,
        "<< /Type /Page /Parent 2 0 R /MediaBox [0 0 100 100] /Contents 4 0 R \
         /Resources << /ColorSpace << /Cs1 5 0 R >> >> >>",
    );
    push_obj(
        &mut pdf,
        4,
        &format!("<< /Length {} >>\nstream\n{content}\nendstream", content.len()),
    );
    for (i, body) in objects.iter().enumerate() {
        push_obj(&mut pdf, 5 + i, body);
    }
    let size = 5 + objects.len();
    let xref = pdf.len();
    pdf.extend_from_slice(format!("xref\n0 {size}\n0000000000 65535 f \r\n").as_bytes());
    for (_, o) in &off {
        pdf.extend_from_slice(format!("{o:010} 00000 n \r\n").as_bytes());
    }
    pdf.extend_from_slice(format!("trailer\n<< /Size {size} /Root 1 0 R >>\nstartxref\n{xref}\n%%EOF\n").as_bytes());
    pdf
}

fn centre_pixel(pdf: Vec<u8>) -> (u8, u8, u8) {
    let doc = PdfDocument::from_bytes(pdf).expect("synthetic PDF parses");
    let img = render_page(&doc, 0, &RenderOptions::default()).expect("page renders");
    let px = image::load_from_memory(&img.data).expect("PNG decodes").to_rgba8();
    let p = px.get_pixel(px.width() / 2, px.height() / 2);
    (p[0], p[1], p[2])
}

#[test]
fn test_indexed_separation_cycle_terminates() {
    let pdf = page_pdf(
        "/Cs1 cs 0 sc 0 0 100 100 re f",
        &[
            "[/Indexed 6 0 R 0 <FF>]",
            "[/Separation /Spot 5 0 R << /FunctionType 2 /Domain [0 1] /N 1 /C0 [0] /C1 [0] >>]",
        ],
    );
    centre_pixel(pdf);
}

#[test]
fn test_lab_indexed_palette_is_expanded_over_the_lab_ranges() {
    // <80 FF 80>: L* = 50, a* = +100, b* ~ 0 -- a strong red/magenta, not the
    // mid grey that reading the bytes as RGB fractions produced.
    let pdf = page_pdf(
        "/Cs1 cs 0 sc 0 0 100 100 re f",
        &["[/Indexed [/Lab << /WhitePoint [0.9505 1 1.089] /Range [-100 100 -100 100] >>] 0 <80FF80>]"],
    );
    let (r, g, _) = centre_pixel(pdf);
    assert!(
        i32::from(r) > i32::from(g) + 80,
        "Lab palette entry should be reddish, got r={r} g={g}"
    );
}
