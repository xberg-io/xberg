//! The glyph-drop diagnostic must not fire on every OCR'd PDF.
//!
//! `outline_glyph` returns `None` for an **empty** glyph as well as a missing
//! one, so the tally cannot by itself tell "the font gave us nothing" from
//! "the font says paint nothing". Two populations are legitimately the latter:
//!
//! - Invisible text, ISO 32000-1:2008 §9.3.6 render modes 3 and 7 — an OCR text
//!   layer under a scanned page image, meant to be searchable and not drawn.
//! - The glyphless fonts OCR tools emit. Tesseract and ocrmypdf ship a
//!   synthetic font, conventionally named with GLYPHLESS, mapping every CID to
//!   a non-zero glyph id with an empty outline and a correct `/ToUnicode`.
//!
//! Every such page renders and extracts perfectly and still emitted one warning
//! per page claiming invisible data loss. `src/extractors/text.rs` already
//! gates on exactly these two signals for the same population; the diagnostic
//! gated on neither. A diagnostic that fires on the healthy common case trains
//! callers to ignore the channel.
//!
//! The fixture is the one from the neighbouring drop-reporting test — a font
//! that genuinely drops a glyph — so these assertions are about the *gate*,
//! not about a page that happens never to drop anything.
//!
//! Render mode 7 is deliberately **not** asserted here. It is invisible too and
//! the gate names it, but such a page is rasterised twice — once with a default
//! graphics state (`render_mode = 0`) and once with mode 7 — and the first pass
//! emits before the gate can see the mode. That double rasterise is a separate
//! defect; asserting the current behaviour here would pin it, and asserting the
//! desired behaviour would fail for a reason this change does not own. Modes 3
//! and the glyphless-font convention are the populations the report is about:
//! Tesseract and ocrmypdf emit `Tr 3`.

use xberg_native_pdf::PdfDocument;
use xberg_native_pdf::extractors::warnings::WarningCategory;
use xberg_native_pdf::rendering::{PageRenderer, RenderOptions};

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
    let entry_selector = 15 - num_tables.leading_zeros() as u16;
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

/// A PDF with `page_count` identical pages, each painting "AB" in the
/// broken embedded font. 'A' paints; 'B' drops.
fn pdf_with_broken_font(page_count: usize, base_font: &str, render_mode: u8) -> Vec<u8> {
    let font = broken_subset_font();
    let content = format!("BT /F1 24 Tf {render_mode} Tr 50 100 Td (AB) Tj ET");
    let content = content.as_bytes();
    let n_objs = 5 + 2 * page_count; // catalog, pages, font, descriptor, fontfile + per page: page, contents

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
        format!(
            "<< /Type /Font /Subtype /TrueType /BaseFont /{base_font} /FirstChar 65 /LastChar 66 \
             /Widths [500 500] /FontDescriptor 4 0 R >>"
        ),
    );
    obj(
        &mut buf,
        &mut off,
        4,
        format!(
            "<< /Type /FontDescriptor /FontName /{base_font} /Flags 4 /FontBBox [0 0 450 700] \
             /ItalicAngle 0 /Ascent 800 /Descent -200 /CapHeight 700 /StemV 80 /FontFile2 5 0 R >>"
        ),
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
    for o in &off[1..=n_objs] {
        buf.extend_from_slice(format!("{o:010} 00000 n \n").as_bytes());
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

/// Render every page and return the glyph-drop warnings. Messages redact the
/// font name, and the fixture holds a single font, so none is filtered by name.
fn drop_warnings(base_font: &str, render_mode: u8) -> Vec<String> {
    let pdf = pdf_with_broken_font(2, base_font, render_mode);
    let doc = PdfDocument::from_bytes(pdf).expect("parse");
    for page in 0..2 {
        let mut renderer = PageRenderer::new(RenderOptions::default());
        let _ = renderer.render_page(&doc, page);
    }
    doc.take_structured_warnings()
        .into_iter()
        .filter(|w| w.category == WarningCategory::GlyphDropped)
        .map(|w| w.message)
        .collect()
}

/// The control: this font really does drop a glyph, and at a visible render
/// mode in an ordinary font the warning must still fire. Without this the
/// tests below would pass vacuously.
#[test]
fn test_genuine_drop_still_warns() {
    let warnings = drop_warnings("BrokenSubset", 0);
    assert!(
        !warnings.is_empty(),
        "the fixture must genuinely drop a glyph or the gate tests prove nothing"
    );
}

/// Invisible text (`Tr 3`) is an OCR layer: not loss.
#[test]
fn invisible_render_mode_three_reports_no_glyph_loss() {
    assert!(
        drop_warnings("BrokenSubset", 3).is_empty(),
        "an invisible OCR text layer was reported as dropped glyphs"
    );
}

/// The glyphless-font naming convention, at a visible render mode: the font
/// itself says it paints nothing.
#[test]
fn test_glyphless_font_reports_no_glyph_loss() {
    assert!(
        drop_warnings("GlyphLessFont", 0).is_empty(),
        "an OCR glyphless font was reported as dropped glyphs"
    );
}
