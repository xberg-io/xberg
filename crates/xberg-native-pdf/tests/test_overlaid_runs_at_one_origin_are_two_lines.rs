//! Two runs drawn over the same horizontal extent from the same origin are
//! two lines of text, not one kerned word.
//!
//! A chart legend that stacks its labels by drawing them at the same place
//! produces span pairs whose baselines differ by a fraction of a thousandth
//! of a point. The XY-cut leaf sort compares baselines with a row-aware
//! comparator (so that jittered OCR baselines keep their reading order), which
//! collapses that difference into one band and interleaves the two labels by
//! x. The flow assembler then sees the second label restarting within half an
//! em of the first label's ORIGIN while overlapping its ink by four em — and
//! no arm of its negative-gap chain claimed that shape, so the two labels
//! concatenated into `maximumminimum`.
//!
//! ISO 32000-1:2008 §9.4.3: the show operators paint the glyphs they are
//! given, so two runs painted over one another are two runs.
//!
//! Geometry is taken from the page that exposed this: two 9.35 pt labels whose
//! origins sit 0.90 pt apart and whose baselines differ by 8.6e-4 pt.
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

/// A page drawing each `(text, x, y)` as its own text object at `size`.
fn page_of_runs(runs: &[(&str, f32, f32)], size: f32) -> Vec<u8> {
    let mut content = String::new();
    for (t, x, y) in runs {
        content.push_str(&format!("BT /F1 {size} Tf {x} {y} Td ({t}) Tj ET\n"));
    }
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

/// The two legend labels overlap by roughly four em from origins 0.90 pt
/// apart — the measured shape of the page that exposed this.
#[test]
fn two_labels_drawn_at_one_origin_do_not_concatenate() {
    let pdf = page_of_runs(
        &[("maximum", 253.198, 285.377_3), ("minimum", 254.10146, 285.37643)],
        9.35073,
    );
    let text = text_of(&pdf);
    assert!(
        !text.contains("maximumminimum"),
        "overlaid runs must not fuse into one token; got {text:?}"
    );
    assert!(
        text.contains("maximum") && text.contains("minimum"),
        "both labels must survive; got {text:?}"
    );
}

/// A normal kerned overlap must stay one word. The widest overlap the
/// assembler treats as reliable kerning is under one em; this pair overlaps
/// by ~0.4 em, far inside the three-em bound the new arm requires.
#[test]
fn test_kerned_overlap_within_one_em_stays_one_word() {
    let pdf = page_of_runs(&[("Effi", 100.0, 700.0), ("ciency", 118.0, 700.0)], 10.0);
    let text = text_of(&pdf);
    let compact: String = text.chars().filter(|c| !c.is_whitespace()).collect();
    assert!(
        compact.contains("Efficiency"),
        "a kerned run must stay one word; got {text:?}"
    );
}

/// A fraction denominator sits about two em back from the relation sign —
/// beyond the half-em origin test — so the existing arms keep handling it and
/// the new one must decline.
#[test]
fn test_displaced_denominator_is_untouched() {
    let pdf = page_of_runs(&[("=", 149.94, 400.0), ("dt", 134.74, 394.0)], 12.0);
    let text = text_of(&pdf);
    assert!(
        text.contains("dt") && text.contains('='),
        "displayed-math runs must survive; got {text:?}"
    );
}
