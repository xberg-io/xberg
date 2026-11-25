use super::*;
use crate::extractors::images::{ColorSpace, ImageData, PdfImage};
use crate::rendering::page_renderer::image_budget::{ImageBudget, JpegDecode};
use crate::rendering::render_png_page_with_budget_evidence;

/// A tall scan painted over its own pixel extent, like a one-page scanned PDF whose page raster is
/// close to the scan resolution: a 1/2 decode would halve the painted resolution, the full-size RGB
/// decode plus its RGBA copy (10 080 000 bytes) exceed [`LIMIT`], and the RGBA raster alone fits.
const WIDTH: u32 = 600;
const HEIGHT: u32 = 2400;
const PAINTED: &[u8] = b"q 600 0 0 2400 0 0 cm /Im0 Do Q\n";
const EXTENT: (f32, f32) = (600.0, 2400.0);
/// The frame padded to whole 32-column MCUs, three components, 1024 row bytes per column.
const ROW_BYTES: u64 = 608 * 3 * 1024;

fn tall_scan() -> PdfImage {
    PdfImage::new(
        WIDTH,
        HEIGHT,
        ColorSpace::DeviceRGB,
        8,
        ImageData::Jpeg(source::jpeg_of(WIDTH, HEIGHT)),
    )
}

fn in_place_required(image: &PdfImage, canvas: u64) -> u64 {
    let ImageData::Jpeg(bytes) = image.data() else {
        unreachable!()
    };
    u64::from(WIDTH * HEIGHT * 4) + ROW_BYTES + (bytes.capacity() + 2 * bytes.len()) as u64 + canvas
}

#[test]
fn a_tall_scan_that_no_reduction_covers_renders_from_an_in_place_decode() {
    let image = tall_scan();
    assert!(
        ImageBudget::new(LIMIT).admit_jpeg(&image, 0).is_err(),
        "the full decode and its RGBA copy exceed the budget"
    );
    assert!(
        ImageBudget::new(LIMIT).admit_jpeg_reducing(&image, 0, EXTENT).is_err(),
        "no reduced decode covers the painted extent"
    );
    let ImageData::Jpeg(jpeg) = image.data() else {
        unreachable!()
    };
    let document = source::document_painting_of(jpeg, (WIDTH, HEIGHT), PAINTED, "");
    let (rendered, _) = renderer()
        .render_page_with_budget(0, &document, budget())
        .expect("the RGBA raster the page paints from fits the budget");
    assert_eq!((rendered.width, rendered.height), (32, 32));
    assert!(
        is_black_somewhere(&rendered),
        "the in-place decode must paint the source"
    );
}

#[test]
fn an_in_place_decode_is_kept_at_exactly_the_limit_and_refused_one_byte_under() {
    let image = tall_scan();
    let canvas = 1000;
    let required = in_place_required(&image, canvas);
    assert!(required < LIMIT as u64);
    let admit = |limit: u64| ImageBudget::new(limit as usize).admit_jpeg_decode(&image, canvas, EXTENT, true);
    assert_eq!(admit(required).unwrap(), JpegDecode::InPlace);
    let mut budget = ImageBudget::new(required as usize - 1);
    assert!(budget.admit_jpeg_decode(&image, canvas, EXTENT, true).is_err());
    assert!(budget.exceeded(), "the refusal is the budget class");
}

#[test]
fn a_page_whose_in_place_raster_exceeds_the_budget_is_still_refused() {
    let image = tall_scan();
    let ImageData::Jpeg(jpeg) = image.data() else {
        unreachable!()
    };
    let document = source::document_painting_of(jpeg, (WIDTH, HEIGHT), PAINTED, "");
    let limit = 7 * 1024 * 1024;
    assert!(in_place_required(&image, 0) > limit as u64);
    let (error, exceeded) = render_png_page_with_budget_evidence(&document, 0, 72, 72, limit)
        .err()
        .expect("the RGBA raster alone exceeds the budget");
    assert!(exceeded, "{error}");
    assert!(error.to_string().contains("working budget"), "{error}");
}

#[test]
fn a_masked_or_multi_scan_jpeg_is_never_decoded_in_place() {
    let image = tall_scan();
    let mut budget = ImageBudget::new(LIMIT);
    assert!(
        budget.admit_jpeg_decode(&image, 0, EXTENT, false).is_err(),
        "a mask reads the source raster an in-place decode does not keep"
    );
    assert!(budget.exceeded());

    let mut split = source::jpeg_of(WIDTH, HEIGHT);
    assert!(image_budget::single_scan_frame(&split));
    let scan = split.windows(2).position(|pair| pair == [0xff, 0xda]).unwrap();
    assert_eq!(split[scan + 4], 3);
    split[scan + 4] = 1;
    assert!(!image_budget::single_scan_frame(&split));
    let split = PdfImage::new(WIDTH, HEIGHT, ColorSpace::DeviceRGB, 8, ImageData::Jpeg(split));
    assert!(
        ImageBudget::new(LIMIT)
            .admit_jpeg_decode(&split, 0, EXTENT, true)
            .is_err()
    );

    let mut progressive = source::jpeg_of(WIDTH, HEIGHT);
    let frame = source::sof_marker_position(&progressive);
    progressive[frame + 1] = 0xc2;
    assert!(!image_budget::single_scan_frame(&progressive));
}

#[test]
fn an_in_place_decode_matches_the_full_decode_and_its_rgba_conversion() {
    let (width, height) = (37, 23);
    let rgb: Vec<u8> = (0..width * height * 3).map(|index| (index * 7 % 251) as u8).collect();
    let gray: Vec<u8> = (0..width * height).map(|index| (index * 11 % 253) as u8).collect();
    for (pixels, color) in [
        (rgb, image::ExtendedColorType::Rgb8),
        (gray, image::ExtendedColorType::L8),
    ] {
        let mut encoded = Vec::new();
        image::codecs::jpeg::JpegEncoder::new(&mut encoded)
            .encode(&pixels, width, height, color)
            .unwrap();
        let expected = image::load_from_memory(&encoded).unwrap().to_rgba8();
        let decoded = image_budget::decode_jpeg_into_rgba(&encoded, (width, height)).unwrap();
        assert_eq!(decoded, expected, "{color:?}");
        assert!(image_budget::decode_jpeg_into_rgba(&encoded, (width, height + 1)).is_err());
    }
}
