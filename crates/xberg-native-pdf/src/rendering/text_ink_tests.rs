use super::render_text_ink_rasters;
use crate::document::PdfDocument;

const DARK: u8 = 128;

fn pdf(content: &str, extra_objects: &[&str], font_resource: &str) -> PdfDocument {
    let mut objects = vec![
        "<< /Type /Catalog /Pages 2 0 R >>".to_string(),
        "<< /Type /Pages /Kids [3 0 R] /Count 1 >>".to_string(),
        format!(
            "<< /Type /Page /Parent 2 0 R /MediaBox [0 0 200 200] /Contents 4 0 R \
             /Resources << /Font << /F1 {font_resource} >> >> >>"
        ),
        format!("<< /Length {} >>\nstream\n{content}\nendstream", content.len()),
        "<< /Type /Font /Subtype /Type1 /BaseFont /Helvetica >>".to_string(),
    ];
    objects.extend(extra_objects.iter().map(|object| (*object).to_string()));
    let mut bytes = b"%PDF-1.4\n".to_vec();
    let mut offsets = Vec::new();
    for (index, object) in objects.iter().enumerate() {
        offsets.push(bytes.len());
        bytes.extend_from_slice(format!("{} 0 obj\n{object}\nendobj\n", index + 1).as_bytes());
    }
    let xref = bytes.len();
    bytes.extend_from_slice(format!("xref\n0 {}\n0000000000 65535 f \n", objects.len() + 1).as_bytes());
    for offset in offsets {
        bytes.extend_from_slice(format!("{offset:010} 00000 n \n").as_bytes());
    }
    bytes.extend_from_slice(
        format!(
            "trailer\n<< /Size {} /Root 1 0 R >>\nstartxref\n{xref}\n%%EOF\n",
            objects.len() + 1
        )
        .as_bytes(),
    );
    PdfDocument::from_bytes(bytes).expect("open fixture PDF")
}

fn text_pdf(content: &str) -> PdfDocument {
    pdf(content, &[], "5 0 R")
}

fn dark(raster: &[u8]) -> usize {
    raster.iter().filter(|&&value| value < DARK).count()
}

#[test]
fn glyph_ink_appears_only_in_the_drawn_raster() {
    let rasters = render_text_ink_rasters(&text_pdf("BT /F1 60 Tf 20 80 Td (HHH) Tj ET"), 0, 72, 1 << 26, || false)
        .unwrap()
        .unwrap();
    assert!(dark(&rasters.drawn) > 100, "the text paints");
    assert_eq!(dark(&rasters.without_glyphs), 0);
}

#[test]
fn invisible_text_leaves_both_rasters_equal() {
    let rasters = render_text_ink_rasters(
        &text_pdf("BT /F1 60 Tf 3 Tr 20 80 Td (HHH) Tj ET"),
        0,
        72,
        1 << 26,
        || false,
    )
    .unwrap()
    .unwrap();
    assert_eq!(dark(&rasters.drawn), 0);
    assert_eq!(rasters.drawn, rasters.without_glyphs);
}

#[test]
fn text_used_only_as_a_clip_still_clips_what_is_painted_through_it() {
    let rasters = render_text_ink_rasters(
        &text_pdf("q BT /F1 120 Tf 7 Tr 20 60 Td (H) Tj ET 0 g 0 0 200 200 re f Q"),
        0,
        72,
        1 << 26,
        || false,
    )
    .unwrap()
    .unwrap();
    let inked = dark(&rasters.without_glyphs);
    assert!(inked > 100, "the fill shows through the glyph-shaped clip");
    assert!(inked < 200 * 200 / 2, "and only through it: {inked}");
    assert_eq!(rasters.drawn, rasters.without_glyphs);
}

#[test]
fn type3_glyph_ink_stays_in_the_glyph_free_raster() {
    let font = "<< /Type /Font /Subtype /Type3 /FontBBox [0 0 1000 1000] /FontMatrix [0.001 0 0 0.001 0 0] \
                /CharProcs << /a 7 0 R >> /Encoding << /Type /Encoding /Differences [97 /a] >> \
                /FirstChar 97 /LastChar 97 /Widths [1000] >>";
    let char_proc = "<< /Length 38 >>\nstream\n1000 0 0 0 800 800 d1 0 0 800 800 re f\nendstream";
    let document = pdf("BT /F1 60 Tf 20 80 Td (aaa) Tj ET", &[font, char_proc], "6 0 R");
    let rasters = render_text_ink_rasters(&document, 0, 72, 1 << 26, || false)
        .unwrap()
        .unwrap();
    assert!(dark(&rasters.drawn) > 100);
    assert!(
        dark(&rasters.without_glyphs) > 100,
        "Type3 paint is not treated as glyph ink"
    );
}

#[test]
fn a_cancelled_render_returns_nothing() {
    let document = text_pdf("BT /F1 60 Tf 20 80 Td (HHH) Tj ET");
    assert!(
        render_text_ink_rasters(&document, 0, 72, 1 << 26, || true)
            .unwrap()
            .is_none()
    );
}
