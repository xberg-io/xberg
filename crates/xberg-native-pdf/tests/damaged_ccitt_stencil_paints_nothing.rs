//! A CCITT stencil whose data cannot be decoded must not paint a solid block.
//!
//! ISO 32000-1:2008 §8.9.6.2: for an image mask with the default `/Decode`, a
//! sample of 0 marks the page with the current colour. `decompress_ccitt`
//! returns `Ok(all-zero)` when both decoders fail, so every sample reads as
//! "paint" and a damaged stencil covers its whole footprint in the fill
//! colour — silently, with only a log warning.
//!
//! Undecodable data must not resolve toward painting more. Either the stencil
//! draws nothing, or the error propagates and the image is skipped; both leave
//! the page as the file would look without the stencil, which is the graceful
//! degradation. A solid rectangle over the page is not.

use xberg_native_pdf::PdfDocument;
use xberg_native_pdf::rendering::{RenderOptions, render_page};

/// A page painting a white backdrop, then a CCITT `/ImageMask` in red whose
/// encoded data is `data`.
fn ccitt_stencil_pdf(data: &[u8], decode: &str) -> Vec<u8> {
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
        "<< /Type /Page /Parent 2 0 R /MediaBox [0 0 200 200] \
         /Resources << /XObject << /Im0 5 0 R >> >> /Contents 4 0 R >>",
    );
    // White page, then the stencil painted red across the middle 100x100.
    let content = b"1 1 1 rg 0 0 200 200 re f\n1 0 0 rg\nq 100 0 0 100 50 50 cm /Im0 Do Q\n";
    off[4] = buf.len();
    buf.extend_from_slice(format!("4 0 obj\n<< /Length {} >>\nstream\n", content.len()).as_bytes());
    buf.extend_from_slice(content);
    buf.extend_from_slice(b"\nendstream\nendobj\n");
    off[5] = buf.len();
    buf.extend_from_slice(
        format!(
            "5 0 obj\n<< /Type /XObject /Subtype /Image /Width 64 /Height 64 \
             /ImageMask true /BitsPerComponent 1 {decode} /Filter /CCITTFaxDecode \
             /DecodeParms << /K -1 /Columns 64 /Rows 64 >> /Length {} >>\nstream\n",
            data.len()
        )
        .as_bytes(),
    );
    buf.extend_from_slice(data);
    buf.extend_from_slice(b"\nendstream\nendobj\n");

    let xref = buf.len();
    buf.extend_from_slice(b"xref\n0 6\n0000000000 65535 f \n");
    for id in 1..=5 {
        buf.extend_from_slice(format!("{:010} 00000 n \n", off[id]).as_bytes());
    }
    buf.extend_from_slice(b"trailer\n<< /Size 6 /Root 1 0 R >>\nstartxref\n");
    buf.extend_from_slice(format!("{xref}\n%%EOF\n").as_bytes());
    buf
}

/// Fraction of the stencil's footprint that came out red.
fn red_fraction(pdf: Vec<u8>) -> f64 {
    let doc = PdfDocument::from_bytes(pdf).expect("parse");
    let img = render_page(&doc, 0, &RenderOptions::with_dpi(72).as_raw()).expect("render");
    let (w, h) = (img.width as usize, img.height as usize);
    let (mut red, mut total) = (0usize, 0usize);
    // The stencil occupies PDF (50,50)..(150,150); in raster rows that is the
    // middle band either way, so sample the central square.
    for y in (h / 4)..(h * 3 / 4) {
        for x in (w / 4)..(w * 3 / 4) {
            let i = (y * w + x) * 4;
            total += 1;
            if img.data[i] > 200 && img.data[i + 1] < 80 && img.data[i + 2] < 80 {
                red += 1;
            }
        }
    }
    red as f64 / total.max(1) as f64
}

/// Bytes that are not decodable CCITT under any reading.
const JUNK: [u8; 6] = [0xDE, 0xAD, 0xBE, 0xEF, 0x00, 0xFF];

/// The reported symptom: a damaged stencil painted its whole footprint.
#[test]
fn test_damaged_ccitt_stencil_does_not_paint_a_solid_block() {
    let f = red_fraction(ccitt_stencil_pdf(&JUNK, ""));
    assert!(
        f < 0.5,
        "an undecodable CCITT stencil covered {:.0}% of its footprint in the fill colour",
        f * 100.0
    );
}

/// An empty stream is the same class of failure.
#[test]
fn test_empty_ccitt_stencil_does_not_paint_a_solid_block() {
    let f = red_fraction(ccitt_stencil_pdf(&[], ""));
    assert!(
        f < 0.5,
        "an empty CCITT stencil covered {:.0}% of its footprint",
        f * 100.0
    );
}

/// And under a reversed `/Decode`, where the polarity of "paint" flips — the
/// failure must not simply move to the other array.
#[test]
fn test_damaged_ccitt_stencil_with_reversed_decode_paints_nothing() {
    let f = red_fraction(ccitt_stencil_pdf(&JUNK, "/Decode [1 0]"));
    assert!(
        f < 0.5,
        "an undecodable CCITT stencil with /Decode [1 0] covered {:.0}% of its footprint",
        f * 100.0
    );
}

/// The render must still complete — degrading must not become an abort.
#[test]
fn test_damaged_ccitt_stencil_still_renders_the_page() {
    let doc = PdfDocument::from_bytes(ccitt_stencil_pdf(&JUNK, "")).expect("parse");
    let img = render_page(&doc, 0, &RenderOptions::with_dpi(72).as_raw()).expect("render must return");
    assert_eq!((img.width, img.height), (200, 200));
}
