use std::io::Cursor;
use std::num::NonZeroU32;
use std::sync::Arc;

use super::{RasterizeOptions, pages_for_call_with_options, pages_for_call_with_security_limits};
use crate::engine::parsed::ParsedDocument;
use crate::extractors::security::SecurityLimits;
use crate::heuristics::StructuredCallMode;

fn bounded(max_edge: u32) -> RasterizeOptions {
    RasterizeOptions {
        max_image_edge: NonZeroU32::new(max_edge),
    }
}

fn rgb_image(width: u32, height: u32, format: image::ImageFormat) -> Vec<u8> {
    let image = image::DynamicImage::ImageRgb8(image::RgbImage::from_pixel(width, height, image::Rgb([12, 34, 56])));
    let mut bytes = Vec::new();
    image
        .write_to(&mut Cursor::new(&mut bytes), format)
        .expect("encode a valid bounded synthetic image");
    bytes
}

fn limits(max_content_size: usize) -> SecurityLimits {
    SecurityLimits {
        max_content_size,
        ..Default::default()
    }
}

#[tokio::test]
async fn configured_max_edge_allows_sixteen_megapixel_jpeg_under_default_budget() {
    let bytes = rgb_image(4_625, 3_496, image::ImageFormat::Jpeg);
    let pages = pages_for_call_with_options(
        &bytes,
        "image/jpeg",
        StructuredCallMode::VisionOnly,
        200,
        &SecurityLimits::default(),
        bounded(1_568),
    )
    .await
    .expect("the caller's model-size cap must precede PNG encoding");

    assert_eq!(pages.len(), 1);
    assert_eq!(pages[0].page_number, 1);
    let output = image::load_from_memory(&pages[0].png_bytes).expect("decode the actual returned PNG");
    assert_eq!((output.width(), output.height()), (1_568, 1_185));
}

#[tokio::test]
async fn configured_max_edge_does_not_bypass_the_source_decode_budget() {
    let bytes = crate::extraction::image_decode::bmp_with_declared_dimensions(6_000, 6_000);
    let error = pages_for_call_with_options(
        &bytes,
        "image/bmp",
        StructuredCallMode::VisionOnly,
        200,
        &limits(1_024),
        bounded(64),
    )
    .await
    .expect_err("a small requested output cannot authorize an oversized source allocation");

    assert!(error.to_string().contains("failed to decode image"));
    assert!(
        error
            .to_string()
            .contains("security_limits.max_content_size (1024 bytes)")
    );
}

#[tokio::test]
async fn resizing_accounts_for_source_and_destination_live_at_once() {
    let error = pages_for_call_with_options(
        &rgb_image(256, 128, image::ImageFormat::Png),
        "image/png",
        StructuredCallMode::VisionOnly,
        200,
        &limits(150_000),
        bounded(255),
    )
    .await
    .expect_err("the resize destination cannot be allocated outside the live-image budget");

    assert!(
        error
            .to_string()
            .contains("failed to resize image within security budget")
    );
    assert!(
        error
            .to_string()
            .contains("security_limits.max_content_size (150000 bytes)")
    );
}

#[tokio::test]
async fn resized_png_encoding_still_obeys_the_callers_budget() {
    let error = pages_for_call_with_options(
        &rgb_image(256, 128, image::ImageFormat::Png),
        "image/png",
        StructuredCallMode::VisionOnly,
        200,
        &limits(200_000),
        bounded(64),
    )
    .await
    .expect_err("a permitted resize must not bypass the later PNG encoding budget");

    assert!(
        error
            .to_string()
            .contains("failed to encode image within security budget")
    );
    assert!(
        error
            .to_string()
            .contains("security_limits.max_content_size (200000 bytes)")
    );
}

#[tokio::test]
async fn max_edge_never_upscales_or_changes_an_already_small_image() {
    let bytes = rgb_image(256, 128, image::ImageFormat::Png);
    let selected = limits(600_000);
    let legacy =
        pages_for_call_with_security_limits(&bytes, "image/png", StructuredCallMode::VisionOnly, 200, &selected)
            .await
            .expect("legacy small-image rendering succeeds");
    let bounded = pages_for_call_with_options(
        &bytes,
        "image/png",
        StructuredCallMode::VisionOnly,
        200,
        &selected,
        bounded(1_568),
    )
    .await
    .expect("an already small image stays unchanged");

    assert_eq!(legacy.len(), 1);
    assert_eq!(bounded.len(), 1);
    assert_eq!(bounded[0].png_bytes, legacy[0].png_bytes);
}

#[test]
fn render_memo_distinguishes_each_max_edge_and_the_legacy_full_size() {
    let memo = ParsedDocument::from_bytes_with_security_limits(
        Arc::from(rgb_image(256, 128, image::ImageFormat::Png)),
        &limits(600_000),
    )
    .expect("construct a lazy render memo");

    for (options, expected) in [
        (bounded(64), (64, 32)),
        (bounded(128), (128, 64)),
        (RasterizeOptions::default(), (256, 128)),
        (bounded(64), (64, 32)),
    ] {
        let pages = memo
            .rendered_pages_with_options(200, options)
            .expect("render using the selected max edge, not a differently sized cached value");
        assert_eq!(pages.len(), 1);
        let image = image::load_from_memory(&pages[0].png_bytes).expect("decode the actual memoized PNG");
        assert_eq!((image.width(), image.height()), expected);
    }
    let pages = memo
        .rendered_pages(200)
        .expect("legacy rendering retains its own full-size output");
    let image = image::load_from_memory(&pages[0].png_bytes).expect("decode the legacy memoized PNG");
    assert_eq!((image.width(), image.height()), (256, 128));
}
