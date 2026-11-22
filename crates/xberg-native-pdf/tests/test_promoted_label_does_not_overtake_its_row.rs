//! A promoted rowspan label must not overtake the value on the row above it.
//!
//! `reorder_rowspan_labels` re-keys a candidate label to `anchor + 1.0`, where
//! `anchor` is the baseline of the dense row it belongs above. The offset is
//! deliberately smaller than the 3 pt row band, so the promoted span lands in
//! the SAME band as that row and then sorts within it on x — which is the
//! whole point: a label promoted to a row must read at that row's left edge.
//!
//! `row_aware_span_cmp` breaks a within-band x tie on the baseline, larger y
//! first. The promoted key is 1.0 pt LARGER than the row it was promoted onto,
//! so wherever the promoted span shares an x with a real member of that row —
//! a two-line table cell, whose continuation line starts at the same left edge
//! as the line above it — the synthetic +1.0 wins the tie and the continuation
//! is emitted BEFORE the line it continues, with the real row member pushed
//! out behind it.
//!
//! Geometry is the load-bearing part of this fixture, and four properties are
//! each required:
//!   * the value column's continuation rows (y=613.42, y=586.06) sit on
//!     baselines the label column never uses, so they are misread as labels;
//!   * the label column's widest entry reaches within 18 pt of the value
//!     column, so `has_clean_column_gutter` does not call the page
//!     two-column — otherwise `assemble_text_from_spans` never runs the
//!     row-aware sort at all;
//!   * body text above AND below the table, so the label column is the dense
//!     one and the value column counts as sparse (5 * 2 < 12);
//!   * `599.50` and `599.50 + 1.0` round into the same 3 pt band.
//!
//! Poppler, PyMuPDF, pdfium and pdfminer.six all read the value column in
//! drawing order on this page.

use xberg_native_pdf::PdfDocument;

/// Body prose, a two-column schedule whose right-hand cells wrap onto a
/// second line, and more body prose.
fn schedule_pdf() -> Vec<u8> {
    // (x, y, text)
    let spans: &[(f32, f32, &str)] = &[
        (
            57.6,
            759.24,
            "Due to scheduled summer vacation weeks the laboratory is modifying its testing schedule",
        ),
        (
            57.6,
            745.80,
            "and the tests below will now be performed once a month with a maximum of five patients",
        ),
        (
            57.6,
            732.36,
            "per week until the regular schedule resumes later in the year for every requesting ward",
        ),
        (
            57.6,
            719.02,
            "Specimens arriving outside the windows in the schedule below cannot be analysed at all",
        ),
        (
            57.6,
            705.58,
            "Requesting wards should confirm the collection window with the laboratory in advance",
        ),
        (
            57.6,
            692.14,
            "The laboratory will not accept a specimen that has not been pre-booked by the ward",
        ),
        (
            57.6,
            678.70,
            "Questions about the schedule can be directed to the biochemist on call at any time",
        ),
        (
            57.6,
            665.26,
            "The modified testing schedule for the coming weeks is set out in the table below",
        ),
        // Label column.
        (60.0, 640.78, "Modified Schedule"),
        (60.0, 626.86, "Week of July 17, 2022"),
        (60.0, 599.50, "Week of August 7, 2022"),
        (60.0, 572.11, "Week of September 4, 2022"),
        // Value column; the two "continuation" lines wrap their cell.
        (200.0, 640.78, "Reception of Specimen"),
        (200.0, 626.86, "Row one specimen note"),
        (200.0, 613.42, "Row one continuation line"),
        (200.0, 599.50, "Row two specimen note"),
        (200.0, 586.06, "Row two continuation line"),
        (
            57.6,
            544.75,
            "Patients must be pre-booked for testing by contacting client services before collection",
        ),
        (
            57.6,
            531.31,
            "Specimens that arrive without a booking will not be accepted and will not be analysed",
        ),
        (
            57.6,
            517.87,
            "Should you have any questions you can refer them to the biochemist on call for the day",
        ),
        (
            57.6,
            504.43,
            "We thank you for your understanding and your cooperation with the modified schedule",
        ),
        (
            57.6,
            490.99,
            "This notice replaces every earlier notice covering the same collection weeks entirely",
        ),
        (
            57.6,
            477.55,
            "A copy of this notice has been sent to every ward that requests these tests routinely",
        ),
    ];

    let mut content = String::new();
    for (x, y, text) in spans {
        content.push_str(&format!("BT /F1 11.04 Tf 1 0 0 1 {x} {y} Tm ({text}) Tj ET\n"));
    }
    let content = content.into_bytes();

    let objects: Vec<Vec<u8>> = vec![
        b"<< /Type /Catalog /Pages 2 0 R >>".to_vec(),
        b"<< /Type /Pages /Kids [3 0 R] /Count 1 >>".to_vec(),
        b"<< /Type /Page /Parent 2 0 R /MediaBox [0 0 612 792] /Contents 4 0 R \
           /Resources << /Font << /F1 5 0 R >> >> >>"
            .to_vec(),
        [
            format!("<< /Length {} >>\nstream\n", content.len()).into_bytes(),
            content.clone(),
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

#[test]
fn test_promoted_label_does_not_overtake_the_row_it_was_promoted_onto() {
    let doc = PdfDocument::from_bytes(schedule_pdf()).expect("open");
    let text = doc.extract_text(0).expect("extract");

    let note = text
        .find("Row two specimen note")
        .unwrap_or_else(|| panic!("value missing:\n{text}"));
    let cont = text
        .find("Row two continuation line")
        .unwrap_or_else(|| panic!("continuation missing:\n{text}"));

    assert!(
        note < cont,
        "a cell's second line was emitted before its first, and the first line \
         was pushed out behind it:\n{text}"
    );

    // The row must still read label-then-value, on one line.
    let label = text.find("Week of August 7, 2022").expect("label missing");
    assert!(label < note, "the row's label must precede its value:\n{text}");
}
