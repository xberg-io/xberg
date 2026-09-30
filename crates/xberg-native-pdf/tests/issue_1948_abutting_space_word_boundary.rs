//! Regression for xberg-io/xberg#1948: on a tight face the glyphs on either side
//! of a source space can abut or overlap, so their bbox gap is zero or negative.
//! The word merger used to read that as "one word" and fuse the two words,
//! producing `corrispettivisuperiori` / `alcosto` in table cells while the page
//! text kept the correct spacing.
//!
//! The PDF is hand-built (no external fixture): the text is drawn with a
//! negative character spacing (`Tc`), which pulls the glyphs on both sides of
//! the literal space closer than a normal space's advance, so the words abut
//! within the merge threshold while the space character is still present.

use xberg_native_pdf::PdfDocument;

fn obj(buf: &mut Vec<u8>, offsets: &mut [usize], id: usize, body: &str) {
    offsets[id] = buf.len();
    buf.extend_from_slice(format!("{id} 0 obj\n").as_bytes());
    buf.extend_from_slice(body.as_bytes());
    buf.extend_from_slice(b"\nendobj\n");
}

fn stream_obj(buf: &mut Vec<u8>, offsets: &mut [usize], id: usize, dict: &str, data: &[u8]) {
    offsets[id] = buf.len();
    buf.extend_from_slice(format!("{id} 0 obj\n").as_bytes());
    buf.extend_from_slice(format!("<< {dict} /Length {} >>\nstream\n", data.len()).as_bytes());
    buf.extend_from_slice(data);
    buf.extend_from_slice(b"\nendstream\nendobj\n");
}

/// One page showing `corrispettivi superiori`, its inter-word gap tightened by
/// `-2 Tc` so the two words' boxes abut within `0.15 em`.
fn abutting_space_pdf() -> Vec<u8> {
    let content = b"BT /F1 12 Tf -2 Tc 72 700 Td (corrispettivi superiori) Tj ET\n";

    let mut buf: Vec<u8> = Vec::new();
    let mut off = vec![0usize; 6];
    buf.extend_from_slice(b"%PDF-1.7\n%\xE2\xE3\xCF\xD3\n");

    obj(&mut buf, &mut off, 1, "<< /Type /Catalog /Pages 2 0 R >>");
    obj(&mut buf, &mut off, 2, "<< /Type /Pages /Kids [3 0 R] /Count 1 >>");
    obj(
        &mut buf,
        &mut off,
        3,
        "<< /Type /Page /Parent 2 0 R /MediaBox [0 0 612 792] \
         /Resources << /Font << /F1 5 0 R >> >> /Contents 4 0 R >>",
    );
    stream_obj(&mut buf, &mut off, 4, "", content);
    obj(
        &mut buf,
        &mut off,
        5,
        "<< /Type /Font /Subtype /Type1 /BaseFont /Helvetica /Encoding /WinAnsiEncoding >>",
    );

    let xref_off = buf.len();
    buf.extend_from_slice(b"xref\n0 6\n0000000000 65535 f \n");
    for offset in &off[1..=5] {
        buf.extend_from_slice(format!("{offset:010} 00000 n \n").as_bytes());
    }
    buf.extend_from_slice(b"trailer\n<< /Size 6 /Root 1 0 R >>\nstartxref\n");
    buf.extend_from_slice(format!("{xref_off}\n%%EOF\n").as_bytes());
    buf
}

#[test]
fn abutting_glyphs_across_a_space_stay_two_words() {
    let doc = PdfDocument::from_bytes(abutting_space_pdf()).unwrap();
    let words: Vec<String> = doc.extract_words(0).unwrap().into_iter().map(|w| w.text).collect();

    assert_eq!(
        words,
        vec!["corrispettivi".to_string(), "superiori".to_string()],
        "the source space must survive a zero-width space glyph",
    );
    assert!(
        !words.iter().any(|w| w == "corrispettivisuperiori"),
        "the two words were fused across a source space: {words:?}",
    );
}

#[test]
fn page_text_keeps_the_space() {
    let doc = PdfDocument::from_bytes(abutting_space_pdf()).unwrap();
    let text = doc.extract_text(0).unwrap();
    assert!(
        text.contains("corrispettivi superiori"),
        "page text lost the source space: {text:?}",
    );
}
