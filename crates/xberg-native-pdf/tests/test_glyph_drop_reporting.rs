//! A dropped glyph must surface a warning, on every page it happens.
//!
//! The fixture embeds a TrueType font whose only cmap subtable is
//! byte-indexed (Macintosh Roman, format 0) and maps just 'A'. Rendering
//! "AB" paints the 'A' box and drops the 'B': its code resolves to glyph 0,
//! which is non-whitespace and never painted, while the cursor still
//! advances. That gap is indistinguishable from real whitespace downstream,
//! so the renderer must push a `GlyphDropped` warning.
//!
//! The warning latch is page-scoped (the `k_zero_warning_emitted` pattern):
//! one warning per font per page, and the next page warns again. A
//! process-lifetime latch fails this test, because bulk ingestion would go
//! silent after the first page.

mod common;

use xberg_native_pdf::PdfDocument;
use xberg_native_pdf::extractors::warnings::{WarningCategory, drain_global_warnings};
use xberg_native_pdf::rendering::{PageRenderer, RenderOptions};

/// The shared box-glyph fixture font (see `common::build_box_glyph_font`),
/// here mapping 'A' to glyph 1 so "AB" paints the box and drops the 'B'.
fn broken_subset_font() -> Vec<u8> {
    common::build_box_glyph_font(common::BoxGlyphFont {
        outline: [50, 0, 450, 700],
        head_bbox: [0, 0, 450, 700],
        lsb: 0,
        cmap_byte: b'A',
    })
}

/// A PDF with `page_count` identical pages, each painting "AB" in the
/// broken embedded font. 'A' paints; 'B' drops.
fn pdf_with_broken_font(page_count: usize) -> Vec<u8> {
    let font = broken_subset_font();
    let content = b"BT /F1 24 Tf 50 100 Td (AB) Tj ET";
    let n_objs = 5 + 2 * page_count;

    let mut buf: Vec<u8> = Vec::new();
    let mut off = vec![0usize; n_objs + 1];
    buf.extend_from_slice(b"%PDF-1.7\n%\xE2\xE3\xCF\xD3\n");

    let obj = |buf: &mut Vec<u8>, off: &mut Vec<usize>, id: usize, body: String| {
        off[id] = buf.len();
        buf.extend_from_slice(format!("{id} 0 obj\n{body}\nendobj\n").as_bytes());
    };

    let kids: Vec<String> = (0..page_count).map(|p| format!("{} 0 R", 6 + 2 * p)).collect();
    obj(&mut buf, &mut off, 1, "<< /Type /Catalog /Pages 2 0 R >>".to_string());
    obj(
        &mut buf,
        &mut off,
        2,
        format!("<< /Type /Pages /Kids [{}] /Count {page_count} >>", kids.join(" ")),
    );
    obj(
        &mut buf,
        &mut off,
        3,
        "<< /Type /Font /Subtype /TrueType /BaseFont /BrokenSubset /FirstChar 65 /LastChar 66 \
         /Widths [500 500] /FontDescriptor 4 0 R >>"
            .to_string(),
    );
    obj(
        &mut buf,
        &mut off,
        4,
        "<< /Type /FontDescriptor /FontName /BrokenSubset /Flags 4 /FontBBox [0 0 450 700] \
         /ItalicAngle 0 /Ascent 800 /Descent -200 /CapHeight 700 /StemV 80 /FontFile2 5 0 R >>"
            .to_string(),
    );
    off[5] = buf.len();
    buf.extend_from_slice(
        format!(
            "5 0 obj\n<< /Length {} /Length1 {} >>\nstream\n",
            font.len(),
            font.len()
        )
        .as_bytes(),
    );
    buf.extend_from_slice(&font);
    buf.extend_from_slice(b"\nendstream\nendobj\n");

    for p in 0..page_count {
        let page_id = 6 + 2 * p;
        let contents_id = page_id + 1;
        obj(
            &mut buf,
            &mut off,
            page_id,
            format!(
                "<< /Type /Page /Parent 2 0 R /MediaBox [0 0 200 200] \
                 /Resources << /Font << /F1 3 0 R >> >> /Contents {contents_id} 0 R >>"
            ),
        );
        off[contents_id] = buf.len();
        buf.extend_from_slice(format!("{contents_id} 0 obj\n<< /Length {} >>\nstream\n", content.len()).as_bytes());
        buf.extend_from_slice(content);
        buf.extend_from_slice(b"\nendstream\nendobj\n");
    }

    let xref = buf.len();
    buf.extend_from_slice(format!("xref\n0 {}\n0000000000 65535 f \n", n_objs + 1).as_bytes());
    for &offset in &off[1..=n_objs] {
        buf.extend_from_slice(format!("{offset:010} 00000 n \n").as_bytes());
    }
    buf.extend_from_slice(
        format!(
            "trailer\n<< /Size {} /Root 1 0 R >>\nstartxref\n{xref}\n%%EOF\n",
            n_objs + 1
        )
        .as_bytes(),
    );
    buf
}

/// Glyph-drop warnings drained from the global sink.
///
/// No font-name filter. `TextRasterizer::report_drops` builds its message with
/// `tally.warning("redacted")` — the PDF font name is deliberately withheld so it
/// cannot leak into logs — so filtering on the fixture's `BrokenSubset` name matched
/// nothing and this test could never pass. It was masked because the unit-test script
/// aborts on the first failing binary, and `test_decoder_characterization` (which had
/// the identical defect) failed first. Draining before and after each render is what
/// makes the result attributable, not the name. ~keep
fn drain_broken_subset_warnings() -> Vec<String> {
    drain_global_warnings()
        .into_iter()
        .filter(|w| w.category == WarningCategory::GlyphDropped)
        .map(|w| w.message)
        .collect()
}

#[test]
fn a_dropped_glyph_warns_on_every_page_it_paints() {
    let doc = PdfDocument::from_bytes(pdf_with_broken_font(2)).expect("parse fixture");
    let mut renderer = PageRenderer::new(RenderOptions::default());
    let _ = drain_broken_subset_warnings();

    renderer.render_page(&doc, 0).expect("render page 0");
    let first = drain_broken_subset_warnings();
    assert_eq!(
        first.len(),
        1,
        "page 0 must warn exactly once for the broken font: {first:?}"
    );
    assert!(
        first[0].contains("0x42"),
        "the warning must name the dropped code 0x42: {}",
        first[0]
    );

    renderer.render_page(&doc, 1).expect("render page 1");
    let second = drain_broken_subset_warnings();
    assert_eq!(
        second.len(),
        1,
        "page 1 must warn again — the latch is page-scoped, not process-scoped: {second:?}"
    );
}
