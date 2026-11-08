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
fn page_pdf(size: (u32, u32), content: &[u8], font_objects: Vec<Vec<u8>>) -> PdfDocument {
    page_pdf_with_resources(size, content, font_objects, "")
}

/// [`page_pdf`] with `extra_resources` added to the page's resource dictionary.
fn page_pdf_with_resources(
    (width, height): (u32, u32),
    content: &[u8],
    font_objects: Vec<Vec<u8>>,
    extra_resources: &str,
) -> PdfDocument {
    let mut objects = vec![
        b"<< /Type /Catalog /Pages 2 0 R >>".to_vec(),
        b"<< /Type /Pages /Kids [3 0 R] /Count 1 >>".to_vec(),
        format!(
            "<< /Type /Page /Parent 2 0 R /MediaBox [0 0 {width} {height}] /Contents 4 0 R \
             /Resources << /Font << /F0 5 0 R >> {extra_resources} >> >>"
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
    let by_flags = one_glyph(vec![
        b"<< /Type /Font /Subtype /Type1 /BaseFont /Helvetica /Encoding /WinAnsiEncoding /FontDescriptor 6 0 R >>".to_vec(),
        b"<< /Type /FontDescriptor /FontName /Helvetica /Flags 262176 /FontBBox [-166 -225 1000 931] /Ascent 718 /Descent -207 /CapHeight 718 /ItalicAngle 0 /StemV 88 >>".to_vec(),
    ]);
    assert!(by_flags.data == bold.data, "ForceBold draws with DejaVu Sans Bold");

    let plain = one_glyph(vec![
        b"<< /Type /Font /Subtype /Type1 /BaseFont /Helvetica /Encoding /WinAnsiEncoding >>".to_vec(),
    ]);
    assert!(plain.data == regular.data, "Helvetica draws with DejaVu Sans");
}

fn render_with_database(doc: &PdfDocument, db: Arc<fontdb::Database>) -> RenderedImage {
    let mut renderer = PageRenderer::new(RenderOptions::with_dpi(72).as_raw());
    renderer.text_rasterizer = TextRasterizer::with_fontdb(db);
    renderer
        .render_page(doc, 0)
        .expect("render through the supplied font provider")
}

fn provided_font_case() {
    let mut db = fontdb::Database::new();
    db.load_font_data(crate::fonts::bundled::DEJAVU_SANS_BOLD.to_vec());
    let face = db
        .query(&fontdb::Query {
            families: &[fontdb::Family::Name("DejaVu Sans")],
            weight: fontdb::Weight::NORMAL,
            stretch: fontdb::Stretch::Normal,
            style: fontdb::Style::Normal,
        })
        .expect("the provider supplies a face for this family");
    assert_eq!(db.face(face).unwrap().weight, fontdb::Weight::BOLD);

    let content = b"BT /F0 60 Tf 18 22 Td (O) Tj ET";
    let provided = page_pdf(
        (100, 100),
        content,
        vec![b"<< /Type /Font /Subtype /Type1 /BaseFont /DejaVu#20Sans /Encoding /WinAnsiEncoding >>".to_vec()],
    );
    let actual = render_with_database(&provided, Arc::new(db));
    let bold = render_without_system_fonts(&page_pdf(
        (100, 100),
        content,
        embedded_dejavu("DejaVuSans-Bold", crate::fonts::bundled::DEJAVU_SANS_BOLD, b'O'),
    ));
    let regular = render_without_system_fonts(&page_pdf(
        (100, 100),
        content,
        embedded_dejavu("DejaVuSans", crate::fonts::bundled::DEJAVU_SANS, b'O'),
    ));
    assert_ne!(bold.data, regular.data);
    assert_eq!(
        actual.data, bold.data,
        "the provider wins despite regular PDF weight metadata"
    );
}

fn ink_count(image: &RenderedImage) -> usize {
    image
        .data
        .as_chunks::<4>()
        .0
        .iter()
        .filter(|pixel| pixel[0] < 128)
        .count()
}

fn cjk_database(present: bool) -> Arc<fontdb::Database> {
    let db = fontdb::Database::new();
    #[cfg(feature = "cjk-render-fallback")]
    let db = {
        let mut db = db;
        if present {
            db.load_font_data(crate::fonts::form_fallback::render_cjk_fallback_bytes().to_vec());
        }
        db
    };
    #[cfg(not(feature = "cjk-render-fallback"))]
    assert!(!present, "the CJK provider needs its explicit font feature");
    assert_eq!(
        db.query(&fontdb::Query {
            families: &[fontdb::Family::Name("Droid Sans Fallback")],
            weight: fontdb::Weight::NORMAL,
            stretch: fontdb::Stretch::Normal,
            style: fontdb::Style::Normal,
        })
        .is_some(),
        present
    );
    Arc::new(db)
}

fn mixed_cjk_case(present: bool) {
    let db = cjk_database(present);
    let widths = "600 ".repeat(256);
    let font = format!(
        "<< /Type /Font /Subtype /Type1 /BaseFont /Helvetica /FirstChar 0 /LastChar 255 /Widths [{widths}] /Encoding << /Type /Encoding /BaseEncoding /WinAnsiEncoding /Differences [131 /uni4E2D] >> >>"
    );
    let full = run_pdf(&font, b"Hello world \\203");
    let text: String = full
        .extract_spans(0)
        .unwrap()
        .iter()
        .map(|span| span.text.as_str())
        .collect();
    assert_eq!(text, "Hello world 中");
    let actual = render_with_database(&full, db.clone());
    let prefix = render_with_database(&run_pdf(&font, b"Hello world "), db.clone());
    let changed = differing_columns(&actual, &prefix);
    assert!(!changed.is_empty(), "a missing glyph must remain visible as a box");
    assert!(
        *changed.first().unwrap() > 170,
        "only the final glyph changes: {changed:?}"
    );
    assert!(
        ink_count(&actual) > ink_count(&prefix),
        "the last glyph must not vanish"
    );
    assert_following_latin_position(&font, db);
    if present {
        #[cfg(feature = "cjk-render-fallback")]
        assert_real_cjk_glyph(&actual);
    }
}

fn assert_following_latin_position(font: &str, db: Arc<fontdb::Database>) {
    let leading_cjk = render_with_database(&run_pdf(font, b"\\203 O"), db.clone());
    let leading_latin = render_with_database(&run_pdf(font, b"X O"), db.clone());
    assert!(
        leading_cjk
            .data
            .as_chunks::<{ 400 * 4 }>()
            .0
            .iter()
            .zip(leading_latin.data.as_chunks::<{ 400 * 4 }>().0.iter())
            .all(|(actual, expected)| actual[38 * 4..] == expected[38 * 4..]),
        "the CJK glyph must preserve the PDF-declared advance before following Latin text"
    );
}

#[cfg(feature = "cjk-render-fallback")]
fn assert_real_cjk_glyph(actual: &RenderedImage) {
    let font = crate::fonts::form_fallback::render_cjk_fallback_bytes();
    let mut objects = embedded_dejavu("DroidSansFallback", font, 131);
    objects[0] = String::from_utf8(objects[0].clone())
        .unwrap()
        .replace(
            "/Encoding /WinAnsiEncoding",
            "/Encoding << /Type /Encoding /BaseEncoding /WinAnsiEncoding /Differences [131 /uni4E2D] >>",
        )
        .into_bytes();
    let x = 10.0 + (0..12).fold(0.0_f32, |sum, _| sum + 600.0 * 24.0 / 1000.0);
    let content = format!("BT /F0 24 Tf {x} 20 Td (\\203) Tj ET");
    let expected = render_without_system_fonts(&page_pdf((400, 60), content.as_bytes(), objects));
    let mask = |image: &RenderedImage| {
        image
            .data
            .as_chunks::<{ 400 * 4 }>()
            .0
            .iter()
            .flat_map(|row| {
                row[(x.floor() as usize) * 4..]
                    .as_chunks::<4>()
                    .0
                    .iter()
                    .map(|pixel| pixel[0] < 128)
            })
            .collect::<Vec<_>>()
    };
    assert!(
        mask(actual) == mask(&expected),
        "the supplied CJK glyph has the embedded face's outline and orientation"
    );
}

#[test]
fn mixed_latin_cjk_fallback_isolated() {
    const CHILD: &str = "XBERG_LATIN_CJK_FALLBACK_CHILD_2510";
    if let Ok(mode) = std::env::var(CHILD) {
        assert!(matches!(mode.as_str(), "absent" | "present"));
        mixed_cjk_case(mode == "present");
        return;
    }
    let modes = if cfg!(feature = "cjk-render-fallback") {
        vec!["absent", "present"]
    } else {
        vec!["absent"]
    };
    for mode in modes {
        let output = std::process::Command::new(std::env::current_exe().unwrap())
            .args([
                "--exact",
                "rendering::page_renderer::latin_font_fallback_tests::mixed_latin_cjk_fallback_isolated",
                "--nocapture",
            ])
            .env(CHILD, mode)
            .output()
            .expect("run isolated font-provider case");
        let stdout = String::from_utf8_lossy(&output.stdout);
        assert!(
            stdout.contains("running 1 test"),
            "child must collect exactly one case: {stdout}"
        );
        assert!(
            output.status.success(),
            "{mode}: {stdout} {}",
            String::from_utf8_lossy(&output.stderr)
        );
        assert!(stdout.contains("1 passed"), "{stdout}");
    }
}

#[test]
fn provided_font_is_used_before_the_bundled_regular_face() {
    const CHILD: &str = "XBERG_PROVIDED_LATIN_FONT_CHILD_2510";
    if std::env::var(CHILD).as_deref() == Ok("child") {
        provided_font_case();
        return;
    }
    let output = std::process::Command::new(std::env::current_exe().unwrap()).args([
        "--exact", "rendering::page_renderer::latin_font_fallback_tests::provided_font_is_used_before_the_bundled_regular_face", "--nocapture",
    ]).env(CHILD, "child").output().expect("run isolated provider case");
    let stdout = String::from_utf8_lossy(&output.stdout);
    assert!(stdout.contains("running 1 test"), "{stdout}");
    assert!(
        output.status.success(),
        "{stdout} {}",
        String::from_utf8_lossy(&output.stderr)
    );
    assert!(stdout.contains("1 passed"), "{stdout}");
}

/// A form XObject (object 8) that shows `text` in a font named only by the form's own
/// resource dictionary, which is an indirect object (object 9).
fn form_with_indirect_font_resources(text: &str) -> RenderedImage {
    let mut objects = embedded_dejavu("DejaVuSans", crate::fonts::bundled::DEJAVU_SANS, b'0');
    objects[0] = b"<< /Type /Font /Subtype /TrueType /BaseFont /DejaVuSans /Encoding /WinAnsiEncoding \
        /FirstChar 48 /LastChar 56 /Widths [750 750 750 750 750 750 750 750 750] /FontDescriptor 6 0 R >>"
        .to_vec();
    objects.push(pdf_stream(
        "/Type /XObject /Subtype /Form /BBox [0 0 60 20] /Resources 9 0 R ",
        format!("BT /F1 10 Tf 0 4 Td ({text}) Tj ET").as_bytes(),
    ));
    objects.push(b"<< /Font << /F1 5 0 R >> >>".to_vec());
    let doc = page_pdf_with_resources(
        (100, 40),
        b"q 1 0 0 1 20 10 cm /Fm0 Do Q",
        objects,
        "/XObject << /Fm0 8 0 R >>",
    );
    render_without_system_fonts(&doc)
}

fn separate_ink_columns(image: &RenderedImage) -> usize {
    let width = image.width as usize;
    let has_ink_in_column = |x: usize| {
        (0..image.height as usize).any(|y| has_ink(image.data[(y * width + x) * 4..][..4].try_into().unwrap()))
    };
    let columns: Vec<bool> = (0..width).map(has_ink_in_column).collect();
    columns.windows(2).filter(|pair| !pair[0] && pair[1]).count()
}

/// GH#2685: a page number drawn by a form whose font resources are an indirect object lost every glyph
/// after the first, because the renderer never loaded that font and spaced the glyphs by raw font units.
#[test]
fn a_form_with_indirect_font_resources_draws_every_glyph_of_a_page_number() {
    let one = separate_ink_columns(&form_with_indirect_font_resources("3"));
    let three = separate_ink_columns(&form_with_indirect_font_resources("308"));
    assert_eq!(one, 1, "control: a one-digit page number draws one glyph");
    assert_eq!(three, 3, "all three digits of the page number are drawn");
}

/// A separation space whose full tint is red.
const RED_SPOT: &str =
    "[/Separation /Spot /DeviceRGB << /FunctionType 2 /Domain [0 1] /C0 [1 1 1] /C1 [1 0 0] /N 1 >>]";

fn filled_centre_pixel(image: &RenderedImage) -> [u8; 4] {
    let at = ((image.height as usize / 2) * image.width as usize + image.width as usize / 2) * 4;
    image.data[at..][..4].try_into().unwrap()
}

/// A page that fills a rectangle with the full tint of the colour space `/Spot`. The space is named by the
/// resources of the page when `through_form` is false, and by those of a form (object 7, indirect) otherwise.
fn spot_fill(through_form: bool) -> RenderedImage {
    let font = b"<< /Type /Font /Subtype /Type1 /BaseFont /Helvetica >>".to_vec();
    let doc = if through_form {
        let form = pdf_stream(
            "/Type /XObject /Subtype /Form /BBox [0 0 60 20] /Resources 7 0 R ",
            b"/Spot cs 1 scn 0 0 60 20 re f",
        );
        let resources = format!("<< /ColorSpace << /Spot {RED_SPOT} >> >>").into_bytes();
        page_pdf_with_resources(
            (100, 40),
            b"q 1 0 0 1 20 10 cm /Fm0 Do Q",
            vec![font, form, resources],
            "/XObject << /Fm0 6 0 R >>",
        )
    } else {
        page_pdf_with_resources(
            (100, 40),
            b"/Spot cs 1 scn 20 10 60 20 re f",
            vec![font],
            &format!("/ColorSpace << /Spot {RED_SPOT} >>"),
        )
    };
    render_without_system_fonts(&doc)
}

/// A form whose colour spaces are an indirect object paints with them, as a page whose resources name the
/// same space does.
#[test]
fn a_form_with_indirect_colour_space_resources_paints_with_them() {
    let control = filled_centre_pixel(&spot_fill(false));
    let through_form = filled_centre_pixel(&spot_fill(true));
    let is_red = |pixel: [u8; 4]| pixel[0] > 200 && pixel[1] < 80 && pixel[2] < 80;
    assert!(
        is_red(control),
        "control: the page's own colour space paints red, got {control:?}"
    );
    assert!(
        is_red(through_form),
        "the form's indirect colour space paints red, got {through_form:?}"
    );
}

#[derive(Default)]
struct CapturedWarnings(std::sync::Arc<std::sync::Mutex<Vec<std::collections::BTreeMap<String, String>>>>);

impl<S: tracing::Subscriber> tracing_subscriber::Layer<S> for CapturedWarnings {
    fn on_event(&self, event: &tracing::Event<'_>, _: tracing_subscriber::layer::Context<'_, S>) {
        struct Fields(std::collections::BTreeMap<String, String>);
        impl tracing::field::Visit for Fields {
            fn record_debug(&mut self, field: &tracing::field::Field, value: &dyn std::fmt::Debug) {
                self.0.insert(field.name().to_string(), format!("{value:?}"));
            }
            fn record_str(&mut self, field: &tracing::field::Field, value: &str) {
                self.0.insert(field.name().to_string(), value.to_string());
            }
        }
        if *event.metadata().level() == tracing::Level::WARN {
            let mut fields = Fields(Default::default());
            event.record(&mut fields);
            self.0.lock().unwrap().push(fields.0);
        }
    }
}

/// A form whose `/Resources` reference cannot load still renders, and the renderer logs a warning with the
/// object number. A reference to a missing object is a null (ISO 32000-1 7.3.10) and loads without an error,
/// so the reference here names generation 1 of object 5, which the file holds only as generation 0.
#[test]
fn a_form_whose_resources_reference_cannot_load_logs_its_object_number() {
    use tracing_subscriber::layer::SubscriberExt as _;
    let form = pdf_stream(
        "/Type /XObject /Subtype /Form /BBox [0 0 60 20] /Resources 5 1 R ",
        b"0 0 60 20 re f",
    );
    let doc = page_pdf_with_resources(
        (100, 40),
        b"q 1 0 0 1 20 10 cm /Fm0 Do Q",
        vec![b"<< /Type /Font /Subtype /Type1 /BaseFont /Helvetica >>".to_vec(), form],
        "/XObject << /Fm0 6 0 R >>",
    );
    let captured = CapturedWarnings::default();
    let warnings = captured.0.clone();
    let subscriber = tracing_subscriber::registry().with(captured);
    let image = tracing::subscriber::with_default(subscriber, || render_without_system_fonts(&doc));
    assert_eq!((image.width, image.height), (100, 40), "the page still renders");
    let warnings = warnings.lock().unwrap();
    let ours: Vec<_> = warnings
        .iter()
        .filter(|fields| fields.get("operation").map(String::as_str) == Some("load_render_resources"))
        .collect();
    assert_eq!(ours.len(), 1, "one warning for the one unreadable object: {warnings:?}");
    assert_eq!(ours[0].get("object_number").map(String::as_str), Some("5"));
    assert_eq!(ours[0].get("error_code").map(String::as_str), Some("parse_error"));
}
