//! A JPEG 2000 image painted far below its stored size is decoded at a reduced resolution, but an
//! `/SMask` is sampled against the image's own samples by nearest neighbour, so a mask with detail
//! finer than the reduction would alias to a single value. Such an image is decoded at full size.

use xberg_native_pdf::document::PdfDocument;
use xberg_native_pdf::extractors::warnings::WarningCategory;
use xberg_native_pdf::rendering::{RenderOptions, render_page};

/// 816x1056, one grey component, with several resolution levels.
const SAMPLE_GRAY_JP2: &[u8] = include_bytes!("fixtures/jpx/sample_gray.jp2");
const IMAGE_WIDTH: usize = 816;
const IMAGE_HEIGHT: usize = 1056;
const PAGE_WIDTH: usize = 102;
const PAGE_HEIGHT: usize = 132;

/// A page painted red with the JPEG 2000 image over all of it, carrying an `/SMask` whose columns
/// alternate between opaque and transparent.
fn pdf_with_striped_smask(smask: bool) -> Vec<u8> {
    let content = format!(
        "1 0 0 rg\n0 0 {PAGE_WIDTH} {PAGE_HEIGHT} re f\nq\n{PAGE_WIDTH} 0 0 {PAGE_HEIGHT} 0 0 cm\n/Im1 Do\nQ\n"
    );
    let smask_entry = if smask { " /SMask 6 0 R" } else { "" };
    let mask: Vec<u8> = (0..IMAGE_WIDTH * IMAGE_HEIGHT)
        .map(|i| if (i % IMAGE_WIDTH).is_multiple_of(2) { 0 } else { 255 })
        .collect();
    let mut buf: Vec<u8> = b"%PDF-1.5\n".to_vec();
    let mut offsets = Vec::new();
    offsets.push(buf.len());
    buf.extend_from_slice(b"1 0 obj\n<< /Type /Catalog /Pages 2 0 R >>\nendobj\n");
    offsets.push(buf.len());
    buf.extend_from_slice(b"2 0 obj\n<< /Type /Pages /Kids [3 0 R] /Count 1 >>\nendobj\n");
    offsets.push(buf.len());
    buf.extend_from_slice(
        format!(
            "3 0 obj\n<< /Type /Page /Parent 2 0 R /MediaBox [0 0 {PAGE_WIDTH} {PAGE_HEIGHT}] /Contents 4 0 R \
             /Resources << /XObject << /Im1 5 0 R >> >> >>\nendobj\n"
        )
        .as_bytes(),
    );
    offsets.push(buf.len());
    buf.extend_from_slice(
        format!(
            "4 0 obj\n<< /Length {} >>\nstream\n{content}\nendstream\nendobj\n",
            content.len()
        )
        .as_bytes(),
    );
    offsets.push(buf.len());
    buf.extend_from_slice(
        format!(
            "5 0 obj\n<< /Type /XObject /Subtype /Image /Width {IMAGE_WIDTH} /Height {IMAGE_HEIGHT} \
             /BitsPerComponent 8 /ColorSpace /DeviceGray /Filter /JPXDecode{smask_entry} /Length {} >>\nstream\n",
            SAMPLE_GRAY_JP2.len()
        )
        .as_bytes(),
    );
    buf.extend_from_slice(SAMPLE_GRAY_JP2);
    buf.extend_from_slice(b"\nendstream\nendobj\n");
    offsets.push(buf.len());
    buf.extend_from_slice(
        format!(
            "6 0 obj\n<< /Type /XObject /Subtype /Image /Width {IMAGE_WIDTH} /Height {IMAGE_HEIGHT} \
             /BitsPerComponent 8 /ColorSpace /DeviceGray /Length {} >>\nstream\n",
            mask.len()
        )
        .as_bytes(),
    );
    buf.extend_from_slice(&mask);
    buf.extend_from_slice(b"\nendstream\nendobj\n");
    let xref = buf.len();
    buf.extend_from_slice(format!("xref\n0 {}\n0000000000 65535 f \n", offsets.len() + 1).as_bytes());
    for offset in &offsets {
        buf.extend_from_slice(format!("{offset:010} 00000 n \n").as_bytes());
    }
    buf.extend_from_slice(
        format!(
            "trailer\n<< /Size {} /Root 1 0 R >>\nstartxref\n{xref}\n%%EOF\n",
            offsets.len() + 1
        )
        .as_bytes(),
    );
    buf
}

#[test]
fn a_finely_striped_soft_mask_blends_instead_of_aliasing_to_one_value() {
    let doc = PdfDocument::from_bytes(pdf_with_striped_smask(true)).expect("fixture parses");
    let page = render_page(&doc, 0, &RenderOptions::with_dpi(72).as_raw()).expect("render page 0");
    assert_eq!(
        (page.width as usize, page.height as usize),
        (PAGE_WIDTH, PAGE_HEIGHT),
        "test premise"
    );

    // Over the red page the green channel is zero unless the image shows through, and an image
    // sampled only at the mask's transparent columns never does.
    let green_total: u64 = (0..PAGE_WIDTH * PAGE_HEIGHT)
        .map(|pixel| u64::from(page.data[pixel * 4 + 1]))
        .sum();
    let mean_green = green_total / (PAGE_WIDTH * PAGE_HEIGHT) as u64;
    assert!(
        mean_green > 20,
        "the half-opaque mask must let the image show through, mean green was {mean_green}"
    );
}

/// The same image without a mask is decoded at the reduced size; the dictionary agrees with the
/// codestream, so the reduced samples' smaller size must not be reported as a disagreement.
#[test]
fn a_reduced_decode_of_a_matching_dictionary_records_no_size_mismatch() {
    let doc = PdfDocument::from_bytes(pdf_with_striped_smask(false)).expect("fixture parses");
    render_page(&doc, 0, &RenderOptions::with_dpi(72).as_raw()).expect("render page 0");
    let warnings = doc.structured_warnings();
    assert!(
        !warnings.iter().any(|w| w.category == WarningCategory::JpxSizeMismatch),
        "the dictionary matches the codestream: {warnings:?}"
    );
}
