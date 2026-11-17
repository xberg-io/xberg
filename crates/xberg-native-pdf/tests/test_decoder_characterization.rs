//! Characterization: the unified glyph decoder must give extraction and
//! rendering the same view of the same bytes, across the font families the
//! two former decoders diverged on.
//!
//! Each family embeds a font built in code (four glyphs: .notdef and three
//! boxes with advances 500/600/700 per mille), paints the same logical text,
//! and pins: the decoded string, the per-char sequence, the per-char
//! advances against the declared widths, and — under `rendering` — that a
//! render paints without a `GlyphDropped` warning for every glyph the
//! extractor decodes. The subset family holds the inverse: a code the font
//! cannot paint keeps its character in extraction and raises exactly one
//! drop warning in rendering.

use xberg_native_pdf::PdfDocument;

fn be16(v: u16) -> [u8; 2] {
    v.to_be_bytes()
}

fn be32(v: u32) -> [u8; 4] {
    v.to_be_bytes()
}

/// Which character-to-glyph table the font carries.
#[derive(Clone, Copy)]
enum CmapKind {
    /// One byte-indexed subtable (platform 1, encoding 0, format 0):
    /// the simple-TrueType-subset shape, resolved byte -> GID.
    ByteIndexed,
    /// One Windows Unicode subtable (platform 3, encoding 1, format 4):
    /// the shape Unicode shaping resolves against.
    WindowsUnicode,
}

/// A minimal TrueType font: glyph 0 empty; glyphs 1..=n_mapped boxes with
/// advances 500/600/700, mapped from 'A'/'B'/'C'. `n_mapped` below 3 leaves
/// the remaining letters unmapped (they resolve to glyph 0).
fn test_font(kind: CmapKind, n_mapped: u16) -> Vec<u8> {
    assert!((1..=3).contains(&n_mapped));
    let mut one_glyph: Vec<u8> = Vec::new();
    one_glyph.extend(be16(1));
    one_glyph.extend(be16(50));
    one_glyph.extend(be16(0));
    one_glyph.extend(be16(450));
    one_glyph.extend(be16(700));
    one_glyph.extend(be16(3));
    one_glyph.extend(be16(0));
    one_glyph.extend([0x01, 0x01, 0x01, 0x01]);
    for dx in [50i16, 400, 0, -400] {
        one_glyph.extend(dx.to_be_bytes());
    }
    for dy in [0i16, 0, 700, 0] {
        one_glyph.extend(dy.to_be_bytes());
    }

    let mut glyf: Vec<u8> = Vec::new();
    let mut loca: Vec<u8> = Vec::new();
    loca.extend(be16(0));
    loca.extend(be16(0));
    for _ in 0..3 {
        glyf.extend(&one_glyph);
        loca.extend(be16((glyf.len() / 2) as u16));
    }

    let mut head: Vec<u8> = Vec::new();
    head.extend(be32(0x0001_0000));
    head.extend(be32(0));
    head.extend(be32(0));
    head.extend(be32(0x5F0F_3CF5));
    head.extend(be16(0));
    head.extend(be16(1000));
    head.extend([0u8; 16]);
    head.extend(be16(0));
    head.extend(be16(0));
    head.extend(be16(450));
    head.extend(be16(700));
    head.extend(be16(0));
    head.extend(be16(8));
    head.extend(be16(2));
    head.extend(be16(0));
    head.extend(be16(0));

    let mut hhea: Vec<u8> = Vec::new();
    hhea.extend(be32(0x0001_0000));
    hhea.extend(be16(800));
    hhea.extend((-200i16).to_be_bytes());
    hhea.extend(be16(0));
    hhea.extend(be16(700));
    hhea.extend(be16(0));
    hhea.extend(be16(0));
    hhea.extend(be16(450));
    hhea.extend(be16(1));
    hhea.extend(be16(0));
    hhea.extend(be16(0));
    hhea.extend([0u8; 8]);
    hhea.extend(be16(0));
    hhea.extend(be16(4));

    let mut hmtx: Vec<u8> = Vec::new();
    for adv in [500u16, 500, 600, 700] {
        hmtx.extend(be16(adv));
        hmtx.extend(be16(0));
    }

    let mut maxp: Vec<u8> = Vec::new();
    maxp.extend(be32(0x0001_0000));
    maxp.extend(be16(4));
    maxp.extend([0u8; 26]);

    let mut cmap: Vec<u8> = Vec::new();
    cmap.extend(be16(0));
    cmap.extend(be16(1));
    match kind {
        CmapKind::ByteIndexed => {
            cmap.extend(be16(1));
            cmap.extend(be16(0));
            cmap.extend(be32(12));
            cmap.extend(be16(0));
            cmap.extend(be16(262));
            cmap.extend(be16(0));
            let mut glyph_ids = [0u8; 256];
            for g in 1..=n_mapped {
                glyph_ids[0x40 + g as usize] = g as u8;
            }
            cmap.extend(glyph_ids);
        }
        CmapKind::WindowsUnicode => {
            cmap.extend(be16(3));
            cmap.extend(be16(1));
            cmap.extend(be32(12));
            cmap.extend(be16(4));
            cmap.extend(be16(32));
            cmap.extend(be16(0));
            cmap.extend(be16(4));
            cmap.extend(be16(4));
            cmap.extend(be16(1));
            cmap.extend(be16(0));
            cmap.extend(be16(0x40 + n_mapped));
            cmap.extend(be16(0xFFFF));
            cmap.extend(be16(0));
            cmap.extend(be16(0x41));
            cmap.extend(be16(0xFFFF));
            cmap.extend(be16(0xFFC0));
            cmap.extend(be16(1));
            cmap.extend(be16(0));
            cmap.extend(be16(0));
        }
    }

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
    font.extend(be32(0x0001_0000));
    font.extend(be16(num_tables));
    let entry_selector = 15 - num_tables.leading_zeros() as u16;
    let search_range = (1u16 << entry_selector) * 16;
    font.extend(be16(search_range));
    font.extend(be16(entry_selector));
    font.extend(be16(num_tables * 16 - search_range));
    let mut offset = 12 + 16 * tables.len();
    for (tag, data) in &tables {
        font.extend_from_slice(*tag);
        font.extend(be32(0));
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

/// Assemble a one-page PDF from raw object bodies; object 5 is the font
/// program stream, appended by the caller through `font`.
fn build_pdf(font_objects: &[String], content: &str, font: &[u8], font_obj_id: usize) -> Vec<u8> {
    let n_objs = 4 + font_objects.len() + 1;
    let mut buf: Vec<u8> = Vec::new();
    let mut off = vec![0usize; n_objs + 1];
    buf.extend_from_slice(b"%PDF-1.7\n%\xE2\xE3\xCF\xD3\n");
    {
        let obj = |buf: &mut Vec<u8>, off: &mut Vec<usize>, id: usize, body: &str| {
            off[id] = buf.len();
            buf.extend_from_slice(format!("{id} 0 obj\n{body}\nendobj\n").as_bytes());
        };
        obj(&mut buf, &mut off, 1, "<< /Type /Catalog /Pages 2 0 R >>");
        obj(&mut buf, &mut off, 2, "<< /Type /Pages /Kids [3 0 R] /Count 1 >>");
        obj(
            &mut buf,
            &mut off,
            3,
            "<< /Type /Page /Parent 2 0 R /MediaBox [0 0 300 200] \
             /Resources << /Font << /F1 5 0 R >> >> /Contents 4 0 R >>",
        );
        off[4] = buf.len();
        buf.extend_from_slice(
            format!(
                "4 0 obj\n<< /Length {} >>\nstream\n{content}\nendstream\nendobj\n",
                content.len() + 1
            )
            .as_bytes(),
        );
        for (i, body) in font_objects.iter().enumerate() {
            obj(&mut buf, &mut off, 5 + i, body);
        }
    }
    off[font_obj_id] = buf.len();
    buf.extend_from_slice(
        format!(
            "{font_obj_id} 0 obj\n<< /Length {} /Length1 {} >>\nstream\n",
            font.len(),
            font.len()
        )
        .as_bytes(),
    );
    buf.extend_from_slice(font);
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

/// Simple embedded TrueType painting "ABC" at 10 pt.
fn simple_truetype_pdf(kind: CmapKind, text: &str) -> Vec<u8> {
    let font = test_font(kind, 3);
    let content = format!("BT /F1 10 Tf 20 100 Td ({text}) Tj ET");
    let descriptor = "<< /Type /FontDescriptor /FontName /CharBox /Flags 4 \
                      /FontBBox [0 0 450 700] /ItalicAngle 0 /Ascent 800 /Descent -200 \
                      /CapHeight 700 /StemV 80 /FontFile2 7 0 R >>";
    let font_dict = "<< /Type /Font /Subtype /TrueType /BaseFont /CharBox /FirstChar 65 \
                     /LastChar 67 /Widths [500 600 700] /FontDescriptor 6 0 R >>";
    build_pdf(&[font_dict.to_string(), descriptor.to_string()], &content, &font, 7)
}

/// Type0 / Identity-H CID font painting CIDs 1..=3 ("ABC") at 10 pt.
fn type0_identity_pdf() -> Vec<u8> {
    let font = test_font(CmapKind::ByteIndexed, 3);
    let content = "BT /F1 10 Tf 20 100 Td <000100020003> Tj ET".to_string();
    let tounicode = "/CIDInit /ProcSet findresource begin\n\
                     12 dict begin begincmap\n\
                     /CMapName /T0Chars def /CMapType 2 def\n\
                     1 begincodespacerange <0000> <FFFF> endcodespacerange\n\
                     3 beginbfchar\n<0001> <0041>\n<0002> <0042>\n<0003> <0043>\nendbfchar\n\
                     endcmap CMapName currentdict /CMap defineresource pop end end";
    let font_dict = "<< /Type /Font /Subtype /Type0 /BaseFont /CharBox /Encoding /Identity-H \
                     /DescendantFonts [6 0 R] /ToUnicode 8 0 R >>";
    let cid_font = "<< /Type /Font /Subtype /CIDFontType2 /BaseFont /CharBox \
                    /CIDSystemInfo << /Registry (Adobe) /Ordering (Identity) /Supplement 0 >> \
                    /DW 1000 /W [1 [500] 2 [600] 3 [700]] /CIDToGIDMap /Identity \
                    /FontDescriptor 7 0 R >>";
    let descriptor = "<< /Type /FontDescriptor /FontName /CharBox /Flags 4 \
                      /FontBBox [0 0 450 700] /ItalicAngle 0 /Ascent 800 /Descent -200 \
                      /CapHeight 700 /StemV 80 /FontFile2 9 0 R >>";
    let tounicode_obj = format!("<< /Length {} >>\nstream\n{tounicode}\nendstream", tounicode.len() + 1);
    build_pdf(
        &[
            font_dict.to_string(),
            cid_font.to_string(),
            descriptor.to_string(),
            tounicode_obj,
        ],
        &content,
        &font,
        9,
    )
}

/// A subset mapping only 'A', painting "AB" at 10 pt: 'B' has no glyph.
fn subset_missing_glyph_pdf() -> Vec<u8> {
    let font = test_font(CmapKind::ByteIndexed, 1);
    let content = "BT /F1 10 Tf 20 100 Td (AB) Tj ET".to_string();
    let descriptor = "<< /Type /FontDescriptor /FontName /AAAAAA+CharBox /Flags 4 \
                      /FontBBox [0 0 450 700] /ItalicAngle 0 /Ascent 800 /Descent -200 \
                      /CapHeight 700 /StemV 80 /FontFile2 7 0 R >>";
    let font_dict = "<< /Type /Font /Subtype /TrueType /BaseFont /AAAAAA+CharBox /FirstChar 65 \
                     /LastChar 66 /Widths [500 500] /FontDescriptor 6 0 R >>";
    build_pdf(&[font_dict.to_string(), descriptor.to_string()], &content, &font, 7)
}

fn simple_truetype_with_zero_widths(content: &str, mappings: &[(u8, u16)], widths: &str) -> Vec<u8> {
    let font = test_font(CmapKind::ByteIndexed, 3);
    let mut bfchars = String::new();
    for &(code, unicode) in mappings {
        bfchars.push_str(&format!("<{code:02X}> <{unicode:04X}>\n"));
    }
    let tounicode = format!(
        "/CIDInit /ProcSet findresource begin\n\
         12 dict begin begincmap\n\
         /CMapName /ZeroWidthChars def /CMapType 2 def\n\
         1 begincodespacerange <00> <FF> endcodespacerange\n\
         {} beginbfchar\n{}endbfchar\n\
         endcmap CMapName currentdict /CMap defineresource pop end end",
        mappings.len(),
        bfchars
    );
    let descriptor = "<< /Type /FontDescriptor /FontName /CharBox /Flags 4 \
                      /FontBBox [0 0 450 700] /ItalicAngle 0 /Ascent 800 /Descent -200 \
                      /CapHeight 700 /StemV 80 /FontFile2 8 0 R >>";
    let font_dict = format!(
        "<< /Type /Font /Subtype /TrueType /BaseFont /CharBox /FirstChar 65 \
         /LastChar 67 /Widths [{widths}] /FontDescriptor 6 0 R /ToUnicode 7 0 R >>"
    );
    let tounicode_obj = format!("<< /Length {} >>\nstream\n{tounicode}\nendstream", tounicode.len() + 1);
    build_pdf(&[font_dict, descriptor.to_string(), tounicode_obj], content, &font, 8)
}

fn type3_zero_width_pdf() -> Vec<u8> {
    let font = test_font(CmapKind::ByteIndexed, 1);
    let content = "BT /F1 10 Tf 20 100 Td (AA) Tj ET";
    let char_proc = "0 0 d0\n0 0 450 700 re f";
    let font_dict = "<< /Type /Font /Subtype /Type3 /BaseFont /Overlay \
                     /FontBBox [0 0 450 700] /FontMatrix [0.001 0 0 0.001 0 0] \
                     /CharProcs << /A 6 0 R >> \
                     /Encoding << /Type /Encoding /Differences [65 /A] >> \
                     /FirstChar 65 /LastChar 65 /Widths [0] /Resources << >> >>";
    let char_proc_obj = format!("<< /Length {} >>\nstream\n{char_proc}\nendstream", char_proc.len() + 1);
    build_pdf(&[font_dict.to_string(), char_proc_obj], content, &font, 7)
}

/// The decoded string, the per-char sequence, and each char's advance.
fn chars_and_advances(pdf: Vec<u8>) -> (String, Vec<char>, Vec<f32>) {
    let doc = PdfDocument::from_bytes(pdf).expect("fixture parses");
    let text: String = doc
        .extract_text(0)
        .expect("extract_text")
        .chars()
        .filter(|c| !c.is_whitespace())
        .collect();
    let chars = doc.extract_chars(0).expect("extract_chars");
    let seq: Vec<char> = chars.iter().map(|c| c.char).collect();
    let advances: Vec<f32> = chars.windows(2).map(|w| w[1].origin_x - w[0].origin_x).collect();
    (text, seq, advances)
}

fn assert_advances(advances: &[f32], expected: &[f32]) {
    assert_eq!(advances.len(), expected.len(), "advance count: {advances:?}");
    for (got, want) in advances.iter().zip(expected) {
        assert!(
            (got - want).abs() < 0.05,
            "advance {got} != expected {want} (all: {advances:?})"
        );
    }
}

#[test]
fn byte_indexed_truetype_decodes_abc_with_declared_advances() {
    let (text, seq, advances) = chars_and_advances(simple_truetype_pdf(CmapKind::ByteIndexed, "ABC"));
    assert_eq!(text, "ABC");
    assert_eq!(seq, vec!['A', 'B', 'C']);
    // /Widths [500 600 700] at 10 pt: A advances 5, B advances 6. ~keep
    assert_advances(&advances, &[5.0, 6.0]);
}

#[test]
fn unicode_cmap_truetype_decodes_abc_with_declared_advances() {
    let (text, seq, advances) = chars_and_advances(simple_truetype_pdf(CmapKind::WindowsUnicode, "ABC"));
    assert_eq!(text, "ABC");
    assert_eq!(seq, vec!['A', 'B', 'C']);
    assert_advances(&advances, &[5.0, 6.0]);
}

#[test]
fn type0_identity_cid_decodes_abc_with_w_array_advances() {
    let (text, seq, advances) = chars_and_advances(type0_identity_pdf());
    assert_eq!(text, "ABC");
    assert_eq!(seq, vec!['A', 'B', 'C']);
    // /W [1 [500] 2 [600] 3 [700]] at 10 pt. ~keep
    assert_advances(&advances, &[5.0, 6.0]);
}

#[test]
fn subset_missing_glyph_keeps_the_character_in_extraction() {
    let (text, seq, _) = chars_and_advances(subset_missing_glyph_pdf());
    // The byte decodes to 'B' even though the font cannot paint it: the
    // extractor's job is the text layer, and dropping the char would turn a
    // rendering defect into silent text loss. ~keep
    assert_eq!(text, "AB");
    assert_eq!(seq, vec!['A', 'B']);
}

#[test]
fn malformed_zero_width_spacing_glyph_uses_embedded_advance_for_extraction() {
    let pdf = simple_truetype_with_zero_widths("BT /F1 10 Tf 20 100 Td (AAA) Tj ET", &[(65, 0x003F)], "0 600 700");
    let (text, seq, advances) = chars_and_advances(pdf);
    assert_eq!(text, "???");
    assert_eq!(seq, vec!['?', '?', '?']);
    assert_advances(&advances, &[5.0, 5.0]);
}

#[test]
fn zero_width_combining_mark_remains_an_overlay() {
    let pdf = simple_truetype_with_zero_widths(
        "BT /F1 10 Tf 20 100 Td (ABC) Tj ET",
        &[(65, 0x0041), (66, 0x0301), (67, 0x0042)],
        "500 0 600",
    );
    let (_, seq, advances) = chars_and_advances(pdf);
    assert_eq!(seq, vec!['A', '\u{301}', 'B']);
    assert_advances(&advances, &[5.0, 0.0]);
}

#[test]
fn type3_zero_width_glyph_remains_an_overlay() {
    let (text, seq, advances) = chars_and_advances(type3_zero_width_pdf());
    assert_eq!(text, "AA");
    assert_eq!(seq, vec!['A']);
    assert!(
        advances.is_empty(),
        "overlay glyphs must share one origin: {advances:?}"
    );
}

#[test]
fn explicit_tj_displacement_remains_authoritative_for_zero_width_glyph() {
    let pdf = simple_truetype_with_zero_widths(
        "BT /F1 10 Tf 20 100 Td [(A) 500 (A)] TJ ET",
        &[(65, 0x003F)],
        "0 600 700",
    );
    let (text, seq, advances) = chars_and_advances(pdf);
    assert_eq!(text, "??");
    assert_eq!(seq, vec!['?', '?']);
    assert_advances(&advances, &[5.0]);
}

#[test]
fn zero_tj_number_does_not_disable_malformed_width_repair() {
    let pdf =
        simple_truetype_with_zero_widths("BT /F1 10 Tf 20 100 Td [(A) 0 (A)] TJ ET", &[(65, 0x003F)], "0 600 700");
    let (text, seq, advances) = chars_and_advances(pdf);
    assert_eq!(text, "??");
    assert_eq!(seq, vec!['?', '?']);
    assert_advances(&advances, &[5.0]);
}

mod render_parity {
    use super::*;
    use xberg_native_pdf::extractors::warnings::WarningCategory;
    use xberg_native_pdf::rendering::{RenderOptions, render_page};

    fn glyph_drop_warnings_for(pdf: Vec<u8>) -> Vec<String> {
        let doc = PdfDocument::from_bytes(pdf).expect("fixture parses");
        render_page(&doc, 0, &RenderOptions::default()).expect("render");
        doc.structured_warnings()
            .into_iter()
            .filter(|w| w.category == WarningCategory::GlyphDropped)
            .map(|w| w.message)
            .collect()
    }

    /// Every glyph the extractor decodes, the renderer paints: a mapped
    /// font raises no drop warning on any family.
    #[test]
    fn fully_mapped_fonts_render_without_drop_warnings() {
        for (label, pdf) in [
            ("byte-indexed", simple_truetype_pdf(CmapKind::ByteIndexed, "ABC")),
            ("unicode-cmap", simple_truetype_pdf(CmapKind::WindowsUnicode, "ABC")),
            ("type0-identity", type0_identity_pdf()),
        ] {
            let warnings = glyph_drop_warnings_for(pdf);
            assert!(warnings.is_empty(), "{label}: unexpected drop warnings: {warnings:?}");
        }
    }

    /// The inverse contract: a code the font cannot paint stays in the text
    /// layer (test above) and surfaces as exactly one drop warning here.
    #[test]
    fn subset_missing_glyph_raises_one_drop_warning() {
        let warnings = glyph_drop_warnings_for(subset_missing_glyph_pdf());
        assert_eq!(warnings.len(), 1, "expected one drop warning: {warnings:?}");
        assert!(
            warnings[0].contains("0x42"),
            "the warning must name the dropped code 0x42: {}",
            warnings[0]
        );
    }
}
