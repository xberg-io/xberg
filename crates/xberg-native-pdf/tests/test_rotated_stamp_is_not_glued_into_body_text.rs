//! A run at 90 degrees to the body is its own region, not part of a body line.
//!
//! The XY-cut leaf sort bands spans by baseline (3 pt) and orders within a band
//! by x. Both keys assume a shared writing axis. ISO 32000-1:2008 §9.4.4: "Both
//! the glyph's shape and its displacement (horizontal or vertical) shall be
//! interpreted in text space" — a run whose text matrix differs by 90 degrees
//! advances along a different page axis, so its extent is not comparable to a
//! body run's along page-x.
//!
//! arXiv preprints carry a rotated sidebar stamp. Its baseline lands inside a
//! body line's band, and because its origin sits near the page edge it sorted
//! to the FRONT of that band on x — emitted inside the sentence. No separator
//! followed, because the gap test subtracted the stamp's along-axis advance
//! from a horizontal x: `72 - (32 + 343.30) = -303.30` read as "no gap", giving
//! `...2025financial`.
//!
//! §14.8.2.3.1 makes a marginal sideways stamp its own region rather than a
//! member of a body row.
//!
//! Hand-built synthetic PDF; no third-party fixture.

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

/// Two horizontal body lines plus a 90-degree stamp whose baseline falls inside
/// the second line's 3 pt band, at a much smaller x — the measured shape.
fn page_with_rotated_stamp() -> Vec<u8> {
    let content = "\
BT /F1 10 Tf 1 0 0 1 72 235.69 Tm (healthcare systems to) Tj ET\n\
BT /F1 10 Tf 1 0 0 1 72 224.78 Tm (financial networks) Tj ET\n\
BT /F1 20 Tf 0 1 -1 0 32 224.35 Tm (STAMPTEXT) Tj ET\n";
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

fn extract(pdf: &[u8]) -> String {
    let doc = PdfDocument::from_bytes(pdf.to_vec()).expect("synthetic PDF parses");
    doc.extract_text(0).expect("text")
}

/// The stamp must not appear glued to a body word.
#[test]
fn test_stamp_is_not_fused_into_a_body_word() {
    let text = extract(&page_with_rotated_stamp());
    assert!(
        !text.contains("STAMPTEXTfinancial") && !text.contains("toSTAMPTEXT"),
        "the rotated stamp must not fuse with a body word; got {text:?}"
    );
}

/// And it must not be inserted between the two body lines' words, which is the
/// ordering half of the same defect.
#[test]
fn test_body_sentence_stays_contiguous() {
    let text = extract(&page_with_rotated_stamp());
    let compact: String = text.split_whitespace().collect::<Vec<_>>().join(" ");
    assert!(
        compact.contains("healthcare systems to") && compact.contains("financial networks"),
        "both body lines must survive intact; got {compact:?}"
    );
    assert!(
        !compact.contains("to STAMPTEXT financial"),
        "the stamp must not be ordered into the middle of the sentence; got {compact:?}"
    );
}
