//! A caption set in small capitals is one word, not an initial and a remainder.
//!
//! Legal and regulatory tables title themselves with a full-size initial
//! capital followed by small capitals — `COORDINATES` is drawn as `C` at the
//! caption size and `OORDINATES` a couple of points smaller, on one baseline,
//! the second run starting at the first's advance edge.
//!
//! ISO 32000-1:2008 §9.4.4 makes the glyph advance the only thing that moves
//! the text position, so the second run begins exactly where the first ends:
//! there is no gap between them beyond the initial's own advance. A reader that
//! widens its word-space test at a font-size change reads that advance as a
//! space and emits `C OORDINATES`, and the caption stops being searchable under
//! the word it is titled with.
//!
//! Measured on the page this comes from, both halves of the caption abut to the
//! hundredth of a point and carry the same size step:
//!
//! ```text
//! size=8.00  x0=148.66 x1=153.55  "T"
//! size=6.45  x0=153.55 x1=169.91  "ABLE"            <- joins
//! size=8.00  x0=172.57 x1=229.48  "66.01-11(5)-C"
//! size=6.45  x0=229.48 x1=271.80  "OORDINATES"      <- splits
//! ```
//!
//! The pair that splits is the one whose first run is long. That is what the
//! fixture reproduces: a single-glyph run before the size step is not enough to
//! exhibit the defect, so a fixture built that way would pass against it.
//!
//! The counter-case is in the same file, because a rule that simply stopped
//! separating at a size change would glue a heading to the paragraph beneath it
//! and a footnote marker's digits to the following word.

use xberg_native_pdf::PdfDocument;

/// Helvetica advance widths at 1000 units/em for the glyphs used here.
fn advance(c: char, size: f32) -> f32 {
    let per_mille = match c {
        'C' | 'O' | 'D' => 722.0,
        'T' | 'S' => 611.0,
        'A' | 'B' | 'E' | 'R' => 667.0,
        'L' | 'I' | 'N' | 'M' | 'X' | 'U' => 556.0,
        '0'..='9' => 556.0,
        '(' | ')' => 333.0,
        '-' => 333.0,
        '.' => 278.0,
        ' ' => 278.0,
        _ => 556.0,
    };
    per_mille / 1000.0 * size
}

fn width_of(s: &str, size: f32) -> f32 {
    s.chars().map(|c| advance(c, size)).sum()
}

/// A page carrying `<initial>` at `big` points and `<rest>` at `small` points,
/// the second starting exactly at the first's advance edge — the geometry a
/// small-capitals run actually has.
fn small_caps_page(initial: &str, rest: &str, big: f32, small: f32) -> Vec<u8> {
    let x0 = 72.0_f32;
    let x1 = x0 + width_of(initial, big);
    let content = format!(
        "BT /F1 {big} Tf {x0:.2} 700 Td ({initial}) Tj ET\n\
         BT /F1 {small} Tf {x1:.2} 700 Td ({rest}) Tj ET\n"
    );
    build_page(&content)
}

fn text_of(pdf: Vec<u8>) -> String {
    let doc = PdfDocument::from_bytes(pdf).expect("open");
    doc.extract_text(0).expect("text")
}

#[test]
fn test_small_capitals_word_is_not_split_after_its_initial() {
    // The initial is the last glyph of a long run, as it is on the page.
    let text = text_of(small_caps_page("66.01-11(5)-C", "OORDINATES", 8.0, 6.45));
    assert!(
        text.contains("-COORDINATES"),
        "a small-capitals caption is one word; the size change between its \
         initial and its remainder is not a word space. Got: {text:?}"
    );
    assert!(
        !text.contains("C OORDINATES"),
        "the initial was separated from its own word: {text:?}"
    );
}

/// A two-capital initial run at the caption's size step: the shape on which the
/// word-space heuristic split the caption after the initial.
#[test]
fn test_multi_capital_initial_stays_with_its_small_capitals() {
    let text = text_of(small_caps_page("CA", "LIFORNIA", 8.0, 6.45));
    assert!(
        text.contains("CALIFORNIA"),
        "the size step between the initial and its remainder is not a word space. Got: {text:?}"
    );
}

/// The control from the same caption: a single-glyph run before the same size
/// step. This one already comes out joined, so it is here to show the fixture
/// above is testing the long-run case and not the size step on its own.
#[test]
fn test_single_glyph_initial_also_stays_with_its_word() {
    let text = text_of(small_caps_page("T", "ABLE", 8.0, 6.45));
    assert!(
        text.contains("TABLE"),
        "a single-glyph initial and its small capitals are one word. Got: {text:?}"
    );
}

/// The counter-direction. Two runs that are genuinely separate words stay
/// separate: a real space glyph's width sits between them, which is what a word
/// space looks like and what a size change alone must not imitate.
#[test]
fn test_real_word_space_still_separates_two_words() {
    let x0 = 72.0_f32;
    let x1 = x0 + width_of("TABLE", 11.0) + advance(' ', 11.0);
    let text = text_of(build_page(&format!(
        "BT /F1 11 Tf {x0:.2} 700 Td (TABLE) Tj ET\n\
         BT /F1 8 Tf {x1:.2} 700 Td (OF) Tj ET\n"
    )));
    assert!(
        !text.contains("TABLEOF"),
        "a genuine word space must still separate two words: {text:?}"
    );
}

/// Minimal single-content-stream page writer, shared by the counter-case.
fn build_page(content: &str) -> Vec<u8> {
    let content = content.as_bytes().to_vec();
    let objects: Vec<Vec<u8>> = vec![
        b"<< /Type /Catalog /Pages 2 0 R >>".to_vec(),
        b"<< /Type /Pages /Kids [3 0 R] /Count 1 >>".to_vec(),
        b"<< /Type /Page /Parent 2 0 R /MediaBox [0 0 612 792] /Contents 4 0 R \
           /Resources << /Font << /F1 5 0 R >> >> >>"
            .to_vec(),
        [
            format!("<< /Length {} >>\nstream\n", content.len()).into_bytes(),
            content,
            b"\nendstream".to_vec(),
        ]
        .concat(),
        b"<< /Type /Font /Subtype /Type1 /BaseFont /Helvetica >>".to_vec(),
    ];
    let mut pdf = b"%PDF-1.7\n".to_vec();
    let mut offsets = Vec::new();
    for (i, body) in objects.iter().enumerate() {
        offsets.push(pdf.len());
        pdf.extend_from_slice(format!("{} 0 obj\n", i + 1).as_bytes());
        pdf.extend_from_slice(body);
        pdf.extend_from_slice(b"\nendobj\n");
    }
    let xref = pdf.len();
    let n = objects.len() + 1;
    pdf.extend_from_slice(format!("xref\n0 {n}\n0000000000 65535 f \n").as_bytes());
    for off in &offsets {
        pdf.extend_from_slice(format!("{off:010} 00000 n \n").as_bytes());
    }
    pdf.extend_from_slice(format!("trailer\n<< /Size {n} /Root 1 0 R >>\nstartxref\n{xref}\n%%EOF\n").as_bytes());
    pdf
}
