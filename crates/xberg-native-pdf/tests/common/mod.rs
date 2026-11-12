//! Shared helpers for the integration suite.
//!
//! Integration tests are separate crates, so this is included with `mod common;`
//! rather than being a library. Each test binary uses only part of it, hence the
//! crate-level `dead_code` allow.
//!
//! `fixture-hygiene` requires reproducers to be minimal PDFs constructed in
//! code. Before this module the same builder was copy-pasted into a dozen test
//! files — eleven of them byte-identical — which is how they drifted apart.

#![allow(dead_code)]

/// Write `data` to a uniquely-named file inside a fresh temporary directory
/// and return both. Each call gets its own directory (via `tempfile`), so
/// concurrent test processes never observe one another's writes — unlike a
/// fixed path under `std::env::temp_dir()`, which is shared by every process
/// on the machine.
///
/// Keep the returned `TempDir` alive for as long as the path is used: it
/// deletes the directory (and the file in it) when dropped. Use the same
/// `TempDir` (via `.path()`) to build sibling paths, e.g. an output file
/// written alongside the input.
pub fn write_temp_pdf(data: &[u8], name: &str) -> (tempfile::TempDir, std::path::PathBuf) {
    let dir = tempfile::tempdir().expect("Failed to create temp dir");
    let path = dir.path().join(name);
    std::fs::write(&path, data).expect("Failed to write temp file");
    (dir, path)
}

pub fn build_minimal_pdf_raw(content: &[u8], page_extra: &[u8]) -> Vec<u8> {
    let mut pdf = b"%PDF-1.4\n".to_vec();
    let mut offsets = vec![0usize];

    offsets.push(pdf.len());
    pdf.extend_from_slice(b"1 0 obj\n<< /Type /Catalog /Pages 2 0 R >>\nendobj\n");

    offsets.push(pdf.len());
    pdf.extend_from_slice(b"2 0 obj\n<< /Type /Pages /Kids [3 0 R] /Count 1 >>\nendobj\n");

    offsets.push(pdf.len());
    pdf.extend_from_slice(b"3 0 obj\n<< ");
    pdf.extend_from_slice(page_extra);
    pdf.extend_from_slice(b" /Contents 4 0 R /Resources << /Font << /F1 5 0 R >> >> >>\nendobj\n");

    offsets.push(pdf.len());
    pdf.extend_from_slice(format!("4 0 obj\n<< /Length {} >>\nstream\n", content.len()).as_bytes());
    pdf.extend_from_slice(content);
    pdf.extend_from_slice(b"\nendstream\nendobj\n");

    offsets.push(pdf.len());
    pdf.extend_from_slice(
        b"5 0 obj\n<< /Type /Font /Subtype /Type1 /BaseFont /Helvetica \
          /Encoding /WinAnsiEncoding >>\nendobj\n",
    );

    finalize_pdf(pdf, &offsets)
}

/// Append the cross-reference table, trailer and `startxref` for a body whose
/// object offsets are `offsets` (index 0 is the free head and is ignored).
pub fn finalize_pdf(mut pdf: Vec<u8>, offsets: &[usize]) -> Vec<u8> {
    let xref_pos = pdf.len();
    pdf.extend_from_slice(format!("xref\n0 {}\n", offsets.len()).as_bytes());
    pdf.extend_from_slice(b"0000000000 65535 f\r\n");
    for &off in &offsets[1..] {
        pdf.extend_from_slice(format!("{off:010} 00000 n\r\n").as_bytes());
    }
    pdf.extend_from_slice(
        format!(
            "trailer\n<< /Size {} /Root 1 0 R >>\nstartxref\n{}\n%%EOF\n",
            offsets.len(),
            xref_pos
        )
        .as_bytes(),
    );
    pdf
}

/// Standard-14 font resource names the (now-removed) PDF writer registered
/// into every page's `/Resources /Font` dictionary, keyed by the literal
/// name a `Tf` operator references directly (e.g. `/Helvetica-Bold 12 Tf`).
const STANDARD_14_FONTS: [&str; 12] = [
    "Helvetica",
    "Helvetica-Bold",
    "Helvetica-Oblique",
    "Helvetica-BoldOblique",
    "Times-Roman",
    "Times-Bold",
    "Times-Italic",
    "Times-BoldItalic",
    "Courier",
    "Courier-Bold",
    "Courier-Oblique",
    "Courier-BoldOblique",
];

/// Escape a text-show string for a PDF literal string operand. Covers the
/// ASCII subset (parens, backslash, and the common whitespace escapes)
/// these hand-built fixtures use; non-ASCII text is written through
/// unescaped (callers needing WinAnsi/Unicode mapping should not use this
/// helper).
pub fn pdf_escape_text(text: &str) -> String {
    let mut out = String::with_capacity(text.len());
    for ch in text.chars() {
        match ch {
            '(' => out.push_str("\\("),
            ')' => out.push_str("\\)"),
            '\\' => out.push_str("\\\\"),
            '\n' => out.push_str("\\n"),
            '\r' => out.push_str("\\r"),
            '\t' => out.push_str("\\t"),
            other => out.push(other),
        }
    }
    out
}

/// Emit `text` as its own `BT ... Tf ... Tm ... Tj ET` text object at
/// `(x, y)` in `font`/`size`.
///
/// This previously appended a degenerate stroked rectangle (`0 0 0 0 re S`),
/// on the theory that the no-op path was needed to force the next text run
/// into a fresh `BT`. It is not: each call already emits a complete, balanced
/// `BT ... ET`, so consecutive runs are separate text objects either way, and
/// the full suite is green without it. What the rectangle did do was put a
/// real path into every fixture's content stream, so `extract_rects` returned
/// one for a text-only page and `test_rect_and_line_extraction_empty` failed.
/// Do not reintroduce it: a text fixture must contain no path operators.
pub fn text_run_op(text: &str, x: f32, y: f32, font: &str, size: f32) -> String {
    format!(
        "BT\n/{font} {size} Tf\n1 0 0 1 {x} {y} Tm\n({}) Tj\nET\n",
        pdf_escape_text(text)
    )
}

/// Build a one-page PDF whose `/Resources /Font` dictionary carries all
/// twelve Standard-14 fonts, keyed by their literal PDF names — matching
/// what `xberg_native_pdf::writer::PdfWriter::finish` used to register
/// unconditionally on every page, back when these fixtures were built with
/// the (now-removed) PDF writer. `content` is the assembled content stream
/// (see [`text_run_op`]); `page_extra` is inserted into the page
/// dictionary alongside `/Contents` and `/Resources` (e.g.
/// `b"/Type /Page /Parent 2 0 R /MediaBox [0 0 612 792]"`).
pub fn build_pdf_with_standard_fonts(content: &[u8], page_extra: &[u8]) -> Vec<u8> {
    let mut pdf = b"%PDF-1.4\n".to_vec();
    let mut offsets = vec![0usize];

    offsets.push(pdf.len());
    pdf.extend_from_slice(b"1 0 obj\n<< /Type /Catalog /Pages 2 0 R >>\nendobj\n");

    offsets.push(pdf.len());
    pdf.extend_from_slice(b"2 0 obj\n<< /Type /Pages /Kids [3 0 R] /Count 1 >>\nendobj\n");

    let first_font_obj_id = 5usize;
    let mut font_dict_entries = String::new();
    for (i, name) in STANDARD_14_FONTS.iter().enumerate() {
        font_dict_entries.push_str(&format!("/{name} {} 0 R ", first_font_obj_id + i));
    }

    offsets.push(pdf.len());
    pdf.extend_from_slice(b"3 0 obj\n<< ");
    pdf.extend_from_slice(page_extra);
    pdf.extend_from_slice(
        format!(" /Contents 4 0 R /Resources << /Font << {font_dict_entries}>> >> >>\nendobj\n").as_bytes(),
    );

    offsets.push(pdf.len());
    pdf.extend_from_slice(format!("4 0 obj\n<< /Length {} >>\nstream\n", content.len()).as_bytes());
    pdf.extend_from_slice(content);
    pdf.extend_from_slice(b"\nendstream\nendobj\n");

    for (i, name) in STANDARD_14_FONTS.iter().enumerate() {
        debug_assert_eq!(offsets.len(), first_font_obj_id + i);
        offsets.push(pdf.len());
        pdf.extend_from_slice(
            format!(
                "{} 0 obj\n<< /Type /Font /Subtype /Type1 /BaseFont /{name} /Encoding /WinAnsiEncoding >>\nendobj\n",
                first_font_obj_id + i
            )
            .as_bytes(),
        );
    }

    finalize_pdf(pdf, &offsets)
}

/// Build a Letter-size (612×792 pt) one-page PDF from `content` (see
/// [`text_run_op`]) and extract page 0's plain text. Matches the
/// `build_and_extract` idiom several reading-order/line-grouping
/// regression tests used when they were built with the (now-removed) PDF
/// writer.
pub fn build_and_extract_page0(content: &str) -> String {
    let bytes = build_pdf_with_standard_fonts(content.as_bytes(), b"/Type /Page /Parent 2 0 R /MediaBox [0 0 612 792]");
    let doc = xberg_native_pdf::document::PdfDocument::from_bytes(bytes).expect("open PDF");
    doc.extract_text(0).expect("extract page 0")
}

/// Assemble an sfnt (TrueType) font program from `(tag, data)` tables. The
/// tables must be sorted by tag; checksums are left zero, which the parsers
/// in this crate accept.
pub fn build_sfnt(tables: &[(&[u8; 4], Vec<u8>)]) -> Vec<u8> {
    let num_tables = u16::try_from(tables.len()).expect("table count fits u16");
    let entry_selector = 15 - num_tables.leading_zeros() as u16;
    let search_range = (1u16 << entry_selector) * 16;
    let mut font: Vec<u8> = Vec::new();
    font.extend(0x0001_0000u32.to_be_bytes());
    font.extend(num_tables.to_be_bytes());
    font.extend(search_range.to_be_bytes());
    font.extend(entry_selector.to_be_bytes());
    font.extend((num_tables * 16 - search_range).to_be_bytes());
    let mut offset = 12 + 16 * tables.len();
    for (tag, data) in tables {
        font.extend_from_slice(*tag);
        font.extend(0u32.to_be_bytes());
        font.extend(u32::try_from(offset).expect("offset fits u32").to_be_bytes());
        font.extend(u32::try_from(data.len()).expect("length fits u32").to_be_bytes());
        offset += data.len().div_ceil(4) * 4;
    }
    for (_, data) in tables {
        font.extend_from_slice(data);
        font.extend(std::iter::repeat_n(0u8, data.len().div_ceil(4) * 4 - data.len()));
    }
    font
}

/// Parameters of the minimal TrueType fixture font assembled by
/// [`build_box_glyph_font`]: glyph 0 empty, glyph 1 a filled rectangle, and a
/// single byte-indexed cmap subtable (platform 1, encoding 0, format 0)
/// mapping exactly one byte to glyph 1.
pub struct BoxGlyphFont {
    /// Outline rectangle of glyph 1: `(x_min, y_min, x_max, y_max)` in font
    /// units of the 1000-unit em.
    pub outline: [u16; 4],
    /// Bounding box the `head` table reports.
    pub head_bbox: [u16; 4],
    /// Left side bearing `hmtx` reports for glyph 1.
    pub lsb: u16,
    /// The one byte the cmap maps to glyph 1; every other byte resolves to
    /// glyph 0.
    pub cmap_byte: u8,
}

/// Assemble the shared box-glyph fixture font.
///
/// `knockout_authorfix_2855`, `test_glyph_drop_reporting` and
/// `gh1632_whitespace_inferred_glyph` each carried a private copy of this
/// builder that differed only in the rectangle, the mapped byte and the
/// glyph-1 left side bearing; those are the parameters. The advance width
/// (500), ascent/descent (800/-200) and `hhea` fields are fixed, matching
/// all three originals byte for byte.
pub fn build_box_glyph_font(spec: BoxGlyphFont) -> Vec<u8> {
    let [x0, y0, x1, y1] = spec.outline;

    // Glyph 1: one contour, four on-curve points tracing the rectangle.
    let mut glyf: Vec<u8> = Vec::new();
    glyf.extend(1u16.to_be_bytes());
    for v in spec.outline {
        glyf.extend(v.to_be_bytes());
    }
    glyf.extend(3u16.to_be_bytes());
    glyf.extend(0u16.to_be_bytes());
    glyf.extend([0x01; 4]);
    for dx in [x0 as i16, (x1 - x0) as i16, 0, -((x1 - x0) as i16)] {
        glyf.extend(dx.to_be_bytes());
    }
    for dy in [0i16, 0, (y1 - y0) as i16, 0] {
        glyf.extend(dy.to_be_bytes());
    }

    // loca, short format (offset/2): glyph 0 empty, glyph 1 = all of glyf. ~keep
    let mut loca: Vec<u8> = Vec::new();
    loca.extend(0u16.to_be_bytes());
    loca.extend(0u16.to_be_bytes());
    loca.extend(
        u16::try_from(glyf.len() / 2)
            .expect("glyph 1 fits short loca")
            .to_be_bytes(),
    );

    let mut head: Vec<u8> = Vec::new();
    head.extend(0x0001_0000u32.to_be_bytes());
    head.extend(0u32.to_be_bytes());
    head.extend(0u32.to_be_bytes());
    head.extend(0x5F0F_3CF5u32.to_be_bytes());
    head.extend(0u16.to_be_bytes());
    head.extend(1000u16.to_be_bytes());
    head.extend([0u8; 16]);
    for v in spec.head_bbox {
        head.extend(v.to_be_bytes());
    }
    head.extend(0u16.to_be_bytes());
    head.extend(8u16.to_be_bytes());
    head.extend(2u16.to_be_bytes());
    head.extend(0u16.to_be_bytes());
    head.extend(0u16.to_be_bytes());

    let mut hhea: Vec<u8> = Vec::new();
    hhea.extend(0x0001_0000u32.to_be_bytes());
    hhea.extend(800u16.to_be_bytes());
    hhea.extend((-200i16).to_be_bytes());
    hhea.extend(0u16.to_be_bytes());
    hhea.extend(500u16.to_be_bytes());
    for _ in 0..2 {
        hhea.extend(0u16.to_be_bytes());
    }
    hhea.extend(x1.to_be_bytes());
    hhea.extend(1u16.to_be_bytes());
    hhea.extend(0u16.to_be_bytes());
    hhea.extend(0u16.to_be_bytes());
    hhea.extend([0u8; 8]);
    hhea.extend(0u16.to_be_bytes());
    hhea.extend(2u16.to_be_bytes());

    let mut hmtx: Vec<u8> = Vec::new();
    for lsb in [0u16, spec.lsb] {
        hmtx.extend(500u16.to_be_bytes());
        hmtx.extend(lsb.to_be_bytes());
    }

    let mut maxp: Vec<u8> = Vec::new();
    maxp.extend(0x0001_0000u32.to_be_bytes());
    maxp.extend(2u16.to_be_bytes());
    maxp.extend([0u8; 26]);

    let mut cmap: Vec<u8> = Vec::new();
    cmap.extend(0u16.to_be_bytes());
    cmap.extend(1u16.to_be_bytes());
    cmap.extend(1u16.to_be_bytes());
    cmap.extend(0u16.to_be_bytes());
    cmap.extend(12u32.to_be_bytes());
    cmap.extend(0u16.to_be_bytes());
    cmap.extend(262u16.to_be_bytes());
    cmap.extend(0u16.to_be_bytes());
    let mut glyph_ids = [0u8; 256];
    glyph_ids[usize::from(spec.cmap_byte)] = 1;
    cmap.extend(glyph_ids);

    build_sfnt(&[
        (b"cmap", cmap),
        (b"glyf", glyf),
        (b"head", head),
        (b"hhea", hhea),
        (b"hmtx", hmtx),
        (b"loca", loca),
        (b"maxp", maxp),
    ])
}
