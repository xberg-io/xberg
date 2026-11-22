//! A shading `/BBox` that cannot be rasterized is resolved in the direction
//! that paints *less*, not more.
//!
//! Table 78 makes a shading's `/BBox` a clip in the shading's target space.
//! When its device bounds exceed what `f32` can represent, the renderer cannot
//! build a mask for it — and the two ways that happens are opposites:
//!
//! * bounds that are merely enormous but still **reach the page** restrict
//!   nothing visible, so discarding the box is harmless;
//! * bounds placed wholly **off-page** exclude everything, so discarding the
//!   box paints the entire shading the file asked to be hidden.
//!
//! §8.5.4: content outside the clipping path shall not be painted. Annex C.1
//! licenses *having* an arithmetic limit; it does not license resolving past
//! one in the direction that paints more.
//!
//! Both directions are asserted, because a renderer that simply drops every
//! unrasterizable shading passes the off-page case for the wrong reason.
//!
//! The coordinates are written out in full rather than as `1e9`: ISO 32000-1
//! §7.3.3 does not permit exponential notation for a real, so a `1e9` in the
//! file never parses and the whole `/BBox` is silently ignored. That produced a
//! convincing-looking failure that was entirely the fixture's fault.

use xberg_native_pdf::PdfDocument;
use xberg_native_pdf::rendering::{RenderOptions, render_page};

/// A page whose whole area is filled with an axial shading, given `bbox` as
/// the shading's `/BBox`.
fn shading_with_bbox(bbox: &str) -> Vec<u8> {
    let content: &[u8] = b"/Pattern cs /P0 scn 0 0 200 100 re f\n";
    let objects: Vec<Vec<u8>> = vec![
        b"<< /Type /Catalog /Pages 2 0 R >>".to_vec(),
        b"<< /Type /Pages /Kids [3 0 R] /Count 1 >>".to_vec(),
        b"<< /Type /Page /Parent 2 0 R /MediaBox [0 0 200 100] /Contents 4 0 R \
           /Resources << /Pattern << /P0 5 0 R >> >> >>"
            .to_vec(),
        [
            format!("<< /Length {} >>\nstream\n", content.len()).into_bytes(),
            content.to_vec(),
            b"\nendstream".to_vec(),
        ]
        .concat(),
        format!(
            "<< /Type /Pattern /PatternType 2 /Matrix [1 0 0 1 0 0] \
             /Shading << /ShadingType 2 /ColorSpace /DeviceRGB /Coords [0 0 200 0] \
             /Extend [true true] {bbox} /Function 6 0 R >> >>"
        )
        .into_bytes(),
        b"<< /FunctionType 2 /Domain [0 1] /N 1 /C0 [1 0 0] /C1 [0 0 1] >>".to_vec(),
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

/// Fraction of the page carrying non-white ink.
fn inked(bbox: &str) -> f64 {
    let doc = PdfDocument::from_bytes(shading_with_bbox(bbox)).expect("fixture parses");
    let img = render_page(&doc, 0, &RenderOptions::with_dpi(72)).expect("page renders");
    let px = image::load_from_memory(&img.data).expect("PNG decodes").to_rgba8();
    let total = px.pixels().len() as f64;
    let ink = px
        .pixels()
        .filter(|p| !(p[0] > 245 && p[1] > 245 && p[2] > 245))
        .count() as f64;
    ink / total
}

/// The control: an ordinary `/BBox` covering the page paints the gradient.
#[test]
fn test_ordinary_bbox_paints_the_shading() {
    let f = inked("/BBox [0 0 200 100]");
    assert!(
        f > 0.9,
        "the shading did not paint under an ordinary /BBox (inked {f:.3}); the \
         other cases below would then pass for the wrong reason"
    );
}

/// A `/BBox` placed wholly off-page excludes everything, so nothing paints —
/// even though its bounds cannot be rasterized.
#[test]
fn test_bbox_wholly_off_page_hides_the_shading() {
    let f = inked("/BBox [1000000000 1000000000 2000000000 2000000000]");
    assert!(
        f < 0.02,
        "a /BBox lying wholly off-page painted {f:.3} of the page. §8.5.4: \
         content outside the clipping path shall not be painted — an \
         unrasterizable box must be resolved toward painting less, not more."
    );
}

/// A `/BBox` that is merely enormous but still encloses the page restricts
/// nothing visible, so the shading still paints. This is the case the previous
/// behaviour was written for, and an empty mask here would wrongly erase it.
#[test]
fn test_bbox_enclosing_the_page_still_paints() {
    let f = inked("/BBox [-1000000000 -1000000000 1000000000 1000000000]");
    assert!(
        f > 0.9,
        "a /BBox enclosing the whole page erased the shading (inked {f:.3}); it \
         restricts nothing visible and must not clip anything away"
    );
}

/// Diagnostic: an ordinary, perfectly rasterizable `/BBox` that simply sits
/// off the page. If this paints, `/BBox` is not reaching the pattern path at
/// all and the unrasterizable case above is a symptom rather than the defect.
#[test]
fn test_small_off_page_bbox_hides_the_shading() {
    let f = inked("/BBox [500 500 600 600]");
    assert!(
        f < 0.02,
        "an ordinary off-page /BBox painted {f:.3} of the page — Table 78 makes \
         /BBox a clip in the shading's target space"
    );
}
