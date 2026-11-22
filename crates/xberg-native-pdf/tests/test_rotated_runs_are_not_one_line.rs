//! Two runs on different writing axes are not the same line.
//!
//! ISO 32000-1:2008 §9.4.4 puts a glyph's displacement along the writing
//! direction that the text matrix establishes, so two runs whose matrices
//! differ in rotation are on different axes and cannot be one line.
//!
//! The shared text assembly decided the separator from an axis-aligned gap:
//!
//! ```text
//! let prev_end_x = prev.bbox.x + prev.bbox.width;
//! let gap = span.bbox.x - prev_end_x;
//! ```
//!
//! For a run on a diagonal baseline, `bbox.width` is the width of a box drawn
//! around that diagonal, not an advance along it. On a perspective diagram
//! whose labels sit at 20.3, 25.3, 30.4 and 35.5 degrees the boxes overlap,
//! the gap comes out negative, and consecutive labels concatenated with no
//! separator at all — `Opt_Decoder` and `Opt_Heads` became
//! `Opt_DecoderOpt_Heads`.
//!
//! `pipeline/converters/mod.rs` already guarded this for the converter path;
//! the shared assembly behind `extract_text` never consulted
//! `rotation_degrees`.

use xberg_native_pdf::PdfDocument;

/// One page with two short runs drawn at clearly different rotations, placed
/// so their axis-aligned boxes overlap the way a perspective diagram's labels
/// do.
fn two_rotated_runs() -> Vec<u8> {
    // cos/sin for ~20 and ~35 degrees.
    let content = b"BT /F1 10 Tf\n\
                    0.940 0.342 -0.342 0.940 200 300 Tm (Alpha) Tj\n\
                    0.819 0.574 -0.574 0.819 206 292 Tm (Beta) Tj\n\
                    ET\n"
        .to_vec();

    let mut pdf = Vec::new();
    let mut off = [0usize; 7];
    macro_rules! push {
        ($s:expr) => {
            pdf.extend_from_slice($s.as_bytes())
        };
    }
    push!("%PDF-1.7\n");
    off[1] = pdf.len();
    push!("1 0 obj\n<< /Type /Catalog /Pages 2 0 R >>\nendobj\n");
    off[2] = pdf.len();
    push!("2 0 obj\n<< /Type /Pages /Kids [3 0 R] /Count 1 >>\nendobj\n");
    off[3] = pdf.len();
    push!(
        "3 0 obj\n<< /Type /Page /Parent 2 0 R /MediaBox [0 0 612 792] \
         /Contents 4 0 R /Resources << /Font << /F1 5 0 R >> >> >>\nendobj\n"
    );
    off[4] = pdf.len();
    push!(format!("4 0 obj\n<< /Length {} >>\nstream\n", content.len()));
    pdf.extend_from_slice(&content);
    push!("endstream\nendobj\n");
    off[5] = pdf.len();
    push!(
        "5 0 obj\n<< /Type /Font /Subtype /Type1 /BaseFont /Helvetica \
         /Encoding /WinAnsiEncoding >>\nendobj\n"
    );

    let xref = pdf.len();
    push!("xref\n0 6\n0000000000 65535 f \r\n");
    for offset in &off[1..=5] {
        push!(format!("{:010} 00000 n \r\n", *offset));
    }
    push!(format!(
        "trailer\n<< /Size 6 /Root 1 0 R >>\nstartxref\n{xref}\n%%EOF\n"
    ));
    pdf
}

#[test]
fn runs_at_different_rotations_do_not_concatenate() {
    let doc = PdfDocument::from_bytes(two_rotated_runs()).expect("synthetic PDF parses");
    let text = doc.extract_text(0).expect("extract page 0");

    assert!(
        !text.contains("AlphaBeta") && !text.contains("BetaAlpha"),
        "two runs at different rotations were glued into one token: {text:?}"
    );
    assert!(
        text.contains("Alpha") && text.contains("Beta"),
        "both runs should still be present: {text:?}"
    );
}
