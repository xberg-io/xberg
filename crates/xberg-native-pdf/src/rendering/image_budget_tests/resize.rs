use super::*;
use crate::extractors::images::{ColorSpace, ImageData, PdfImage};
use crate::rendering::page_renderer::image_budget::{ImageBudget, JpegDecode, Placement, resize_working_bytes};
use crate::rendering::render_png_page_with_budget_evidence;

/// A 1200 px square painted over 1000 px: `render_image` pre-resizes it to 1000 x 1000.
const SOURCE: u32 = 1200;
const PAINTED: &[u8] = b"q 1000 0 0 1000 0 0 cm /Im0 Do Q\n";
/// The 1000 x 1000 output (4 000 000), the vertical-first intermediate of 1200 x 1000 pixels plus
/// one alignment pixel (4 800 004), and two coefficient tables of 1000 outputs with a 5-slot
/// window (1000 * (5 * 10 + 64) = 114 000 each).
const RESIZE: u64 = 9_028_004;

fn square() -> PdfImage {
    PdfImage::new(
        SOURCE,
        SOURCE,
        ColorSpace::DeviceRGB,
        8,
        ImageData::Jpeg(source::jpeg_of(SOURCE, SOURCE)),
    )
}

fn encoded(image: &PdfImage) -> u64 {
    let ImageData::Jpeg(bytes) = image.data() else {
        unreachable!()
    };
    (bytes.capacity() + bytes.len()) as u64
}

#[test]
fn resize_working_bytes_counts_the_output_intermediate_and_coefficients() {
    assert_eq!(resize_working_bytes((1200, 1200), (1000, 1000)), RESIZE);
    // One resized axis: no intermediate, and only that axis's table (window 2 * 6 + 1 = 13).
    assert_eq!(resize_working_bytes((1, 512), (1, 100)), 400 + 100 * (13 * 10 + 64));
    assert_eq!(resize_working_bytes((32, 32), (32, 32)), 32 * 32 * 4);
    assert_eq!(resize_working_bytes((1, 1), (u32::MAX, u32::MAX)), u64::MAX);
}

#[test]
fn a_decode_the_old_charge_admitted_is_refused_when_its_pre_resize_does_not_fit() {
    let image = square();
    let canvas = 1000;
    let old_charge = u64::from(SOURCE * SOURCE) * (3 + 4) + encoded(&image) + canvas;
    assert!(
        ImageBudget::new(old_charge as usize).admit_jpeg(&image, canvas).is_ok(),
        "painted at its own size, the decode and its RGBA copy fit"
    );
    // The pre-resize outweighs the 4 320 000-byte RGB decode it follows, so the true peak is the
    // RGBA raster plus the resize working set.
    let required = u64::from(SOURCE * SOURCE) * 4 + RESIZE + encoded(&image) + canvas;
    assert!(required > old_charge);
    let shrunk = Placement::extent(1000.0, 1000.0);
    let mut budget = ImageBudget::new(old_charge as usize);
    assert!(budget.admit_jpeg_decode(&image, canvas, shrunk, true).is_err());
    assert!(budget.exceeded(), "the refusal is the budget class");
    let admit = |limit: u64| ImageBudget::new(limit as usize).admit_jpeg_decode(&image, canvas, shrunk, true);
    assert_eq!(admit(required).unwrap(), JpegDecode::Full);
    assert!(
        admit(required - 1).is_err(),
        "neither a reduced nor an in-place decode avoids the pre-resize"
    );
}

#[test]
fn a_page_whose_pre_resize_exceeds_the_budget_is_refused_and_renders_once_it_fits() {
    let document = source::document_painting(&source::jpeg_of(SOURCE, SOURCE), SOURCE, PAINTED, "");
    let refused = 12 * 1024 * 1024;
    let (error, exceeded) = render_png_page_with_budget_evidence(&document, 0, 72, 72, refused)
        .err()
        .expect("the 14.8 MB decode-and-resize peak exceeds 12 MiB, though the 10.1 MB decode fits");
    assert!(exceeded, "{error}");
    assert!(error.to_string().contains("working budget"), "{error}");
    let (rendered, _) = renderer()
        .render_page_with_budget(
            0,
            &document,
            Some(PngRenderBudget {
                max_working_bytes: 16 * 1024 * 1024,
                minimum_dpi: 72,
            }),
        )
        .expect("the decode-and-resize peak fits 16 MiB");
    assert!(is_black_somewhere(&rendered), "the pre-resized image must paint");
}
