//! The OCR raster of an image file, with a reduced-scale decode for a JPEG that is over the limit (GH#2339).

use super::decode_image_to_rgb8_with_security_limits;
use crate::error::{Result, XbergError};
use crate::extraction::image_decode::{ImageDecodeBudget, jpeg_rgb8_peak_bytes};
use crate::extractors::security::SecurityLimits;
use image::ColorType;

/// The IDCT scale denominators jpeg-decoder supports below full size, finest first.
const REDUCED_SCALE_DENOMINATORS: [u32; 3] = [2, 4, 8];

/// The raster that OCR reads, and the size of the source image it comes from.
pub(crate) struct OcrRaster {
    pub(crate) image: image::RgbImage,
    pub(crate) source_width: u32,
    pub(crate) source_height: u32,
    /// The IDCT scale the raster was decoded at: 1 is full size, 2 is half size.
    pub(crate) scale_denominator: u32,
}

impl OcrRaster {
    /// The resolution of the raster, from a resolution known for the source image.
    pub(crate) fn raster_dpi(&self, source_dpi: Option<f64>) -> Option<f64> {
        source_dpi.map(|dpi| dpi / f64::from(self.scale_denominator))
    }
}

/// The frame header of a JPEG that jpeg-decoder can decode at a reduced scale.
struct ScalableJpeg<'a> {
    decoder: jpeg_decoder::Decoder<&'a [u8]>,
    width: u32,
    height: u32,
    color_type: ColorType,
}

/// Read the frame header of a sequential 8-bit grey or RGB JPEG. Every other image gives `None`:
/// progressive and lossless JPEGs hold full-size buffers at every scale, and CMYK has no RGB form here.
fn scalable_jpeg(bytes: &[u8]) -> Option<ScalableJpeg<'_>> {
    if !bytes.starts_with(&[0xFF, 0xD8]) {
        return None;
    }
    let mut decoder = jpeg_decoder::Decoder::new(bytes);
    decoder.read_info().ok()?;
    let info = decoder.info()?;
    if info.coding_process != jpeg_decoder::CodingProcess::DctSequential {
        return None;
    }
    let color_type = match info.pixel_format {
        jpeg_decoder::PixelFormat::L8 => ColorType::L8,
        jpeg_decoder::PixelFormat::RGB24 => ColorType::Rgb8,
        jpeg_decoder::PixelFormat::L16 | jpeg_decoder::PixelFormat::CMYK32 => return None,
    };
    Some(ScalableJpeg {
        decoder,
        width: u32::from(info.width),
        height: u32::from(info.height),
        color_type,
    })
}

/// The dimensions of a JPEG whose full-size decode is over the limit but that OCR can decode at a
/// reduced scale. The image extractor reads its metadata from this instead of a full-size decode.
pub(crate) fn oversized_jpeg_dimensions(bytes: &[u8], limits: &SecurityLimits) -> Option<(u32, u32)> {
    let jpeg = scalable_jpeg(bytes)?;
    let fits = |width, height| rgb8_fits(&jpeg, width, height, bytes.len(), limits);
    REDUCED_SCALE_DENOMINATORS
        .iter()
        .any(|&denominator| fits(jpeg.width.div_ceil(denominator), jpeg.height.div_ceil(denominator)))
        .then_some((jpeg.width, jpeg.height))
}

fn rgb8_fits(
    jpeg: &ScalableJpeg<'_>,
    width: u32,
    height: u32,
    encoded_bytes: usize,
    limits: &SecurityLimits,
) -> bool {
    let encoded_bytes = u64::try_from(encoded_bytes).unwrap_or(u64::MAX);
    jpeg_rgb8_peak_bytes(width, height, jpeg.color_type, encoded_bytes)
        .and_then(|peak| ImageDecodeBudget::from_security_limits(limits).validate(width, height, peak))
        .is_ok()
}

/// Decode an image file for OCR.
///
/// A JPEG whose full-size raster does not fit `security_limits.max_content_size`, or fails
/// `raster_fits`, is decoded at the finest of the 1/2, 1/4 and 1/8 IDCT scales that fits both.
/// Every other image, and a JPEG that does not fit even at 1/8, takes the full-size decode and
/// its error.
pub(crate) fn decode_ocr_raster(
    bytes: &[u8],
    limits: &SecurityLimits,
    raster_fits: impl Fn(u32, u32) -> bool,
) -> Result<OcrRaster> {
    if let Some(raster) = decode_reduced_jpeg(bytes, limits, &raster_fits)? {
        return Ok(raster);
    }
    let image = decode_image_to_rgb8_with_security_limits(bytes, limits)?;
    let (source_width, source_height) = image.dimensions();
    Ok(OcrRaster {
        image,
        source_width,
        source_height,
        scale_denominator: 1,
    })
}

fn decode_reduced_jpeg(
    bytes: &[u8],
    limits: &SecurityLimits,
    raster_fits: &impl Fn(u32, u32) -> bool,
) -> Result<Option<OcrRaster>> {
    let Some(mut jpeg) = scalable_jpeg(bytes) else {
        return Ok(None);
    };
    let fits = |jpeg: &ScalableJpeg<'_>, width, height| {
        rgb8_fits(jpeg, width, height, bytes.len(), limits) && raster_fits(width, height)
    };
    if fits(&jpeg, jpeg.width, jpeg.height) {
        return Ok(None);
    }
    let Some(denominator) = REDUCED_SCALE_DENOMINATORS.into_iter().find(|&denominator| {
        fits(&jpeg, jpeg.width.div_ceil(denominator), jpeg.height.div_ceil(denominator))
    }) else {
        return Ok(None);
    };
    let planned = (jpeg.width.div_ceil(denominator), jpeg.height.div_ceil(denominator));
    let requested = |length: u32| u16::try_from(length).unwrap_or(u16::MAX);
    let scaled = jpeg
        .decoder
        .scale(requested(planned.0), requested(planned.1))
        .map_err(jpeg_error)?;
    if (u32::from(scaled.0), u32::from(scaled.1)) != planned {
        return Ok(None);
    }
    let (width, height) = planned;
    let output_bytes = usize::from(jpeg.color_type.channel_count())
        .saturating_mul(usize::from(scaled.0))
        .saturating_mul(usize::from(scaled.1));
    jpeg.decoder.set_max_decoding_buffer_size(output_bytes);
    let pixels = jpeg.decoder.decode().map_err(jpeg_error)?;
    let image = match jpeg.color_type {
        ColorType::L8 => image::GrayImage::from_raw(width, height, pixels)
            .map(|grey| image::DynamicImage::ImageLuma8(grey).into_rgb8()),
        _ => image::RgbImage::from_raw(width, height, pixels),
    }
    .ok_or_else(|| XbergError::parsing("A reduced JPEG decode returned fewer pixels than its size".to_string()))?;
    Ok(Some(OcrRaster {
        image,
        source_width: jpeg.width,
        source_height: jpeg.height,
        scale_denominator: denominator,
    }))
}

fn jpeg_error(error: jpeg_decoder::Error) -> XbergError {
    XbergError::parsing(format!("Failed to decode image: {error}"))
}
