//! A rotated line must group into one `TextLine`, not one line per word.
//!
//! `extract_text_lines` bands words by y. A rotated run advances along y, so
//! every word of one visual line lands in its own band and the page comes back
//! as one-word lines. `extract_text` does not have this problem — it assembles
//! rotated pages in their own frame — so the two surfaces disagree about the
//! same page.

use xberg_native_pdf::document::PdfDocument;

/// One 90°-rotated line whose words are each positioned by their own `Tm`, the
/// way a rotated table row or chart axis is typically drawn. All four share a
/// perpendicular offset (x) and advance along the writing axis (y), so they are
/// one visual line drawn as four runs.
///
/// The words sit about 11 pt apart at 10 pt type — a wide word gap. They were
/// originally 40 to 60 pt apart, which is a column gap rather than a word gap:
/// wider than the `max(3 x font size, 30 pt)` at which the line grouping
/// separates columns, so an UPRIGHT line spaced that way splits into two lines
/// as well. Now that the rotated path applies the same threshold along its own
/// writing axis, the original spacing described two lines rather than one. The
/// fixture's subject is unchanged: several runs, one visual line.
fn rotated_line_of_separate_runs_pdf() -> Vec<u8> {
    let mut content = Vec::new();
    content.extend_from_slice(b"BT /F1 10 Tf\n");
    for (y, word) in [(150, "the"), (175, "quick"), (210, "brown"), (247, "fox")] {
        content.extend_from_slice(format!("0 1 -1 0 200 {y} Tm ({word}) Tj\n").as_bytes());
    }
    content.extend_from_slice(b"ET");
    build_minimal_pdf_raw(&content, b"/Type /Page /Parent 2 0 R /MediaBox [0 0 612 792]")
}

#[test]
fn test_rotated_line_drawn_as_separate_runs_groups_into_one_line() {
    let doc = PdfDocument::from_bytes(rotated_line_of_separate_runs_pdf()).expect("parse fixture");
    let words = doc.extract_words(0).expect("extract words");
    let lines = doc.extract_text_lines(0).expect("extract lines");

    // The fixture must be rotated and must reach line grouping as several runs,
    // or the assertion below proves nothing.
    assert!(
        doc.extract_chars(0)
            .expect("extract chars")
            .iter()
            .any(|c| (c.rotation_degrees - 90.0).abs() < 0.5),
        "fixture produced no rotated glyphs"
    );
    assert!(
        doc.extract_spans(0).expect("extract spans").len() >= 4,
        "fixture collapsed into fewer runs than it draws"
    );

    assert_eq!(
        lines.len(),
        1,
        "one visual line came back as {} lines: {:?}",
        lines.len(),
        lines.iter().map(|l| l.text.trim()).collect::<Vec<_>>()
    );
    assert_eq!(words.len(), 4, "expected 4 words, got {}", words.len());
}

fn build_minimal_pdf_raw(content: &[u8], page_extra: &[u8]) -> Vec<u8> {
    let mut pdf = b"%PDF-1.4\n".to_vec();

    let off1 = pdf.len();
    pdf.extend_from_slice(b"1 0 obj\n<< /Type /Catalog /Pages 2 0 R >>\nendobj\n");

    let off2 = pdf.len();
    pdf.extend_from_slice(b"2 0 obj\n<< /Type /Pages /Kids [3 0 R] /Count 1 >>\nendobj\n");

    let off3 = pdf.len();
    pdf.extend_from_slice(b"3 0 obj\n<< ");
    pdf.extend_from_slice(page_extra);
    pdf.extend_from_slice(b" /Contents 4 0 R /Resources << /Font << /F1 5 0 R >> >> >>\nendobj\n");

    let off4 = pdf.len();
    pdf.extend_from_slice(format!("4 0 obj\n<< /Length {} >>\nstream\n", content.len()).as_bytes());
    pdf.extend_from_slice(content);
    pdf.extend_from_slice(b"\nendstream\nendobj\n");

    let off5 = pdf.len();
    pdf.extend_from_slice(
        b"5 0 obj\n<< /Type /Font /Subtype /Type1 /BaseFont /Helvetica /Encoding /WinAnsiEncoding >>\nendobj\n",
    );

    let xref_pos = pdf.len();
    let offsets = [0usize, off1, off2, off3, off4, off5];
    pdf.extend_from_slice(format!("xref\n0 {}\n", offsets.len()).as_bytes());
    pdf.extend_from_slice(format!("{:010} 65535 f\r\n", 0).as_bytes());
    for &off in &offsets[1..] {
        pdf.extend_from_slice(format!("{:010} 00000 n\r\n", off).as_bytes());
    }
    pdf.extend_from_slice(
        format!(
            "trailer\n<< /Size {} /Root 1 0 R >>\nstartxref\n{}\n%%EOF\n",
            offsets.len(),
            xref_pos
        )
        .as_bytes(),
    );
    pdf
}

/// A subscript inside a rotated run sits off the run's writing axis by the
/// subscript drop, not by a line height. It must stay attached to the formula
/// it belongs to: `N`, a smaller `2`, and `O` drawn as three runs of one
/// rotated label are one chemical formula, not three fragments separated by
/// unrelated page content.
fn rotated_formula_with_subscript_pdf() -> Vec<u8> {
    let mut content = Vec::new();
    // A 90-degree label "N2O" whose middle glyph is dropped below the baseline,
    // the shape a rotated chart axis uses. Under a 90-degree matrix the drop is
    // a displacement along -x, i.e. PERPENDICULAR to the +y writing axis — the
    // exact quantity the continuation test measures.
    //
    // One BT/ET and one Tf size throughout, deliberately: `ET` flushes the run
    // buffer and so does a Tf size change, either of which would leave the
    // buffer empty at the next Tm and make the continuation test unreachable —
    // the assertion below would then hold no matter what that test decided.
    content.extend_from_slice(b"BT /F1 18 Tf\n");
    content.extend_from_slice(b"0 1 -1 0 246 244 Tm (N) Tj\n");
    content.extend_from_slice(b"0 1 -1 0 242 258 Tm (2) Tj\n");
    content.extend_from_slice(b"0 1 -1 0 246 266 Tm (O) Tj\n");
    content.extend_from_slice(b"ET");
    build_minimal_pdf_raw(&content, b"/Type /Page /Parent 2 0 R /MediaBox [0 0 612 792]")
}

/// A page with no resolvable `/MediaBox` anywhere in its tree skips the
/// off-page span filter, so a run positioned at a huge coordinate reaches line
/// grouping. Quantizing such an offset to 1/100 pt saturates the i64 cast;
/// the `+-1` widening on the saturated value must not overflow (a panic in
/// debug, an inverted `BTreeMap::range` panic in release).
fn huge_offset_rotated_runs_pdf() -> Vec<u8> {
    let mut content = Vec::new();
    content.extend_from_slice(b"BT /F1 10 Tf\n");
    // A normal quarter-turn run first, so the offset index is non-empty when
    // the saturated offsets are looked up.
    content.extend_from_slice(b"0 1 -1 0 200 150 Tm (Alpha) Tj\n");
    // Real syntax, not integer: the lexer rejects an i64-overflowing integer
    // literal but passes an in-f32-range real through unclamped.
    content.extend_from_slice(b"0 1 -1 0 100000000000000000000.0 150 Tm (Far) Tj\n");
    content.extend_from_slice(b"0 1 -1 0 -100000000000000000000.0 150 Tm (Near) Tj\n");
    content.extend_from_slice(b"ET");
    build_minimal_pdf_raw(&content, b"/Type /Page /Parent 2 0 R")
}

/// Garbage coordinates in a crafted PDF must degrade to extra lines, never
/// panic the library.
#[test]
fn huge_rotated_offsets_do_not_panic_line_grouping() {
    let doc = PdfDocument::from_bytes(huge_offset_rotated_runs_pdf()).expect("parse fixture");
    let lines = doc.extract_text_lines(0).expect("extract lines");
    assert!(
        lines.iter().any(|l| l.text.contains("Alpha")),
        "on-page rotated run went missing: {:?}",
        lines.iter().map(|l| l.text.trim()).collect::<Vec<_>>()
    );
}

/// On a `/Rotate` page span bboxes are rect-mapped into the displayed frame
/// while `rotation_degrees` keeps describing the pre-display one, so the
/// quarter-turn offset would be read off the wrong axis; such pages keep one
/// line per run. Two parallel rotated lines drawn at the same y land on the
/// same displayed `bbox.x`, so grouping them by that offset would fuse them —
/// this pins the fallback.
fn parallel_rotated_lines_on_rotated_page_pdf() -> Vec<u8> {
    let mut content = Vec::new();
    content.extend_from_slice(b"BT /F1 10 Tf\n");
    content.extend_from_slice(b"0 1 -1 0 200 150 Tm (Alpha) Tj\n");
    content.extend_from_slice(b"0 1 -1 0 300 150 Tm (Bravo) Tj\n");
    content.extend_from_slice(b"ET");
    build_minimal_pdf_raw(
        &content,
        b"/Type /Page /Parent 2 0 R /MediaBox [0 0 612 792] /Rotate 90",
    )
}

#[test]
fn test_rotate_page_keeps_one_line_per_rotated_run() {
    let doc = PdfDocument::from_bytes(parallel_rotated_lines_on_rotated_page_pdf()).expect("parse fixture");
    let lines = doc.extract_text_lines(0).expect("extract lines");

    assert!(
        doc.extract_chars(0)
            .expect("extract chars")
            .iter()
            .any(|c| (c.rotation_degrees - 90.0).abs() < 0.5),
        "fixture produced no rotated glyphs"
    );

    for l in &lines {
        assert!(
            !(l.text.contains("Alpha") && l.text.contains("Bravo")),
            "parallel rotated lines fused on a /Rotate page: {:?}",
            l.text
        );
    }
    assert_eq!(
        lines.len(),
        2,
        "expected one line per run, got {:?}",
        lines.iter().map(|l| l.text.trim()).collect::<Vec<_>>()
    );
}

/// A baseline drop inside a rotated run is a sub-glyph perpendicular offset, not
/// a line break: the formula must not be split apart by the continuation test.
#[test]
fn rotated_subscript_formula_stays_contiguous() {
    let doc = PdfDocument::from_bytes(rotated_formula_with_subscript_pdf()).expect("parse fixture");
    let text = doc.extract_text(0).expect("extract text");
    let flat: String = text.split_whitespace().collect::<Vec<_>>().join("");
    assert!(flat.contains("N2O"), "rotated subscripted formula came apart: {text:?}");
}

/// A 90-degree line whose subscript run is drawn LAST in the content stream
/// but sits mid-line geometrically: "H" then "0" then "2", with the "2"
/// dropped slightly off the shared baseline offset the way a chart axis
/// label draws H2O. Arrival order would read "H 0) 2".
fn rotated_line_with_late_subscript_pdf() -> Vec<u8> {
    let mut content = Vec::new();
    content.extend_from_slice(b"BT /F1 10 Tf\n");
    for (x, y, word) in [(200, 150, "H"), (200, 175, "0"), (197, 162, "2")] {
        content.extend_from_slice(format!("0 1 -1 0 {x} {y} Tm ({word}) Tj\n").as_bytes());
    }
    content.extend_from_slice(b"ET");
    build_minimal_pdf_raw(&content, b"/Type /Page /Parent 2 0 R /MediaBox [0 0 612 792]")
}

#[test]
fn test_merged_rotated_line_reads_in_writing_axis_order() {
    let doc = PdfDocument::from_bytes(rotated_line_with_late_subscript_pdf()).expect("parse fixture");
    let lines = doc.extract_text_lines(0).expect("extract lines");
    assert_eq!(
        lines.len(),
        1,
        "one visual line came back as {} lines: {:?}",
        lines.len(),
        lines.iter().map(|l| l.text.trim()).collect::<Vec<_>>()
    );
    let text = &lines[0].text;
    let (h, two, zero) = (
        text.find('H').expect("H in line"),
        text.find('2').expect("2 in line"),
        text.find('0').expect("0 in line"),
    );
    assert!(
        h < two && two < zero,
        "members must read along the writing axis, not in drawing order: {text:?}"
    );
}

/// Both words of the first run advance along y from one shared span origin.
/// The run "Alpha Bravo" ends about 20 pt before "Charlie" starts, a word gap
/// well inside the 30 pt column threshold, so the three words are one line.
#[test]
fn test_rotated_multiword_run_is_not_split_from_its_continuation() {
    let content = b"BT /F1 10 Tf\n0 1 -1 0 200 200 Tm (Alpha Bravo) Tj\n0 1 -1 0 200 270 Tm (Charlie) Tj\nET";
    let pdf = build_minimal_pdf_raw(content, b"/Type /Page /Parent 2 0 R /MediaBox [0 0 612 792]");
    let doc = PdfDocument::from_bytes(pdf).expect("parse fixture");

    assert_eq!(doc.extract_words(0).expect("extract words").len(), 3);
    let lines = doc.extract_text_lines(0).expect("extract lines");
    assert_eq!(
        lines.len(),
        1,
        "one visual line came back as {} lines: {:?}",
        lines.len(),
        lines.iter().map(|l| l.text.trim()).collect::<Vec<_>>()
    );
}
