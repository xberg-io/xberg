//! Image extraction functionality.
//!
//! This module provides functions for extracting metadata and EXIF data from images,
//! including support for multi-frame TIFF files.

use crate::error::{Result, XbergError};
use crate::extraction::exif::extract_exif_data;
use crate::extraction::heif::is_heif_container;
#[cfg(feature = "ocr")]
use crate::extraction::image_decode::image_dimension_error;
use crate::extraction::image_decode::{
    ImageDecodeBudget, decode_standard_image_with_security_limits, decode_standard_rgb8_with_security_limits,
    decoded_byte_count,
};
use crate::extractors::security::SecurityLimits;
use std::collections::HashMap;
#[cfg(feature = "ocr")]
use std::io::Cursor;

/// JP2 file signature: 12-byte box starting with length 0x0000000C and type "jP  "
const JP2_MAGIC: &[u8] = &[0x00, 0x00, 0x00, 0x0C, 0x6A, 0x50, 0x20, 0x20];

#[cfg(feature = "ocr")]
fn jp2_peak_decoded_bytes(width: u32, height: u32, num_channels: u8, has_alpha: bool) -> Result<u64> {
    let pixel_count = decoded_byte_count(width, height, 1)?;
    let source_channels = u64::from(num_channels) + u64::from(u8::from(has_alpha));
    let source_bytes = pixel_count
        .checked_mul(source_channels)
        .ok_or_else(|| image_dimension_error(width, height, u64::MAX, u64::MAX))?;
    let rgb_bytes = pixel_count
        .checked_mul(u64::from(image::ColorType::Rgb8.bytes_per_pixel()))
        .ok_or_else(|| image_dimension_error(width, height, u64::MAX, u64::MAX))?;
    if num_channels == 3 && !has_alpha {
        Ok(rgb_bytes)
    } else {
        source_bytes
            .checked_add(rgb_bytes)
            .ok_or_else(|| image_dimension_error(width, height, u64::MAX, u64::MAX))
    }
}

#[cfg(feature = "ocr")]
fn jp2_peak_live_bytes(
    width: u32,
    height: u32,
    num_channels: u8,
    has_alpha: bool,
    encoded_bytes: usize,
) -> Result<u64> {
    jp2_peak_decoded_bytes(width, height, num_channels, has_alpha)?
        .checked_add(u64::try_from(encoded_bytes).unwrap_or(u64::MAX))
        .ok_or_else(|| image_dimension_error(width, height, u64::MAX, u64::MAX))
}

#[cfg(feature = "ocr")]
fn jbig2_gray_peak_live_bytes(width: u32, height: u32, encoded_bytes: usize) -> Result<u64> {
    decoded_byte_count(width, height, u64::from(image::ColorType::L8.bytes_per_pixel()))?
        .checked_add(u64::try_from(encoded_bytes).unwrap_or(u64::MAX))
        .ok_or_else(|| image_dimension_error(width, height, u64::MAX, u64::MAX))
}

#[cfg(feature = "ocr")]
fn jbig2_rgb_peak_live_bytes(width: u32, height: u32, encoded_bytes: usize) -> Result<u64> {
    let decoded_and_encoded = jbig2_gray_peak_live_bytes(width, height, encoded_bytes)?;
    decoded_and_encoded
        .checked_add(decoded_byte_count(
            width,
            height,
            u64::from(image::ColorType::Rgb8.bytes_per_pixel()),
        )?)
        .ok_or_else(|| image_dimension_error(width, height, u64::MAX, u64::MAX))
}

#[cfg(feature = "ocr")]
fn validate_encoded_image_input(bytes: &[u8], limits: &SecurityLimits) -> Result<()> {
    ImageDecodeBudget::from_security_limits(limits).validate(1, 1, u64::try_from(bytes.len()).unwrap_or(u64::MAX))
}

/// Check if bytes start with JPEG 2000 magic bytes.
pub(crate) fn is_jp2(bytes: &[u8]) -> bool {
    bytes.len() >= JP2_MAGIC.len() && bytes[..JP2_MAGIC.len()] == *JP2_MAGIC
}

/// Check if bytes start with J2K codestream magic (SOC marker).
#[cfg(feature = "ocr")]
pub(crate) fn is_j2k(bytes: &[u8]) -> bool {
    bytes.len() >= 4 && bytes[0] == 0xFF && bytes[1] == 0x4F && bytes[2] == 0xFF && bytes[3] == 0x51
}

/// Image metadata extracted from an image file.
#[derive(Debug, Clone)]
pub(crate) struct ExtractedImageMetadata {
    /// Image width in pixels
    pub(crate) width: u32,
    /// Image height in pixels
    pub(crate) height: u32,
    /// Image format (e.g., "PNG", "JPEG")
    pub(crate) format: String,
    /// EXIF data if available
    pub(crate) exif_data: HashMap<String, String>,
}

/// Parse JP2 file header boxes to extract image dimensions.
///
/// Supports both JP2 container format (ISO 15444-1 Annex I) and raw J2K codestream.
/// Uses pure Rust header parsing without external dependencies.
fn decode_jp2_metadata(bytes: &[u8]) -> Result<ExtractedImageMetadata> {
    if is_jp2(bytes) {
        return parse_jp2_boxes(bytes);
    }

    if bytes.len() >= 2 && bytes[0] == 0xFF && bytes[1] == 0x4F {
        return parse_j2k_siz(bytes);
    }

    Err(XbergError::parsing("Not a valid JPEG 2000 file".to_string()))
}

/// Parse JP2 container boxes to find ihdr (Image Header) box.
fn parse_jp2_boxes(bytes: &[u8]) -> Result<ExtractedImageMetadata> {
    let mut offset = 0;
    let len = bytes.len();

    while offset + 8 <= len {
        let box_len =
            u32::from_be_bytes([bytes[offset], bytes[offset + 1], bytes[offset + 2], bytes[offset + 3]]) as usize;
        let box_type = &bytes[offset + 4..offset + 8];

        let (data_start, actual_len) = if box_len == 1 && offset + 16 <= len {
            let ext_len = u64::from_be_bytes([
                bytes[offset + 8],
                bytes[offset + 9],
                bytes[offset + 10],
                bytes[offset + 11],
                bytes[offset + 12],
                bytes[offset + 13],
                bytes[offset + 14],
                bytes[offset + 15],
            ]) as usize;
            (offset + 16, ext_len)
        } else if box_len == 0 {
            (offset + 8, len - offset)
        } else {
            (offset + 8, box_len)
        };

        if box_type == b"ihdr" && data_start + 8 <= len {
            let height = u32::from_be_bytes([
                bytes[data_start],
                bytes[data_start + 1],
                bytes[data_start + 2],
                bytes[data_start + 3],
            ]);
            let width = u32::from_be_bytes([
                bytes[data_start + 4],
                bytes[data_start + 5],
                bytes[data_start + 6],
                bytes[data_start + 7],
            ]);
            return Ok(ExtractedImageMetadata {
                width,
                height,
                format: "JPEG2000".to_string(),
                exif_data: extract_exif_data(bytes),
            });
        }

        if box_type == b"jp2h" {
            let end = offset + actual_len.min(len - offset);
            let mut sub_offset = data_start;
            while sub_offset + 8 <= end {
                let sub_len = u32::from_be_bytes([
                    bytes[sub_offset],
                    bytes[sub_offset + 1],
                    bytes[sub_offset + 2],
                    bytes[sub_offset + 3],
                ]) as usize;
                let sub_type = &bytes[sub_offset + 4..sub_offset + 8];
                let sub_data = sub_offset + 8;

                if sub_type == b"ihdr" && sub_data + 8 <= len {
                    let height = u32::from_be_bytes([
                        bytes[sub_data],
                        bytes[sub_data + 1],
                        bytes[sub_data + 2],
                        bytes[sub_data + 3],
                    ]);
                    let width = u32::from_be_bytes([
                        bytes[sub_data + 4],
                        bytes[sub_data + 5],
                        bytes[sub_data + 6],
                        bytes[sub_data + 7],
                    ]);
                    return Ok(ExtractedImageMetadata {
                        width,
                        height,
                        format: "JPEG2000".to_string(),
                        exif_data: extract_exif_data(bytes),
                    });
                }

                if sub_len < 8 {
                    break;
                }
                sub_offset += sub_len;
            }
        }

        if actual_len < 8 {
            break;
        }
        offset += actual_len;
    }

    Err(XbergError::parsing("JP2 file missing ihdr box".to_string()))
}

/// Parse J2K raw codestream SIZ marker for image dimensions.
fn parse_j2k_siz(bytes: &[u8]) -> Result<ExtractedImageMetadata> {
    if let Some(offset) = memchr::memmem::find(bytes, &[0xFF, 0x51]) {
        let data_start = offset + 4;
        if data_start + 18 <= bytes.len() {
            let xsiz = u32::from_be_bytes([
                bytes[data_start + 2],
                bytes[data_start + 3],
                bytes[data_start + 4],
                bytes[data_start + 5],
            ]);
            let ysiz = u32::from_be_bytes([
                bytes[data_start + 6],
                bytes[data_start + 7],
                bytes[data_start + 8],
                bytes[data_start + 9],
            ]);
            let xosiz = u32::from_be_bytes([
                bytes[data_start + 10],
                bytes[data_start + 11],
                bytes[data_start + 12],
                bytes[data_start + 13],
            ]);
            let yosiz = u32::from_be_bytes([
                bytes[data_start + 14],
                bytes[data_start + 15],
                bytes[data_start + 16],
                bytes[data_start + 17],
            ]);

            let width = xsiz.saturating_sub(xosiz);
            let height = ysiz.saturating_sub(yosiz);

            return Ok(ExtractedImageMetadata {
                width,
                height,
                format: "JPEG2000".to_string(),
                exif_data: extract_exif_data(bytes),
            });
        }
    }

    Err(XbergError::parsing("J2K codestream missing SIZ marker".to_string()))
}

/// Decode JPEG 2000 image bytes to an RGB image using hayro-jpeg2000.
///
/// Pure Rust, memory-safe decoder. No temp files needed.
#[cfg(all(feature = "ocr", test))]
pub(crate) fn decode_jp2_to_rgb(bytes: &[u8]) -> Result<image::RgbImage> {
    let limits = SecurityLimits::default();
    decode_jp2_to_rgb_with_security_limits(bytes, &limits)
}

#[cfg(feature = "ocr")]
fn decode_jp2_to_rgb_with_security_limits(bytes: &[u8], limits: &SecurityLimits) -> Result<image::RgbImage> {
    use hayro_jpeg2000::{DecodeSettings, DecoderContext, Image as Jp2Image};

    validate_encoded_image_input(bytes, limits)?;
    let jp2 = Jp2Image::new(bytes, &DecodeSettings::default())
        .map_err(|e| XbergError::parsing(format!("JP2 decode failed: {}", e)))?;
    let width = jp2.width();
    let height = jp2.height();
    let has_alpha = jp2.has_alpha();
    let num_channels = jp2.color_space().num_channels();
    let peak_live_bytes = jp2_peak_live_bytes(width, height, num_channels, has_alpha, bytes.len())?;
    ImageDecodeBudget::from_security_limits(limits).validate(width, height, peak_live_bytes)?;
    // hayro-jpeg2000 0.4 threads a caller-owned `DecoderContext` through `decode` so the
    // sample buffers can be reused across images, and returns a borrowing `DecodedImage`
    // rather than the interleaved `Vec<u8>` 0.3 handed back. `data_u8` is that same
    // interleaved unsigned-8-bit view, so everything below is unchanged.
    let mut decoder_context = DecoderContext::default();
    let pixels = jp2
        .decode(&mut decoder_context)
        .map_err(|e| XbergError::parsing(format!("JP2 pixel decode failed: {}", e)))?
        .data_u8();

    let rgb_bytes = match (num_channels, has_alpha) {
        (1, false) => {
            let mut rgb = Vec::with_capacity(pixels.len() * 3);
            for &g in &pixels {
                rgb.push(g);
                rgb.push(g);
                rgb.push(g);
            }
            rgb
        }
        (1, true) => {
            let mut rgb = Vec::with_capacity((pixels.len() / 2) * 3);
            for chunk in pixels.chunks_exact(2) {
                rgb.push(chunk[0]);
                rgb.push(chunk[0]);
                rgb.push(chunk[0]);
            }
            rgb
        }
        (3, false) => pixels,
        (3, true) => {
            let mut rgb = Vec::with_capacity((pixels.len() / 4) * 3);
            for chunk in pixels.chunks_exact(4) {
                rgb.push(chunk[0]);
                rgb.push(chunk[1]);
                rgb.push(chunk[2]);
            }
            rgb
        }
        (4, false) => {
            let mut rgb = Vec::with_capacity((pixels.len() / 4) * 3);
            for chunk in pixels.chunks_exact(4) {
                let c = chunk[0] as f32 / 255.0;
                let m = chunk[1] as f32 / 255.0;
                let y = chunk[2] as f32 / 255.0;
                let k = chunk[3] as f32 / 255.0;
                rgb.push(((1.0 - c) * (1.0 - k) * 255.0) as u8);
                rgb.push(((1.0 - m) * (1.0 - k) * 255.0) as u8);
                rgb.push(((1.0 - y) * (1.0 - k) * 255.0) as u8);
            }
            rgb
        }
        _ => {
            return Err(XbergError::parsing(format!(
                "Unsupported JP2 color space: {} channels, alpha={}",
                num_channels, has_alpha
            )));
        }
    };

    image::RgbImage::from_raw(width, height, rgb_bytes)
        .ok_or_else(|| XbergError::parsing("Failed to construct RGB image from JP2 data".to_string()))
}

/// JBIG2 file signature: 0x97 0x4A 0x42 0x32 0x0D 0x0A 0x1A 0x0A
#[cfg(feature = "ocr")]
const JBIG2_MAGIC: &[u8] = &[0x97, 0x4A, 0x42, 0x32, 0x0D, 0x0A, 0x1A, 0x0A];

/// Check if bytes start with JBIG2 magic bytes.
#[cfg(feature = "ocr")]
pub(crate) fn is_jbig2(bytes: &[u8]) -> bool {
    bytes.len() >= JBIG2_MAGIC.len() && bytes[..JBIG2_MAGIC.len()] == *JBIG2_MAGIC
}

/// Decode JBIG2 image bytes to a grayscale image using hayro-jbig2.
///
/// JBIG2 is a bi-level (1-bit) image compression format commonly used in scanned PDFs.
/// The decoder converts black/white pixels to grayscale (0/255) for OCR processing.
#[cfg(feature = "ocr")]
fn decode_jbig2_to_gray_with_security_limits(bytes: &[u8], limits: &SecurityLimits) -> Result<image::GrayImage> {
    use hayro_jbig2::{Decoder, Image};

    struct GrayDecoder {
        pixels: Vec<u8>,
        max_pixels: usize,
        exceeded_dimensions: bool,
    }

    impl Decoder for GrayDecoder {
        fn push_pixel(&mut self, black: bool) {
            if self.pixels.len() >= self.max_pixels {
                self.exceeded_dimensions = true;
                return;
            }
            self.pixels.push(if black { 0 } else { 255 });
        }

        fn push_pixel_chunk(&mut self, black: bool, chunk_count: u32) {
            let luma = if black { 0 } else { 255 };
            let Some(count) = (chunk_count as usize).checked_mul(8) else {
                self.exceeded_dimensions = true;
                return;
            };
            let Some(new_len) = self.pixels.len().checked_add(count) else {
                self.exceeded_dimensions = true;
                return;
            };
            if new_len > self.max_pixels {
                self.exceeded_dimensions = true;
                return;
            }
            self.pixels.resize(new_len, luma);
        }

        fn next_line(&mut self) {}
    }

    validate_encoded_image_input(bytes, limits)?;
    let jbig2_image = Image::new(bytes).map_err(|e| XbergError::parsing(format!("JBIG2 decode failed: {e}")))?;
    let width = jbig2_image.width();
    let height = jbig2_image.height();
    let decoded_bytes = decoded_byte_count(width, height, u64::from(image::ColorType::L8.bytes_per_pixel()))?;
    let peak_live_bytes = jbig2_gray_peak_live_bytes(width, height, bytes.len())?;
    ImageDecodeBudget::from_security_limits(limits).validate(width, height, peak_live_bytes)?;

    let max_pixels = usize::try_from(decoded_bytes)
        .map_err(|_| image_dimension_error(width, height, decoded_bytes, decoded_bytes))?;
    let mut pixels = Vec::new();
    pixels
        .try_reserve_exact(max_pixels)
        .map_err(|error| XbergError::parsing(format!("Failed to reserve JBIG2 decoded image buffer: {error}")))?;
    let mut decoder = GrayDecoder {
        pixels,
        max_pixels,
        exceeded_dimensions: false,
    };
    jbig2_image
        .decode(&mut decoder)
        .map_err(|e| XbergError::parsing(format!("JBIG2 decode failed: {e}")))?;
    if decoder.exceeded_dimensions {
        return Err(XbergError::Validation {
            message: format!("JBIG2 decompressed beyond its declared {width}x{height} image dimensions"),
            source: None,
        });
    }

    image::GrayImage::from_raw(width, height, decoder.pixels)
        .ok_or_else(|| XbergError::parsing("Failed to construct grayscale image from JBIG2 data".to_string()))
}

/// Load image bytes for OCR, with JPEG 2000 and JBIG2 fallback support.
///
/// The standard `image` crate does not support JPEG 2000 or JBIG2 formats.
/// This function detects these formats by magic bytes and uses `hayro-jpeg2000`
/// / `hayro-jbig2` for decoding, falling back to the standard `image` crate
/// for all other formats.
#[cfg(feature = "ocr")]
pub(crate) fn load_image_for_ocr(image_bytes: &[u8], limits: &SecurityLimits) -> Result<image::DynamicImage> {
    decode_image_to_rgb8_with_security_limits(image_bytes, limits).map(image::DynamicImage::ImageRgb8)
}

/// Meters-per-inch, used to convert a PNG `pHYs` pixels-per-metre density into DPI.
#[cfg(feature = "ocr-pipeline")]
const PNG_METERS_PER_INCH: f64 = 0.0254;

/// Read the embedded pixel density from a PNG's `pHYs` chunk, if present, and report it as DPI.
///
/// PNG stores physical pixel density in pixels-per-unit on each axis, independent of the image's
/// actual pixel dimensions (GH#1630: xberg 1.1.5 discarded this and always assumed 72 DPI, so a
/// genuine 300 DPI scan was rescaled even when the caller's `target_dpi` was already 300).
///
/// Performs a bounded scan of the chunk stream (8-byte signature, then `length + type + data +
/// crc` chunks), stopping at the first `IDAT` since `pHYs` always precedes the image data when
/// present. Returns `None` — leaving the caller's existing 72 DPI assumption untouched — for
/// anything that is not a well-formed PNG carrying a metre-unit `pHYs` chunk with a positive,
/// finite horizontal density: not a PNG, no `pHYs` chunk, a malformed chunk stream, an
/// unspecified/unknown unit (unit code 0), or (defensively) a non-finite conversion.
///
/// Only the horizontal density (`xppu`) is reported. `source_dpi` downstream is a single scalar
/// hint to Tesseract; real-world scans essentially never have anisotropic `pHYs` axes, so `xppu`
/// alone is enough without adding a second, almost-always-identical value to plumb through.
///
/// `ocr-pipeline`, not `ocr`, is the gate: `ocr` implies `ocr-pipeline`, and every caller of this
/// function (`ocr::processor::execution::resolve_known_source_dpi` via the `ocr` backend, and
/// `extractors::image::normalize_image_bytes_for_ocr` via the extractor boundary, which only
/// requires `ocr-pipeline`) must see the same symbol regardless of which builds `ocr` on top of.
#[cfg(feature = "ocr-pipeline")]
pub(crate) fn png_pixel_density_dpi(bytes: &[u8]) -> Option<f64> {
    const PNG_SIGNATURE: [u8; 8] = [0x89, b'P', b'N', b'G', 0x0D, 0x0A, 0x1A, 0x0A];
    const PHYS_CHUNK_DATA_LEN: usize = 9;
    const PHYS_UNIT_METER: u8 = 1;

    if bytes.len() < PNG_SIGNATURE.len() || bytes[..PNG_SIGNATURE.len()] != PNG_SIGNATURE {
        return None;
    }

    let mut offset = PNG_SIGNATURE.len();
    while offset.checked_add(8)? <= bytes.len() {
        let length = u32::from_be_bytes(bytes[offset..offset + 4].try_into().ok()?) as usize;
        let chunk_type = &bytes[offset + 4..offset + 8];
        let data_start = offset + 8;
        let data_end = data_start.checked_add(length)?;
        if data_end.checked_add(4)? > bytes.len() {
            return None;
        }
        if chunk_type == b"IDAT" {
            return None;
        }
        if chunk_type == b"pHYs" {
            if length != PHYS_CHUNK_DATA_LEN {
                return None;
            }
            let data = &bytes[data_start..data_end];
            let xppu = u32::from_be_bytes(data[0..4].try_into().ok()?);
            let unit = data[8];
            if unit != PHYS_UNIT_METER || xppu == 0 {
                return None;
            }
            let dpi = f64::from(xppu) * PNG_METERS_PER_INCH;
            return (dpi.is_finite() && dpi > 0.0).then_some(dpi);
        }
        offset = data_end + 4;
    }
    None
}

/// Read a caller-supplied source-resolution hint out of `OcrConfig.backend_options`, in DPI.
///
/// Mirrors `ocr::tesseract_backend::TesseractBackend::source_dpi_from_backend_options` (which
/// reads the same `[SOURCE_DPI_BACKEND_OPTION]` key off the same `backend_options` field after
/// the config has been converted to a `TesseractConfig`): an absent, malformed, or non-positive
/// value is "unknown" rather than an error. That copy lives in a module gated on the full `ocr`
/// feature and is only reachable once a caller has a backend, so it cannot be reused directly
/// from the extractor boundary (`extractors::image::normalize_image_bytes_for_ocr`), which only
/// requires `ocr-pipeline` and runs *before* any backend conversion. This is the canonical
/// implementation for that earlier point; `TesseractBackend`'s copy should eventually delegate
/// here instead of re-implementing the same three-line filter (GH#1621 is exactly this drift
/// shape: two independently maintained readers of the same value).
///
/// [SOURCE_DPI_BACKEND_OPTION]: crate::core::config::ocr::SOURCE_DPI_BACKEND_OPTION
#[cfg(feature = "ocr-pipeline")]
pub(crate) fn explicit_source_dpi_from_ocr_config(config: &crate::core::config::OcrConfig) -> Option<f64> {
    config
        .backend_options
        .as_ref()
        .and_then(|options| options.get(crate::core::config::ocr::SOURCE_DPI_BACKEND_OPTION))
        .and_then(serde_json::Value::as_f64)
        .filter(|dpi| dpi.is_finite() && *dpi > 0.0)
}

/// Resolve the resolution to report for a raster, in priority order:
///
/// 1. `explicit_source_dpi` — a caller-supplied hint (see
///    [`explicit_source_dpi_from_ocr_config`] for the `OcrConfig.backend_options` reader; the PDF
///    OCR route supplies its own via `TesseractConfig::source_dpi` directly). A caller that
///    explicitly said "my source is 72 DPI" must not be overridden by embedded metadata.
/// 2. The image's own embedded pixel-density metadata, when present (GH#1630; currently PNG
///    `pHYs` only — see [`png_pixel_density_dpi`]).
/// 3. `None`, meaning the historical 72 DPI assumption applies further down the pipeline.
///
/// Shared by both consumers — `ocr::processor::execution::perform_ocr` and
/// `extractors::image::normalize_image_bytes_for_ocr` — so the precedence cannot drift between
/// them the way two independent PNG chunk scanners would (GH#1621).
#[cfg(feature = "ocr-pipeline")]
pub(crate) fn resolve_known_source_dpi(explicit_source_dpi: Option<f64>, image_bytes: &[u8]) -> Option<f64> {
    explicit_source_dpi.or_else(|| png_pixel_density_dpi(image_bytes))
}

pub(crate) fn decode_image_to_rgb8_with_security_limits(
    image_bytes: &[u8],
    limits: &SecurityLimits,
) -> Result<image::RgbImage> {
    #[cfg(feature = "ocr")]
    {
        if is_jp2(image_bytes) || is_j2k(image_bytes) {
            return decode_jp2_to_rgb_with_security_limits(image_bytes, limits);
        }
        if is_jbig2(image_bytes) {
            let gray = decode_jbig2_to_gray_with_security_limits(image_bytes, limits)?;
            let (width, height) = gray.dimensions();
            let peak_bytes = jbig2_rgb_peak_live_bytes(width, height, image_bytes.len())?;
            ImageDecodeBudget::from_security_limits(limits).validate(width, height, peak_bytes)?;
            return Ok(image::DynamicImage::ImageLuma8(gray).into_rgb8());
        }
    }
    decode_standard_rgb8_with_security_limits(image_bytes, limits)
}

// Both callers are `#[cfg(feature = "ocr")]` tests in this file's `tests` module, so a
// bare `cfg(test)` leaves it dead in any test build without `ocr` (the
// `formula-recognition,pdf` CI leg). ~keep
#[cfg(all(test, feature = "ocr"))]
pub(crate) fn decode_image_with_security_limits(
    image_bytes: &[u8],
    limits: &SecurityLimits,
) -> Result<image::DynamicImage> {
    #[cfg(feature = "ocr")]
    {
        if is_jp2(image_bytes) || is_j2k(image_bytes) {
            return decode_jp2_to_rgb_with_security_limits(image_bytes, limits).map(image::DynamicImage::ImageRgb8);
        }
        if is_jbig2(image_bytes) {
            return decode_jbig2_to_gray_with_security_limits(image_bytes, limits).map(image::DynamicImage::ImageLuma8);
        }
    }
    decode_standard_image_with_security_limits(image_bytes, limits)
}

/// Extract metadata from image bytes.
///
/// Extracts dimensions, format, and EXIF data from the image.
/// Standard formats are header-probed without allocating their pixel buffers; JPEG 2000
/// dimensions come from JP2/J2K headers, and HEIF-family dimensions come from the primary
/// image handle when the `heic` feature is enabled. EXIF is read from the original bytes.
#[cfg(test)]
pub(crate) fn extract_image_metadata(bytes: &[u8]) -> Result<ExtractedImageMetadata> {
    let limits = SecurityLimits::default();
    extract_image_metadata_with_security_limits(bytes, &limits)
}

pub(crate) fn extract_image_metadata_with_security_limits(
    bytes: &[u8],
    limits: &SecurityLimits,
) -> Result<ExtractedImageMetadata> {
    let budget = ImageDecodeBudget::from_security_limits(limits);
    if (is_jp2(bytes) || (bytes.len() >= 2 && bytes[0] == 0xFF && bytes[1] == 0x4F))
        && let Ok(metadata) = decode_jp2_metadata(bytes)
    {
        let decoded_bytes = decoded_byte_count(
            metadata.width,
            metadata.height,
            u64::from(image::ColorType::Rgb8.bytes_per_pixel()),
        )?;
        budget.validate(metadata.width, metadata.height, decoded_bytes)?;
        #[cfg(feature = "ocr")]
        decode_jp2_to_rgb_with_security_limits(bytes, limits)?;
        return Ok(metadata);
    }

    if is_heif_container(bytes) {
        let exif_data = extract_exif_data(bytes);
        #[cfg(feature = "heic")]
        {
            use xberg_libheif::HeifContext;

            let context = HeifContext::read_from_bytes(bytes)
                .map_err(|error| XbergError::parsing(format!("Failed to read HEIF container: {error}")))?;
            let handle = context
                .primary_image_handle()
                .map_err(|error| XbergError::parsing(format!("Failed to read HEIF primary image handle: {error}")))?;
            let width = handle.width();
            let height = handle.height();
            let decoded_bytes =
                decoded_byte_count(width, height, u64::from(image::ColorType::Rgba8.bytes_per_pixel()))?;
            budget.validate(width, height, decoded_bytes)?;
            return Ok(ExtractedImageMetadata {
                width,
                height,
                format: "HEIF".to_string(),
                exif_data,
            });
        }
        #[cfg(not(feature = "heic"))]
        {
            let _ = exif_data;
            return Err(XbergError::parsing(
                "HEIF/HEIC/AVIF decoding requires the `heic` Cargo feature".to_string(),
            ));
        }
    }

    let decoded = decode_standard_image_with_security_limits(bytes, limits)?;
    let format = image::guess_format(bytes)
        .map_err(|error| XbergError::parsing(format!("Failed to read image format: {error}")))?;
    Ok(ExtractedImageMetadata {
        width: decoded.width(),
        height: decoded.height(),
        format: format!("{format:?}").to_uppercase(),
        exif_data: extract_exif_data(bytes),
    })
}

/// Result of OCR extraction from an image with optional page tracking.
#[cfg_attr(alef, alef(skip))]
#[derive(Debug, Clone)]
pub struct ImageOcrResult {
    /// Extracted text content
    pub content: String,
    /// Character byte boundaries per frame (for multi-frame TIFFs)
    pub boundaries: Option<Vec<crate::types::PageBoundary>>,
    /// Per-frame content information
    pub page_contents: Option<Vec<crate::types::PageContent>>,
}

/// Detects the number of frames in a TIFF file.
///
/// Returns the count of image frames/pages in a TIFF. Single-frame TIFFs return 1.
/// Invalid or non-TIFF data returns an error.
///
/// # Arguments
/// * `bytes` - Raw TIFF file bytes
///
/// # Returns
/// Frame count if valid TIFF, error otherwise.
#[cfg(feature = "ocr")]
pub(crate) fn detect_tiff_frame_count(bytes: &[u8]) -> Result<usize> {
    use tiff::decoder::Decoder;
    let mut decoder =
        Decoder::new(Cursor::new(bytes)).map_err(|e| XbergError::parsing(format!("TIFF decode: {}", e)))?;

    let mut count = 1;
    while decoder.next_image().is_ok() {
        count += 1;
    }
    Ok(count)
}

/// Extract text from image bytes using OCR with optional page tracking for multi-frame TIFFs.
///
/// This function:
/// - Detects if the image is a multi-frame TIFF
/// - For multi-frame TIFFs with PageConfig enabled, iterates frames and tracks boundaries
/// - For single-frame images or when page tracking is disabled, runs OCR on the whole image
/// - Returns (content, boundaries, page_contents) tuple
///
/// # Arguments
/// * `bytes` - Image file bytes
/// * `mime_type` - MIME type (e.g., "image/tiff")
/// * `ocr_result` - OCR backend result containing the text
/// * `page_config` - Optional page configuration for boundary tracking
///
/// # Returns
/// ImageOcrResult with content and optional boundaries for pagination
#[cfg(feature = "ocr")]
pub(crate) fn extract_text_from_image_with_ocr(
    bytes: &[u8],
    mime_type: &str,
    ocr_result: String,
    page_config: Option<&crate::core::config::PageConfig>,
) -> Result<ImageOcrResult> {
    let is_tiff = mime_type.to_lowercase().contains("tiff");
    let should_track_pages = page_config.is_some() && is_tiff;

    if !should_track_pages {
        return Ok(ImageOcrResult {
            content: ocr_result,
            boundaries: None,
            page_contents: None,
        });
    }

    let frame_count = detect_tiff_frame_count(bytes)?;

    if frame_count <= 1 {
        return Ok(ImageOcrResult {
            content: ocr_result,
            boundaries: None,
            page_contents: None,
        });
    }

    let content_len = ocr_result.len();
    let content_per_frame = content_len.checked_div(frame_count).unwrap_or(content_len);

    let mut boundaries = Vec::new();
    let mut page_contents = Vec::new();
    let mut byte_offset = 0;

    for frame_num in 1..=frame_count {
        let frame_end = if frame_num == frame_count {
            content_len
        } else {
            let raw_end = (frame_num * content_per_frame).min(content_len);
            (raw_end..=content_len)
                .find(|&i| ocr_result.is_char_boundary(i))
                .unwrap_or(content_len)
        };

        boundaries.push(crate::types::PageBoundary {
            byte_start: byte_offset,
            byte_end: frame_end,
            page_number: frame_num as u32,
        });

        let frame_text = &ocr_result[byte_offset..frame_end];
        page_contents.push(crate::types::PageContent {
            page_number: frame_num as u32,
            content: frame_text.to_string(),
            tables: vec![],
            image_indices: vec![],
            image_preprocessing: None,
            hierarchy: None,
            is_blank: Some(crate::extraction::blank_detection::is_page_text_blank(frame_text)),
            layout_regions: None,
            speaker_notes: None,
            section_name: None,
            sheet_name: None,
            ocr_confidence: None,
        });

        byte_offset = frame_end;
    }

    Ok(ImageOcrResult {
        content: ocr_result,
        boundaries: Some(boundaries),
        page_contents: Some(page_contents),
    })
}

#[cfg(test)]
mod tests {
    use super::*;
    use image::{ImageBuffer, ImageFormat, Rgb, RgbImage};
    use std::io::Cursor;

    fn create_test_image(width: u32, height: u32, format: ImageFormat) -> Vec<u8> {
        let img: RgbImage = ImageBuffer::from_fn(width, height, |x, y| {
            let r = ((x as f32 / width as f32) * 255.0) as u8;
            let g = ((y as f32 / height as f32) * 255.0) as u8;
            let b = 128;
            Rgb([r, g, b])
        });

        let mut bytes: Vec<u8> = Vec::new();
        let mut cursor = Cursor::new(&mut bytes);
        img.write_to(&mut cursor, format).unwrap();
        bytes
    }

    fn image_decode_limits(max_content_size: usize) -> crate::extractors::security::SecurityLimits {
        crate::extractors::security::SecurityLimits {
            max_content_size,
            ..Default::default()
        }
    }

    #[cfg(feature = "ocr")]
    #[test]
    fn should_reject_oversized_declared_dimensions_before_ocr_decode() {
        let bytes = crate::extraction::image_decode::bmp_with_declared_dimensions(100, 100);
        let limits = image_decode_limits(1024);

        let error = decode_image_with_security_limits(&bytes, &limits)
            .expect_err("oversized decoded dimensions must be rejected from the header probe");

        assert!(matches!(error, XbergError::Validation { .. }));
        assert!(error.to_string().contains("100x100"));
        assert!(error.to_string().contains("security_limits.max_content_size"));
    }

    #[cfg(feature = "ocr")]
    #[test]
    fn should_decode_normal_image_within_security_budget() {
        let bytes = create_test_image(2, 2, ImageFormat::Png);
        let limits = image_decode_limits(1024);

        let image = decode_image_with_security_limits(&bytes, &limits)
            .expect("normal image within the decoded-byte budget should load");

        assert_eq!((image.width(), image.height()), (2, 2));
    }

    /// GH#1554 regression: `load_image_for_ocr` hardcoded `SecurityLimits::default()`
    /// instead of taking the caller's configured limits, so a legitimate high-resolution
    /// scan the caller had explicitly permitted was refused anyway. A 6100x6100 solid-color
    /// image decodes to 6100 * 6100 * 3 = 111,630,000 bytes, which exceeds the default
    /// `max_content_size` of 100 MiB (104,857,600 bytes) but fits comfortably under a
    /// caller-configured 200 MiB limit. PNG compresses a solid color to a few hundred bytes,
    /// so the encoded fixture stays small even though the decoded budget does not. ~keep
    #[cfg(feature = "ocr")]
    #[test]
    fn should_permit_high_resolution_scan_under_configured_limit_default_rejects() {
        let width = 6100;
        let height = 6100;
        let img: RgbImage = ImageBuffer::from_pixel(width, height, Rgb([200u8, 100, 50]));
        let mut bytes: Vec<u8> = Vec::new();
        img.write_to(&mut Cursor::new(&mut bytes), ImageFormat::Png).unwrap();

        let default_limits = SecurityLimits::default();
        let default_error = load_image_for_ocr(&bytes, &default_limits)
            .expect_err("a 111,630,000-byte decode must be refused under the 100 MiB default");
        assert!(matches!(default_error, XbergError::Validation { .. }));
        let default_message = default_error.to_string();
        assert!(
            default_message.contains("security_limits.max_content_size (104857600 bytes)"),
            "the refusal must identify the default field and ceiling actually enforced: {default_message}"
        );

        let configured_limits = SecurityLimits {
            max_content_size: 200 * 1024 * 1024,
            ..Default::default()
        };
        let image = load_image_for_ocr(&bytes, &configured_limits)
            .expect("a caller-configured 200 MiB limit must permit the same 111,630,000-byte decode");
        assert_eq!((image.width(), image.height()), (width, height));
    }

    #[test]
    fn metadata_rejects_corrupt_pixels_within_security_budget() {
        let bytes = crate::extraction::image_decode::bmp_with_declared_dimensions(10, 10);
        let limits = image_decode_limits(10_000);

        let error = extract_image_metadata_with_security_limits(&bytes, &limits)
            .expect_err("metadata extraction must validate bounded pixel data, not only the header");

        assert!(matches!(error, XbergError::Parsing { .. }));
    }

    #[test]
    fn test_extract_png_image_returns_correct_metadata() {
        let bytes = create_test_image(100, 80, ImageFormat::Png);
        let result = extract_image_metadata(&bytes);

        assert!(result.is_ok());
        let metadata = result.unwrap();
        assert_eq!(metadata.width, 100);
        assert_eq!(metadata.height, 80);
        assert_eq!(metadata.format, "PNG");
    }

    #[test]
    fn test_extract_jpeg_image_returns_correct_metadata() {
        let bytes = create_test_image(200, 150, ImageFormat::Jpeg);
        let result = extract_image_metadata(&bytes);

        assert!(result.is_ok());
        let metadata = result.unwrap();
        assert_eq!(metadata.width, 200);
        assert_eq!(metadata.height, 150);
        assert_eq!(metadata.format, "JPEG");
    }

    #[test]
    fn test_extract_webp_image_returns_correct_metadata() {
        let bytes = create_test_image(120, 90, ImageFormat::WebP);
        let result = extract_image_metadata(&bytes);

        assert!(result.is_ok());
        let metadata = result.unwrap();
        assert_eq!(metadata.width, 120);
        assert_eq!(metadata.height, 90);
        assert_eq!(metadata.format, "WEBP");
    }

    #[test]
    fn test_extract_bmp_image_returns_correct_metadata() {
        let bytes = create_test_image(50, 50, ImageFormat::Bmp);
        let result = extract_image_metadata(&bytes);

        assert!(result.is_ok());
        let metadata = result.unwrap();
        assert_eq!(metadata.width, 50);
        assert_eq!(metadata.height, 50);
        assert_eq!(metadata.format, "BMP");
    }

    #[test]
    fn test_extract_tiff_image_returns_correct_metadata() {
        let bytes = create_test_image(180, 120, ImageFormat::Tiff);
        let result = extract_image_metadata(&bytes);

        assert!(result.is_ok());
        let metadata = result.unwrap();
        assert_eq!(metadata.width, 180);
        assert_eq!(metadata.height, 120);
        assert_eq!(metadata.format, "TIFF");
    }

    #[test]
    fn test_extract_gif_image_returns_correct_metadata() {
        let bytes = create_test_image(64, 64, ImageFormat::Gif);
        let result = extract_image_metadata(&bytes);

        assert!(result.is_ok());
        let metadata = result.unwrap();
        assert_eq!(metadata.width, 64);
        assert_eq!(metadata.height, 64);
        assert_eq!(metadata.format, "GIF");
    }

    #[test]
    fn test_extract_image_extreme_aspect_ratio() {
        let bytes = create_test_image(1000, 10, ImageFormat::Png);
        let result = extract_image_metadata(&bytes);

        assert!(result.is_ok());
        let metadata = result.unwrap();
        assert_eq!(metadata.width, 1000);
        assert_eq!(metadata.height, 10);
        assert!(metadata.width / metadata.height >= 100);
    }

    #[test]
    fn test_extract_image_dimensions_correctly() {
        let bytes = create_test_image(640, 480, ImageFormat::Png);
        let result = extract_image_metadata(&bytes);

        assert!(result.is_ok());
        let metadata = result.unwrap();
        assert_eq!(metadata.width, 640);
        assert_eq!(metadata.height, 480);
    }

    #[test]
    fn test_extract_image_format_correctly() {
        let png_bytes = create_test_image(100, 100, ImageFormat::Png);
        let jpeg_bytes = create_test_image(100, 100, ImageFormat::Jpeg);

        let png_metadata = extract_image_metadata(&png_bytes).unwrap();
        let jpeg_metadata = extract_image_metadata(&jpeg_bytes).unwrap();

        assert_eq!(png_metadata.format, "PNG");
        assert_eq!(jpeg_metadata.format, "JPEG");
    }

    #[test]
    fn test_extract_image_without_exif_returns_empty_map() {
        let bytes = create_test_image(100, 100, ImageFormat::Png);
        let result = extract_image_metadata(&bytes);

        assert!(result.is_ok());
        let metadata = result.unwrap();
        assert!(metadata.exif_data.is_empty());
    }

    #[test]
    fn test_extract_exif_data_from_jpeg_with_exif() {
        let bytes = create_test_image(100, 100, ImageFormat::Jpeg);
        let result = extract_image_metadata(&bytes);

        assert!(result.is_ok());
        let metadata = result.unwrap();
        assert_eq!(metadata.exif_data.len(), 0);
    }

    #[test]
    fn test_extract_image_metadata_invalid_returns_error() {
        let invalid_bytes = vec![0, 1, 2, 3, 4, 5];
        let result = extract_image_metadata(&invalid_bytes);
        assert!(result.is_err());
    }

    #[test]
    fn test_extract_image_corrupted_data_returns_error() {
        let mut bytes = create_test_image(100, 100, ImageFormat::Png);
        if bytes.len() > 50 {
            for byte in bytes.iter_mut().take(50).skip(20) {
                *byte = 0xFF;
            }
        }

        let _result = extract_image_metadata(&bytes);
    }

    #[test]
    fn test_extract_image_empty_bytes_returns_error() {
        let empty_bytes: Vec<u8> = Vec::new();
        let result = extract_image_metadata(&empty_bytes);
        assert!(result.is_err());
    }

    #[test]
    fn test_extract_image_unsupported_format_returns_error() {
        let unsupported_bytes = vec![0x00, 0x00, 0x00, 0x0C, 0x6A, 0x50, 0x20, 0x20, 0x0D, 0x0A, 0x87, 0x0A];
        let result = extract_image_metadata(&unsupported_bytes);
        assert!(result.is_err());
    }

    #[test]
    fn test_extract_very_small_image_1x1_pixel() {
        let bytes = create_test_image(1, 1, ImageFormat::Png);
        let result = extract_image_metadata(&bytes);

        assert!(result.is_ok());
        let metadata = result.unwrap();
        assert_eq!(metadata.width, 1);
        assert_eq!(metadata.height, 1);
        assert_eq!(metadata.format, "PNG");
    }

    #[test]
    fn test_extract_large_image_dimensions() {
        let bytes = create_test_image(2048, 1536, ImageFormat::Png);
        let result = extract_image_metadata(&bytes);

        assert!(result.is_ok());
        let metadata = result.unwrap();
        assert_eq!(metadata.width, 2048);
        assert_eq!(metadata.height, 1536);
    }

    #[test]
    fn test_extract_image_with_no_metadata_has_empty_exif() {
        let bytes = create_test_image(100, 100, ImageFormat::Png);
        let result = extract_image_metadata(&bytes);

        assert!(result.is_ok());
        let metadata = result.unwrap();
        assert!(metadata.exif_data.is_empty());
    }

    #[cfg(feature = "heic")]
    #[test]
    fn test_extract_image_metadata_handles_heic() {
        for (label, relative) in [
            ("heic", "images/test.heic"),
            ("heif", "images/test.heif"),
            ("avif", "images/test.avif"),
        ] {
            let Some(bytes) = crate::utils::read_test_fixture(relative) else {
                continue;
            };
            let meta = extract_image_metadata(&bytes).unwrap_or_else(|e| panic!("{label}: {e}"));
            assert!(meta.width > 0, "{label}: width should be > 0");
            assert!(meta.height > 0, "{label}: height should be > 0");
            assert_eq!(meta.format, "HEIF", "{label}: unexpected format tag");
        }
    }

    #[cfg(not(feature = "heic"))]
    #[test]
    fn test_extract_image_metadata_heic_without_feature_errors() {
        let mut heic_stub = Vec::from(&b"\x00\x00\x00\x18ftypheicheic"[..]);
        heic_stub.extend_from_slice(&[0u8; 12]);
        let err = extract_image_metadata(&heic_stub).expect_err("heic without feature should error");
        let msg = err.to_string();
        assert!(msg.contains("heic"), "expected `heic` mention in error: {msg}");
    }

    #[test]
    fn test_extract_exif_data_returns_empty_map_for_non_jpeg() {
        let png_bytes = create_test_image(100, 100, ImageFormat::Png);
        let exif_data = extract_exif_data(&png_bytes);
        assert!(exif_data.is_empty());
    }

    #[test]
    fn test_extract_rectangular_image_portrait_orientation() {
        let bytes = create_test_image(400, 800, ImageFormat::Jpeg);
        let result = extract_image_metadata(&bytes);

        assert!(result.is_ok());
        let metadata = result.unwrap();
        assert_eq!(metadata.width, 400);
        assert_eq!(metadata.height, 800);
        assert!(metadata.height > metadata.width);
    }

    #[test]
    fn test_extract_rectangular_image_landscape_orientation() {
        let bytes = create_test_image(800, 400, ImageFormat::Png);
        let result = extract_image_metadata(&bytes);

        assert!(result.is_ok());
        let metadata = result.unwrap();
        assert_eq!(metadata.width, 800);
        assert_eq!(metadata.height, 400);
        assert!(metadata.width > metadata.height);
    }

    #[test]
    fn test_extract_square_image_equal_dimensions() {
        let bytes = create_test_image(512, 512, ImageFormat::Png);
        let result = extract_image_metadata(&bytes);

        assert!(result.is_ok());
        let metadata = result.unwrap();
        assert_eq!(metadata.width, 512);
        assert_eq!(metadata.height, 512);
        assert_eq!(metadata.width, metadata.height);
    }

    #[test]
    fn test_extract_metadata_preserves_format_case() {
        let png_bytes = create_test_image(100, 100, ImageFormat::Png);
        let jpeg_bytes = create_test_image(100, 100, ImageFormat::Jpeg);
        let webp_bytes = create_test_image(100, 100, ImageFormat::WebP);

        let png_meta = extract_image_metadata(&png_bytes).unwrap();
        let jpeg_meta = extract_image_metadata(&jpeg_bytes).unwrap();
        let webp_meta = extract_image_metadata(&webp_bytes).unwrap();

        assert_eq!(png_meta.format, "PNG");
        assert_eq!(jpeg_meta.format, "JPEG");
        assert_eq!(webp_meta.format, "WEBP");
    }

    #[test]
    fn test_jp2_magic_detection() {
        assert!(is_jp2(&[0x00, 0x00, 0x00, 0x0C, 0x6A, 0x50, 0x20, 0x20, 0x0D, 0x0A]));
        assert!(!is_jp2(&[0x89, 0x50, 0x4E, 0x47]));
        assert!(!is_jp2(&[0x00, 0x00]));
        assert!(!is_jp2(&[]));
    }

    #[test]
    fn test_extract_jp2_rust_logo_metadata() {
        let Some(bytes) = crate::utils::read_test_fixture("images/rust-logo-512x512-blk.jp2") else {
            return;
        };
        let result = extract_image_metadata(&bytes);
        assert!(result.is_ok(), "Failed to extract JP2 metadata: {:?}", result.err());
        let metadata = result.unwrap();
        assert_eq!(metadata.width, 512);
        assert_eq!(metadata.height, 512);
        assert_eq!(metadata.format, "JPEG2000");
    }

    #[test]
    fn test_extract_jp2_hadley_crater_metadata() {
        let Some(bytes) = crate::utils::read_test_fixture("images/Hadley_Crater.jp2") else {
            return;
        };
        let result = extract_image_metadata(&bytes);
        assert!(result.is_ok(), "Failed to extract JP2 metadata: {:?}", result.err());
        let metadata = result.unwrap();
        assert!(metadata.width > 0);
        assert!(metadata.height > 0);
        assert_eq!(metadata.format, "JPEG2000");
    }

    #[test]
    fn test_parse_jp2_boxes_invalid_data() {
        let invalid = vec![0x00, 0x00, 0x00, 0x0C, 0x6A, 0x50, 0x20, 0x20, 0x0D, 0x0A, 0x87, 0x0A];
        let result = decode_jp2_metadata(&invalid);
        assert!(result.is_err());
    }

    #[test]
    fn test_jp2_magic_detection_comprehensive() {
        assert!(is_jp2(&[
            0x00, 0x00, 0x00, 0x0C, 0x6A, 0x50, 0x20, 0x20, 0x0D, 0x0A, 0x87, 0x0A
        ]));
        assert!(!is_jp2(&[0xFF, 0x4F, 0xFF, 0x51]));
        assert!(!is_jp2(&[0x89, 0x50, 0x4E, 0x47]));
        assert!(!is_jp2(&[]));
    }

    // GH#1630: PNG pHYs density detection.
    #[cfg(feature = "ocr")]
    mod png_density {
        use super::*;
        use image::ImageFormat;

        /// Standard PNG CRC-32 (polynomial 0xEDB88320), needed to splice a well-formed `pHYs`
        /// chunk into a real encoded PNG for the test fixtures below.
        fn png_crc32(data: &[u8]) -> u32 {
            let mut crc: u32 = 0xFFFF_FFFF;
            for &byte in data {
                crc ^= u32::from(byte);
                for _ in 0..8 {
                    let mask = (crc & 1).wrapping_neg();
                    crc = (crc >> 1) ^ (0xEDB8_8320 & mask);
                }
            }
            !crc
        }

        fn png_chunk(chunk_type: &[u8; 4], data: &[u8]) -> Vec<u8> {
            let mut body = Vec::with_capacity(4 + data.len());
            body.extend_from_slice(chunk_type);
            body.extend_from_slice(data);
            let mut out = Vec::with_capacity(4 + body.len() + 4);
            out.extend_from_slice(&(u32::try_from(data.len()).unwrap()).to_be_bytes());
            out.extend_from_slice(&body);
            out.extend_from_slice(&png_crc32(&body).to_be_bytes());
            out
        }

        /// Splice a `pHYs` chunk expressing `ppu` pixels-per-metre (both axes) into a real
        /// encoded PNG, immediately after IHDR (signature 8 bytes + IHDR chunk 25 bytes), which
        /// is where an encoder conventionally places it and always before IDAT.
        fn png_with_phys(png_bytes: &[u8], ppu: u32) -> Vec<u8> {
            const SIGNATURE_AND_IHDR_LEN: usize = 8 + 25;
            let mut data = Vec::with_capacity(9);
            data.extend_from_slice(&ppu.to_be_bytes());
            data.extend_from_slice(&ppu.to_be_bytes());
            data.push(1); // unit = meter
            let phys = png_chunk(b"pHYs", &data);

            let mut out = Vec::with_capacity(png_bytes.len() + phys.len());
            out.extend_from_slice(&png_bytes[..SIGNATURE_AND_IHDR_LEN]);
            out.extend_from_slice(&phys);
            out.extend_from_slice(&png_bytes[SIGNATURE_AND_IHDR_LEN..]);
            out
        }

        /// 11811 px/m is the reporter's fixture value: 11811 * 0.0254 = 299.9994 DPI, not an
        /// exact 300 — the detector and any consumer must tolerate that rather than requiring an
        /// exact integer match.
        const ELEVEN_THOUSAND_EIGHT_HUNDRED_ELEVEN_PPU: u32 = 11811;
        const EXPECTED_DPI_FROM_11811_PPU: f64 = 299.999_4;
        const DPI_TOLERANCE: f64 = 0.001;

        #[test]
        fn should_decode_source_density_from_png_phys_chunk() {
            let png = create_test_image(4, 4, ImageFormat::Png);
            let with_phys = png_with_phys(&png, ELEVEN_THOUSAND_EIGHT_HUNDRED_ELEVEN_PPU);

            let dpi = png_pixel_density_dpi(&with_phys).expect("pHYs chunk must be detected");

            assert!(
                (dpi - EXPECTED_DPI_FROM_11811_PPU).abs() < DPI_TOLERANCE,
                "expected ~{EXPECTED_DPI_FROM_11811_PPU} DPI, got {dpi}"
            );
        }

        #[test]
        fn should_return_none_for_png_without_phys_chunk() {
            let png = create_test_image(4, 4, ImageFormat::Png);

            assert_eq!(
                png_pixel_density_dpi(&png),
                None,
                "a PNG with no embedded density metadata must not report a fabricated DPI"
            );
        }

        #[test]
        fn should_return_none_for_non_png_bytes() {
            assert_eq!(png_pixel_density_dpi(b"not a png"), None);
            assert_eq!(png_pixel_density_dpi(&[]), None);
        }

        #[test]
        fn should_return_none_for_unspecified_phys_unit() {
            let png = create_test_image(4, 4, ImageFormat::Png);
            let mut with_phys = png_with_phys(&png, ELEVEN_THOUSAND_EIGHT_HUNDRED_ELEVEN_PPU);
            // Flip the unit byte (last byte of the 9-byte pHYs data) from meter (1) to
            // unspecified (0): signature(8) + IHDR(25) + pHYs length/type(8) + xppu/yppu(8).
            let unit_byte_offset = 8 + 25 + 8 + 8;
            with_phys[unit_byte_offset] = 0;

            assert_eq!(png_pixel_density_dpi(&with_phys), None);
        }
    }
}

#[cfg(all(test, feature = "ocr"))]
mod jp2_decode_tests {
    use super::*;

    #[test]
    fn jp2_peak_counts_encoded_input_between_old_and_new_thresholds() {
        let peak = jp2_peak_live_bytes(10, 10, 1, false, 100).expect("valid JP2 peak");
        let limits = SecurityLimits {
            max_content_size: 450,
            ..Default::default()
        };

        let error = ImageDecodeBudget::from_security_limits(&limits)
            .validate(10, 10, peak)
            .expect_err("encoded JP2 bytes must remain live alongside gray-to-RGB conversion");

        assert!(matches!(error, XbergError::Validation { .. }));
    }

    #[test]
    fn jbig2_peaks_count_encoded_input_and_gray_to_rgb_conversion() {
        let decode_peak = jbig2_gray_peak_live_bytes(10, 10, 100).expect("valid JBIG2 decode peak");
        let conversion_peak = jbig2_rgb_peak_live_bytes(10, 10, 100).expect("valid JBIG2 RGB peak");

        assert_eq!(decode_peak, 200);
        assert_eq!(conversion_peak, 500);
        let limits = SecurityLimits {
            max_content_size: 450,
            ..Default::default()
        };
        let error = ImageDecodeBudget::from_security_limits(&limits)
            .validate(10, 10, conversion_peak)
            .expect_err("encoded JBIG2, gray pixels, and RGB pixels must be live together");
        assert!(matches!(error, XbergError::Validation { .. }));
    }

    #[test]
    fn test_decode_jp2_to_rgb() {
        let Some(bytes) = crate::utils::read_test_fixture("images/rust-logo-512x512-blk.jp2") else {
            return;
        };
        let rgb = decode_jp2_to_rgb(&bytes).expect("Should decode JP2 to RGB");
        assert_eq!(rgb.width(), 512);
        assert_eq!(rgb.height(), 512);
    }

    #[test]
    fn test_is_j2k() {
        assert!(!is_j2k(&[]));
        assert!(!is_j2k(&[0xFF]));
        assert!(is_j2k(&[0xFF, 0x4F, 0xFF, 0x51, 0x00]));
        assert!(!is_j2k(&[0xFF, 0x4F, 0x00, 0x51]));
    }

    #[test]
    fn test_jbig2_magic_detection() {
        assert!(is_jbig2(&[0x97, 0x4A, 0x42, 0x32, 0x0D, 0x0A, 0x1A, 0x0A, 0x01]));
        assert!(!is_jbig2(&[0x89, 0x50, 0x4E, 0x47]));
        assert!(!is_jbig2(&[]));
        assert!(!is_jbig2(&[0x97, 0x4A]));
    }
}
