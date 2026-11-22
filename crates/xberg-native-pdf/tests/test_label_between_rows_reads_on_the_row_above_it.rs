//! A label drawn between two rows is read above the row it sits above.
//!
//! A dense timetable sets its time labels at a 5.31pt pitch and its stage
//! labels larger, beside them, with baselines that fall *between* the time
//! rows. Reading order among spans that share no row is settled by geometry
//! alone: a run drawn above another must be emitted before it.
//!
//! Quantizing each baseline onto a fixed 3pt grid does not preserve that. The
//! grid is sized for 10-12pt body text; at a 5.31pt pitch one band spans two
//! rows, so whether a label bands with the row above or the row below depends
//! on where its baseline happens to fall against the grid's phase rather than
//! on where it is drawn. Here every label lands in the band of the row *below*
//! it and is emitted after that row, inverting it against a span it is
//! plainly drawn above.
//!
//! ISO 32000-1:2008 §9.4.4 computes the glyph displacement along the writing
//! axis and sets the component for the other axis to 0, so a horizontal run's
//! vertical position is exactly where it was placed — the geometry is not
//! approximate, and the ordering must not depend on grid phase.
//!
//! The assertion is the ordering invariant, not a particular transcription:
//! the reference engines read this layout as separate blocks and do not agree
//! among themselves on how to interleave a label with a time row, so what is
//! pinned here is only that a span drawn above another is not emitted after it.

use xberg_native_pdf::PdfDocument;

const TOP: f32 = 787.05;
const PITCH: f32 = 765.35 / 144.0; // 5.31493pt — below 2x the 3pt row band
const TIME_X: f32 = 20.69;
/// 60pt right of the times. A second text column this far over is what makes
/// the page read as multi-column; without it the page takes a different
/// ordering branch entirely and the defect does not arise.
const NAME_X: f32 = 80.69;
/// Each label sits this far above the row below it — close enough to land in
/// that row's band, far enough that its baseline is unambiguously higher.
const NAME_LIFT: f32 = 2.03;
const BAND: f32 = 3.0;

fn row_y(k: usize) -> f32 {
    ((TOP - k as f32 * PITCH) * 100.0).round() / 100.0
}
fn band_key(y: f32) -> i32 {
    (y / BAND + 0.5).floor() as i32
}

/// Rows where a label placed `NAME_LIFT` above the row lands in that row's
/// band while remaining strictly above it. The collision depends on the
/// absolute phase against the grid, not on the offset alone, so the rows are
/// selected by computing it rather than assumed.
fn colliding_rows(rows: usize) -> Vec<usize> {
    let mut out = Vec::new();
    for k in 2..rows {
        let ny = ((row_y(k) + NAME_LIFT) * 100.0).round() / 100.0;
        if band_key(ny) == band_key(row_y(k))
            && band_key(ny) != band_key(row_y(k - 1))
            && row_y(k - 1) > ny
            && ny > row_y(k)
        {
            out.push(k);
            if out.len() == 8 {
                break;
            }
        }
    }
    out
}

fn timetable(rows: usize) -> (Vec<u8>, Vec<usize>) {
    let names = colliding_rows(rows);
    let mut ops = String::from("BT /F2 10 Tf\n1 0 0 1 100.65 806.29 Tm (Friday) Tj\nET\n");
    // The time labels must not be bold: same-x, same-size bold spans one line
    // apart are collapsed into one atomic block that cannot be reordered
    // internally, which hides the defect.
    ops.push_str("BT /F1 5 Tf\n");
    for k in 0..rows {
        ops.push_str(&format!(
            "1 0 0 1 {TIME_X:.2} {:.2} Tm ({:02}:{:02}) Tj\n",
            row_y(k),
            11 + (k * 5) / 60,
            (k * 5) % 60
        ));
    }
    ops.push_str("ET\nBT /F1 8 Tf\n");
    for (i, &k) in names.iter().enumerate() {
        ops.push_str(&format!(
            "1 0 0 1 {NAME_X:.2} {:.2} Tm (Band{i}) Tj\n",
            ((row_y(k) + NAME_LIFT) * 100.0).round() / 100.0
        ));
    }
    ops.push_str("ET\n");

    let content = ops.into_bytes();
    let objects: Vec<Vec<u8>> = vec![
        b"<< /Type /Catalog /Pages 2 0 R >>".to_vec(),
        b"<< /Type /Pages /Kids [3 0 R] /Count 1 >>".to_vec(),
        b"<< /Type /Page /Parent 2 0 R /MediaBox [0 0 595.28 841.89] \
           /Resources << /Font << /F1 5 0 R /F2 6 0 R >> >> /Contents 4 0 R >>"
            .to_vec(),
        [
            format!("<< /Length {} >>\nstream\n", content.len()).into_bytes(),
            content,
            b"\nendstream".to_vec(),
        ]
        .concat(),
        b"<< /Type /Font /Subtype /Type1 /BaseFont /Helvetica /Encoding /WinAnsiEncoding >>".to_vec(),
        b"<< /Type /Font /Subtype /Type1 /BaseFont /Helvetica-Bold /Encoding /WinAnsiEncoding >>".to_vec(),
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
    (pdf, names)
}

#[test]
#[ignore = "needs the row-snapped XY-cut leaf order in pipeline/reading_order/xycut.rs"]
fn test_label_is_not_emitted_after_a_row_it_is_drawn_above() {
    let (pdf, names) = timetable(40);
    assert!(names.len() >= 4, "fixture must place several labels; got {names:?}");
    let doc = PdfDocument::from_bytes(pdf).expect("open");
    let text = doc.extract_text(0).expect("text");

    for (i, &k) in names.iter().enumerate() {
        let label = format!("Band{i}");
        let row_below = format!("{:02}:{:02}", 11 + (k * 5) / 60, (k * 5) % 60);
        let l = text
            .find(&label)
            .unwrap_or_else(|| panic!("{label} missing from:\n{text}"));
        let r = text
            .find(&row_below)
            .unwrap_or_else(|| panic!("{row_below} missing from:\n{text}"));
        assert!(
            l < r,
            "{label} is drawn above {row_below} ({:.2} vs {:.2}) and must be read \
             before it; got {label}@{l} {row_below}@{r} in:\n{text}",
            (row_y(k) + NAME_LIFT),
            row_y(k)
        );
    }
}
