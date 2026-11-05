use super::{retained_ocr_source_bytes, upright_raster_for_backend};
use crate::core::config::OcrConfig;
use crate::extractors::image::reserve_retained_ocr_raster;
use crate::extractors::security::SecurityLimits;
use crate::plugins::PageOrientationHandling;
use crate::plugins::registry::get_ocr_backend_registry;
use image::ImageEncoder;
use std::sync::Arc;

#[test]
fn rotated_png_reserves_the_distinct_original_allocation_for_both_pdf_routes() {
    const SOURCE_CAPACITY: usize = 32 * 1024;
    const LIMIT: usize = 128 * 1024;
    let raster = image::RgbImage::from_pixel(8, 16, image::Rgb([21, 34, 55]));
    let mut encoded = Vec::with_capacity(SOURCE_CAPACITY);
    image::codecs::png::PngEncoder::new(&mut encoded)
        .write_image(raster.as_raw(), 8, 16, image::ColorType::Rgb8.into())
        .unwrap();
    let original = Arc::new(encoded);
    let upright = |angle| {
        upright_raster_for_backend(&original, 8, 16, angle, PageOrientationHandling::RequiresUpright, None).unwrap()
    };
    let (same, width, height, angle) = upright(0);
    assert!(Arc::ptr_eq(&original, &same));
    assert_eq!((width, height, angle), (8, 16, 0));
    let (rotated, width, height, angle) = upright(90);
    assert!(!Arc::ptr_eq(&original, &rotated));
    assert_eq!((width, height, angle), (16, 8, 90));
    assert!(original.capacity() > original.len().max(rotated.capacity()));

    crate::plugins::ensure_ocr_backends_initialized();
    let backend = get_ocr_backend_registry().read().get("tesseract").unwrap();
    assert!(get_ocr_backend_registry().read().is_builtin_tesseract(&backend));
    let config = || OcrConfig {
        security_limits: Some(SecurityLimits {
            max_content_size: LIMIT,
            ..Default::default()
        }),
        ..Default::default()
    };
    let remaining = |retained| {
        let (original, effective) = reserve_retained_ocr_raster(&backend, (config(), config()), retained).unwrap();
        [
            original.security_limits.unwrap().max_content_size,
            effective.security_limits.unwrap().max_content_size,
        ]
    };

    for raster_bytes in [0, raster.as_raw().len()] {
        let unrotated = retained_ocr_source_bytes(&original, &same, raster_bytes);
        assert_eq!(remaining(unrotated), [LIMIT - raster_bytes; 2]);

        let rotated = retained_ocr_source_bytes(&original, &rotated, raster_bytes);
        assert_eq!(remaining(rotated), [LIMIT - raster_bytes - original.capacity(); 2]);
    }
    let overflow = retained_ocr_source_bytes(&original, &rotated, usize::MAX);
    assert!(reserve_retained_ocr_raster(&backend, (config(), config()), overflow).is_err());
}
