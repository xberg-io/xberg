//! A page of vector artwork is not a scan, and nothing is said about it.
//!
//! Page classification drives two visible behaviours: `pages_needing_ocr`, and
//! (historically) a sentence injected into the markdown telling the reader the
//! page is "a scanned/rasterised image … run OCR to recover its content". On a
//! page holding only vector drawings both are false — there is no raster, and
//! OCR reads nothing from it.
//!
//! The classifier reached that verdict twice over. The outlined-text branch
//! keys on path *density*, which is `paths / (paths + glyphs + images)` and so
//! saturates at 1.0 the moment a page has no glyphs and no images — four
//! drawings score exactly what four thousand outlined glyphs would. And the
//! terminal fallback, commented "Nothing usable, some raster → OCR", never
//! tested that any raster was present.
//!
//! These are end-to-end on purpose. The same conditions are covered by unit
//! tests over `PageSignals`, but those construct the signals by hand and would
//! keep passing if the path count stopped being measured from the page at all.

use xberg_native_pdf::PdfDocument;

/// A page whose content stream is `body`, with no images and no text.
fn page(body: &str) -> Vec<u8> {
    let content = body.as_bytes().to_vec();

    let mut buf: Vec<u8> = Vec::new();
    let mut off = vec![0usize; 5];
    let obj = |buf: &mut Vec<u8>, off: &mut Vec<usize>, id: usize, body: &str| {
        off[id] = buf.len();
        buf.extend_from_slice(format!("{id} 0 obj\n{body}\nendobj\n").as_bytes());
    };

    buf.extend_from_slice(b"%PDF-1.7\n%\xE2\xE3\xCF\xD3\n");
    obj(&mut buf, &mut off, 1, "<< /Type /Catalog /Pages 2 0 R >>");
    obj(&mut buf, &mut off, 2, "<< /Type /Pages /Kids [3 0 R] /Count 1 >>");
    obj(
        &mut buf,
        &mut off,
        3,
        "<< /Type /Page /Parent 2 0 R /MediaBox [0 0 200 120] /Contents 4 0 R >>",
    );
    off[4] = buf.len();
    buf.extend_from_slice(format!("4 0 obj\n<< /Length {} >>\nstream\n", content.len()).as_bytes());
    buf.extend_from_slice(&content);
    buf.extend_from_slice(b"\nendstream\nendobj\n");

    let xref = buf.len();
    buf.extend_from_slice(b"xref\n0 5\n0000000000 65535 f \n");
    for o in &off[1..=4] {
        buf.extend_from_slice(format!("{o:010} 00000 n \n").as_bytes());
    }
    buf.extend_from_slice(b"trailer\n<< /Size 5 /Root 1 0 R >>\nstartxref\n");
    buf.extend_from_slice(format!("{xref}\n%%EOF\n").as_bytes());
    buf
}

/// Four filled-and-stroked rectangles, drawn with `B` — the operator a chart
/// or a pattern swatch uses. No text, no image.
fn artwork_pdf() -> Vec<u8> {
    let mut b = String::from("0 0 1 rg 0 0 0 RG 0.5 w\n");
    for i in 0..4 {
        let x = 10.0 + i as f32 * 45.0;
        b.push_str(&format!("{x} 20 m {x} 90 l {} 90 l {} 20 l h B\n", x + 35.0, x + 35.0));
    }
    page(&b)
}

/// The page must not be reported as needing OCR.
#[test]
fn vector_artwork_does_not_need_ocr() {
    let doc = PdfDocument::from_bytes(artwork_pdf()).expect("parse");
    let c = doc.classify_document().expect("classify");
    assert!(
        c.pages_needing_ocr.is_empty(),
        "a page of vector drawings has no raster to read: {:?}",
        c.pages_needing_ocr
    );
}

/// And nothing is written into the extracted content about it.
#[test]
fn vector_artwork_yields_no_prose() {
    let doc = PdfDocument::from_bytes(artwork_pdf()).expect("parse");
    let md = doc.extract_text_auto(0).expect("text");
    assert!(
        !md.to_lowercase().contains("ocr"),
        "the page contains no raster and no text; nothing should be asserted \
         about it in the content:\n{md}"
    );
    assert!(
        md.trim().is_empty(),
        "a page with no text should extract as nothing:\n{md}"
    );
}

/// The control that stops the fix passing by never reporting OCR: a page that
/// really is a scan still does. A single image covering the page, no text.
#[test]
fn test_real_scan_still_needs_ocr() {
    // A grey image scaled over the whole page. 16x16 rather than a token 2x2:
    // images below a few pixels are filtered as spacers, and a fixture under
    // that floor would pass by never being seen rather than by being judged.
    let img = "q 200 0 0 120 0 0 cm /Im0 Do Q\n";
    let content = img.as_bytes().to_vec();
    let pixels: Vec<u8> = (0..16 * 16 * 3).map(|i| 90 + (i % 60) as u8).collect();

    let mut buf: Vec<u8> = Vec::new();
    let mut off = vec![0usize; 6];
    let obj = |buf: &mut Vec<u8>, off: &mut Vec<usize>, id: usize, body: &str| {
        off[id] = buf.len();
        buf.extend_from_slice(format!("{id} 0 obj\n{body}\nendobj\n").as_bytes());
    };
    buf.extend_from_slice(b"%PDF-1.7\n%\xE2\xE3\xCF\xD3\n");
    obj(&mut buf, &mut off, 1, "<< /Type /Catalog /Pages 2 0 R >>");
    obj(&mut buf, &mut off, 2, "<< /Type /Pages /Kids [3 0 R] /Count 1 >>");
    obj(
        &mut buf,
        &mut off,
        3,
        "<< /Type /Page /Parent 2 0 R /MediaBox [0 0 200 120] \
         /Resources << /XObject << /Im0 5 0 R >> >> /Contents 4 0 R >>",
    );
    off[4] = buf.len();
    buf.extend_from_slice(format!("4 0 obj\n<< /Length {} >>\nstream\n", content.len()).as_bytes());
    buf.extend_from_slice(&content);
    buf.extend_from_slice(b"\nendstream\nendobj\n");
    off[5] = buf.len();
    buf.extend_from_slice(
        format!(
            "5 0 obj\n<< /Type /XObject /Subtype /Image /Width 16 /Height 16 \
             /ColorSpace /DeviceRGB /BitsPerComponent 8 /Length {} >>\nstream\n",
            pixels.len()
        )
        .as_bytes(),
    );
    buf.extend_from_slice(&pixels);
    buf.extend_from_slice(b"\nendstream\nendobj\n");
    let xref = buf.len();
    buf.extend_from_slice(b"xref\n0 6\n0000000000 65535 f \n");
    for o in &off[1..=5] {
        buf.extend_from_slice(format!("{o:010} 00000 n \n").as_bytes());
    }
    buf.extend_from_slice(b"trailer\n<< /Size 6 /Root 1 0 R >>\nstartxref\n");
    buf.extend_from_slice(format!("{xref}\n%%EOF\n").as_bytes());

    let doc = PdfDocument::from_bytes(buf).expect("parse");
    let c = doc.classify_document().expect("classify");
    assert_eq!(
        c.pages_needing_ocr,
        vec![0],
        "a full-page image with no text layer is exactly what OCR is for"
    );
}
