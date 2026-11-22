//! A clip the renderer cannot rasterise must not be resolved toward painting more (ISO 32000-1:2008
//! §8.5.4). On the ink plates, a clip excluding the whole page must leave the plate empty; an
//! enormous clip that still encloses the page restricts nothing and must leave the ink intact.

use xberg_native_pdf::PdfDocument;
use xberg_native_pdf::rendering::render_separation;

/// A page that clips to `clip_rect`, then fills the whole page with pure cyan.
fn clipped_cyan_fill_pdf(clip_rect: &str) -> Vec<u8> {
    let content = format!("q {clip_rect} re W n 1 0 0 0 k 0 0 200 200 re f Q\n");
    let objects = [
        "<< /Type /Catalog /Pages 2 0 R >>".to_string(),
        "<< /Type /Pages /Kids [3 0 R] /Count 1 >>".to_string(),
        "<< /Type /Page /Parent 2 0 R /MediaBox [0 0 200 200] /Contents 4 0 R >>".to_string(),
        format!("<< /Length {} >>\nstream\n{content}\nendstream", content.len()),
    ];
    let mut pdf = b"%PDF-1.7\n".to_vec();
    let mut offsets = Vec::new();
    for (i, body) in objects.iter().enumerate() {
        offsets.push(pdf.len());
        pdf.extend_from_slice(format!("{} 0 obj\n{body}\nendobj\n", i + 1).as_bytes());
    }
    let xref = pdf.len();
    pdf.extend_from_slice(format!("xref\n0 {}\n0000000000 65535 f \n", objects.len() + 1).as_bytes());
    for offset in &offsets {
        pdf.extend_from_slice(format!("{offset:010} 00000 n \n").as_bytes());
    }
    pdf.extend_from_slice(
        format!(
            "trailer\n<< /Size {} /Root 1 0 R >>\nstartxref\n{xref}\n%%EOF\n",
            objects.len() + 1
        )
        .as_bytes(),
    );
    pdf
}

/// Fraction of the cyan plate carrying more than half tint.
fn cyan_fraction(clip_rect: &str) -> f64 {
    let doc = PdfDocument::from_bytes(clipped_cyan_fill_pdf(clip_rect)).expect("fixture parses");
    let plate = render_separation(&doc, 0, "Cyan", 72).expect("plate renders");
    plate.data.iter().filter(|v| **v > 128).count() as f64 / plate.data.len() as f64
}

#[test]
fn offpage_unrasterisable_clip_leaves_the_plate_empty() {
    let fraction = cyan_fraction("2000000000 2000000000 1000 1000");
    assert!(
        fraction < 0.01,
        "a clip excluding the page still inked {fraction:.3} of the plate"
    );
}

#[test]
fn enclosing_unrasterisable_clip_keeps_the_ink() {
    let fraction = cyan_fraction("-2000000000 -2000000000 4000000000 4000000000");
    assert!(
        fraction > 0.9,
        "a clip enclosing the page erased the plate: {fraction:.3}"
    );
}

#[test]
fn ordinary_clip_inks_its_own_region() {
    let fraction = cyan_fraction("0 0 100 200");
    assert!((0.4..0.6).contains(&fraction), "a half-page clip inked {fraction:.3}");
}

#[test]
fn unclipped_fill_inks_the_page() {
    let fraction = cyan_fraction("0 0 200 200");
    assert!(fraction > 0.9, "an all-page clip restricted the plate: {fraction:.3}");
}
