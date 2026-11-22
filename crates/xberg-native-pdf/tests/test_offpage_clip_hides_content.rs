//! A clipping path the renderer cannot rasterise must not be resolved in the
//! direction that paints more.
//!
//! ISO 32000-1:2008 §8.5.4: content outside the clipping path shall not be
//! painted. Annex C.1 does say a conforming reader has arithmetic limits and
//! that exceeding one is an error — so having a device-coordinate ceiling is
//! legitimate. What it does not license is the *response*: dropping the clip is
//! neither raising an error nor skipping the construct, it renders something
//! the file did not describe.
//!
//! Two unrasterisable clips need opposite answers, which is why the previous
//! blanket "drop it" was wrong in one direction and right in the other:
//!
//! - A clip whose device bounds **miss the page entirely** excludes everything.
//!   The correct output is blank; dropping it painted the whole page.
//! - A clip with enormous coordinates that still **encloses** the page
//!   restricts nothing visible, so dropping it is harmless — and an empty mask
//!   there would wrongly erase every later draw.

use xberg_native_pdf::PdfDocument;
use xberg_native_pdf::rendering::{ImageFormat, RenderOptions, render_page};

/// A page that clips to `clip_rect`, then fills the whole page black.
fn clipped_fill_pdf(clip_rect: &str) -> Vec<u8> {
    let content = format!("q {clip_rect} re W n 0 0 0 rg 0 0 200 200 re f Q\n");

    let mut buf: Vec<u8> = Vec::new();
    let mut off = vec![0usize; 5];
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
        "<< /Type /Page /Parent 2 0 R /MediaBox [0 0 200 200] /Contents 4 0 R >>",
    );
    off[4] = buf.len();
    buf.extend_from_slice(format!("4 0 obj\n<< /Length {} >>\nstream\n", content.len()).as_bytes());
    buf.extend_from_slice(content.as_bytes());
    buf.extend_from_slice(b"\nendstream\nendobj\n");

    let xref = buf.len();
    buf.extend_from_slice(b"xref\n0 5\n0000000000 65535 f \n");
    for offset in &off[1..=4] {
        buf.extend_from_slice(format!("{:010} 00000 n \n", offset).as_bytes());
    }
    buf.extend_from_slice(b"trailer\n<< /Size 5 /Root 1 0 R >>\nstartxref\n");
    buf.extend_from_slice(format!("{xref}\n%%EOF\n").as_bytes());
    buf
}

/// Fraction of the page that came out dark.
fn dark_fraction(pdf: Vec<u8>) -> f64 {
    let doc = PdfDocument::from_bytes(pdf).expect("parse");
    let mut opts = RenderOptions::default();
    opts.dpi = 72;
    opts.format = ImageFormat::RawRgba8;
    let img = render_page(&doc, 0, &opts).expect("render");
    let (w, h) = (img.width as usize, img.height as usize);
    let (mut dark, mut total) = (0usize, 0usize);
    for y in 0..h {
        for x in 0..w {
            let i = (y * w + x) * 4;
            total += 1;
            if img.data[i] < 128 && img.data[i + 1] < 128 && img.data[i + 2] < 128 {
                dark += 1;
            }
        }
    }
    dark as f64 / total.max(1) as f64
}

/// The reported symptom: a clip placed far off-page with coordinates past the
/// device ceiling excluded everything, and the page must come out blank.
#[test]
fn test_offpage_unrasterisable_clip_paints_nothing() {
    // Well past MAX_DEVICE_COORD (5e8) and nowhere near the page.
    let f = dark_fraction(clipped_fill_pdf("2000000000 2000000000 1000 1000"));
    assert!(
        f < 0.01,
        "a clip excluding the whole page still painted {:.0}% of it",
        f * 100.0
    );
}

/// The other direction: an enormous clip that still contains the page
/// restricts nothing visible, so the fill must survive. An empty mask here
/// would erase the page — the failure the previous behaviour was guarding.
#[test]
fn test_enclosing_unrasterisable_clip_still_paints() {
    let f = dark_fraction(clipped_fill_pdf("-2000000000 -2000000000 4000000000 4000000000"));
    assert!(
        f > 0.9,
        "a clip enclosing the page wrongly erased it; only {:.0}% painted",
        f * 100.0
    );
}

/// An ordinary clip is unaffected — the guard must only engage past the
/// device ceiling.
#[test]
fn test_ordinary_clip_still_clips_normally() {
    let f = dark_fraction(clipped_fill_pdf("0 0 100 200"));
    assert!(
        (0.4..0.6).contains(&f),
        "a half-page clip should paint about half the page, got {:.0}%",
        f * 100.0
    );
}

/// And no clip at all paints everything.
#[test]
fn test_unclipped_fill_paints_the_page() {
    let f = dark_fraction(clipped_fill_pdf("0 0 200 200"));
    assert!(f > 0.9, "an all-page clip should not restrict the fill");
}
