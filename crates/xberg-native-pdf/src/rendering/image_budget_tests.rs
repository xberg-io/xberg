use super::*;
#[path = "image_budget_tests/source.rs"]
mod source;

const LIMIT: usize = 8 * 1024 * 1024;
const LARGE: u32 = 4096;

fn renderer() -> PageRenderer {
    PageRenderer::new(RenderOptions {
        dpi: 72,
        format: ImageFormat::RawRgba8,
        ..Default::default()
    })
}

fn budget() -> Option<PngRenderBudget> {
    Some(PngRenderBudget {
        max_working_bytes: LIMIT,
        minimum_dpi: 72,
    })
}

fn assert_refused(renderer: &mut PageRenderer, document: &PdfDocument) {
    let error = renderer
        .render_page_with_budget(0, document, budget())
        .err()
        .expect("source JPEG rasters exceed the supplied working-image budget");
    assert!(error.to_string().contains("image"), "{error}");
    assert!(error.to_string().contains("budget"), "{error}");
}

#[test]
fn source_jpeg_budget_failure_does_not_poison_a_reused_renderer() {
    let document = source::document(&source::jpeg(LARGE), LARGE, false);
    let mut renderer = renderer();
    assert_refused(&mut renderer, &document);
    let small = source::document(&source::jpeg(32), 32, false);
    let (image, _) = renderer.render_page_with_budget(0, &small, budget()).unwrap();
    assert_eq!((image.width, image.height), (32, 32));
    assert!(image.data.chunks_exact(4).any(|pixel| pixel[..3] == [0, 0, 0]));
}

#[test]
fn prior_unbounded_render_cannot_bypass_a_later_source_image_budget() {
    let document = source::document(&source::jpeg(LARGE), LARGE, false);
    let mut renderer = renderer();
    let (image, _) = renderer.render_page_with_budget(0, &document, None).unwrap();
    assert!(image.data.chunks_exact(4).any(|pixel| pixel[..3] == [0, 0, 0]));
    assert_refused(&mut renderer, &document);
}

#[test]
fn jpeg_header_dimensions_cannot_hide_behind_a_smaller_xobject_dictionary() {
    let document = source::document(&source::jpeg(LARGE), 32, false);
    assert_refused(&mut renderer(), &document);
}

#[test]
fn an_image_budget_failure_never_returns_a_partial_overlay_page() {
    let document = source::document(&source::jpeg(LARGE), LARGE, true);
    assert_refused(&mut renderer(), &document);
}

#[test]
fn jpeg_budget_charges_actual_rgb_rgba_canvas_and_encoded_capacity() {
    use crate::extractors::images::{ColorSpace, ImageData, PdfImage};
    let encoded = source::jpeg(32);
    let retained_canvas = 32 * 32 * 4;
    let required = 32 * 32 * (3 + 4) + retained_canvas + 2 * encoded.len();
    let image = PdfImage::new(32, 32, ColorSpace::DeviceRGB, 8, ImageData::Jpeg(encoded.clone()));
    assert!(
        image_budget::ImageBudget::new(required)
            .admit_jpeg(&image, retained_canvas as u64)
            .is_ok()
    );
    assert!(
        image_budget::ImageBudget::new(required - 1)
            .admit_jpeg(&image, retained_canvas as u64)
            .is_err()
    );
}

#[test]
fn an_unqualified_cmyk_dictionary_does_not_guess_the_encoded_jpeg_channels() {
    use crate::extractors::images::{ColorSpace, ImageData, PdfImage};
    let image = PdfImage::new(32, 32, ColorSpace::DeviceCMYK, 8, ImageData::Jpeg(source::jpeg(32)));
    let mut budget = image_budget::ImageBudget::new(LIMIT);
    assert!(budget.admit_jpeg(&image, 0).is_err());
    assert!(
        budget.error().is_some(),
        "unqualified source failure must propagate past per-paint catches"
    );
}
