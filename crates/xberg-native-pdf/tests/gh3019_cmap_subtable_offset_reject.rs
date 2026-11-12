//! GH#3019: a malformed embedded TrueType font — its cmap subtable offset
//! mutated to a huge value after build, the way the #2954/#3003 red arm did —
//! used to overflow the parser's `cmap_offset + subtable_offset` arithmetic
//! (debug panic, crashing the renderer) or silently wrap in release. The font
//! must be rejected with a parse error so rendering continues on the fallback
//! path (`text_rasterizer.rs`'s invalid-embedded-font branch) instead.

mod common;

use xberg_native_pdf::PdfDocument;
use xberg_native_pdf::rendering::{PageRenderer, RenderOptions};

/// The shared box-glyph fixture font (see `common::build_box_glyph_font`)
/// mapping `M` to glyph 1, with the first cmap record's subtable offset
/// corrupted to a value that overflows any u32 offset arithmetic.
fn corrupted_font() -> Vec<u8> {
    let mut font = common::build_box_glyph_font(common::BoxGlyphFont {
        outline: [250, 0, 750, 800],
        head_bbox: [250, 0, 750, 800],
        lsb: 250,
        cmap_byte: b'M',
    });
    // `build_sfnt` sorts the tables by tag, so `cmap` is the first table
    // body: 12-byte sfnt header + 7 × 16-byte directory entries puts it at
    // 124; within the cmap, version (2) + subtable count (2) + platform (2)
    // + encoding (2) puts the record's offset field at 132. ~keep
    const SUBTABLE_OFFSET_FIELD: usize = 12 + 7 * 16 + 8;
    assert_eq!(
        &font[SUBTABLE_OFFSET_FIELD..SUBTABLE_OFFSET_FIELD + 4],
        &12u32.to_be_bytes(),
        "subtable offset field location in the box-glyph fixture font"
    );
    font[SUBTABLE_OFFSET_FIELD..SUBTABLE_OFFSET_FIELD + 4].copy_from_slice(&0xFFFF_FFF0u32.to_be_bytes());
    font
}

/// A one-page PDF embedding `corrupted_font` as a `/TrueType` font program
/// (`FontFile2`), assembled like `gh1632_whitespace_inferred_glyph`.
fn corrupted_font_pdf() -> Vec<u8> {
    let font = corrupted_font();
    let content = b"BT /F1 24 Tf 50 100 Td (M) Tj ET";
    let n_objs = 7usize;

    let mut buf: Vec<u8> = Vec::new();
    let mut off = vec![0usize; n_objs + 1];
    buf.extend_from_slice(b"%PDF-1.7\n%\xE2\xE3\xCF\xD3\n");

    let obj = |buf: &mut Vec<u8>, off: &mut Vec<usize>, id: usize, body: String| {
        off[id] = buf.len();
        buf.extend_from_slice(format!("{id} 0 obj\n{body}\nendobj\n").as_bytes());
    };

    obj(&mut buf, &mut off, 1, "<< /Type /Catalog /Pages 2 0 R >>".to_string());
    obj(
        &mut buf,
        &mut off,
        2,
        "<< /Type /Pages /Kids [6 0 R] /Count 1 >>".to_string(),
    );
    obj(
        &mut buf,
        &mut off,
        3,
        format!(
            "<< /Type /Font /Subtype /TrueType /BaseFont /BrokenSubset /FirstChar 0 /LastChar 255 \
             /Widths [{}] /FontDescriptor 4 0 R >>",
            vec!["500"; 256].join(" ")
        ),
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
    obj(
        &mut buf,
        &mut off,
        6,
        "<< /Type /Page /Parent 2 0 R /MediaBox [0 0 200 200] \
         /Resources << /Font << /F1 3 0 R >> >> /Contents 7 0 R >>"
            .to_string(),
    );
    off[7] = buf.len();
    buf.extend_from_slice(format!("7 0 obj\n<< /Length {} >>\nstream\n", content.len()).as_bytes());
    buf.extend_from_slice(content);
    buf.extend_from_slice(b"\nendstream\nendobj\n");

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

/// Rendering a page whose only embedded font is the corrupted one must not
/// panic (the #3019 regression): the huge subtable offset is rejected with a
/// parse error, the cmap falls back to `None` ("using fallback font
/// encoding", `font_dict.rs`), and the page renders — with no ink from the
/// malformed font, which cannot resolve its glyph anymore, rather than a
/// crash or garbage outlines.
#[test]
fn corrupted_cmap_offset_renders_via_fallback_instead_of_crashing() {
    let doc = PdfDocument::from_bytes(corrupted_font_pdf()).expect("the malformed font does not break document parse");
    let image = PageRenderer::new(RenderOptions::with_dpi(72).as_raw())
        .render_page(&doc, 0)
        .expect("render must recover from the malformed embedded font, not crash");
    assert!(
        image.data.iter().all(|&v| v == 255),
        "the malformed font's box glyph must not be painted from garbage cmap data"
    );
    // Text extraction walks the same rejected cmap and must recover too. ~keep
    let text = doc
        .extract_text(0)
        .expect("extraction recovers from the malformed font");
    assert_eq!(text, "M", "fallback encoding still decodes the content byte");
}
