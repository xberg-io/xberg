//! A span that ends before the previous one begins must not be glued to it.
//!
//! Scanned books carry an invisible OCR text layer (`3 Tr`) drawn one word
//! per `BT … ET` block, each word at its own font size and with baselines
//! that jitter by a couple of points. The heterogeneous sizes keep the
//! `Tm`-run merge from folding the words into one span, and the jitter is
//! large enough that the reading-order sort can emit them right-to-left.
//! The inline-flow separator rule then measured `current.x - prev_end_x`,
//! saw a negative number, read it as sub-em kerning, and concatenated:
//! `It is the` came out of `extract_text` as `theisIt`.
//!
//! ISO 32000-1:2008 §9.4.3 — the show operators paint the glyphs they are
//! given; three separately positioned words are three tokens, never one.
//! A span lying entirely to the left of the previous span's origin cannot
//! be its continuation, so the two must be separated whatever order they
//! arrive in.

use xberg_native_pdf::document::PdfDocument;

/// Single page, one `BT … ET` block per word, each with its own font size
/// and absolute `Tm`, all inside an invisible text-rendering-mode block.
fn make_ocr_layer_pdf(words: &[(&str, f64, f64, f64)]) -> Vec<u8> {
    let mut stream = String::from("q 3 Tr\n");
    for (word, x, y, size) in words {
        stream.push_str(&format!("BT /F1 {size} Tf 1 0 0 1 {x:.2} {y:.2} Tm ({word}) Tj ET\n"));
    }
    stream.push_str("Q\n");

    let mut pdf: Vec<u8> = Vec::new();
    macro_rules! push {
        ($s:expr) => {
            pdf.extend_from_slice($s.as_bytes())
        };
    }

    push!("%PDF-1.4\n");
    let off1 = pdf.len();
    push!("1 0 obj\n<< /Type /Catalog /Pages 2 0 R >>\nendobj\n");
    let off2 = pdf.len();
    push!("2 0 obj\n<< /Type /Pages /Kids [3 0 R] /Count 1 >>\nendobj\n");
    let off3 = pdf.len();
    push!(
        "3 0 obj\n\
         << /Type /Page /Parent 2 0 R /MediaBox [0 0 612 792]\n\
            /Contents 4 0 R /Resources << /Font << /F1 5 0 R >> >> >>\n\
         endobj\n"
    );
    let off4 = pdf.len();
    let stream_bytes = stream.as_bytes();
    push!(format!("4 0 obj\n<< /Length {} >>\nstream\n", stream_bytes.len()));
    pdf.extend_from_slice(stream_bytes);
    push!("\nendstream\nendobj\n");
    let off5 = pdf.len();
    push!("5 0 obj\n<< /Type /Font /Subtype /Type1 /BaseFont /Helvetica >>\nendobj\n");
    let xref_off = pdf.len();
    push!(format!(
        "xref\n0 6\n\
         0000000000 65535 f \r\n\
         {off1:010} 00000 n \r\n\
         {off2:010} 00000 n \r\n\
         {off3:010} 00000 n \r\n\
         {off4:010} 00000 n \r\n\
         {off5:010} 00000 n \r\n"
    ));
    push!(format!(
        "trailer\n<< /Size 6 /Root 1 0 R >>\nstartxref\n{xref_off}\n%%EOF\n"
    ));
    pdf
}

#[test]
fn words_emitted_right_to_left_are_still_separated_in_text() {
    // Baselines jitter over ~2.5pt and the sizes differ per word, mirroring
    // the OCR layer of a scanned dictionary page.
    let pdf = make_ocr_layer_pdf(&[
        ("It", 72.0, 700.0, 6.0),
        ("is", 84.0, 701.4, 3.0),
        ("the", 92.0, 702.5, 6.0),
    ]);
    let doc = PdfDocument::from_bytes(pdf).unwrap();
    let visible = doc.extract_text(0).unwrap();

    for glued in ["theis", "isIt", "theIt", "Itis", "isthe", "Itthe"] {
        assert!(
            !visible.contains(glued),
            "separately positioned words must not be concatenated; \
             found {glued:?} in {visible:?}"
        );
    }
    for word in ["It", "is", "the"] {
        assert!(
            visible.split_whitespace().any(|t| t == word),
            "{word:?} must survive as its own token; got {visible:?}"
        );
    }
}

#[test]
fn overlapping_glyph_advance_still_joins() {
    // Control: the second span starts *inside* the first — an over-wide
    // advance estimate, not a reading discontinuity — and must stay joined.
    // `co` is 12pt wide at 12pt Helvetica but the box is declared wider, so
    // `mpany` begins before it ends while still extending past its origin.
    let pdf = make_ocr_layer_pdf(&[("co", 72.0, 700.0, 12.0), ("mpany", 79.0, 700.0, 12.0)]);
    let doc = PdfDocument::from_bytes(pdf).unwrap();
    let visible = doc.extract_text(0).unwrap();

    assert!(
        visible.contains("company"),
        "a forward-overlapping continuation must stay joined; got {visible:?}"
    );
}
