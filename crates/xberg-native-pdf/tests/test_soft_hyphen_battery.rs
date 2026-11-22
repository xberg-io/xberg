//! Every soft-hyphen case in one place, with the authority for each.
//!
//! This behaviour was changed three times in one release and each change
//! patched whichever unit test happened to fail, which is how a rule that
//! deletes a drawn character survived three reviews. The cases are enumerated
//! here instead, so a change to any of them shows up as a labelled failure
//! rather than a test to edit.
//!
//! The two rules the table encodes:
//!
//! 1. **A marker inside one span is always kept.** A span is one run drawn on
//!    one line, so no break occurs at that point and the glyph was painted.
//!    ISO 32000-1:2008 Annex D Note 5 (`docs/spec/pdf.md`:41814): WinAnsi code
//!    255 is a soft hyphen whose meaning is a break, "but it shall be
//!    typographically the same as hyphen". Deleting it deletes a character the
//!    reader sees.
//! 2. **A marker at a seam is judged by geometry.** §14.8.2.2.3 makes U+00AD a
//!    break offered inside a word, and §9.4.2 puts the only evidence of where
//!    the line ended in the glyph positions. That evidence exists at the seam
//!    between two spans and nowhere later.
//!
//! Every "keep" row below is corroborated by poppler, MuPDF, pdfium, pypdf and
//! pdfminer, all of which preserve the marker, and by v0.3.77, which had no
//! soft-hyphen handling at all and therefore kept all of them.

use xberg_native_pdf::PdfDocument;

/// One page, one text line per entry, each drawn as a single `Tj` so the
/// marker lands *inside* one span. `\255` is the soft hyphen under
/// WinAnsiEncoding.
fn one_span_page(lines: &[&str]) -> Vec<u8> {
    let mut content = String::from("BT /F1 10 Tf\n");
    for (i, l) in lines.iter().enumerate() {
        content.push_str(&format!("1 0 0 1 72 {} Tm ({l}) Tj\n", 700 - 20 * i));
    }
    content.push_str("ET\n");
    build(content.into_bytes())
}

/// Two spans a line apart, returning to the left margin: a real line wrap.
fn wrap_page(first: &str, second: &str) -> Vec<u8> {
    build(format!("BT /F1 10 Tf\n1 0 0 1 300 700 Tm ({first}) Tj\n1 0 0 1 72 686 Tm ({second}) Tj\nET\n").into_bytes())
}

fn build(content: Vec<u8>) -> Vec<u8> {
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
        b"<< /Type /Font /Subtype /Type1 /BaseFont /Helvetica /Encoding /WinAnsiEncoding >>".to_vec(),
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

fn text_of(pdf: Vec<u8>) -> String {
    let doc = PdfDocument::from_bytes(pdf).expect("fixture parses");
    doc.extract_text(0).expect("extract")
}

/// A marker inside one span survives, whatever sits either side of it.
///
/// Each row is `(drawn, must_appear, why)`. `must_appear` is written with the
/// marker as `\u{ad}` so a deletion fails loudly rather than matching a
/// substring.
#[test]
fn test_marker_inside_one_span_always_survives() {
    let cases: &[(&str, &str, &str)] = &[
        (
            "Campus\\255Main",
            "Campus\u{ad}Main",
            "letter-letter: the case that was being deleted; poppler and MuPDF both keep it",
        ),
        (
            "Pharmaceu\\255ticals",
            "Pharmaceu\u{ad}ticals",
            "letter-letter mid-word, one run, so no wrap happened here",
        ),
        (
            "Mac\\255Donald",
            "Mac\u{ad}Donald",
            "uppercase continuation: a hyphenated surname, not two words",
        ),
        (
            "SS\\2552541",
            "SS\u{ad}2541",
            "letter-digit: a part number whose hyphen is drawn",
        ),
        (
            "2023\\25506\\25515",
            "2023\u{ad}06\u{ad}15",
            "digit-digit: a date, and two markers in one run",
        ),
        (
            "un\\255be\\255liev\\255able",
            "un\u{ad}be\u{ad}liev\u{ad}able",
            "three markers in one run all survive",
        ),
        (
            "Cross-\\255sectional",
            "Cross-\u{ad}sectional",
            "a real hyphen followed by a marker: both characters are drawn",
        ),
    ];
    let drawn: Vec<&str> = cases.iter().map(|c| c.0).collect();
    let out = text_of(one_span_page(&drawn));
    let mut failures = Vec::new();
    for (drawn, expect, why) in cases {
        if !out.contains(expect) {
            failures.push(format!("  drawn {drawn:?}\n    expected {expect:?}\n    because {why}"));
        }
    }
    assert!(
        failures.is_empty(),
        "a soft hyphen inside one span was deleted — Annex D Note 5 makes it \
         typographically a hyphen, so the page draws it:\n{}\n--- actual ---\n{out}",
        failures.join("\n")
    );
}

/// The seam cases, judged by geometry. A real wrap closes; nothing else does.
#[test]
fn test_seam_closes_only_a_real_line_wrap() {
    // One line down, back to the left margin: the shape of a wrap.
    let wrapped = text_of(wrap_page("admini\\255", "stration"));
    assert!(
        wrapped.contains("administration"),
        "a genuine line wrap must rejoin, and the marker goes with it:\n{wrapped}"
    );
    assert!(
        !wrapped.contains('\u{ad}'),
        "a closed wrap must not leave its marker behind:\n{wrapped:?}"
    );
}

/// A marker with no letter after it is not hyphenation at all.
#[test]
fn test_marker_not_between_letters_is_untouched() {
    let out = text_of(one_span_page(&["ends\\255", "\\255starts", "a\\255 b"]));
    for expect in ["ends\u{ad}", "\u{ad}starts"] {
        assert!(out.contains(expect), "expected {expect:?} to survive; got:\n{out:?}");
    }
}

/// A hyphen-minus followed by a wrap marker at a line end is an already
/// hyphenated compound broken for layout: the real hyphen must survive and the
/// two halves must not fuse into `Crosssectional`.
#[test]
fn test_wrap_marker_after_a_real_hyphen() {
    let out = text_of(wrap_page("Cross-\\255", "sectional"));
    assert!(
        !out.contains("Crosssectional"),
        "the real hyphen of a hyphenated compound was eaten by the wrap:\n{out:?}"
    );
    assert!(out.contains("Cross-"), "the compound's hyphen must survive:\n{out:?}");
}
