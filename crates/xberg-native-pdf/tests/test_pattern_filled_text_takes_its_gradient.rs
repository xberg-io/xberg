//! Text filled with a shading pattern paints the gradient, not flat black.
//!
//! ISO 32000-1:2008 §8.7.4.1 (`docs/spec/pdf.md`:12899-12902) names the text
//! operators explicitly among those a shading pattern may paint with:
//!
//! > By setting a shading pattern as the current colour in the graphics state,
//! > a PDF content stream may use it with painting operators such as **f**
//! > (fill), **S** (stroke), **Tj** (show text) ... to paint a path, character
//! > glyph, or mask with a smooth colour transition.
//!
//! The path half of that was routed to the shading painter; text was not, and
//! the rasterizer had no pattern awareness at all — the fill resolved to
//! `Pattern [] -> (0.0, 0.0, 0.0)` before it reached glyph paint. A coloured
//! pattern is selected by `/P0 scn` with **no operands**, so there is no
//! colour to fall back to and the stale one is used; in a stream whose first
//! colour operator is that `scn`, the stale value is the initial black.
//!
//! Table 77 (`docs/spec/pdf.md`:12929) fixes the frame: a type 2 pattern's
//! coordinates are in *pattern* space, which §8.7.2 (:12338-12342) defines
//! against the parent content stream's **default** space. So the gradient must
//! not track the text matrix — only the glyph coverage comes from the text.

use xberg_native_pdf::PdfDocument;
use xberg_native_pdf::rendering::{RenderOptions, render_page};

fn be16(v: u16) -> [u8; 2] {
    v.to_be_bytes()
}

fn be32(v: u32) -> [u8; 4] {
    v.to_be_bytes()
}

/// A minimal TrueType font: glyph 0 empty, glyph 1 a box, and a single
/// byte-indexed cmap subtable (platform 1, encoding 0, format 0) that maps
/// only 'A' (0x41) to glyph 1. Every other byte resolves to glyph 0.
fn broken_subset_font() -> Vec<u8> {
    // glyf: glyph 0 is empty (zero bytes). Glyph 1 is one square contour,
    // four on-curve points, 16-bit deltas.
    let mut glyf: Vec<u8> = Vec::new();
    glyf.extend(be16(1)); // numberOfContours
    glyf.extend(be16(50)); // xMin
    glyf.extend(be16(0)); // yMin
    glyf.extend(be16(450)); // xMax
    glyf.extend(be16(700)); // yMax
    glyf.extend(be16(3)); // endPtsOfContours[0]
    glyf.extend(be16(0)); // instructionLength
    glyf.extend([0x01, 0x01, 0x01, 0x01]); // flags: on-curve, long deltas
    for dx in [50i16, 400, 0, -400] {
        glyf.extend(dx.to_be_bytes());
    }
    for dy in [0i16, 0, 700, 0] {
        glyf.extend(dy.to_be_bytes());
    }

    // loca (short format, offset/2): glyph 0 empty, glyph 1 = all of glyf.
    let mut loca: Vec<u8> = Vec::new();
    loca.extend(be16(0));
    loca.extend(be16(0));
    loca.extend(be16((glyf.len() / 2) as u16));

    let mut head: Vec<u8> = Vec::new();
    head.extend(be32(0x0001_0000)); // version
    head.extend(be32(0)); // fontRevision
    head.extend(be32(0)); // checkSumAdjustment
    head.extend(be32(0x5F0F_3CF5)); // magicNumber
    head.extend(be16(0)); // flags
    head.extend(be16(1000)); // unitsPerEm
    head.extend([0u8; 16]); // created + modified
    head.extend(be16(0)); // xMin
    head.extend(be16(0)); // yMin
    head.extend(be16(450)); // xMax
    head.extend(be16(700)); // yMax
    head.extend(be16(0)); // macStyle
    head.extend(be16(8)); // lowestRecPPEM
    head.extend(be16(2)); // fontDirectionHint
    head.extend(be16(0)); // indexToLocFormat: short
    head.extend(be16(0)); // glyphDataFormat

    let mut hhea: Vec<u8> = Vec::new();
    hhea.extend(be32(0x0001_0000)); // version
    hhea.extend(be16(800)); // ascender
    hhea.extend((-200i16).to_be_bytes()); // descender
    hhea.extend(be16(0)); // lineGap
    hhea.extend(be16(500)); // advanceWidthMax
    hhea.extend(be16(0)); // minLeftSideBearing
    hhea.extend(be16(0)); // minRightSideBearing
    hhea.extend(be16(450)); // xMaxExtent
    hhea.extend(be16(1)); // caretSlopeRise
    hhea.extend(be16(0)); // caretSlopeRun
    hhea.extend(be16(0)); // caretOffset
    hhea.extend([0u8; 8]); // reserved
    hhea.extend(be16(0)); // metricDataFormat
    hhea.extend(be16(2)); // numberOfHMetrics

    let mut hmtx: Vec<u8> = Vec::new();
    for _ in 0..2 {
        hmtx.extend(be16(500)); // advanceWidth
        hmtx.extend(be16(0)); // leftSideBearing
    }

    let mut maxp: Vec<u8> = Vec::new();
    maxp.extend(be32(0x0001_0000)); // version
    maxp.extend(be16(2)); // numGlyphs
    maxp.extend([0u8; 26]); // limits, unused by the parser

    // cmap: version 0, one encoding record → format 0 subtable mapping
    // only 0x41 → glyph 1.
    let mut cmap: Vec<u8> = Vec::new();
    cmap.extend(be16(0)); // version
    cmap.extend(be16(1)); // numTables
    cmap.extend(be16(1)); // platformID: Macintosh
    cmap.extend(be16(0)); // encodingID: Roman
    cmap.extend(be32(12)); // subtable offset
    cmap.extend(be16(0)); // format 0
    cmap.extend(be16(262)); // length
    cmap.extend(be16(0)); // language
    let mut glyph_ids = [0u8; 256];
    glyph_ids[0x41] = 1;
    cmap.extend(glyph_ids);

    // Assemble the sfnt: table records sorted by tag.
    let tables: [(&[u8; 4], &Vec<u8>); 7] = [
        (b"cmap", &cmap),
        (b"glyf", &glyf),
        (b"head", &head),
        (b"hhea", &hhea),
        (b"hmtx", &hmtx),
        (b"loca", &loca),
        (b"maxp", &maxp),
    ];
    let num_tables = tables.len() as u16;
    let mut font: Vec<u8> = Vec::new();
    font.extend(be32(0x0001_0000)); // sfnt version
    font.extend(be16(num_tables));
    let entry_selector = 15 - num_tables.leading_zeros() as u16; // floor(log2 n)
    let search_range = (1u16 << entry_selector) * 16;
    font.extend(be16(search_range));
    font.extend(be16(entry_selector));
    font.extend(be16(num_tables * 16 - search_range));

    let mut offset = 12 + 16 * tables.len();
    for (tag, data) in &tables {
        font.extend_from_slice(*tag);
        font.extend(be32(0)); // checksum: not verified by the parser
        font.extend(be32(offset as u32));
        font.extend(be32(data.len() as u32));
        offset += data.len().div_ceil(4) * 4;
    }
    for (_, data) in &tables {
        font.extend_from_slice(data);
        font.extend(std::iter::repeat_n(0u8, data.len().div_ceil(4) * 4 - data.len()));
    }
    font
}

/// A 200x100 page drawing one embedded glyph under a red-to-blue axial shading
/// pattern. No colour operator precedes the `scn`, so the fill is still the
/// initial black — the same shape as the path-side fixture, which makes the
/// failure unambiguous.
fn pattern_filled_text(render_mode: u8) -> Vec<u8> {
    pattern_filled_text_under(render_mode, None)
}

/// The same page with a luminosity soft mask in force, whose group paints the whole page with
/// `mask_paint` (`0 g` hides everything, `1 g` shows everything).
fn pattern_filled_text_under(render_mode: u8, mask_paint: Option<&str>) -> Vec<u8> {
    let font = broken_subset_font();
    let state = if mask_paint.is_some() { "/GS0 gs " } else { "" };
    let content = format!("{state}BT /F1 96 Tf {render_mode} Tr /Pattern cs /P0 scn 1 0 0 1 10 20 Tm (A) Tj ET\n");
    let content = content.into_bytes();

    let mut pdf: Vec<u8> = Vec::new();
    let last = 11usize;
    let mut off = vec![0usize; last + 1];
    let obj = |pdf: &mut Vec<u8>, off: &mut Vec<usize>, id: usize, body: &str| {
        off[id] = pdf.len();
        pdf.extend_from_slice(format!("{id} 0 obj\n{body}\nendobj\n").as_bytes());
    };

    pdf.extend_from_slice(b"%PDF-1.7\n");
    obj(&mut pdf, &mut off, 1, "<< /Type /Catalog /Pages 2 0 R >>");
    obj(&mut pdf, &mut off, 2, "<< /Type /Pages /Kids [3 0 R] /Count 1 >>");
    obj(
        &mut pdf,
        &mut off,
        3,
        "<< /Type /Page /Parent 2 0 R /MediaBox [0 0 200 100] /Contents 4 0 R \
         /Resources << /Font << /F1 7 0 R >> /Pattern << /P0 5 0 R >> /ExtGState << /GS0 10 0 R >> >> >>",
    );
    off[4] = pdf.len();
    pdf.extend_from_slice(format!("4 0 obj\n<< /Length {} >>\nstream\n", content.len()).as_bytes());
    pdf.extend_from_slice(&content);
    pdf.extend_from_slice(b"endstream\nendobj\n");
    // The gradient runs across the glyph body only, so the two sampled points
    // sit far apart on the ramp. Extend keeps the ends solid.
    obj(
        &mut pdf,
        &mut off,
        5,
        "<< /Type /Pattern /PatternType 2 /Matrix [1 0 0 1 0 0] \
         /Shading << /ShadingType 2 /ColorSpace /DeviceRGB /Coords [14 0 58 0] \
         /Extend [true true] /Function 6 0 R >> >>",
    );
    obj(
        &mut pdf,
        &mut off,
        6,
        "<< /FunctionType 2 /Domain [0 1] /N 1 /C0 [1 0 0] /C1 [0 0 1] >>",
    );
    obj(
        &mut pdf,
        &mut off,
        7,
        "<< /Type /Font /Subtype /TrueType /BaseFont /TestGlyph /FirstChar 65 \
         /LastChar 65 /Widths [500] /FontDescriptor 8 0 R >>",
    );
    obj(
        &mut pdf,
        &mut off,
        8,
        "<< /Type /FontDescriptor /FontName /TestGlyph /Flags 4 \
         /FontBBox [0 0 450 700] /ItalicAngle 0 /Ascent 800 /Descent -200 \
         /CapHeight 700 /StemV 80 /FontFile2 9 0 R >>",
    );
    off[9] = pdf.len();
    pdf.extend_from_slice(
        format!(
            "9 0 obj\n<< /Length {} /Length1 {} >>\nstream\n",
            font.len(),
            font.len()
        )
        .as_bytes(),
    );
    pdf.extend_from_slice(&font);
    pdf.extend_from_slice(b"\nendstream\nendobj\n");
    obj(
        &mut pdf,
        &mut off,
        10,
        "<< /Type /ExtGState /SMask << /Type /Mask /S /Luminosity /G 11 0 R >> >>",
    );
    let mask_content = format!("{} 0 0 200 100 re f", mask_paint.unwrap_or("1 g"));
    off[11] = pdf.len();
    pdf.extend_from_slice(
        format!(
            "11 0 obj\n<< /Type /XObject /Subtype /Form /BBox [0 0 200 100] \
             /Group << /S /Transparency /CS /DeviceGray >> /Length {} >>\nstream\n{mask_content}\nendstream\nendobj\n",
            mask_content.len()
        )
        .as_bytes(),
    );

    let xref = pdf.len();
    pdf.extend_from_slice(format!("xref\n0 {}\n0000000000 65535 f \r\n", last + 1).as_bytes());
    for offset in &off[1..=last] {
        pdf.extend_from_slice(format!("{offset:010} 00000 n \r\n").as_bytes());
    }
    pdf.extend_from_slice(
        format!(
            "trailer\n<< /Size {} /Root 1 0 R >>\nstartxref\n{xref}\n%%EOF\n",
            last + 1
        )
        .as_bytes(),
    );
    pdf
}

fn render(pdf: Vec<u8>) -> image::RgbaImage {
    let doc = PdfDocument::from_bytes(pdf).expect("synthetic PDF parses");
    let img = render_page(&doc, 0, &RenderOptions::default()).expect("page renders");
    image::load_from_memory(&img.data).expect("PNG decodes").to_rgba8()
}

/// Every pixel the glyph covers, as (x, y, rgb).
fn glyph_pixels(px: &image::RgbaImage) -> Vec<(u32, u32, [u8; 3])> {
    px.enumerate_pixels()
        .filter(|(_, _, p)| p[3] > 250 && !(p[0] > 245 && p[1] > 245 && p[2] > 245))
        .map(|(x, y, p)| (x, y, [p[0], p[1], p[2]]))
        .collect()
}

#[test]
fn pattern_filled_text_is_not_painted_black() {
    let px = render(pattern_filled_text(0));
    let painted = glyph_pixels(&px);
    assert!(
        !painted.is_empty(),
        "nothing painted at all — the fixture, not the fix, is wrong"
    );
    let black = painted
        .iter()
        .filter(|(_, _, c)| u32::from(c[0]) + u32::from(c[2]) < 200)
        .count();
    assert!(
        black * 4 < painted.len(),
        "{black} of {} painted pixels are near-black; a shading pattern fill on \
         text is being painted as a solid colour",
        painted.len()
    );
}

#[test]
fn pattern_filled_text_takes_the_gradient() {
    let px = render(pattern_filled_text(0));
    let painted = glyph_pixels(&px);
    let min_x = painted.iter().map(|(x, _, _)| *x).min().expect("painted");
    let max_x = painted.iter().map(|(x, _, _)| *x).max().expect("painted");
    let mean_red = |lo: u32, hi: u32| -> f64 {
        let v: Vec<f64> = painted
            .iter()
            .filter(|(x, _, _)| *x >= lo && *x <= hi)
            .map(|(_, _, c)| f64::from(c[0]))
            .collect();
        v.iter().sum::<f64>() / v.len().max(1) as f64
    };
    let band = (max_x - min_x) / 4;
    let left = mean_red(min_x, min_x + band);
    let right = mean_red(max_x - band, max_x);
    assert!(
        left - right > 15.0,
        "red should fall left-to-right across the glyph ({left:.1} -> {right:.1}); \
         the shading is being resolved to a single flat sample"
    );
}

/// Render mode 3 paints nothing (§9.3.6). The coverage gs deliberately forces
/// mode 0, so a mask-driven route that does not exclude mode 3 would make every
/// invisible OCR layer paint a gradient across the page.
#[test]
fn test_invisible_show_paints_no_pattern() {
    let px = render(pattern_filled_text(3));
    assert!(
        glyph_pixels(&px).is_empty(),
        "mode 3 is invisible, but the pattern painted anyway"
    );
}

/// The soft mask in force at the show bounds a pattern-filled glyph like any other paint: a mask
/// that is black everywhere hides it, and one that is white everywhere leaves it.
#[test]
fn test_pattern_filled_text_honours_the_soft_mask() {
    let shown = glyph_pixels(&render(pattern_filled_text_under(0, Some("1 g"))));
    assert!(!shown.is_empty(), "a fully open mask must leave the glyph painted");
    let hidden = glyph_pixels(&render(pattern_filled_text_under(0, Some("0 g"))));
    assert!(
        hidden.is_empty(),
        "a fully closed soft mask left {} glyph pixels painted",
        hidden.len()
    );
}
