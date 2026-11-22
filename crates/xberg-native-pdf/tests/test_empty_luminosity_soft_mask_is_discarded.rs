//! A Luminosity soft mask whose group paints nothing is not a mask.
//!
//! Read literally, ISO 32000-1:2008 §11.6.5.2 (`docs/spec/pdf.md`:23894-23899)
//! still yields a mask for such a group:
//!
//! > If the subtype is **Luminosity**, the transparency group XObject **G**
//! > shall be composited with a fully opaque backdrop whose colour is
//! > everywhere defined by the soft-mask dictionary's **BC** entry. The
//! > computed result colour shall then be converted to a single-component
//! > luminosity value …
//!
//! A group that paints nothing leaves that backdrop untouched, and Table 144
//! (pdf.md:23924) defaults `/BC` to "the colour space's initial value,
//! representing black" — luminosity 0. So the mask would be 0 everywhere and
//! the masked content would disappear.
//!
//! Every reference engine disagrees, and not by computing a different value:
//! they discard the mask. Deleting `/SMask` from one such file and
//! re-rendering gives MuPDF byte-identical output. On the file that surfaced
//! this — a page whose whole content is a 918x427 photograph, masked away by a
//! group whose content stream is the two bytes `q Q` — we rendered pure white
//! where MuPDF, pdfium, poppler and Ghostscript all paint the picture.
//!
//! The rule is deliberately narrow: a group that paints something genuinely
//! dark still masks. Only a group that contributes no luminosity at all is
//! discarded.

use xberg_native_pdf::PdfDocument;
use xberg_native_pdf::rendering::{RenderOptions, render_page};

/// A page that fills a black square under an ExtGState soft mask whose
/// luminosity group contains `group_content`.
fn page_with_luminosity_mask(group_content: &str) -> Vec<u8> {
    let content = b"/GS1 gs 0 0 0 rg 0 0 100 100 re f\n".to_vec();
    let group = group_content.as_bytes().to_vec();

    let mut pdf = Vec::new();
    let mut off = [0usize; 9];
    macro_rules! push {
        ($s:expr) => {
            pdf.extend_from_slice($s.as_bytes())
        };
    }
    push!("%PDF-1.7\n");
    off[1] = pdf.len();
    push!("1 0 obj\n<< /Type /Catalog /Pages 2 0 R >>\nendobj\n");
    off[2] = pdf.len();
    push!("2 0 obj\n<< /Type /Pages /Kids [3 0 R] /Count 1 >>\nendobj\n");
    off[3] = pdf.len();
    push!(
        "3 0 obj\n<< /Type /Page /Parent 2 0 R /MediaBox [0 0 100 100] \
         /Contents 4 0 R /Resources << /ExtGState << /GS1 5 0 R >> >> >>\nendobj\n"
    );
    off[4] = pdf.len();
    push!(format!("4 0 obj\n<< /Length {} >>\nstream\n", content.len()));
    pdf.extend_from_slice(&content);
    push!("endstream\nendobj\n");
    off[5] = pdf.len();
    push!(
        "5 0 obj\n<< /Type /ExtGState /BM /Normal \
         /SMask << /Type /Mask /S /Luminosity /G 6 0 R >> >>\nendobj\n"
    );
    off[6] = pdf.len();
    push!(format!(
        "6 0 obj\n<< /Type /XObject /Subtype /Form /BBox [0 0 100 100] \
         /Group << /Type /Group /S /Transparency /CS /DeviceGray >> \
         /Resources << >> /Length {} >>\nstream\n",
        group.len()
    ));
    pdf.extend_from_slice(&group);
    push!("endstream\nendobj\n");

    let xref = pdf.len();
    push!("xref\n0 7\n0000000000 65535 f \r\n");
    for offset in &off[1..=6] {
        push!(format!("{:010} 00000 n \r\n", offset));
    }
    push!(format!(
        "trailer\n<< /Size 7 /Root 1 0 R >>\nstartxref\n{xref}\n%%EOF\n"
    ));
    pdf
}

/// Mean luminance, 0 (black) to 255 (white).
fn mean_luma(pdf: Vec<u8>) -> f64 {
    let doc = PdfDocument::from_bytes(pdf).expect("synthetic PDF parses");
    let img = render_page(&doc, 0, &RenderOptions::default()).expect("page renders");
    let px = image::load_from_memory(&img.data).expect("PNG decodes").to_rgba8();
    let n = px.pixels().len() as f64;
    px.pixels()
        .map(|p| (f64::from(p[0]) + f64::from(p[1]) + f64::from(p[2])) / 3.0)
        .sum::<f64>()
        / n
}

#[test]
fn test_group_that_paints_nothing_does_not_mask() {
    // `q Q` draws nothing, so there is no luminosity to derive a mask from.
    // Taking §11.6.5.2 literally would give mask 0 and erase the black
    // square, leaving a white page.
    let luma = mean_luma(page_with_luminosity_mask("q Q\n"));
    assert!(
        luma < 60.0,
        "the black fill was masked away by a group that paints nothing: \
         mean luma {luma:.1} (a blank page is ~255)"
    );
}

#[test]
fn test_group_that_paints_black_still_masks() {
    // The counter-case, and the reason the rule above must stay narrow: a
    // group that genuinely paints black has luminosity 0, and that *is* a
    // mask. The fill must disappear.
    let luma = mean_luma(page_with_luminosity_mask("0 0 0 rg 0 0 100 100 re f\n"));
    assert!(
        luma > 200.0,
        "a luminosity group painting black should mask the fill away, \
         leaving a light page: mean luma {luma:.1}"
    );
}

#[test]
fn test_group_that_paints_white_does_not_mask() {
    // And white luminosity is a fully opaque mask, so the fill survives.
    let luma = mean_luma(page_with_luminosity_mask("1 1 1 rg 0 0 100 100 re f\n"));
    assert!(
        luma < 60.0,
        "a luminosity group painting white should leave the fill visible: \
         mean luma {luma:.1}"
    );
}
