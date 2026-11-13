//! Renderer robustness against malformed, document-controlled geometry.
//!
//! Every construct here is legal PDF syntax carrying out-of-range values.
//! The renderer must skip the construct and return a `Result` — never panic,
//! and never fabricate a plausible-but-wrong result.

use xberg_native_pdf::document::PdfDocument;
use xberg_native_pdf::rendering::{RenderOptions, render_page};

/// Assemble a PDF with a correct xref from raw object bodies.
/// `objects[i]` is the body of object i+1 (no "N 0 obj"/"endobj" wrapper).
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

fn obj(s: &str) -> Vec<u8> {
    s.as_bytes().to_vec()
}

fn stream_obj(dict: &str, data: &[u8]) -> Vec<u8> {
    let mut v = format!("<< {} /Length {} >>\nstream\n", dict, data.len()).into_bytes();
    v.extend_from_slice(data);
    v.extend_from_slice(b"\nendstream");
    v
}

/// Build a page carrying one `/AP` dictionary with the given entries, or no
/// annotation at all when `ap_entries` is `None`.
fn annotation_ap_pdf(ap_entries: Option<&str>) -> Vec<u8> {
    let red = stream_obj(
        "/Type /XObject /Subtype /Form /BBox [0 0 30 30]",
        b"1 0 0 rg 0 0 30 30 re f",
    );
    let green = stream_obj(
        "/Type /XObject /Subtype /Form /BBox [0 0 30 30]",
        b"0 1 0 rg 0 0 30 30 re f",
    );
    let annots = match ap_entries {
        Some(_) => "/Annots [5 0 R]",
        None => "",
    };
    let annot = match ap_entries {
        Some(ap) => {
            format!("<< /Type /Annot /Subtype /Square /Rect [10 10 40 40] /F 4 /AP << {ap} >> >>")
        }
        None => "<< /Type /Annot /Subtype /Square /Rect [10 10 40 40] /F 4 >>".to_string(),
    };
    let objects = vec![
        obj("<< /Type /Catalog /Pages 2 0 R >>"),
        obj("<< /Type /Pages /Kids [3 0 R] /Count 1 >>"),
        obj(&format!(
            "<< /Type /Page /Parent 2 0 R /MediaBox [0 0 50 50] /Contents 4 0 R {annots} >>"
        )),
        stream_obj("", b""),
        obj(&annot),
        red,
        green,
    ];
    build_pdf(&objects)
}

/// Render the same bytes `n` times, reparsing each time, and return the set of
/// distinct pixel buffers.
fn distinct_renders(pdf: &[u8], n: usize) -> std::collections::HashSet<Vec<u8>> {
    let mut distinct = std::collections::HashSet::new();
    for _ in 0..n {
        let doc = PdfDocument::from_bytes(pdf.to_vec()).expect("synthetic PDF parses");
        let img = render_page(&doc, 0, &RenderOptions::default()).expect("page renders");
        distinct.insert(img.data);
    }
    distinct
}

/// ISO 32000-1 §12.5.5: `/N` is the normal appearance. An `/AP` carrying only
/// `/D` and `/R` has no normal appearance, so nothing is drawn — picking an
/// arbitrary sibling stream made identical bytes render differently per run.
#[test]
fn annotation_appearance_without_normal_entry_draws_nothing() {
    let blank = distinct_renders(&annotation_ap_pdf(None), 1)
        .into_iter()
        .next()
        .expect("baseline render");

    let renders = distinct_renders(&annotation_ap_pdf(Some("/D 6 0 R /R 7 0 R")), 64);
    assert_eq!(
        renders.len(),
        1,
        "identical bytes produced {} distinct renders",
        renders.len()
    );
    assert_eq!(
        renders.into_iter().next().expect("one render"),
        blank,
        "an /AP without /N must draw no appearance"
    );
}

/// Control: the same annotation WITH `/N` draws that stream, deterministically.
/// Without this the test above would also pass if annotations stopped rendering.
#[test]
fn annotation_appearance_with_normal_entry_is_deterministic() {
    let blank = distinct_renders(&annotation_ap_pdf(None), 1)
        .into_iter()
        .next()
        .expect("baseline render");

    let renders = distinct_renders(&annotation_ap_pdf(Some("/N 6 0 R /R 7 0 R")), 64);
    assert_eq!(
        renders.len(),
        1,
        "identical bytes produced {} distinct renders",
        renders.len()
    );
    assert_ne!(
        renders.into_iter().next().expect("one render"),
        blank,
        "the /N appearance stream must actually be drawn"
    );
}

#[test]
fn font_ink_proof_excludes_overlapping_annotation_appearances() {
    let font = "/Font << /F1 << /Type /Font /Subtype /Type1 /BaseFont /Helvetica >> >>";
    let objects = vec![
        obj("<< /Type /Catalog /Pages 2 0 R >>"),
        obj("<< /Type /Pages /Kids [3 0 R] /Count 1 >>"),
        obj(&format!(
            "<< /Type /Page /Parent 2 0 R /MediaBox [0 0 60 60] /Contents 4 0 R /Resources << {font} >> /Annots [5 0 R] >>"
        )),
        stream_obj("", b"1 g BT /F1 18 Tf 10 20 Td (ID) Tj ET"),
        obj("<< /Type /Annot /Subtype /Square /Rect [10 20 40 50] /F 4 /AP << /N 6 0 R >> >>"),
        stream_obj(
            &format!("/Type /XObject /Subtype /Form /BBox [0 0 30 30] /Resources << {font} >>"),
            b"BT /F1 18 Tf 0 0 Td (ID) Tj ET",
        ),
    ];
    let doc = PdfDocument::from_bytes(build_pdf(&objects)).unwrap();
    let visible = render_page(&doc, 0, &RenderOptions::with_dpi(72).as_raw()).unwrap();
    assert!(visible.data.as_chunks::<4>().0.iter().any(|pixel| pixel[0] < 128));
    let proof = xberg_native_pdf::rendering::render_font_ink_rasters(&doc, 0, 72, 1 << 26, "F1", || false)
        .unwrap()
        .unwrap();
    assert_eq!(proof.drawn, proof.without_glyphs);
    assert!(proof.drawn.iter().all(|pixel| *pixel > 240));
}
