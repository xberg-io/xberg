//! Two lines drawn on top of one another stay two lines.
//!
//! A revised running footer is often stamped over the one it replaces rather
//! than removed: two complete strings, in different fonts and sizes, on
//! baselines under 2 pt apart, occupying the same horizontal band. ISO
//! 32000-1:2008 §9.4.4 (docs/spec/pdf.md:17438) moves the text position only
//! along the writing axis, so each run's left edge and advance are exactly what
//! the file drew — and the two runs' x-intervals overlap almost entirely. Two
//! runs cannot share a line and share the space on it; the overlap is the proof
//! that they are two lines.
//!
//! Row assignment reads only vertical agreement, so it puts both runs on one
//! row and then orders that row by left edge, splicing one footer into the
//! middle of the other. This asserts the span order rather than one rendered
//! surface, because the order is what every surface inherits: text, markdown
//! and html each recover from it to a different degree, and on the page that
//! exposed this the plain-text surface did not recover at all.
//!
//! The geometry is a real superimposed footer's: 7 pt over 9 pt, cap tops
//! 0.15 pt apart and baselines 1.86 pt apart — inside the row tolerance by
//! either edge — with the lower run's left edge 3.55 pt inside the upper one's.

use xberg_native_pdf::PdfDocument;

fn build_pdf(content: Vec<u8>) -> Vec<u8> {
    let objects: Vec<Vec<u8>> = vec![
        b"<< /Type /Catalog /Pages 2 0 R >>".to_vec(),
        b"<< /Type /Pages /Kids [3 0 R] /Count 1 >>".to_vec(),
        b"<< /Type /Page /Parent 2 0 R /MediaBox [0 0 612 792] /Contents 4 0 R \
           /Resources << /Font << /F1 5 0 R /F2 6 0 R >> >> >>"
            .to_vec(),
        [
            format!("<< /Length {} >>\nstream\n", content.len()).into_bytes(),
            content,
            b"\nendstream".to_vec(),
        ]
        .concat(),
        b"<< /Type /Font /Subtype /Type1 /BaseFont /Helvetica >>".to_vec(),
        b"<< /Type /Font /Subtype /Type1 /BaseFont /Times-Roman >>".to_vec(),
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

/// The order the words come back in, one entry per span.
fn span_order(pdf: Vec<u8>) -> Vec<String> {
    let doc = PdfDocument::from_bytes(pdf).expect("open");
    doc.extract_spans(0)
        .expect("spans")
        .into_iter()
        .map(|s| s.text.trim().to_string())
        .filter(|t| !t.is_empty())
        .collect()
}

fn position_of(order: &[String], needle: &str) -> usize {
    order
        .iter()
        .position(|t| t.contains(needle))
        .unwrap_or_else(|| panic!("`{needle}` is missing from the span order: {order:?}"))
}

/// Two superimposed footers. The upper (7 pt, `/F1`) runs `Alpha … Bravo`; the
/// lower (9 pt, `/F2`) runs `Charlie … Delta`, and its left edge sits 3.55 pt
/// inside the upper one's.
fn superimposed_footers() -> Vec<u8> {
    build_pdf(
        "\
BT /F1 7 Tf 1 0 0 1 217.27 545.28 Tm (Alpha) Tj ET
BT /F1 7 Tf 1 0 0 1 293.23 545.28 Tm (Bravo) Tj ET
BT /F2 9 Tf 1 0 0 1 220.82 543.43 Tm (Charlie) Tj ET
BT /F2 9 Tf 1 0 0 1 345.97 543.43 Tm (Delta) Tj ET
"
        .to_string()
        .into_bytes(),
    )
}

#[test]
fn test_footer_stamped_over_another_is_not_spliced_into_it() {
    let order = span_order(superimposed_footers());

    let (alpha, bravo) = (position_of(&order, "Alpha"), position_of(&order, "Bravo"));
    let (charlie, delta) = (position_of(&order, "Charlie"), position_of(&order, "Delta"));

    assert!(
        bravo < charlie,
        "the upper footer must be read whole before the lower one begins, not \
         spliced with it by left edge; got: {order:?}"
    );
    assert!(
        alpha < bravo && charlie < delta,
        "each footer must keep its own left-to-right order; got: {order:?}"
    );
}

/// The counter-case. Two runs on one real line — same baseline, no horizontal
/// overlap — must still be joined into one row and ordered by left edge, which
/// is what row assignment is for.
#[test]
fn two_runs_side_by_side_on_one_baseline_still_share_a_line() {
    let order = span_order(build_pdf(
        "\
BT /F1 9 Tf 1 0 0 1 300 545.00 Tm (Charlie) Tj ET
BT /F2 9 Tf 1 0 0 1 100 545.00 Tm (Alpha) Tj ET
"
        .to_string()
        .into_bytes(),
    ));
    assert!(
        position_of(&order, "Alpha") < position_of(&order, "Charlie"),
        "two runs on one baseline that do not overlap are one line, read left \
         to right whatever order they were drawn in; got: {order:?}"
    );
}
