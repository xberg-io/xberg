//! A clipping path made of a single `m` clips everything out.
//!
//! ISO 32000-1:2008 §8.5.3.3.1: "A single-point open subpath (specified by a
//! trailing m operator) shall produce no output." §8.5.4 then sets the clip to
//! the *intersection* of the current clipping path and the new one, and an
//! intersection with an empty region is empty — never "leave the previous clip
//! alone".
//!
//! tiny-skia's `PathBuilder::finish` rejects a lone move-to and returns `None`,
//! and reading that as "no clip was asked for" left the previous, far larger
//! clip in force. On a real scanned page that let an unbounded radial `sh`
//! with `/Extend [true true]` paint DeviceN black across the whole sheet
//! (§8.7.4.3 and Table 77: `sh` fills the current clipping region), turning a
//! white TV-listings grid into a black rectangle.
//!
//! Hand-built synthetic PDF; no third-party fixture.

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

fn stream_obj(dict: &str, data: &[u8]) -> Vec<u8> {
    let mut v = format!("<< {} /Length {} >>\nstream\n", dict, data.len()).into_bytes();
    v.extend_from_slice(data);
    v.extend_from_slice(b"\nendstream");
    v
}

fn page_with(content: &str) -> Vec<u8> {
    build_pdf(&[
        b"<< /Type /Catalog /Pages 2 0 R >>".to_vec(),
        b"<< /Type /Pages /Kids [3 0 R] /Count 1 >>".to_vec(),
        b"<< /Type /Page /Parent 2 0 R /MediaBox [0 0 200 200] /Contents 4 0 R \
           /Resources << >> >>"
            .to_vec(),
        stream_obj("", content.as_bytes()),
    ])
}

/// Fraction of the page that carries ink.
fn coverage(pdf: &[u8]) -> f64 {
    let doc = PdfDocument::from_bytes(pdf.to_vec()).expect("synthetic PDF parses");
    let img = render_page(&doc, 0, &RenderOptions::default()).expect("page renders");
    let px = image::load_from_memory(&img.data).expect("PNG decodes").to_rgba8();
    let n = px
        .pixels()
        .filter(|p| p[3] > 0 && (u32::from(p[0]) + u32::from(p[1]) + u32::from(p[2])) / 3 < 250)
        .count();
    n as f64 / px.pixels().len() as f64
}

/// A page-sized black fill under a lone-move-to clip must paint nothing.
#[test]
fn test_lone_move_to_clip_suppresses_everything_after_it() {
    let cov = coverage(&page_with(
        "q 0 0 200 200 re W n 50 50 m W n 0 0 0 rg 0 0 200 200 re f Q",
    ));
    assert!(
        cov < 0.01,
        "a clip whose path is a single move-to encloses no area, so the fill \
         after it must not paint; got coverage {cov:.5}"
    );
}

/// The control: the same fill with no degenerate clip covers the page. Without
/// it, a fix that simply suppressed all painting would pass the test above.
#[test]
fn test_same_fill_without_the_degenerate_clip_still_paints() {
    let cov = coverage(&page_with("q 0 0 200 200 re W n 0 0 0 rg 0 0 200 200 re f Q"));
    assert!(cov > 0.9, "the unclipped fill must still cover the page; got {cov:.5}");
}

/// A two-verb zero-area clip already worked and must keep working — it is the
/// shape the fix converts the lone move-to into.
#[test]
fn test_zero_area_two_verb_clip_also_suppresses_painting() {
    let cov = coverage(&page_with(
        "q 0 0 200 200 re W n 50 50 m 50 50 l W n 0 0 0 rg 0 0 200 200 re f Q",
    ));
    assert!(cov < 0.01, "a zero-area clip encloses nothing; got coverage {cov:.5}");
}
