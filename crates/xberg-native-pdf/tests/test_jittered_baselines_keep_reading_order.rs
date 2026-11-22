//! A line whose words sit on jittered baselines must still read left to right.
//!
//! `XYCutStrategy::sort_indices` — the leaf ordering of the strategy reached
//! by default, since `StructureTreeStrategy` falls back to it whenever there
//! is no structure tree — sorted by `bbox.top()` and fell back to `x` only
//! when the two tops were *exactly* equal. Two things were wrong with that.
//!
//! Exact equality is not a row test: any sub-point difference put two words of
//! one line into different "rows", the `x` tiebreak never ran, and the order
//! degenerated into a pure descending sort. On a scanned book's OCR layer,
//! whose per-word baselines jitter by a couple of points, whole lines came out
//! backwards.
//!
//! And `top()` is the wrong edge. It moves with the font size, so a line
//! mixing 2 pt punctuation with 8 pt words has tops further apart than the
//! line spacing while the baselines agree to a fraction of a point. ISO
//! 32000-1:2008 §9.4.4 puts the glyph displacement along the writing axis,
//! which makes the baseline — not the ascender — what identifies a line.

use xberg_native_pdf::document::PdfDocument;

/// One `BT … ET` per word, each with its own font size and absolute `Tm`.
fn make_page(words: &[(&str, f64, f64, f64)]) -> Vec<u8> {
    let mut stream = String::new();
    for (word, x, y, size) in words {
        stream.push_str(&format!("BT /F1 {size} Tf 1 0 0 1 {x:.2} {y:.2} Tm ({word}) Tj ET\n"));
    }

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
    let bytes = stream.as_bytes();
    push!(format!("4 0 obj\n<< /Length {} >>\nstream\n", bytes.len()));
    pdf.extend_from_slice(bytes);
    push!("\nendstream\nendobj\n");
    let off5 = pdf.len();
    push!("5 0 obj\n<< /Type /Font /Subtype /Type1 /BaseFont /Helvetica >>\nendobj\n");
    let xref = pdf.len();
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
        "trailer\n<< /Size 6 /Root 1 0 R >>\nstartxref\n{xref}\n%%EOF\n"
    ));
    pdf
}

/// Positions of `needles` in the token sequence, in the order they occur.
fn order_of(text: &str, needles: &[&str]) -> Vec<usize> {
    let toks: Vec<&str> = text.split_whitespace().collect();
    needles
        .iter()
        .filter_map(|n| {
            toks.iter()
                .position(|t| t.trim_matches(|c: char| !c.is_alphanumeric()) == *n)
        })
        .collect()
}

/// One line of six words with the geometry of a scanned page's OCR layer,
/// taken from a real one: baselines jittering non-monotonically over 1.4 pt
/// and font sizes from 2.2 pt to 7.9 pt.
///
/// The two spreads are what make this the reproduction. The baselines are far
/// enough apart that an exact-equality row test never fires, so the old
/// comparator's `x` tiebreak was unreachable. And because `top()` moves with
/// the font size, the *tops* span 4.3 pt — wider than the baselines — in an
/// order unrelated to the reading order: sorting on them descending gives
/// Alpha, Echo, Charlie, Foxtrot, Bravo, Delta.
// The font sizes here are measurements in points, not mathematical
// constants; `6.28` is a glyph height, and the lint that reads it as an
// approximation of tau has no bearing on what the fixture is for.
#[allow(clippy::approx_constant)]
fn jittered_line() -> Vec<u8> {
    make_page(&[
        ("Alpha", 18.0, 531.0, 7.90),
        ("Bravo", 60.0, 531.4, 5.11),
        ("Charlie", 100.0, 531.2, 6.83),
        ("Delta", 150.0, 532.4, 2.16),
        ("Echo", 180.0, 531.8, 6.28),
        ("Foxtrot", 220.0, 532.1, 5.51),
    ])
}

#[test]
fn extract_text_reads_a_jittered_line_left_to_right() {
    let doc = PdfDocument::from_bytes(jittered_line()).unwrap();
    let text = doc.extract_text(0).unwrap();
    let seen = order_of(&text, &["Alpha", "Bravo", "Charlie", "Delta", "Echo", "Foxtrot"]);
    assert_eq!(seen.len(), 6, "every word must survive; got {text:?}");
    assert!(
        seen.windows(2).all(|w| w[0] < w[1]),
        "words of one line must read left to right; got {text:?}"
    );
}

/// Control: a genuine line break must still separate. Without this the test
/// above would also pass if row grouping swallowed the whole page into one
/// row and sorted it by x.
#[test]
fn test_real_line_break_still_separates_rows() {
    let doc = PdfDocument::from_bytes(make_page(&[
        ("Second", 200.0, 700.0, 10.0),
        ("First", 18.0, 720.0, 10.0),
    ]))
    .unwrap();
    let text = doc.extract_text(0).unwrap();
    let seen = order_of(&text, &["First", "Second"]);
    assert_eq!(seen, vec![0, 1], "the upper line must come first; got {text:?}");
}
