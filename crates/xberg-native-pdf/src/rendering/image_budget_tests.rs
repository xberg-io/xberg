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
    let (image, _) = renderer.render_page_with_budget(0, &document, None).unwrap();
    assert!(image.data.chunks_exact(4).any(|pixel| pixel[..3] == [0, 0, 0]));
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

#[test]
fn an_affordable_inset_image_retains_its_overlay_and_white_page_margin() {
    let document = source::document_with_paint(&source::jpeg(32), 32, true, true);
    let (image, _) = renderer().render_page_with_budget(0, &document, budget()).unwrap();
    for expected in [[0, 0, 0], [255, 0, 0], [255, 255, 255]] {
        assert!(
            image.data.chunks_exact(4).any(|pixel| pixel[..3] == expected),
            "paint {expected:?}"
        );
    }
}

#[test]
fn jpeg_budget_counts_spare_encoded_capacity_and_refuses_arithmetic_overflow() {
    use crate::extractors::images::{ColorSpace, ImageData, PdfImage};
    let mut bytes = source::jpeg(32);
    bytes.reserve_exact(LIMIT);
    let image = PdfImage::new(32, 32, ColorSpace::DeviceRGB, 8, ImageData::Jpeg(bytes));
    assert!(image_budget::ImageBudget::new(LIMIT).admit_jpeg(&image, 0).is_err());
    let mut budget = image_budget::ImageBudget::new(usize::MAX);
    assert!(budget.admit_jpeg(&image, u64::MAX).is_err());
    assert!(budget.error().is_some());
}

#[test]
fn encoded_unsupported_precision_is_not_admitted_as_an_eight_bit_dictionary() {
    use crate::extractors::images::{ColorSpace, ImageData, PdfImage};
    let mut bytes = source::jpeg(32);
    let frame = bytes.windows(2).position(|pair| pair == [0xff, 0xc0]).unwrap();
    bytes[frame + 4] = 12;
    let image = PdfImage::new(32, 32, ColorSpace::DeviceRGB, 8, ImageData::Jpeg(bytes));
    let mut budget = image_budget::ImageBudget::new(LIMIT);
    assert!(budget.admit_jpeg(&image, 0).is_err());
    assert!(budget.error().is_some());
}

#[test]
fn referenced_jpeg_masks_cannot_bypass_the_source_budget() {
    let dimension = 9000;
    let encoded = source::gray_jpeg(dimension);
    let decoded = image::load_from_memory(&encoded).unwrap();
    assert_eq!(decoded.as_bytes().len(), 81_000_000);
    drop(decoded);
    let mut bypasses = Vec::new();
    for soft in [true, false] {
        let document = source::mask_document(&encoded, dimension, soft);
        let (unbounded, _) = renderer().render_page_with_budget(0, &document, None).unwrap();
        let expected = if soft { [0, 0, 0] } else { [255, 255, 255] };
        assert!(unbounded.data.chunks_exact(4).any(|pixel| pixel[..3] == expected));
        for mib in [40, 64] {
            let result = renderer().render_page_with_budget(
                0,
                &document,
                Some(PngRenderBudget {
                    max_working_bytes: mib * 1024 * 1024,
                    minimum_dpi: 72,
                }),
            );
            if let Err(error) = &result {
                assert!(error.to_string().contains("image working budget"), "{error}");
            }
            if result.is_ok() {
                bypasses.push((soft, mib));
            }
        }
    }
    assert!(
        bypasses.is_empty(),
        "81MB JPEG mask decodes bypassed (soft,MiB): {bypasses:?}"
    );
}

#[test]
fn affordable_jpeg_and_non_jpeg_masks_keep_rendering() {
    let raw = source::raw_mask_document();
    let (image, _) = renderer().render_page_with_budget(0, &raw, budget()).unwrap();
    assert!(image.data.chunks_exact(4).any(|pixel| pixel[..3] == [0, 0, 0]));
    let encoded = source::gray_jpeg(32);
    let document = source::mask_document(&encoded, 32, true);
    let mut renderer = renderer();
    assert!(renderer.render_page_with_budget(0, &document, budget()).is_ok());
    let (image, _) = renderer.render_page_with_budget(0, &document, None).unwrap();
    assert!(image.data.chunks_exact(4).any(|pixel| pixel[..3] == [0, 0, 0]));
    assert!(renderer.render_page_with_budget(0, &raw, budget()).is_ok());
}

#[test]
fn inline_jpeg_images_reuse_the_source_image_budget() {
    let large = source::inline_document(&source::gray_jpeg(LARGE), LARGE);
    let (image, _) = renderer().render_page_with_budget(0, &large, None).unwrap();
    assert_eq!((image.width, image.height), (32, 32));
    assert!(image.data.chunks_exact(4).all(|pixel| pixel[..3] == [255, 255, 255]));
    assert_refused(&mut renderer(), &large);
    let small = source::inline_document(&source::gray_jpeg(32), 32);
    assert!(renderer().render_page_with_budget(0, &small, budget()).is_ok());
}

#[test]
fn jpeg_mask_budget_counts_live_parent_buffers_and_luma_conversion() {
    use crate::extractors::images::{ColorSpace, ImageData, PdfImage};
    let encoded = source::gray_jpeg(32);
    let parent = PdfImage::new(32, 32, ColorSpace::DeviceRGB, 8, ImageData::Jpeg(source::jpeg(32)));
    let dynamic = parent.to_dynamic_image().unwrap();
    let paint = dynamic.to_rgba8();
    let mask = PdfImage::new(32, 32, ColorSpace::DeviceGray, 8, ImageData::Jpeg(encoded.clone()));
    let ImageData::Jpeg(mask_bytes) = mask.data() else {
        panic!("encoded mask");
    };
    let parent_bytes = parent.retained_image_buffer_bytes().unwrap() as usize;
    let dynamic_bytes = dynamic.as_rgb8().unwrap().as_raw().capacity();
    let canvas = 32 * 32 * 4;
    let required = parent_bytes
        + dynamic_bytes
        + paint.as_raw().capacity()
        + canvas
        + 32 * 32 * 2
        + mask_bytes.capacity()
        + mask_bytes.len();
    let mut budget = image_budget::ImageBudget::new(required);
    budget
        .admit_referenced_mask(&mask, &parent, &dynamic, &paint, canvas as u64)
        .unwrap();
    let mut budget = image_budget::ImageBudget::new(required - 1);
    assert!(
        budget
            .admit_referenced_mask(&mask, &parent, &dynamic, &paint, canvas as u64)
            .is_err()
    );
    assert!(budget.error().is_some());
}
