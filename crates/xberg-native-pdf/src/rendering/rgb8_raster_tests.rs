use super::{render_png_page_with_dpi_budget, render_rgb8_page_with_dpi_budget};
use crate::document::PdfDocument;

/// A one-page PDF with text, opaque and translucent fills, and a stroked path.
fn layout_pdf() -> PdfDocument {
    let content = "q 0.2 0.4 0.8 rg 20 20 120 60 re f Q \
                   q /GS1 gs 0.9 0.1 0.1 rg 60 50 100 80 re f Q \
                   q 0 0.6 0 RG 3 w 10 190 m 190 110 l S Q \
                   BT /F1 24 Tf 30 150 Td (Layout 42) Tj ET";
    let objects = [
        "<< /Type /Catalog /Pages 2 0 R >>".to_string(),
        "<< /Type /Pages /Kids [3 0 R] /Count 1 >>".to_string(),
        "<< /Type /Page /Parent 2 0 R /MediaBox [0 0 200 200] /Contents 4 0 R \
         /Resources << /Font << /F1 5 0 R >> /ExtGState << /GS1 6 0 R >> >> >>"
            .to_string(),
        format!("<< /Length {} >>\nstream\n{content}\nendstream", content.len()),
        "<< /Type /Font /Subtype /Type1 /BaseFont /Helvetica >>".to_string(),
        "<< /Type /ExtGState /ca 0.45 /CA 0.45 >>".to_string(),
    ];
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

/// Renders through the PNG bridge and decodes to RGB8, as the layout runner used to.
fn via_png(doc: &PdfDocument, dpi: u32, minimum_dpi: u32, budget: usize) -> (image::RgbImage, u32) {
    let (png, admitted) = render_png_page_with_dpi_budget(doc, 0, dpi, minimum_dpi, budget).unwrap();
    (image::load_from_memory(&png.data).unwrap().into_rgb8(), admitted)
}

#[test]
fn rgb8_bridge_matches_the_png_round_trip_pixel_for_pixel() {
    let doc = layout_pdf();
    for dpi in [72, 150, 200] {
        let (expected, expected_dpi) = via_png(&doc, dpi, dpi, 1 << 28);
        let (raster, admitted) = render_rgb8_page_with_dpi_budget(&doc, 0, dpi, dpi, 1 << 28).unwrap();

        assert_eq!(admitted, expected_dpi, "{dpi} dpi");
        assert_eq!((raster.width, raster.height), expected.dimensions(), "{dpi} dpi");
        let distinct: std::collections::HashSet<&[u8]> = raster.data.chunks(3).collect();
        assert!(distinct.len() > 4, "{dpi} dpi: the fixture paints several colours");
        assert!(raster.data == expected.as_raw()[..], "{dpi} dpi: pixels differ");
    }
}

#[test]
fn rgb8_bridge_admits_the_same_reduced_dpi_under_a_tight_budget() {
    let doc = layout_pdf();
    // 200 pt square at 150 dpi is 417x417; this budget fits roughly 300x300 at 12 bytes a pixel.
    let budget = 300 * 300 * 12 + 256 * 1024;
    let (expected, expected_dpi) = via_png(&doc, 150, 72, budget);
    let (raster, admitted) = render_rgb8_page_with_dpi_budget(&doc, 0, 150, 72, budget).unwrap();

    assert!(
        expected_dpi < 150,
        "the budget must force a lower DPI, got {expected_dpi}"
    );
    assert_eq!(admitted, expected_dpi);
    assert_eq!((raster.width, raster.height), expected.dimensions());
    assert!(raster.data == expected.as_raw()[..], "pixels differ");
}
