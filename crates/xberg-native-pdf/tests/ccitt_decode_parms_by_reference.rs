//! `/DecodeParms` given as an indirect reference must still configure the
//! CCITT decoder.
//!
//! ISO 32000-1:2008 §7.3.10 lets any object be written as an indirect
//! reference, and `/DecodeParms` routinely is. The parameter reader accepted a
//! dictionary or an array and returned `None` for a reference, so the CCITT
//! parameters were never attached to the image — and without them the decode
//! path is skipped entirely and the *still-compressed* codestream is unpacked
//! as though it were packed 1-bit pixels. Everything past the end of the short
//! compressed buffer falls out of bounds and defaults to white, so the page
//! renders almost blank.
//!
//! On the document that exposed this, a 221-byte codestream stood in for
//! 12,341 bytes of a 344x287 image: coverage came out 0.00988 where the ink
//! derivable from the file is 0.07980 and four reference renderers report
//! 0.08004 to 0.08297.

use xberg_native_pdf::PdfDocument;
use xberg_native_pdf::rendering::{RenderOptions, render_page};

/// A 64x32 Group 4 codestream: left half black, right half white, so exactly
/// half the image is ink. Produced with Pillow — `Image.new("1", (64, 32), 1)`
/// with `x < 32` set to 0, saved as TIFF `compression="group4"`, strip bytes
/// extracted. A codestream cannot be written by hand and this repository does
/// not carry binary image fixtures.
const G4: &[u8] = &[
    0x23, 0x60, 0xD5, 0xFF, 0xFF, 0xFF, 0xFF, 0xFF, 0xFF, 0xFF, 0xF8, 0x00, 0x80, 0x08,
];

/// A 100x100 page filled by the CCITT image. `by_reference` selects whether
/// `/DecodeParms` is written inline or as `7 0 R`.
fn ccitt_pdf(by_reference: bool) -> Vec<u8> {
    let parms = "<< /K -1 /Columns 64 /Rows 32 >>";
    let (img_parms, extra_obj) = if by_reference {
        ("/DecodeParms 7 0 R", true)
    } else {
        ("/DecodeParms << /K -1 /Columns 64 /Rows 32 >>", false)
    };
    let content = b"q 100 0 0 100 0 0 cm /Im Do Q";
    let n = if extra_obj { 7 } else { 6 };

    let mut pdf: Vec<u8> = Vec::new();
    let mut off = [0usize; 8];
    macro_rules! push {
        ($s:expr) => {
            pdf.extend_from_slice($s.as_bytes())
        };
    }
    push!("%PDF-1.4\n");
    off[1] = pdf.len();
    push!("1 0 obj\n<< /Type /Catalog /Pages 2 0 R >>\nendobj\n");
    off[2] = pdf.len();
    push!("2 0 obj\n<< /Type /Pages /Kids [3 0 R] /Count 1 >>\nendobj\n");
    off[3] = pdf.len();
    push!(
        "3 0 obj\n<< /Type /Page /Parent 2 0 R /MediaBox [0 0 100 100] \
         /Contents 4 0 R /Resources << /XObject << /Im 5 0 R >> >> >>\nendobj\n"
    );
    off[4] = pdf.len();
    push!(format!("4 0 obj\n<< /Length {} >>\nstream\n", content.len()));
    pdf.extend_from_slice(content);
    push!("\nendstream\nendobj\n");
    off[5] = pdf.len();
    push!(format!(
        "5 0 obj\n<< /Type /XObject /Subtype /Image /Width 64 /Height 32 \
         /BitsPerComponent 1 /ColorSpace /DeviceGray /Filter /CCITTFaxDecode \
         {img_parms} /Length {} >>\nstream\n",
        G4.len()
    ));
    pdf.extend_from_slice(G4);
    push!("\nendstream\nendobj\n");
    off[6] = pdf.len();
    push!("6 0 obj\n<< >>\nendobj\n");
    if extra_obj {
        off[7] = pdf.len();
        push!(format!("7 0 obj\n{parms}\nendobj\n"));
    }

    let xref = pdf.len();
    push!(format!("xref\n0 {}\n0000000000 65535 f \r\n", n + 1));
    for id in 1..=n {
        push!(format!("{:010} 00000 n \r\n", off[id]));
    }
    push!(format!(
        "trailer\n<< /Size {} /Root 1 0 R >>\nstartxref\n{xref}\n%%EOF\n",
        n + 1
    ));
    pdf
}

fn coverage(pdf: Vec<u8>) -> f64 {
    let doc = PdfDocument::from_bytes(pdf).expect("synthetic PDF parses");
    let img = render_page(&doc, 0, &RenderOptions::with_dpi(72).as_raw()).expect("page renders");
    let n = img.data.len() / 4;
    let inked = img
        .data
        .chunks(4)
        .filter(|p| (p[0] as u32 + p[1] as u32 + p[2] as u32) / 3 < 128)
        .count();
    inked as f64 / n as f64
}

#[test]
fn test_indirect_decode_parms_still_decodes_the_ccitt_image() {
    let cov = coverage(ccitt_pdf(true));
    // Half the image is black and it fills the page.
    assert!(
        (cov - 0.5).abs() < 0.05,
        "an indirect /DecodeParms must still configure the decoder; the image \
         is half black and fills the page, so coverage should be ~0.5. Got \
         {cov:.5} — a value near zero means the compressed bytes were unpacked \
         as pixels and ran out."
    );
}

/// Control: the inline form must give the same answer, so the test cannot pass
/// by the decoder ignoring `/DecodeParms` altogether.
#[test]
fn test_inline_form_agrees_with_the_indirect_one() {
    let indirect = coverage(ccitt_pdf(true));
    let inline = coverage(ccitt_pdf(false));
    assert!(
        (indirect - inline).abs() < 0.01,
        "writing /DecodeParms as a reference must not change the result: \
         {indirect:.5} indirect, {inline:.5} inline"
    );
}
