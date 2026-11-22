//! ISO 32000-1:2008 §12.5.5 — an annotation's appearance stream is fitted to
//! its `/Rect`.
//!
//! The appearance is a form XObject with its own coordinate system. The spec's
//! algorithm maps the four corners of `/BBox` through `/Matrix`, takes the
//! smallest upright rectangle enclosing them, and computes the matrix that
//! puts that rectangle onto `/Rect`. Painting the stream at its own scale
//! instead — the renderer only translated to the rectangle's lower-left corner
//! — made a stamp whose `/BBox` was `[0 0 512 543]` cover a fifth of a
//! 612x792 page where its `/Rect` asked for 93x98 pt.

use xberg_native_pdf::document::PdfDocument;
use xberg_native_pdf::rendering::{RenderOptions, render_page};

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

/// A 100x100 page carrying one stamp whose `/Rect` is 20x20 at the origin and
/// whose appearance fills a `/BBox` five times that size. Optionally rotate
/// the appearance 90 degrees about its own origin, which the fit has to absorb.
fn stamp_pdf(matrix: &str) -> Vec<u8> {
    let ap = stream_obj(
        &format!("/Type /XObject /Subtype /Form /BBox [0 0 100 100] {matrix}"),
        b"0 0 0 rg 0 0 100 100 re f",
    );
    let objects = vec![
        obj("<< /Type /Catalog /Pages 2 0 R >>"),
        obj("<< /Type /Pages /Kids [3 0 R] /Count 1 >>"),
        obj("<< /Type /Page /Parent 2 0 R /MediaBox [0 0 100 100] \
               /Contents 4 0 R /Annots [5 0 R] >>"),
        stream_obj("", b""),
        obj("<< /Type /Annot /Subtype /Stamp /Rect [0 0 20 20] /F 4 /AP << /N 6 0 R >> >>"),
        ap,
    ];
    build_pdf(&objects)
}

/// Fraction of the page carrying ink, and the bounding box of that ink as
/// fractions of the page in each axis. Fractions rather than pixels so the
/// assertions do not depend on the renderer's default resolution.
fn ink(pdf: &[u8]) -> (f64, (f64, f64, f64, f64)) {
    let doc = PdfDocument::from_bytes(pdf.to_vec()).expect("synthetic PDF parses");
    let img = render_page(&doc, 0, &RenderOptions::default()).expect("page renders");
    let decoded = image::load_from_memory(&img.data).expect("PNG decodes").to_rgba8();
    let (w, h) = decoded.dimensions();
    let (mut n, mut x0, mut y0, mut x1, mut y1) = (0u64, u32::MAX, u32::MAX, 0u32, 0u32);
    for (x, y, p) in decoded.enumerate_pixels() {
        let lum = (p[0] as u32 + p[1] as u32 + p[2] as u32) / 3;
        if p[3] > 0 && lum < 250 {
            n += 1;
            x0 = x0.min(x);
            y0 = y0.min(y);
            x1 = x1.max(x);
            y1 = y1.max(y);
        }
    }
    assert!(n > 0, "the appearance drew nothing at all");
    (
        n as f64 / (w as f64 * h as f64),
        (
            x0 as f64 / w as f64,
            y0 as f64 / h as f64,
            (x1 + 1) as f64 / w as f64,
            (y1 + 1) as f64 / h as f64,
        ),
    )
}

#[test]
fn appearance_is_scaled_into_the_annotation_rect() {
    let (frac, (x0, y0, x1, y1)) = ink(&stamp_pdf(""));

    // /Rect is 20x20 on a 100x100 page: 4% of it, not 100%.
    assert!(
        (frac - 0.04).abs() < 0.01,
        "the appearance must cover /Rect (4% of the page), not its own /BBox; got {frac:.4}"
    );
    // Lower-left in PDF space is bottom-left in the image: x 0..0.2, y 0.8..1.0.
    assert!(x1 < 0.22, "ink must not extend past /Rect on x; right edge at {x1:.3}");
    assert!(y0 > 0.78, "ink must sit at the bottom of the page; top edge at {y0:.3}");
    assert!(
        x0 < 0.02 && y1 > 0.98,
        "ink must reach the corner; got ({x0:.3},{y1:.3})"
    );
}

#[test]
fn test_rotated_appearance_matrix_is_absorbed_by_the_fit() {
    // /Matrix rotates the BBox 90 degrees about the origin, putting the
    // transformed appearance box at x -100..0. The fit must map that box onto
    // /Rect, so the ink lands in the rectangle either way.
    let (frac, (x0, y0, x1, y1)) = ink(&stamp_pdf("/Matrix [0 1 -1 0 0 0]"));

    assert!(
        (frac - 0.04).abs() < 0.01,
        "a rotated appearance must still be fitted to /Rect; got {frac:.4}"
    );
    assert!(
        x1 < 0.22 && y0 > 0.78,
        "rotated ink escaped /Rect: x1={x1:.3} y0={y0:.3}"
    );
    assert!(
        x0 < 0.02 && y1 > 0.98,
        "rotated ink misses the corner: ({x0:.3},{y1:.3})"
    );
}
