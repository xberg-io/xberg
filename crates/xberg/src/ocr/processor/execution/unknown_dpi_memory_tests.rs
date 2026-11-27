use super::{SecurityLimits, TesseractConfig, prepare_encoded_ocr_image, resolve_known_source_dpi};

fn prepare_unknown_dpi_jpeg(limit_mib: usize, target_dpi: i32) -> crate::Result<super::PreparedOcrImage> {
    let dimension = 1000;
    let rgb = vec![245; dimension as usize * dimension as usize * 3];
    let bytes = crate::extraction::image::jpeg_fixture::baseline_rgb_jpeg(&rgb, dimension, dimension);
    assert!(resolve_known_source_dpi(None, &bytes, dimension.into(), dimension.into()).is_none());
    prepare_encoded_ocr_image(
        &bytes,
        &SecurityLimits {
            max_content_size: limit_mib * 1024 * 1024,
            ..Default::default()
        },
        &TesseractConfig {
            known_full_page_scan: true,
            preprocessing: Some(crate::types::ImagePreprocessingConfig {
                target_dpi,
                ..Default::default()
            }),
            ..Default::default()
        },
        Some(&crate::core::config::ImageExtractionConfig {
            auto_adjust_dpi: false,
            ..Default::default()
        }),
        false,
    )
}

#[test]
fn unknown_dpi_jpeg_cannot_upscale_past_its_image_workspace_limit() {
    let error = match prepare_unknown_dpi_jpeg(40, 300) {
        Ok(prepared) => panic!("unbounded unknown-DPI output {}x{}", prepared.width, prepared.height),
        Err(error) => error.to_string(),
    };
    assert!(error.contains("DPI resize workspace"), "{error}");
    assert!(error.contains("target_dpi"), "{error}");
}

#[test]
fn unknown_dpi_jpeg_fits_at_source_size_under_the_same_limit() {
    let prepared = prepare_unknown_dpi_jpeg(40, 72).expect("source raster and native workspace fit");
    assert_eq!((prepared.width, prepared.height, prepared.source_dpi), (1000, 1000, 72));
}

#[test]
fn unknown_dpi_jpeg_checks_native_budget_even_when_resize_workspace_fits() {
    let error = prepare_unknown_dpi_jpeg(100, 300)
        .err()
        .expect("normalized raster exceeds the native workspace limit")
        .to_string();
    assert!(error.contains("native image workspace after DPI resize"), "{error}");
}

fn densityless_png(width: u32, height: u32) -> Vec<u8> {
    let mut bytes = Vec::new();
    image::DynamicImage::ImageRgb8(image::RgbImage::from_pixel(width, height, image::Rgb([245, 245, 245])))
        .write_to(&mut std::io::Cursor::new(&mut bytes), image::ImageFormat::Png)
        .expect("encode PNG");
    bytes
}

#[test]
fn engine_selected_policy_fits_a_densityless_png_upscale_to_the_workspace() {
    let bytes = densityless_png(800, 1000);
    assert!(resolve_known_source_dpi(None, &bytes, 800, 1000).is_none());
    let prepared = prepare_encoded_ocr_image(
        &bytes,
        &SecurityLimits::default(),
        &TesseractConfig::default(),
        None,
        false,
    )
    .expect("an engine-selected policy shrinks its upscale instead of refusing the page");
    assert!(prepared.width >= 800 && prepared.height >= 1000);
    assert!(u64::from(prepared.width) * u64::from(prepared.height) * 8 <= 100 * 1024 * 1024);
}

#[test]
fn caller_selected_policy_keeps_its_requested_densityless_png_upscale() {
    let bytes = densityless_png(800, 1000);
    let error = prepare_encoded_ocr_image(
        &bytes,
        &SecurityLimits::default(),
        &TesseractConfig {
            preprocessing: Some(Default::default()),
            ..Default::default()
        },
        None,
        false,
    )
    .err()
    .expect("the requested 300 DPI raster does not fit")
    .to_string();
    assert!(error.contains("OCR native image workspace exceeds"), "{error}");
}

fn densityless_jpeg(width: u16, height: u16) -> Vec<u8> {
    let rgb = vec![245; usize::from(width) * usize::from(height) * 3];
    crate::extraction::image::jpeg_fixture::baseline_rgb_jpeg(&rgb, width, height)
}

fn prepare_densityless_jpeg_with_default_policy(
    allow_bounded_inferred_upscale: bool,
) -> crate::Result<super::PreparedOcrImage> {
    let bytes = densityless_jpeg(800, 1000);
    assert!(resolve_known_source_dpi(None, &bytes, 800, 1000).is_none());
    super::encoded_image::prepare_with_dpi_policy(
        &bytes,
        &SecurityLimits::default(),
        &TesseractConfig {
            preprocessing: Some(Default::default()),
            ..Default::default()
        },
        None,
        false,
        allow_bounded_inferred_upscale,
    )
}

#[test]
fn engine_selected_policy_fits_a_densityless_jpeg_upscale_to_the_workspace() {
    let prepared = prepare_densityless_jpeg_with_default_policy(true)
        .expect("an engine-selected policy shrinks its upscale instead of refusing the image");
    assert!(prepared.width > 800 && prepared.height > 1000);
    assert!(u64::from(prepared.width) * u64::from(prepared.height) * 8 <= 100 * 1024 * 1024);
}

#[test]
fn caller_selected_policy_keeps_its_requested_densityless_jpeg_upscale() {
    let error = prepare_densityless_jpeg_with_default_policy(false)
        .err()
        .expect("the requested 300 DPI raster does not fit")
        .to_string();
    assert!(error.contains("native image workspace"), "{error}");
}

/// A real colour page: the receipt is not gray, so it is charged the colour contrast peak (about 17 bytes
/// per pixel) and an engine-selected upscale to 300 DPI cannot fit the default workspace. It shrinks to the
/// largest size that does, instead of failing the retry as a caller-selected policy does. ~keep
#[test]
fn a_real_colour_receipt_shrinks_its_engine_selected_upscale_to_the_default_workspace() {
    let Some(receipt) = crate::utils::read_test_fixture("images/cord_receipt_02.jpg") else {
        return;
    };
    let policy = crate::types::ImagePreprocessingConfig {
        deskew: false,
        contrast_enhance: true,
        binarization_method: "none".into(),
        ..Default::default()
    };
    let prepare = |allow| {
        super::encoded_image::prepare_with_dpi_policy(
            &receipt,
            &SecurityLimits::default(),
            &TesseractConfig {
                preprocessing: Some(policy.clone()),
                ..Default::default()
            },
            None,
            false,
            allow,
        )
    };
    let prepared = prepare(true).expect("the engine-selected upscale shrinks to fit");
    let requested = (3067, 4089);
    assert!(
        prepared.width > 960 && prepared.width < requested.0,
        "{}x{}",
        prepared.width,
        prepared.height
    );
    assert!(
        prepared.height > 1280 && prepared.height < requested.1,
        "{}x{}",
        prepared.width,
        prepared.height
    );
    let error = prepare(false)
        .err()
        .expect("a caller-selected upscale keeps its refusal")
        .to_string();
    assert!(error.contains("native image workspace"), "{error}");
}

/// Ink blocks 8 px wide and 32 px tall on the 8 px JPEG grid decode exactly, so the glyph estimator reads
/// 32 px glyphs and infers a slight (36/32) upscale to 300 DPI. ~keep
fn glyph_block_jpeg(width: u16, height: u16) -> Vec<u8> {
    let (columns, rows) = (usize::from(width), usize::from(height));
    let mut rgb = vec![255; columns * rows * 3];
    for y in (0..rows).filter(|y| y % 64 < 32) {
        for x in (0..columns).filter(|x| x % 32 < 8) {
            let offset = (y * columns + x) * 3;
            rgb[offset..offset + 3].fill(0);
        }
    }
    crate::extraction::image::jpeg_fixture::baseline_rgb_jpeg(&rgb, width, height)
}

/// ndl_meiji_vertical_01.jpg (3630x2777, no density): the inferred 300 DPI raster fits the native phase,
/// but the resize that produces it holds the source, intermediate and destination RGB at once and does
/// not. The engine-selected upscale shrinks to the largest size that fits instead of failing. ~keep
#[test]
fn inferred_upscale_whose_resize_alone_exceeds_the_limit_shrinks_to_fit() {
    let (width, height) = (3630, 2777);
    let bytes = glyph_block_jpeg(width as u16, height as u16);
    assert!(resolve_known_source_dpi(None, &bytes, width, height).is_none());
    let policy = crate::types::ImagePreprocessingConfig::default();
    let decoded = image::load_from_memory(&bytes).expect("decode fixture").to_rgb8();
    let source_dpi = crate::image::preprocessing::inferred_raster_dpi(
        decoded.as_raw(),
        width as usize,
        height as usize,
        policy.target_dpi,
    )
    .expect("the glyph blocks give an inferred resolution");
    let dpi_config = super::ocr_dpi_config(&policy, None);
    let requested =
        crate::image::preprocessing::normalized_image_dimensions(width, height, &dpi_config, Some(source_dpi));
    assert!(
        requested.0 > width && requested.1 > height,
        "{requested:?} at {source_dpi} DPI"
    );
    let limit = super::native_image_budget::known_phase_peak(requested.0, requested.1, bytes.len(), Some(&policy))
        .unwrap()
        .expect("default policy has a modelled native phase") as usize;
    let resize_error =
        super::dpi_workspace::check_known_dpi_resize(width, height, source_dpi, &dpi_config, bytes.len(), limit)
            .expect_err("the requested resize workspace exceeds the native-phase limit");
    assert!(
        resize_error.to_string().contains("DPI resize workspace"),
        "{resize_error}"
    );

    let prepared = prepare_encoded_ocr_image(
        &bytes,
        &SecurityLimits {
            max_content_size: limit,
            ..Default::default()
        },
        &TesseractConfig {
            known_full_page_scan: true,
            ..Default::default()
        },
        None,
        false,
    )
    .expect("the engine-selected upscale shrinks until its resize fits");
    assert!(
        prepared.width > width && prepared.width < requested.0,
        "{}x{} for requested {requested:?}",
        prepared.width,
        prepared.height
    );
    assert!(
        prepared.height > height && prepared.height < requested.1,
        "{}x{} for requested {requested:?}",
        prepared.width,
        prepared.height
    );
}

#[test]
fn ndl_meiji_scan_upscales_within_the_default_limit() {
    let Some(scan) = crate::utils::read_test_fixture("images/ndl_meiji_vertical_01.jpg") else {
        return;
    };
    let prepared = prepare_encoded_ocr_image(
        &scan,
        &SecurityLimits::default(),
        &TesseractConfig::default(),
        None,
        false,
    )
    .expect("the inferred upscale shrinks to the default workspace instead of failing the image");
    assert!(
        prepared.width > 3630 && prepared.height > 2777,
        "{}x{}",
        prepared.width,
        prepared.height
    );
    assert!(
        prepared.width < 4092 && prepared.height < 3130,
        "{}x{}",
        prepared.width,
        prepared.height
    );
}
