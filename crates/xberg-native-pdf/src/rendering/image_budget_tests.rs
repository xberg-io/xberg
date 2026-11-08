use super::*;
#[path = "image_budget_tests/source.rs"]
mod source;

const LIMIT: usize = 8 * 1024 * 1024;
/// Canvas pixels the 32 pt fixture pages are painted over at 72 dpi, far below any reduced scale of
/// the oversized fixtures.
const DEVICE: (f32, f32) = (32.0, 32.0);
/// Refuses the 4096 px fixtures even at 1/8 (about 1.8 MiB of rasters) while admitting the 32 px page.
const TIGHT: usize = 1024 * 1024;
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

fn tight_budget() -> Option<PngRenderBudget> {
    Some(PngRenderBudget {
        max_working_bytes: TIGHT,
        minimum_dpi: 72,
    })
}

fn assert_refused(renderer: &mut PageRenderer, document: &PdfDocument) {
    let error = renderer
        .render_page_with_budget(0, document, tight_budget())
        .err()
        .expect("source JPEG rasters exceed the supplied working-image budget");
    assert!(error.to_string().contains("image"), "{error}");
    assert!(error.to_string().contains("budget"), "{error}");
}

#[test]
fn render_bridge_marks_only_a_qualified_source_image_budget_excess() {
    let oversized = source::document(&source::jpeg(LARGE), LARGE, false);
    let (error, exceeded) = super::super::render_png_page_with_budget_evidence(&oversized, 0, 72, 72, TIGHT)
        .err()
        .expect("large source cannot fit");
    assert!(matches!(error, crate::Error::Image(_)));
    assert!(exceeded);

    let mut malformed = source::jpeg(32);
    let frame = malformed.windows(2).position(|pair| pair == [0xff, 0xc0]).unwrap();
    malformed[frame + 4] = 12;
    let unqualified = source::document(&malformed, 32, false);
    let (error, exceeded) = super::super::render_png_page_with_budget_evidence(&unqualified, 0, 72, 72, LIMIT)
        .err()
        .expect("unqualified source cannot render");
    assert!(matches!(error, crate::Error::Image(_)));
    assert!(!exceeded);
}

#[test]
fn source_jpeg_budget_failure_does_not_poison_a_reused_renderer() {
    let document = source::document(&source::jpeg(LARGE), LARGE, false);
    let mut renderer = renderer();
    assert_refused(&mut renderer, &document);
    let small = source::document(&source::jpeg(32), 32, false);
    let (image, _) = renderer.render_page_with_budget(0, &small, budget()).unwrap();
    assert_eq!((image.width, image.height), (32, 32));
    assert!(
        image
            .data
            .as_chunks::<4>()
            .0
            .iter()
            .any(|pixel| pixel[..3] == [0, 0, 0])
    );
}

#[test]
fn prior_unbounded_render_cannot_bypass_a_later_source_image_budget() {
    let document = source::document(&source::jpeg(LARGE), LARGE, false);
    let mut renderer = renderer();
    let (image, _) = renderer.render_page_with_budget(0, &document, None).unwrap();
    assert!(
        image
            .data
            .as_chunks::<4>()
            .0
            .iter()
            .any(|pixel| pixel[..3] == [0, 0, 0])
    );
    assert_refused(&mut renderer, &document);
    let (image, _) = renderer.render_page_with_budget(0, &document, None).unwrap();
    assert!(
        image
            .data
            .as_chunks::<4>()
            .0
            .iter()
            .any(|pixel| pixel[..3] == [0, 0, 0])
    );
}

#[test]
fn jpeg_header_dimensions_cannot_hide_behind_a_smaller_xobject_dictionary() {
    let document = source::document(&source::jpeg(LARGE), 32, false);
    // The 4096 px frame would fit at 1/8 of the budget if the dictionary were trusted; a mismatch is
    // not reduced and is refused with the budget refusal, under a limit that would admit a reduction.
    let (error, exceeded) = super::super::render_png_page_with_budget_evidence(&document, 0, 72, 72, LIMIT)
        .err()
        .expect("a frame that disagrees with its dictionary is refused");
    assert!(exceeded, "{error}");
    assert!(error.to_string().contains("working budget"), "{error}");
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
            image.data.as_chunks::<4>().0.iter().any(|pixel| pixel[..3] == expected),
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
        assert!(
            unbounded
                .data
                .as_chunks::<4>()
                .0
                .iter()
                .any(|pixel| pixel[..3] == expected)
        );
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
    assert!(
        image
            .data
            .as_chunks::<4>()
            .0
            .iter()
            .any(|pixel| pixel[..3] == [0, 0, 0])
    );
    let encoded = source::gray_jpeg(32);
    let document = source::mask_document(&encoded, 32, true);
    let mut renderer = renderer();
    assert!(renderer.render_page_with_budget(0, &document, budget()).is_ok());
    let (image, _) = renderer.render_page_with_budget(0, &document, None).unwrap();
    assert!(
        image
            .data
            .as_chunks::<4>()
            .0
            .iter()
            .any(|pixel| pixel[..3] == [0, 0, 0])
    );
    assert!(renderer.render_page_with_budget(0, &raw, budget()).is_ok());
}

#[test]
fn inline_jpeg_images_reuse_the_source_image_budget() {
    let large = source::inline_document(&source::gray_jpeg(LARGE), LARGE);
    let (image, _) = renderer().render_page_with_budget(0, &large, None).unwrap();
    assert_eq!((image.width, image.height), (32, 32));
    assert!(
        !image.data.is_empty(),
        "the white-page check must inspect rendered pixels"
    );
    assert!(
        image
            .data
            .as_chunks::<4>()
            .0
            .iter()
            .all(|pixel| pixel[..3] == [255, 255, 255])
    );
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

#[test]
fn text_contrast_probe_declines_raw_and_masked_backgrounds() {
    let jpeg = source::jpeg(32);
    let raw = vec![0; 32 * 32 * 3];
    for (encoding, pixels, expected) in [
        ("/Filter /DCTDecode", jpeg.as_slice(), true),
        ("", raw.as_slice(), false),
        (
            "/Filter /DCTDecode /Mask [255 255 255 255 255 255]",
            jpeg.as_slice(),
            false,
        ),
    ] {
        let doc = source::contrast_title_document(pixels, encoding);
        let chars = doc.extract_chars(0).unwrap();
        let [title] = chars.as_slice() else {
            panic!("expected one native glyph")
        };
        assert_eq!(title.char, 'A');
        assert_eq!(
            super::super::exact_text_show_has_contrast(&doc, b"A", &[title.bbox], LIMIT).unwrap(),
            expected,
            "{encoding}"
        );
    }
}

#[test]
fn wrapped_jpeg_contrast_probe_charges_filters_and_actual_codestream_pixels() {
    const FILTERS: &str = "/Filter [/ASCII85Decode /DCTDecode]";
    let probe = |stored: &[u8], budget| {
        let doc = source::contrast_title_document(stored, FILTERS);
        let glyph = doc.extract_chars(0).unwrap().remove(0);
        super::super::exact_text_show_has_contrast(&doc, b"A", &[glyph.bbox], budget)
    };
    let small = source::wrap_ascii85(&source::jpeg(32));
    assert!(probe(&small, LIMIT).unwrap());
    assert!(!probe(&small, small.len() * 16).unwrap());

    let large = source::wrap_ascii85(&source::jpeg(LARGE));
    // The probe reserves its own raster and sixteen copies of the stored stream before the source
    // budget; leave 512 KiB, too little for any reduced decode of the fixture. ~keep
    let error = probe(&large, 16 * large.len() + (1 << 20) + TIGHT / 2).unwrap_err();
    assert!(error.to_string().contains("JPEG source image rasters"), "{error}");
    assert!(error.to_string().contains("working budget"), "{error}");
}

fn is_black_somewhere(image: &crate::rendering::RenderedImage) -> bool {
    image
        .data
        .as_chunks::<4>()
        .0
        .iter()
        .any(|pixel| pixel[..3] == [0, 0, 0])
}

#[test]
fn an_oversized_jpeg_is_decoded_at_the_finest_reduced_scale_that_fits() {
    use crate::extractors::images::{ColorSpace, ImageData, PdfImage};
    let encoded = source::jpeg(1200);
    let image = PdfImage::new(1200, 1200, ColorSpace::DeviceRGB, 8, ImageData::Jpeg(encoded.clone()));
    assert!(image_budget::ImageBudget::new(LIMIT).admit_jpeg(&image, 0).is_err());
    for (limit, expected) in [(32 * 1024 * 1024, 1), (LIMIT, 2), (1_500_000, 4), (700_000, 8)] {
        let denominator = image_budget::ImageBudget::new(limit)
            .admit_jpeg_reducing(&image, 0, DEVICE)
            .unwrap_or_else(|error| panic!("limit {limit}: {error}"));
        assert_eq!(denominator, expected, "limit {limit}");
    }
    let decoded = image_budget::decode_reduced_jpeg(&encoded, 4).unwrap();
    assert_eq!((decoded.width(), decoded.height()), (300, 300));
    assert!(
        image_budget::ImageBudget::new(500_000)
            .admit_jpeg_reducing(&image, 0, DEVICE)
            .is_err()
    );
}

#[test]
fn a_reduced_decode_never_drops_far_below_the_canvas_resolution() {
    use crate::extractors::images::{ColorSpace, ImageData, PdfImage};
    let image = PdfImage::new(
        1200,
        1200,
        ColorSpace::DeviceRGB,
        8,
        ImageData::Jpeg(source::jpeg(1200)),
    );
    let admit = |device| image_budget::ImageBudget::new(LIMIT).admit_jpeg_reducing(&image, 0, device);
    assert_eq!(admit((600.0, 600.0)).unwrap(), 2);
    // reduced >= 0.9 * painted - 1, so a 600 px axis accepts a painted extent up to 667.78 px.
    assert_eq!(admit((667.0, 667.0)).unwrap(), 2);
    assert_eq!(admit((667.0, 600.0)).unwrap(), 2);
    assert_eq!(admit((600.0, 667.0)).unwrap(), 2);
    assert!(admit((668.0, 600.0)).is_err(), "width 668 is the first refused extent");
    assert!(admit((600.0, 668.0)).is_err(), "height 668 is the first refused extent");
    assert!(admit((700.0, 700.0)).is_err());
}

#[test]
fn a_jpeg_that_fits_keeps_its_full_size_decode() {
    use crate::extractors::images::{ColorSpace, ImageData, PdfImage};
    let image = PdfImage::new(32, 32, ColorSpace::DeviceRGB, 8, ImageData::Jpeg(source::jpeg(32)));
    assert_eq!(
        image_budget::ImageBudget::new(LIMIT)
            .admit_jpeg_reducing(&image, 0, DEVICE)
            .unwrap(),
        1
    );
}

#[test]
fn a_progressive_jpeg_is_refused_not_reduced() {
    use crate::extractors::images::{ColorSpace, ImageData, PdfImage};
    let mut progressive = source::jpeg(1200);
    let frame = progressive.windows(2).position(|pair| pair == [0xff, 0xc0]).unwrap();
    progressive[frame + 1] = 0xc2;
    let image = PdfImage::new(1200, 1200, ColorSpace::DeviceRGB, 8, ImageData::Jpeg(progressive));
    let mut budget = image_budget::ImageBudget::new(LIMIT);
    assert!(budget.admit_jpeg_reducing(&image, 0, DEVICE).is_err());
    assert!(budget.exceeded());
}

#[test]
fn a_cmyk_jpeg_is_refused_not_reduced() {
    use crate::extractors::images::{ColorSpace, ImageData, PdfImage};
    let cmyk = PdfImage::new(
        1200,
        1200,
        ColorSpace::DeviceCMYK,
        8,
        ImageData::Jpeg(source::cmyk_jpeg(1200)),
    );
    let ImageData::Jpeg(bytes) = cmyk.data() else {
        unreachable!()
    };
    assert!(
        image_budget::jpeg_frame(bytes).is_ok(),
        "the fixture is a readable four-component frame"
    );
    assert!(
        !image_budget::reducible_jpeg(&cmyk, bytes),
        "CMYK is excluded from reduction"
    );
    let mut budget = image_budget::ImageBudget::new(LIMIT);
    assert!(budget.admit_jpeg_reducing(&cmyk, 0, DEVICE).is_err());
    assert!(
        budget.exceeded(),
        "the refusal is the budget class, not an unqualified source"
    );
}

#[test]
fn a_truncated_jpeg_whose_reduced_decode_fails_is_refused_not_rendered_blank() {
    let mut truncated = source::jpeg(LARGE);
    let scan = truncated.windows(2).position(|pair| pair == [0xff, 0xda]).unwrap();
    truncated.truncate(scan + 16);
    let document = source::document(&truncated, LARGE, false);
    let (error, exceeded) = super::super::render_png_page_with_budget_evidence(&document, 0, 72, 72, LIMIT)
        .err()
        .expect("a page whose only image cannot be decoded must be refused, not rendered white");
    assert!(matches!(error, crate::Error::Image(_)), "{error}");
    assert!(exceeded);
    let inline = source::inline_document(&truncated_gray(), LARGE);
    assert!(
        super::super::render_png_page_with_budget_evidence(&inline, 0, 72, 72, LIMIT).is_err(),
        "the inline image path records the same refusal"
    );
}

fn truncated_gray() -> Vec<u8> {
    let mut bytes = source::gray_jpeg(LARGE);
    let scan = bytes.windows(2).position(|pair| pair == [0xff, 0xda]).unwrap();
    bytes.truncate(scan + 16);
    bytes
}

#[test]
fn a_page_whose_source_jpeg_exceeds_the_budget_renders_from_a_reduced_decode() {
    let document = source::document(&source::jpeg(LARGE), LARGE, false);
    let (image, _) = renderer().render_page_with_budget(0, &document, budget()).unwrap();
    assert_eq!((image.width, image.height), (32, 32));
    assert!(
        is_black_somewhere(&image),
        "the reduced decode must still paint the source"
    );
    let inline = source::inline_document(&source::gray_jpeg_of(LARGE, 0), LARGE);
    let (image, _) = renderer().render_page_with_budget(0, &inline, budget()).unwrap();
    assert!(
        is_black_somewhere(&image),
        "the inline reduced decode must paint the dark image"
    );
}

#[test]
fn a_frame_the_decoder_would_scale_differently_is_never_admitted_at_that_scale() {
    use crate::extractors::images::{ColorSpace, ImageData, PdfImage};
    // `jpeg-decoder` reaches a one-pixel-wide request on either axis, so at 1/2 and 1/4 it would pick 1/8.
    let encoded = {
        let pixels = vec![0; 4096 * 3];
        let mut bytes = Vec::new();
        image::codecs::jpeg::JpegEncoder::new(&mut bytes)
            .encode(&pixels, 1, 4096, image::ExtendedColorType::Rgb8)
            .unwrap();
        bytes
    };
    let image = PdfImage::new(1, 4096, ColorSpace::DeviceRGB, 8, ImageData::Jpeg(encoded.clone()));
    let limit = 20_000;
    assert!(image_budget::ImageBudget::new(limit).admit_jpeg(&image, 0).is_err());
    let admit = |device| image_budget::ImageBudget::new(limit).admit_jpeg_reducing(&image, 0, device);
    assert_eq!(admit((1.0, 100.0)).unwrap(), 8);
    let mut budget = image_budget::ImageBudget::new(limit);
    assert!(budget.admit_jpeg_reducing(&image, 0, (1.0, 2000.0)).is_err());
    assert!(
        budget.exceeded(),
        "a refusal must be the budget refusal, not a decode error"
    );
    assert_eq!(image_budget::decode_reduced_jpeg(&encoded, 8).unwrap().height(), 512);
}
