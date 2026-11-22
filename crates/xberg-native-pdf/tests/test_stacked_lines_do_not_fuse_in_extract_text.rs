//! Two labels stacked one line apart must not concatenate into one word.
//!
//! `same_line_threshold` deliberately allows 1.2x the SMALLER of the two font
//! sizes so ordinary leading does not produce false line breaks. For two spans
//! of the SAME size that admits everything below 1.2 em -- a plain
//! single-spaced advance included. A wrapped body line is still separated
//! further down, by the large negative `delta_x` it gets from restarting at
//! the left margin; a stacked table-header label is not, because it starts at
//! or to the right of the line above, leaving a horizontal gap near zero.
//!
//! Nothing on that arm then separated them and the two lines fused. ISO
//! 32000-1:2008 9.4.3: the show operators paint the glyphs they are given, so
//! two words drawn on different baselines are two words.
//!
//! The counter-case matters as much as the case: a superscript sits about
//! 0.3 em above its baseline and must stay attached to the token it decorates.
//! Hand-built synthetic PDFs; no third-party fixture.

use xberg_native_pdf::document::PdfDocument;

fn build_pdf(objects: &[Vec<u8>]) -> Vec<u8> {
    let mut out: Vec<u8> = Vec::new();
    out.extend_from_slice(b"%PDF-1.4\n");
    let mut offsets = Vec::new();
    for (i, body) in objects.iter().enumerate() {
        offsets.push(out.len());
        out.extend_from_slice(format!("{} 0 obj\n", i + 1).as_bytes());
        out.extend_from_slice(body);
        out.extend_from_slice(b"\nendobj\n");
    }
    let xref_pos = out.len();
    out.extend_from_slice(format!("xref\n0 {}\n", objects.len() + 1).as_bytes());
    out.extend_from_slice(b"0000000000 65535 f \n");
    for off in &offsets {
        out.extend_from_slice(format!("{off:010} 00000 n \n").as_bytes());
    }
    out.extend_from_slice(
        format!(
            "trailer\n<< /Size {} /Root 1 0 R >>\nstartxref\n{}\n%%EOF\n",
            objects.len() + 1,
            xref_pos
        )
        .as_bytes(),
    );
    out
}

fn stream_obj(dict: &str, data: &[u8]) -> Vec<u8> {
    let mut v = format!("<< {} /Length {} >>\nstream\n", dict, data.len()).into_bytes();
    v.extend_from_slice(data);
    v.extend_from_slice(b"\nendstream");
    v
}

/// One page drawing two text runs at the given sizes and positions.
fn two_run_page((t1, x1, y1, s1): (&str, f32, f32, f32), (t2, x2, y2, s2): (&str, f32, f32, f32)) -> Vec<u8> {
    let content = format!(
        "BT /F1 {s1} Tf {x1} {y1} Td ({t1}) Tj ET\n\
         BT /F1 {s2} Tf {x2} {y2} Td ({t2}) Tj ET\n"
    );
    build_pdf(&[
        b"<< /Type /Catalog /Pages 2 0 R >>".to_vec(),
        b"<< /Type /Pages /Kids [3 0 R] /Count 1 >>".to_vec(),
        b"<< /Type /Page /Parent 2 0 R /MediaBox [0 0 612 792] /Contents 4 0 R \
           /Resources << /Font << /F1 5 0 R >> >> >>"
            .to_vec(),
        stream_obj("", content.as_bytes()),
        b"<< /Type /Font /Subtype /Type1 /BaseFont /Helvetica >>".to_vec(),
    ])
}

fn text_of(pdf: &[u8]) -> String {
    let doc = PdfDocument::from_bytes(pdf.to_vec()).expect("synthetic PDF parses");
    doc.extract_text(0).expect("page extracts")
}

/// The measured geometry from the page that exposed this: two 7.24 pt labels
/// 7.97 pt apart, the upper one starting 0.02 pt after the lower one ends.
#[test]
fn stacked_labels_a_line_apart_do_not_concatenate() {
    let pdf = two_run_page(("Latency", 417.34, 650.76, 7.24), ("Efficiency", 442.31, 658.73, 7.24));
    let text = text_of(&pdf);
    assert!(
        !text.contains("LatencyEfficiency") && !text.contains("EfficiencyLatency"),
        "labels on different baselines must not fuse into one token; got {text:?}"
    );
    assert!(
        text.contains("Latency") && text.contains("Efficiency"),
        "both labels must still be present; got {text:?}"
    );
}

/// The counter-case: a 6 pt superscript 3 pt above a 10 pt baseline is a
/// decoration of the token before it, not a new line, and must stay attached.
/// 3 pt is 0.30 em of the 10 pt run — below the 0.8 em line-advance bound.
#[test]
fn test_superscript_stays_attached_to_its_word() {
    let pdf = two_run_page(("x", 100.0, 700.0, 10.0), ("2", 105.6, 703.0, 6.0));
    let text = text_of(&pdf);
    let compact: String = text.chars().filter(|c| !c.is_whitespace()).collect();
    assert!(
        compact.contains("x2"),
        "a superscript must not be split onto its own line; got {text:?}"
    );
}

/// A footnote marker raised over a same-size comma. Measured from the page
/// that exposed the over-split: the comma and the marker are both 7.04 pt and
/// the marker sits 5.78 pt higher — 0.82 em, which an earlier 0.8 em bound
/// treated as a line advance. It is a decoration of the line, not a new one.
#[test]
fn test_footnote_marker_over_a_small_comma_stays_on_its_line() {
    let pdf = two_run_page((",", 318.05, 550.26, 7.04), ("*", 320.20, 556.04, 7.04));
    let text = text_of(&pdf);
    assert!(
        !text.contains(",\n*") && !text.trim_end().ends_with(','),
        "a marker raised 0.82 em over its comma decorates that line and must \
         not be split onto its own; got {text:?}"
    );
}

/// Two runs genuinely on one baseline, adjacent, are one word and must stay
/// joined — the fix must not separate on horizontal adjacency alone.
#[test]
fn test_kerned_run_on_one_baseline_stays_one_word() {
    let pdf = two_run_page(("Effi", 100.0, 700.0, 7.24), ("ciency", 113.4, 700.0, 7.24));
    let text = text_of(&pdf);
    let compact: String = text.chars().filter(|c| !c.is_whitespace()).collect();
    assert!(
        compact.contains("Efficiency"),
        "a same-baseline kerned run must stay one word; got {text:?}"
    );
}
