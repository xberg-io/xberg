//! GH#1632: a glyph was dropped when its *inferred* Unicode was whitespace.
//!
//! `render_cid_direct` decided whether to paint by asking what character the
//! code represented, not whether there was an outline to draw. For a simple
//! TrueType font with a byte-indexed cmap and no `/Encoding`, that character
//! is the GID reverse-mapped through the font's own (1, 0) subtable and read
//! as ASCII or Mac Roman — the subset's private glyph ordering, decoded as an
//! encoding it never expressed. When that decode landed on whitespace the
//! outline was silently discarded and only the advance applied.
//!
//! Six byte values were affected: 0x09-0x0D (TAB LF VT FF CR), 0x20 (SPACE)
//! and 0xCA (U+00A0 via MAC_ROMAN_HIGH). Re-indexed subsets numbering their
//! glyphs from 0x01 upward are ordinary producer output, so such a font walked
//! straight through the whole run.

mod common;

use xberg_native_pdf::PdfDocument;
use xberg_native_pdf::rendering::{PageRenderer, RenderOptions};

/// The shared box-glyph fixture font (see `common::build_box_glyph_font`),
/// here mapping `code` to glyph 1.
fn subset_font_mapping(code: u8) -> Vec<u8> {
    common::build_box_glyph_font(common::BoxGlyphFont {
        outline: [50, 0, 450, 700],
        head_bbox: [0, 0, 450, 700],
        lsb: 0,
        cmap_byte: code,
    })
}

/// A single byte as PDF string content, escaping what the syntax requires.
fn escape(code: u8) -> Vec<u8> {
    match code {
        b'(' | b')' | b'\\' => vec![b'\\', code],
        _ => vec![code],
    }
}

fn pdf_drawing(code: u8) -> Vec<u8> {
    let page_count = 1usize;
    let font = subset_font_mapping(code);
    let content = [
        b"BT /F1 24 Tf 50 100 Td (".as_slice(),
        &escape(code),
        b") Tj ET".as_slice(),
    ]
    .concat();
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
        buf.extend_from_slice(&content);
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

/// Count pixels that received ink.
fn painted_pixels(code: u8) -> usize {
    let doc = PdfDocument::from_bytes(pdf_drawing(code)).expect("parse fixture");
    let mut renderer = PageRenderer::new(RenderOptions::default());
    let image = renderer.render_page(&doc, 0).expect("render page");
    let decoded = image::load_from_memory(&image.data).expect("decode render").to_luma8();
    decoded.pixels().filter(|pixel| pixel.0[0] < 250).count()
}

#[test]
fn a_glyph_whose_inferred_unicode_is_whitespace_is_still_painted() {
    // 0x41 ('A') is the control: its inferred Unicode is not whitespace, so it
    // painted correctly even before the fix. ~keep
    let control = painted_pixels(0x41);
    assert!(control > 0, "control glyph at 0x41 painted nothing ({control} px)");

    // Every byte whose ASCII / Mac Roman decode is whitespace. Each maps to the
    // same box outline as the control, so each must paint the same ink. ~keep
    for code in [0x09u8, 0x0A, 0x0B, 0x0C, 0x0D, 0x20, 0xCA] {
        let painted = painted_pixels(code);
        assert_eq!(
            painted, control,
            "byte {code:#04x} painted {painted} px but the identical outline at 0x41 painted {control} px \
             — the glyph was suppressed because its inferred Unicode is whitespace"
        );
    }
}
