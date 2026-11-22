//! An ink plate of a rotated page must face the same way the composite does.
//!
//! ISO 32000-1:2008 §7.7.3.3 Table 30: `/Rotate` is a clockwise multiple of
//! 90. Every page transform also carries the PDF y-up to raster y-down flip,
//! so each of the four matrices has a **negative determinant** — a positive
//! one is a mirror, not a turn.
//!
//! The composite renderer's 270° case was corrected to
//! `from_row(0, -s, -s, 0, …)` (determinant −s²). The separation renderer kept
//! its own copy, `from_row(0, s, -s, 0, …)` (determinant +s²), so every ink
//! plate of a `/Rotate 270` page came out mirrored while the composite of the
//! same page did not. `/Rotate -90` — legal, and equal to 270 — was worse: it
//! had fallen through to the unrotated arm and was at least legible, and
//! normalising the angle without fixing the matrix moved it to mirrored.
//!
//! The existing plate rotation test used a centred symmetric square on a
//! square page, which is invariant under every rotation *and* every
//! reflection. These fixtures are deliberately asymmetric on both axes.

use xberg_native_pdf::PdfDocument;
use xberg_native_pdf::rendering::{ImageFormat, RenderOptions, render_page, render_separations};

/// A page carrying a single filled rectangle in one corner — asymmetric on
/// both axes, so a reflection is distinguishable from a rotation.
fn corner_mark_pdf(rotate: i32) -> Vec<u8> {
    let mut buf: Vec<u8> = Vec::new();
    let mut off = vec![0usize; 6];
    let obj = |buf: &mut Vec<u8>, off: &mut Vec<usize>, id: usize, body: &str| {
        off[id] = buf.len();
        buf.extend_from_slice(format!("{id} 0 obj\n{body}\nendobj\n").as_bytes());
    };

    buf.extend_from_slice(b"%PDF-1.7\n%\xE2\xE3\xCF\xD3\n");
    obj(&mut buf, &mut off, 1, "<< /Type /Catalog /Pages 2 0 R >>");
    obj(&mut buf, &mut off, 2, "<< /Type /Pages /Kids [3 0 R] /Count 1 >>");
    obj(
        &mut buf,
        &mut off,
        3,
        &format!(
            "<< /Type /Page /Parent 2 0 R /MediaBox [0 0 400 200] /Rotate {rotate} \
             /Resources << /ColorSpace << /CS0 5 0 R >> >> /Contents 4 0 R >>"
        ),
    );
    // A mark in the PDF lower-left quadrant, painted through a Separation so
    // the plate renderer produces ink for it.
    let content = b"/CS0 cs 1 scn 20 20 80 40 re f\n";
    off[4] = buf.len();
    buf.extend_from_slice(format!("4 0 obj\n<< /Length {} >>\nstream\n", content.len()).as_bytes());
    buf.extend_from_slice(content);
    buf.extend_from_slice(b"\nendstream\nendobj\n");
    obj(
        &mut buf,
        &mut off,
        5,
        "[/Separation /Spot /DeviceGray << /FunctionType 2 /Domain [0 1] /N 1 \
         /C0 [1] /C1 [0] >>]",
    );

    let xref = buf.len();
    buf.extend_from_slice(b"xref\n0 6\n0000000000 65535 f \n");
    for offset in &off[1..=5] {
        buf.extend_from_slice(format!("{:010} 00000 n \n", offset).as_bytes());
    }
    buf.extend_from_slice(b"trailer\n<< /Size 6 /Root 1 0 R >>\nstartxref\n");
    buf.extend_from_slice(format!("{xref}\n%%EOF\n").as_bytes());
    buf
}

/// Which quadrant of the raster holds the ink, as `(left, top)`.
fn ink_quadrant(coverage: &[u8], w: usize, h: usize) -> (bool, bool) {
    let (mut best, mut at) = (0u32, (0usize, 0usize));
    for y in 0..h {
        for x in 0..w {
            let v = coverage[y * w + x] as u32;
            if v > best {
                best = v;
                at = (x, y);
            }
        }
    }
    assert!(best > 0, "no ink found on the plate at all");
    (at.0 < w / 2, at.1 < h / 2)
}

/// The composite's inked quadrant, as `(left, top)` — the reference answer.
fn composite_quadrant(rotate: i32) -> (bool, bool) {
    let doc = PdfDocument::from_bytes(corner_mark_pdf(rotate)).expect("parse");
    let mut opts = RenderOptions::default();
    opts.dpi = 72;
    opts.format = ImageFormat::RawRgba8;
    let img = render_page(&doc, 0, &opts).expect("render");
    let (w, h) = (img.width as usize, img.height as usize);
    // Darkest pixel: the mark is black-ish on white.
    let (mut best, mut at) = (u32::MAX, (0usize, 0usize));
    for y in 0..h {
        for x in 0..w {
            let i = (y * w + x) * 4;
            let lum = img.data[i] as u32 + img.data[i + 1] as u32 + img.data[i + 2] as u32;
            if lum < best {
                best = lum;
                at = (x, y);
            }
        }
    }
    (at.0 < w / 2, at.1 < h / 2)
}

/// The plate's inked quadrant for the same page.
fn plate_quadrant(rotate: i32) -> (bool, bool) {
    let doc = PdfDocument::from_bytes(corner_mark_pdf(rotate)).expect("parse");
    let plates = render_separations(&doc, 0, 72).expect("separations");
    let plate = plates
        .iter()
        .find(|p| p.data.iter().any(|&v| v > 0))
        .expect("at least one inked plate");
    ink_quadrant(&plate.data, plate.width as usize, plate.height as usize)
}

/// The defect, stated as an equivalence: the plate and the composite must
/// place the ink in the same quadrant, at every legal rotation.
#[test]
fn plate_and_composite_agree_on_orientation() {
    for rotate in [0, 90, 180, 270, -90] {
        assert_eq!(
            plate_quadrant(rotate),
            composite_quadrant(rotate),
            "plate and composite disagree at /Rotate {rotate}"
        );
    }
}

/// `/Rotate -90` and `/Rotate 270` are the same page (Table 30), so they must
/// produce the same plate.
#[test]
fn negative_rotation_matches_its_positive_equivalent() {
    assert_eq!(plate_quadrant(-90), plate_quadrant(270));
}

/// A quarter turn must actually move the ink — a test whose fixture is
/// invariant under rotation proves nothing, which is how the mirror shipped.
#[test]
fn test_fixture_is_not_rotation_invariant() {
    assert_ne!(
        plate_quadrant(0),
        plate_quadrant(90),
        "fixture must distinguish orientations or it cannot detect a mirror"
    );
    assert_ne!(plate_quadrant(90), plate_quadrant(270));
}
