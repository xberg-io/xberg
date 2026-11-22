//! A rotated line is split at a gutter, the same way an upright one is.
//!
//! Runs of quarter-turn text are merged into a line by their offset ACROSS the
//! writing axis, which says nothing about their separation ALONG it. Two
//! columns of rotated text therefore fused into single lines however wide the
//! gutter between them, while the upright path splits a line whenever the gap
//! reaches `max(3 × font size, 30 pt)`.
//!
//! A rotated line is the same line with its axes exchanged — ISO 32000-1:2008
//! §9.4.4 puts the glyph displacement along the text matrix's writing
//! direction — so it gets the same rule measured along its own axis.
//!
//! Both quarter turns are covered. The `-90°` branch sorts descending and had
//! no test at all.

use xberg_native_pdf::PdfDocument;

/// One page holding `runs` of `(text_matrix, x, y, text)`.
fn page(runs: &[(&str, f32, f32, &str)]) -> Vec<u8> {
    let mut content = String::new();
    for (tm, x, y, text) in runs {
        content.push_str(&format!("BT /F1 10 Tf {tm} {x} {y} Tm ({text}) Tj ET\n"));
    }
    let content = content.into_bytes();

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
        "<< /Type /Page /Parent 2 0 R /MediaBox [0 0 612 792] \
         /Resources << /Font << /F1 5 0 R >> >> /Contents 4 0 R >>",
    );
    off[4] = buf.len();
    buf.extend_from_slice(format!("4 0 obj\n<< /Length {} >>\nstream\n", content.len()).as_bytes());
    buf.extend_from_slice(&content);
    buf.extend_from_slice(b"\nendstream\nendobj\n");
    obj(
        &mut buf,
        &mut off,
        5,
        "<< /Type /Font /Subtype /Type1 /BaseFont /Helvetica /Encoding /WinAnsiEncoding >>",
    );

    let xref = buf.len();
    buf.extend_from_slice(b"xref\n0 6\n0000000000 65535 f \n");
    for offset in &off[1..=5] {
        buf.extend_from_slice(format!("{:010} 00000 n \n", *offset).as_bytes());
    }
    buf.extend_from_slice(b"trailer\n<< /Size 6 /Root 1 0 R >>\nstartxref\n");
    buf.extend_from_slice(format!("{xref}\n%%EOF\n").as_bytes());
    buf
}

fn lines(runs: &[(&str, f32, f32, &str)]) -> Vec<String> {
    let doc = PdfDocument::from_bytes(page(runs)).expect("parse");
    doc.extract_text_lines(0)
        .expect("lines")
        .into_iter()
        .map(|l| l.text.clone())
        .collect()
}

const P90: &str = "0 1 -1 0";
const M90: &str = "0 -1 1 0";

/// Two columns of `+90°` text. The run advances along `+y`, so the columns are
/// separated along `y` by a gutter far wider than `max(3 × 10, 30)`.
#[test]
fn two_plus_ninety_columns_do_not_fuse() {
    let l = lines(&[
        (P90, 100.0, 200.0, "Alpha"),
        (P90, 100.0, 500.0, "Charlie"),
        (P90, 114.0, 200.0, "Bravo"),
        (P90, 114.0, 500.0, "Delta"),
    ]);
    assert!(
        !l.iter().any(|s| s.contains("Alpha") && s.contains("Charlie")),
        "the two columns fused into one line: {l:?}"
    );
    assert_eq!(l.len(), 4, "expected one line per column entry: {l:?}");
}

/// The `-90°` branch sorts along the opposite direction and had no coverage.
#[test]
fn two_minus_ninety_columns_do_not_fuse() {
    let l = lines(&[
        (M90, 100.0, 600.0, "Alpha"),
        (M90, 100.0, 300.0, "Charlie"),
        (M90, 114.0, 600.0, "Bravo"),
        (M90, 114.0, 300.0, "Delta"),
    ]);
    assert!(
        !l.iter().any(|s| s.contains("Alpha") && s.contains("Charlie")),
        "the two columns fused into one line: {l:?}"
    );
    assert_eq!(l.len(), 4, "expected one line per column entry: {l:?}");
}

/// The control that keeps the split from being a regression: runs adjacent
/// along the writing axis are one line, and must stay one line. "Alpha" is
/// 25.57 pt wide at 10 pt, so starting the next run at 229 leaves a word-sized
/// gap rather than a gutter.
#[test]
fn adjacent_rotated_runs_are_still_one_line() {
    let l = lines(&[(P90, 100.0, 200.0, "Alpha"), (P90, 100.0, 229.0, "Bravo")]);
    assert_eq!(l.len(), 1, "a word gap must not split a rotated line: {l:?}");
    assert!(l[0].contains("Alpha") && l[0].contains("Bravo"), "{l:?}");
}

/// The same control for `-90°`.
#[test]
fn adjacent_minus_ninety_runs_are_still_one_line() {
    let l = lines(&[(M90, 100.0, 500.0, "Alpha"), (M90, 100.0, 471.0, "Bravo")]);
    assert_eq!(l.len(), 1, "a word gap must not split a rotated line: {l:?}");
    assert!(l[0].contains("Alpha") && l[0].contains("Bravo"), "{l:?}");
}

/// Upright text is the reference the rotated rule mirrors, and must be
/// untouched: a wide gutter splits, a word gap does not.
#[test]
fn upright_text_keeps_its_own_behaviour() {
    let flat = "1 0 0 1";
    let gutter = lines(&[(flat, 100.0, 500.0, "Alpha"), (flat, 400.0, 500.0, "Charlie")]);
    assert_eq!(gutter.len(), 2, "an upright gutter should split: {gutter:?}");

    let word = lines(&[(flat, 100.0, 500.0, "Alpha"), (flat, 129.0, 500.0, "Bravo")]);
    assert_eq!(word.len(), 1, "an upright word gap should not split: {word:?}");
}
