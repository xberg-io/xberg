//! A JBIG2 image must honour its `/Decode` array.
//!
//! The JBIG2 decoder expands straight to 8-bit gray, taking a branch that
//! skips the packed-sample block where `/Decode` is otherwise applied -- so
//! the array was read for every other filter and silently dropped for this
//! one. ISO 32000-1:2008 8.9.5.2 Table 90 applies to a JBIG2 image like any
//! other, and Table 145 explicitly permits `/Decode` on a soft-mask image:
//! it constrains only the *default* to `[0 1]`, unlike the entries it marks
//! "Ignored". 11.6.5.3 then requires the alpha be derived "with the effects
//! of the Filter and Decode transformations already performed".
//!
//! This bites hardest on scanned books, where the text layer is a JBIG2
//! `/SMask` and the producer writes `/Decode [1 0]` to normalise whichever
//! polarity its encoder emitted. Dropping the array inverts the mask exactly:
//! the page paints its background through the glyphs and hides everything
//! else.
//!
//! The mask here is a JBIG2 *image* used as an `/SMask`, not an `/ImageMask`
//! stencil -- the stencil path reads `/Decode` separately and is covered by
//! its own test.

use xberg_native_pdf::PdfDocument;
use xberg_native_pdf::rendering::{RenderOptions, render_page};

struct Bits {
    out: Vec<u8>,
    n: u32,
}

impl Bits {
    fn new() -> Self {
        Bits { out: Vec::new(), n: 0 }
    }

    fn push(&mut self, code: &str) {
        for c in code.chars() {
            if self.n.is_multiple_of(8) {
                self.out.push(0);
            }
            if c == '1' {
                let last = self.out.len() - 1;
                self.out[last] |= 0x80 >> (self.n % 8);
            }
            self.n += 1;
        }
    }
}

// ITU-T T.4 codes. Horizontal mode is reference-line independent: it names two
// run lengths starting at a0, which is why every row here can be written the
// same way whatever the row above it was.
const H: &str = "001";
const WHITE_0: &str = "00110101";
const WHITE_8: &str = "10011";
const BLACK_0: &str = "0000110111";
const BLACK_8: &str = "000101";

/// T.6 (MMR) data for an 8x8 bilevel image: `black_rows` all-black rows at the
/// top, the rest all white.
fn mmr_8x8(black_rows: usize) -> Vec<u8> {
    let mut b = Bits::new();
    for row in 0..8 {
        b.push(H);
        if row < black_rows {
            b.push(WHITE_0);
            b.push(BLACK_8);
        } else {
            b.push(WHITE_8);
            b.push(BLACK_0);
        }
    }
    b.out
}

fn be32(v: u32) -> [u8; 4] {
    v.to_be_bytes()
}

/// An embedded-organisation JBIG2 bitstream: a page-information segment and
/// one immediate generic region coded with MMR.
fn jbig2_8x8(black_rows: usize) -> Vec<u8> {
    let mut page_data = Vec::new();
    page_data.extend_from_slice(&be32(8)); // page width
    page_data.extend_from_slice(&be32(8)); // page height
    page_data.extend_from_slice(&be32(0)); // x resolution
    page_data.extend_from_slice(&be32(0)); // y resolution
    page_data.push(0); // page segment flags
    page_data.extend_from_slice(&[0, 0]); // striping information

    let mmr = mmr_8x8(black_rows);
    let mut region = Vec::new();
    region.extend_from_slice(&be32(8)); // region width
    region.extend_from_slice(&be32(8)); // region height
    region.extend_from_slice(&be32(0)); // region x
    region.extend_from_slice(&be32(0)); // region y
    region.push(0); // external combination operator OR
    region.push(0x01); // generic region flags: MMR = 1
    region.extend_from_slice(&mmr);

    let mut out = Vec::new();
    // segment header: number, flags (type in the low 6 bits), referred-to
    // count/retain byte, one-byte page association, data length.
    for (number, seg_type, data) in [(0u32, 48u8, page_data), (1u32, 38u8, region)] {
        out.extend_from_slice(&be32(number));
        out.push(seg_type);
        out.push(0x00);
        out.push(0x01);
        out.extend_from_slice(&be32(data.len() as u32));
        out.extend_from_slice(&data);
    }
    out
}

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

/// A 100x100 page covered by one black 8x8 image whose soft mask is a JBIG2
/// bitmap with its top four rows black.
///
/// A JBIG2 black pixel decodes to gray 0, and 11.6.5.3 makes gray 0 fully
/// transparent, so under the default `/Decode` the ink lands in the *bottom*
/// half. `/Decode [1 0]` swaps that (Table 90 NOTE 3) and moves it to the top.
fn jbig2_smask_page(decode: Option<&str>) -> Vec<u8> {
    let decode_entry = decode.map_or(String::new(), |d| format!("/Decode {d}"));
    let objects = vec![
        b"<< /Type /Catalog /Pages 2 0 R >>".to_vec(),
        b"<< /Type /Pages /Kids [3 0 R] /Count 1 >>".to_vec(),
        b"<< /Type /Page /Parent 2 0 R /MediaBox [0 0 100 100] /Contents 4 0 R \
           /Resources << /XObject << /Im 5 0 R >> >> >>"
            .to_vec(),
        stream_obj("", b"q 100 0 0 100 0 0 cm /Im Do Q"),
        stream_obj(
            "/Type /XObject /Subtype /Image /Width 8 /Height 8 \
             /ColorSpace /DeviceGray /BitsPerComponent 8 /SMask 6 0 R",
            &[0x00u8; 64],
        ),
        stream_obj(
            &format!(
                "/Type /XObject /Subtype /Image /Width 8 /Height 8 \
                 /ColorSpace /DeviceGray /BitsPerComponent 1 \
                 /Filter /JBIG2Decode {decode_entry}"
            ),
            &jbig2_8x8(4),
        ),
    ];
    build_pdf(&objects)
}

/// Ink coverage of the top and bottom 40% bands.
///
/// The bands stop short of the halfway line because the 8x8 mask is scaled up
/// and its alpha interpolates across the boundary; measuring through that
/// would pin a resampling detail rather than the polarity under test.
fn bands(pdf: &[u8]) -> (f64, f64) {
    let doc = PdfDocument::from_bytes(pdf.to_vec()).expect("synthetic PDF parses");
    let img = render_page(&doc, 0, &RenderOptions::with_dpi(72).as_raw()).expect("page renders");
    let (w, h) = (img.width, img.height);
    let band = (h * 2) / 5;
    let (mut top, mut bot) = (0u64, 0u64);
    for (i, p) in img.data.chunks(4).enumerate() {
        let y = i as u32 / w;
        let lum = (u32::from(p[0]) + u32::from(p[1]) + u32::from(p[2])) / 3;
        if p[3] > 0 && lum < 250 {
            if y < band {
                top += 1;
            } else if y >= h - band {
                bot += 1;
            }
        }
    }
    let band_px = f64::from(w) * f64::from(band);
    (top as f64 / band_px, bot as f64 / band_px)
}

#[test]
fn test_jbig2_smask_without_decode_paints_where_the_bitmap_is_white() {
    let (top, bot) = bands(&jbig2_smask_page(None));
    assert!(
        bot > 0.9 && top < 0.1,
        "the mask's black rows are transparent by default, so the ink belongs \
         in the bottom band; got top {top:.4}, bottom {bot:.4}"
    );
}

#[test]
fn decode_1_0_inverts_a_jbig2_smask() {
    let (top, bot) = bands(&jbig2_smask_page(Some("[1 0]")));
    assert!(
        top > 0.9 && bot < 0.1,
        "/Decode [1 0] swaps which samples are opaque, moving the ink to the \
         top band; got top {top:.4}, bottom {bot:.4}. Equal bands mean the \
         array was dropped on the JBIG2 path."
    );
}

/// The two arrangements must actually differ -- a test that passes because
/// nothing painted at all would prove nothing.
#[test]
fn test_decode_array_changes_the_rendered_page() {
    let plain = bands(&jbig2_smask_page(None));
    let inverted = bands(&jbig2_smask_page(Some("[1 0]")));
    assert!(
        (plain.0 - inverted.0).abs() > 0.5 && (plain.1 - inverted.1).abs() > 0.5,
        "default and [1 0] must render differently; got {plain:?} and {inverted:?}"
    );
}
