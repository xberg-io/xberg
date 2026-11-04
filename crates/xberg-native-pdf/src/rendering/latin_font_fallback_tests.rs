use super::*;
use crate::vendor::fontdb;
use std::sync::Arc;

fn latin_pdf(embedded: bool, explicit_winansi: bool, glyph: u8) -> PdfDocument {
    let mut content = b"BT /F0 60 Tf 18 22 Td (".to_vec();
    content.push(glyph);
    content.extend(b") Tj ET");
    let font = if embedded {
        embedded_dejavu("DejaVuSans", crate::fonts::bundled::DEJAVU_SANS, glyph)
    } else {
        let encoding = if explicit_winansi {
            " /Encoding /WinAnsiEncoding"
        } else {
            ""
        };
        vec![format!("<< /Type /Font /Subtype /Type1 /BaseFont /Helvetica{encoding} >>").into_bytes()]
    };
    page_pdf((100, 100), &content, font)
}

/// Objects 5 to 7 of [`page_pdf`]: a WinAnsi TrueType font that embeds `font`.
fn embedded_dejavu(base_font: &str, font: &[u8], glyph: u8) -> Vec<Vec<u8>> {
    vec![
        format!(
            "<< /Type /Font /Subtype /TrueType /BaseFont /{base_font} /Encoding /WinAnsiEncoding /FirstChar {glyph} /LastChar {glyph} /Widths [787] /FontDescriptor 6 0 R >>"
        )
        .into_bytes(),
        format!(
            "<< /Type /FontDescriptor /FontName /{base_font} /Flags 32 /FontBBox [-1021 -463 1793 1232] /Ascent 928 /Descent -236 /CapHeight 729 /ItalicAngle 0 /StemV 80 /FontFile2 7 0 R >>"
        )
        .into_bytes(),
        pdf_stream(&format!("/Length1 {} ", font.len()), font),
    ]
}

/// A one-page PDF whose content uses font `/F0`, the first of `font_objects`.
/// The font objects are numbered from 5.
fn page_pdf((width, height): (u32, u32), content: &[u8], font_objects: Vec<Vec<u8>>) -> PdfDocument {
    let mut objects = vec![
        b"<< /Type /Catalog /Pages 2 0 R >>".to_vec(),
        b"<< /Type /Pages /Kids [3 0 R] /Count 1 >>".to_vec(),
        format!(
            "<< /Type /Page /Parent 2 0 R /MediaBox [0 0 {width} {height}] /Contents 4 0 R /Resources << /Font << /F0 5 0 R >> >> >>"
        )
        .into_bytes(),
        pdf_stream("", content),
    ];
    objects.extend(font_objects);

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

fn render_without_system_fonts(doc: &PdfDocument) -> RenderedImage {
    let mut renderer = PageRenderer::new(RenderOptions::with_dpi(72).as_raw());
    renderer.text_rasterizer = TextRasterizer::with_fontdb(Arc::new(fontdb::Database::new()));
    renderer.render_page(doc, 0).expect("render the synthetic page")
}

/// A 400 x 60 page that shows `run` once, at 24 pt, in the font `font`.
fn run_pdf(font: &str, run: &[u8]) -> PdfDocument {
    let mut content = b"BT /F0 24 Tf 10 20 Td (".to_vec();
    content.extend(run);
    content.extend(b") Tj ET");
    page_pdf((400, 60), &content, vec![font.as_bytes().to_vec()])
}

/// The pixel columns in which two renders of the same page size differ.
fn differing_columns(a: &RenderedImage, b: &RenderedImage) -> Vec<usize> {
    assert_eq!((a.width, a.height), (b.width, b.height));
    let row_bytes = a.width as usize * 4;
    (0..a.width as usize)
        .filter(|&column| {
            a.data
                .chunks(row_bytes)
                .zip(b.data.chunks(row_bytes))
                .any(|(row_a, row_b)| row_a[column * 4..column * 4 + 4] != row_b[column * 4..column * 4 + 4])
        })
        .collect()
}

const HELVETICA_DIFFERENCES: &str = "<< /Type /Font /Subtype /Type1 /BaseFont /Helvetica /Encoding << /Type /Encoding /BaseEncoding /WinAnsiEncoding /Differences [128 /ohungarumlaut 129 /Lslash 130 /minus 131 /uni4E2D] >> >>";

/// #2347: one character outside WinAnsi does not blank its run, whether the
/// bundled face has its glyph or not (the CJK ideograph). The character ends
/// the run, so against the same run with a WinAnsi character in its place,
/// every column before that last cell draws the same.
#[test]
fn a_character_outside_winansi_draws_with_the_rest_of_its_run() {
    let standard = "<< /Type /Font /Subtype /Type1 /BaseFont /Helvetica >>";
    let cases: [(&str, &[u8], &[u8], char); 5] = [
        (standard, b"Hello world \\350", b"Hello world L", '\u{141}'),
        (HELVETICA_DIFFERENCES, b"Hello world \\201", b"Hello world L", '\u{141}'),
        (HELVETICA_DIFFERENCES, b"Hello world \\200", b"Hello world o", '\u{151}'),
        (
            HELVETICA_DIFFERENCES,
            b"Hello world \\202",
            b"Hello world +",
            '\u{2212}',
        ),
        (
            HELVETICA_DIFFERENCES,
            b"Hello world \\203",
            b"Hello world L",
            '\u{4E2D}',
        ),
    ];
    for (font, run, plain, outside) in cases {
        let doc = run_pdf(font, run);
        let text: String = doc
            .extract_spans(0)
            .unwrap()
            .iter()
            .map(|span| span.text.as_str())
            .collect();
        assert!(text.contains(outside), "fixture decodes to {text:?}");
        let changed = differing_columns(
            &render_without_system_fonts(&doc),
            &render_without_system_fonts(&run_pdf(font, plain)),
        );
        let (Some(first), Some(last)) = (changed.first(), changed.last()) else {
            panic!("{outside:?} draws the same as the WinAnsi character in its place");
        };
        assert!(
            *first > 100 && last - first < 24,
            "{outside:?} changes columns {first} to {last}, more than its own cell at the end of the run"
        );
    }
}

/// A bold font, by name or by its descriptor, draws with the bundled
/// DejaVu Sans Bold: the same pixels as that face embedded in the PDF.
#[test]
fn a_bold_standard_font_draws_with_the_bundled_bold_face() {
    let glyph = b'O';
    let one_glyph = |font: Vec<Vec<u8>>| {
        let content = format!("BT /F0 60 Tf 18 22 Td ({}) Tj ET", char::from(glyph));
        render_without_system_fonts(&page_pdf((100, 100), content.as_bytes(), font))
    };
    let bold = one_glyph(embedded_dejavu(
        "DejaVuSans-Bold",
        crate::fonts::bundled::DEJAVU_SANS_BOLD,
        glyph,
    ));
    let regular = one_glyph(embedded_dejavu("DejaVuSans", crate::fonts::bundled::DEJAVU_SANS, glyph));
    assert!(bold.data != regular.data, "the two embedded faces must differ");

    let by_name = one_glyph(vec![
        b"<< /Type /Font /Subtype /Type1 /BaseFont /Helvetica-Bold /Encoding /WinAnsiEncoding >>".to_vec(),
    ]);
    assert!(by_name.data == bold.data, "Helvetica-Bold draws with DejaVu Sans Bold");

    let by_descriptor = one_glyph(vec![
        b"<< /Type /Font /Subtype /Type1 /BaseFont /Helvetica /Encoding /WinAnsiEncoding /FontDescriptor 6 0 R >>"
            .to_vec(),
        b"<< /Type /FontDescriptor /FontName /Helvetica /Flags 32 /FontWeight 700 /FontBBox [-166 -225 1000 931] /Ascent 718 /Descent -207 /CapHeight 718 /ItalicAngle 0 /StemV 88 >>".to_vec(),
    ]);
    assert!(
        by_descriptor.data == bold.data,
        "FontWeight 700 draws with DejaVu Sans Bold"
    );

    let plain = one_glyph(vec![
        b"<< /Type /Font /Subtype /Type1 /BaseFont /Helvetica /Encoding /WinAnsiEncoding >>".to_vec(),
    ]);
    assert!(plain.data == regular.data, "Helvetica draws with DejaVu Sans");
}

/// The bundled face is parsed once per process, as a system face is: more
/// runs drawn with it build no new shaping font.
#[test]
fn the_bundled_face_is_parsed_once() {
    use crate::rendering::text_rasterizer::SHAPING_FONT_BUILDS;

    let doc = run_pdf(
        "<< /Type /Font /Subtype /Type1 /BaseFont /Helvetica /Encoding /WinAnsiEncoding >>",
        b"Hello world",
    );
    render_without_system_fonts(&doc);
    let before = SHAPING_FONT_BUILDS.with(std::cell::Cell::get);
    for _ in 0..3 {
        render_without_system_fonts(&doc);
    }
    assert_eq!(SHAPING_FONT_BUILDS.with(std::cell::Cell::get), before);
}
