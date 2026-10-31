//! Baseline JPEG fixtures for tests that need a real encoded scan.

/// Encode an RGB raster as a baseline JPEG, and check its header with the decoder that OCR uses.
pub(crate) fn baseline_rgb_jpeg(rgb: &[u8], width: u16, height: u16) -> Vec<u8> {
    let mut bytes = Vec::new();
    image::codecs::jpeg::JpegEncoder::new(&mut bytes)
        .encode(rgb, u32::from(width), u32::from(height), image::ExtendedColorType::Rgb8)
        .expect("encode a baseline RGB JPEG");
    let mut decoder = jpeg_decoder::Decoder::new(bytes.as_slice());
    decoder.read_info().expect("read the encoded JPEG header");
    let info = decoder.info().expect("the encoded JPEG has a frame header");
    assert_eq!(
        (info.width, info.height, info.pixel_format),
        (width, height, jpeg_decoder::PixelFormat::RGB24)
    );
    assert_eq!(info.coding_process, jpeg_decoder::CodingProcess::DctSequential);
    bytes
}
