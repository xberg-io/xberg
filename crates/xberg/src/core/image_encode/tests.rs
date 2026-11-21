use super::*;

/// Thin wrapper around `re_encode` that supplies default `SvgOptions` when the
/// `svg` feature is active, so callers in this test module need not repeat the
/// cfg-gated argument at every call site.
fn re_encode_default(image: &mut ExtractedImage, target: ImageOutputFormat) -> Result<bool, EncodeWarning> {
    re_encode(
        image,
        target,
        &SecurityLimits::default(),
        #[cfg(feature = "svg")]
        &SvgOptions::default(),
    )
}

#[test]
fn reencode_honors_request_security_limit() {
    let original = make_png_bytes();
    let mut image = make_image(original.clone(), "png");
    let limits = SecurityLimits {
        max_content_size: 1_024,
        ..Default::default()
    };

    let result = re_encode(
        &mut image,
        ImageOutputFormat::Jpeg { quality: 85 },
        &limits,
        #[cfg(feature = "svg")]
        &SvgOptions::default(),
    );

    assert!(matches!(result, Err(EncodeWarning::EncodeFailed { .. })));
    assert_eq!(image.data, original, "a rejected re-encode must preserve source bytes");
}

/// Create a minimal valid 4×4 PNG image as `Bytes` for use in tests.
fn make_png_bytes() -> Bytes {
    let img = image::RgbImage::new(4, 4);
    let mut buf: Vec<u8> = Vec::new();
    DynamicImage::ImageRgb8(img)
        .write_to(&mut Cursor::new(&mut buf), ImageFormat::Png)
        .expect("test PNG encode");
    Bytes::from(buf)
}

/// Create a minimal valid 4×4 JPEG image as `Bytes` for use in tests.
fn make_jpeg_bytes() -> Bytes {
    use image::codecs::jpeg::JpegEncoder;
    let img = image::RgbImage::new(4, 4);
    let mut buf: Vec<u8> = Vec::new();
    JpegEncoder::new_with_quality(&mut buf, 85)
        .encode_image(&DynamicImage::ImageRgb8(img))
        .expect("test JPEG encode");
    Bytes::from(buf)
}

/// Create a minimal `ExtractedImage` from raw bytes and a format string.
fn make_image(data: Bytes, format: &'static str) -> ExtractedImage {
    ExtractedImage {
        data,
        format: Cow::Borrowed(format),
        ..Default::default()
    }
}

#[test]
fn native_target_no_op() {
    let original_data = make_png_bytes();
    let mut image = make_image(original_data.clone(), "png");
    let result = re_encode_default(&mut image, ImageOutputFormat::Native);
    assert!(matches!(result, Ok(false)), "Native must return Ok(false)");
    assert_eq!(image.data, original_data, "bytes must be untouched");
    assert_eq!(image.format.as_ref(), "png", "format must be untouched");
}

#[test]
fn same_format_no_op() {
    let original_data = make_png_bytes();
    let mut image = make_image(original_data.clone(), "png");
    let result = re_encode_default(&mut image, ImageOutputFormat::Png);
    assert!(matches!(result, Ok(false)), "already-PNG → Png must return Ok(false)");
    assert_eq!(image.data, original_data, "bytes must be untouched");
}

#[test]
fn png_to_jpeg() {
    let mut image = make_image(make_png_bytes(), "png");
    let result = re_encode_default(&mut image, ImageOutputFormat::Jpeg { quality: 85 });
    assert!(
        matches!(result, Ok(true)),
        "png→jpeg must return Ok(true); got {result:?}"
    );
    assert_eq!(image.format.as_ref(), "jpeg");
    let guessed = image::guess_format(&image.data).expect("should detect valid JPEG");
    assert_eq!(guessed, ImageFormat::Jpeg);
}

#[test]
fn jpeg_to_png() {
    let mut image = make_image(make_jpeg_bytes(), "jpeg");
    let result = re_encode_default(&mut image, ImageOutputFormat::Png);
    assert!(
        matches!(result, Ok(true)),
        "jpeg→png must return Ok(true); got {result:?}"
    );
    assert_eq!(image.format.as_ref(), "png");
    let guessed = image::guess_format(&image.data).expect("should detect valid PNG");
    assert_eq!(guessed, ImageFormat::Png);
}

#[test]
fn png_to_webp() {
    let mut image = make_image(make_png_bytes(), "png");
    let result = re_encode_default(&mut image, ImageOutputFormat::Webp { quality: 80 });
    assert!(
        matches!(result, Ok(true)),
        "png→webp must return Ok(true); got {result:?}"
    );
    assert_eq!(image.format.as_ref(), "webp");
    let guessed = image::guess_format(&image.data).expect("should detect valid WebP");
    assert_eq!(guessed, ImageFormat::WebP);
}

/// Without `svg` feature: SVG is untranslatable → `Err(Undecodable)`.
/// With `svg` feature: SVG → PNG rasterizes successfully → `Ok(true)`.
#[test]
fn svg_to_png_behaviour() {
    let svg_bytes = Bytes::from_static(b"<svg xmlns=\"http://www.w3.org/2000/svg\" width=\"4\" height=\"4\"/>");
    let original_data = svg_bytes.clone();
    let mut image = make_image(svg_bytes, "svg");
    let result = re_encode_default(&mut image, ImageOutputFormat::Png);
    #[cfg(not(feature = "svg"))]
    assert!(
        matches!(result, Err(EncodeWarning::Undecodable { ref source_format }) if source_format == "svg"),
        "svg (no svg feature) must return Err(Undecodable); got {result:?}",
    );
    #[cfg(not(feature = "svg"))]
    {
        assert_eq!(image.data, original_data);
        assert_eq!(image.format.as_ref(), "svg");
    }
    #[cfg(feature = "svg")]
    assert!(
        matches!(result, Ok(true)),
        "svg→png (svg feature) must return Ok(true); got {result:?}",
    );
    #[cfg(feature = "svg")]
    {
        assert_eq!(image.format.as_ref(), "png");
        let _ = original_data;
    }
}

#[test]
fn corrupt_png_decode_fails() {
    let corrupt = Bytes::from_static(b"\x89PNG\r\n\x1a\ncorrupt garbage bytes here");
    let original_data = corrupt.clone();
    let mut image = make_image(corrupt, "png");
    let result = re_encode_default(&mut image, ImageOutputFormat::Jpeg { quality: 85 });
    assert!(
        matches!(result, Err(EncodeWarning::DecodeFailed { ref source_format, .. }) if source_format == "png"),
        "corrupt PNG must return Err(DecodeFailed); got {result:?}",
    );
    assert_eq!(image.data, original_data, "bytes must be untouched on decode failure");
}

#[test]
fn should_reject_oversized_declared_dimensions_before_reencoding() {
    let oversized = crate::extraction::image_decode::bmp_with_declared_dimensions(6_000, 6_000);
    let mut image = make_image(Bytes::from(oversized), "bmp");

    let result = re_encode_default(&mut image, ImageOutputFormat::Png);

    assert!(
        matches!(result, Err(EncodeWarning::DecodeFailed { ref message, .. }) if message.contains("security_limits.max_content_size")),
        "oversized image must fail at the decoded-image budget; got {result:?}"
    );
}

#[test]
fn unknown_format_auto_detects() {
    let png_bytes = make_png_bytes();
    let mut image = make_image(png_bytes, "unknown");
    let result = re_encode_default(&mut image, ImageOutputFormat::Jpeg { quality: 85 });
    assert!(
        matches!(result, Ok(true)),
        "unknown-format valid PNG→jpeg must return Ok(true); got {result:?}"
    );
    assert_eq!(image.format.as_ref(), "jpeg");
}

#[test]
fn quality_out_of_range_clamps() {
    let mut image = make_image(make_png_bytes(), "png");
    let result = re_encode_default(&mut image, ImageOutputFormat::Jpeg { quality: 200 });
    assert!(
        matches!(result, Ok(true)),
        "quality 200 should clamp and encode; got {result:?}"
    );
    assert_eq!(image.format.as_ref(), "jpeg");
}

#[cfg(feature = "svg")]
#[test]
fn svg_sanitize_pass_on_native_target() {
    let svg_bytes = Bytes::from_static(b"<svg xmlns=\"http://www.w3.org/2000/svg\" width=\"4\" height=\"4\"/>");
    let mut image = make_image(svg_bytes, "svg");
    let opts = SvgOptions {
        sanitize: true,
        render_dpi: 96.0,
    };
    let result = re_encode(&mut image, ImageOutputFormat::Native, &SecurityLimits::default(), &opts);
    assert!(
        matches!(result, Ok(true)),
        "SVG sanitize on Native must return Ok(true); got {result:?}"
    );
    assert_eq!(image.format.as_ref(), "svg", "format must remain 'svg'");
    std::str::from_utf8(&image.data).expect("sanitized SVG must be valid UTF-8");
}

#[cfg(feature = "svg")]
#[test]
fn svg_no_sanitize_native_is_noop() {
    let svg_bytes = Bytes::from_static(b"<svg xmlns=\"http://www.w3.org/2000/svg\" width=\"4\" height=\"4\"/>");
    let original_data = svg_bytes.clone();
    let mut image = make_image(svg_bytes, "svg");
    let opts = SvgOptions {
        sanitize: false,
        render_dpi: 96.0,
    };
    let result = re_encode(&mut image, ImageOutputFormat::Native, &SecurityLimits::default(), &opts);
    assert!(
        matches!(result, Ok(false)),
        "SVG no-sanitize on Native must return Ok(false); got {result:?}"
    );
    assert_eq!(image.data, original_data);
}

#[cfg(feature = "svg")]
#[test]
fn svg_to_svg_sanitize_roundtrip() {
    let svg_bytes = Bytes::from_static(b"<svg xmlns=\"http://www.w3.org/2000/svg\" width=\"4\" height=\"4\"/>");
    let mut image = make_image(svg_bytes, "svg");
    let result = re_encode(
        &mut image,
        ImageOutputFormat::Svg,
        &SecurityLimits::default(),
        &SvgOptions::default(),
    );
    assert!(
        matches!(result, Ok(true)),
        "svg→svg must return Ok(true); got {result:?}"
    );
    assert_eq!(image.format.as_ref(), "svg");
    std::str::from_utf8(&image.data).expect("output must be valid UTF-8");
}

#[cfg(feature = "svg")]
#[test]
fn raster_to_svg_returns_unsupported_direction() {
    let mut image = make_image(make_png_bytes(), "png");
    let result = re_encode(
        &mut image,
        ImageOutputFormat::Svg,
        &SecurityLimits::default(),
        &SvgOptions::default(),
    );
    assert!(
        matches!(result, Err(EncodeWarning::UnsupportedDirection { ref from_format, to_format: "svg" }) if from_format == "png"),
        "png→svg must return Err(UnsupportedDirection); got {result:?}",
    );
    let guessed = image::guess_format(&image.data).expect("data must still be valid PNG");
    assert_eq!(guessed, ImageFormat::Png);
}

#[cfg(feature = "svg")]
#[test]
fn svg_to_jpeg_rasterizes() {
    let svg_bytes = Bytes::from_static(b"<svg xmlns=\"http://www.w3.org/2000/svg\" width=\"4\" height=\"4\"/>");
    let mut image = make_image(svg_bytes, "svg");
    let result = re_encode(
        &mut image,
        ImageOutputFormat::Jpeg { quality: 85 },
        &SecurityLimits::default(),
        &SvgOptions::default(),
    );
    assert!(
        matches!(result, Ok(true)),
        "svg→jpeg must return Ok(true); got {result:?}"
    );
    assert_eq!(image.format.as_ref(), "jpeg");
    let guessed = image::guess_format(&image.data).expect("output must be valid JPEG");
    assert_eq!(guessed, ImageFormat::Jpeg);
}

#[cfg(feature = "heic")]
fn hevc_encoder_is_registered() -> bool {
    xberg_libheif::LibHeif::new()
        .encoder_for_format(xberg_libheif::CompressionFormat::Hevc)
        .is_ok()
}

/// Decode-only builds link libheif without x265, so HEIF output must fail as
/// `EncoderUnavailable` instead of silently falling back or panicking. ~keep
#[cfg(feature = "heic")]
#[test]
fn png_to_heif_without_hevc_encoder_reports_encoder_unavailable() {
    if hevc_encoder_is_registered() {
        eprintln!("skipping: this build registers an HEVC encoder");
        return;
    }
    let original = make_png_bytes();
    let mut image = make_image(original.clone(), "png");
    let result = re_encode_default(&mut image, ImageOutputFormat::Heif { quality: 80 });
    assert!(
        matches!(result, Err(EncodeWarning::EncoderUnavailable { ref target_format, .. }) if *target_format == "heif"),
        "a decode-only libheif must report EncoderUnavailable for heif; got {result:?}"
    );
    assert_eq!(image.format.as_ref(), "png");
    assert_eq!(image.data, original, "a rejected re-encode must preserve source bytes");
}

/// Opt in on encoder-enabled builds (Homebrew libheif or the macOS
/// `elixir-encoders` profile) with `XBERG_TEST_HEIC_ENCODER=1`. ~keep
#[cfg(feature = "heic")]
#[test]
fn png_to_heif_round_trip() {
    if std::env::var_os("XBERG_TEST_HEIC_ENCODER").is_none() {
        eprintln!("skipping: set XBERG_TEST_HEIC_ENCODER=1 to exercise HEIF output");
        return;
    }
    assert!(
        hevc_encoder_is_registered(),
        "XBERG_TEST_HEIC_ENCODER requires a libheif build with an HEVC encoder"
    );
    let mut image = make_image(make_png_bytes(), "png");
    let result = re_encode_default(&mut image, ImageOutputFormat::Heif { quality: 80 });
    assert!(
        matches!(result, Ok(true)),
        "png→heif must return Ok(true); got {result:?}"
    );
    assert_eq!(image.format.as_ref(), "heif");
    let context = xberg_libheif::HeifContext::read_from_bytes(&image.data).expect("output should be valid HEIF");
    let handle = context.primary_image_handle().expect("should have primary image");
    assert_eq!(handle.width(), 4);
    assert_eq!(handle.height(), 4);
}

#[cfg(feature = "heic")]
#[test]
fn heif_same_format_no_op() {
    let mut image = make_image(Bytes::from_static(b"placeholder"), "heif");
    let result = re_encode_default(&mut image, ImageOutputFormat::Heif { quality: 80 });
    assert!(matches!(result, Ok(false)), "heif→heif must return Ok(false)");
}

#[cfg(feature = "heic")]
#[test]
fn heic_format_string_matches() {
    let mut image = make_image(Bytes::from_static(b"placeholder"), "heic");
    let result = re_encode_default(&mut image, ImageOutputFormat::Heif { quality: 80 });
    assert!(matches!(result, Ok(false)), "heic→Heif must return Ok(false)");
}

#[cfg(feature = "heic")]
#[test]
fn should_reject_oversized_heic_dimensions_before_reencoding_decode() {
    let error = validate_heic_decode_budget(6_000, 6_000, "heic", 0, &SecurityLimits::default())
        .expect_err("oversized HEIC must fail at the decoded-image budget");

    assert!(
        matches!(error, EncodeWarning::DecodeFailed { ref message, .. } if message.contains("security_limits.max_content_size")),
        "unexpected error: {error}"
    );
}
