use super::*;
use crate::vendor::fontdb;
use std::sync::Arc;

fn latin_pdf(embedded: bool, explicit_winansi: bool, glyph: u8) -> PdfDocument {
    let mut content = b"BT /F0 60 Tf 18 22 Td (".to_vec();
    content.push(glyph);
    content.extend(b") Tj ET");
    let mut objects = vec![
        b"<< /Type /Catalog /Pages 2 0 R >>".to_vec(),
        b"<< /Type /Pages /Kids [3 0 R] /Count 1 >>".to_vec(),
        b"<< /Type /Page /Parent 2 0 R /MediaBox [0 0 100 100] /Contents 4 0 R           /Resources << /Font << /F0 5 0 R >> >> >>".to_vec(),
        pdf_stream("", &content),
    ];
    if embedded {
        objects.push(
            format!(
                "<< /Type /Font /Subtype /TrueType /BaseFont /DejaVuSans /Encoding /WinAnsiEncoding /FirstChar {glyph} /LastChar {glyph} /Widths [787] /FontDescriptor 6 0 R >>"
            ).into_bytes(),
        );
        objects.push(
            b"<< /Type /FontDescriptor /FontName /DejaVuSans /Flags 32 /FontBBox [-1021 -463 1793 1232]               /Ascent 928 /Descent -236 /CapHeight 729 /ItalicAngle 0 /StemV 80 /FontFile2 7 0 R >>".to_vec(),
        );
        let font = crate::fonts::bundled::DEJAVU_SANS;
        objects.push(pdf_stream(&format!("/Length1 {} ", font.len()), font));
    } else {
        let encoding = if explicit_winansi {
            " /Encoding /WinAnsiEncoding"
        } else {
            ""
        };
        objects.push(format!("<< /Type /Font /Subtype /Type1 /BaseFont /Helvetica{encoding} >>").into_bytes());
    }

    let mut pdf = b"%PDF-1.4\n".to_vec();
    let mut offsets = Vec::new();
    for object in objects {
        offsets.push(pdf.len());
        pdf.extend(format!("{} 0 obj\n", offsets.len()).as_bytes());
        pdf.extend(object);
        pdf.extend(b"\nendobj\n");
    }
    let xref = pdf.len();
    let count = offsets.len() + 1;
    pdf.extend(format!("xref\n0 {count}\n0000000000 65535 f \n").as_bytes());
    for offset in offsets {
        pdf.extend(format!("{offset:010} 00000 n \n").as_bytes());
    }
    pdf.extend(format!("trailer\n<< /Size {count} /Root 1 0 R >>\nstartxref\n{xref}\n%%EOF\n").as_bytes());
    PdfDocument::from_bytes(pdf).expect("parse synthetic Latin PDF")
}

fn pdf_stream(entries: &str, bytes: &[u8]) -> Vec<u8> {
    let mut stream = format!("<< {entries}/Length {} >>\nstream\n", bytes.len()).into_bytes();
    stream.extend(bytes);
    stream.extend(b"\nendstream");
    stream
}

fn has_ink(pixel: &[u8; 4]) -> bool {
    pixel[0] < 128
}

fn middle_row_runs(image: &RenderedImage) -> Vec<bool> {
    assert_eq!((image.width, image.height), (100, 100));
    let (rows, remainder) = image.data.as_chunks::<400>();
    assert!(remainder.is_empty());
    let row_has_ink = |row: &[u8; 400]| row.as_chunks::<4>().0.iter().any(has_ink);
    let first = rows.iter().position(row_has_ink).expect("glyph has ink");
    let last = rows.iter().rposition(row_has_ink).unwrap();
    let mut runs: Vec<_> = rows[(first + last) / 2]
        .as_chunks::<4>()
        .0
        .iter()
        .map(has_ink)
        .collect();
    runs.dedup();
    runs
}

fn glyph_runs_with_font_provider(embedded: bool, explicit_winansi: bool, db: fontdb::Database) -> Vec<bool> {
    middle_row_runs(&raster_with_font_provider(embedded, explicit_winansi, db, b'O'))
}

fn raster_with_font_provider(embedded: bool, explicit_winansi: bool, db: fontdb::Database, glyph: u8) -> RenderedImage {
    let doc = latin_pdf(embedded, explicit_winansi, glyph);
    let expected = char::from(glyph).to_string();
    let text = || {
        let chars: String = doc.extract_chars(0).unwrap().iter().map(|glyph| glyph.char).collect();
        let spans: String = doc
            .extract_spans(0)
            .unwrap()
            .iter()
            .map(|span| span.text.as_str())
            .collect();
        (chars, spans)
    };
    assert_eq!(text(), (expected.clone(), expected.clone()));
    let mut renderer = PageRenderer::new(RenderOptions::with_dpi(72).as_raw());
    renderer.text_rasterizer = TextRasterizer::with_fontdb(Arc::new(db));
    let raster = renderer
        .render_page(&doc, 0)
        .expect("render Latin fixture through native page renderer");
    assert_eq!(text(), (expected.clone(), expected));
    raster
}

const LATIN_O: [bool; 5] = [false, true, false, true, false];

#[test]
fn embedded_latin_font_renders_a_real_counter_without_system_faces() {
    assert_eq!(
        glyph_runs_with_font_provider(true, true, fontdb::Database::new()),
        LATIN_O
    );
}

#[test]
fn standard_latin_font_renders_a_real_counter_without_system_faces() {
    assert_eq!(
        glyph_runs_with_font_provider(false, true, fontdb::Database::new()),
        LATIN_O
    );
}

#[test]
fn default_standard_encoding_renders_latin_without_system_faces() {
    assert_eq!(
        glyph_runs_with_font_provider(false, false, fontdb::Database::new()),
        LATIN_O
    );
}

#[test]
fn fontless_winansi_raster_preserves_the_accented_e_outline() {
    let glyph = 0xE9;
    let embedded = raster_with_font_provider(true, true, fontdb::Database::new(), glyph);
    let fallback = raster_with_font_provider(false, true, fontdb::Database::new(), glyph);
    let mut occupied_rows: Vec<_> = embedded
        .data
        .as_chunks::<400>()
        .0
        .iter()
        .map(|row| row.as_chunks::<4>().0.iter().any(has_ink))
        .collect();
    occupied_rows.dedup();
    assert_eq!(
        occupied_rows,
        [false, true, false, true, false],
        "separate accent and glyph body"
    );
    assert_eq!(fallback.data.cmp(&embedded.data), std::cmp::Ordering::Equal);
}

#[cfg(feature = "cjk-render-fallback")]
#[test]
fn standard_latin_font_renders_a_real_counter_with_only_the_cjk_face_available() {
    let mut db = fontdb::Database::new();
    db.load_font_data(crate::fonts::form_fallback::render_cjk_fallback_bytes().to_vec());
    let family = fontdb::Family::Name("Droid Sans Fallback");
    assert!(
        db.query(&fontdb::Query {
            families: &[family],
            weight: fontdb::Weight::NORMAL,
            stretch: fontdb::Stretch::Normal,
            style: fontdb::Style::Normal,
        })
        .is_some()
    );
    assert_eq!(glyph_runs_with_font_provider(false, true, db), LATIN_O);
}
