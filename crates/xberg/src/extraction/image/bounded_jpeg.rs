//! Reduced-size JPEG decoding for OCR when the full-resolution raster does not fit
//! `security_limits.max_content_size`.
//!
//! `jpeg-decoder` scales in the IDCT (1/2, 1/4, 1/8), so a reduced decode never holds the
//! full-resolution pixels. The peak is bounded from the frame header before decoding: the
//! scaled component planes, two MCU rows of coefficients (the row being filled and the row the
//! worker takes), the output, the RGB conversion of a grayscale output, and a copy of the encoded
//! bytes for retained APP data. A progressive scan keeps all coefficients at full resolution, and
//! its final pass copies every row of one component before the worker runs. The `rayon` worker
//! finishes each sequential row before the next one is read; the thread worker that `jpeg-decoder`
//! uses without `rayon` queues rows without a bound. `jpeg-decoder`'s own output-size check is set
//! to the admitted output as a second stop.

#[cfg(test)]
mod tests;

use crate::error::{Result, XbergError};
use crate::extraction::image_decode::{decoded_byte_count, image_dimension_error};
use crate::extractors::security::SecurityLimits;

use super::{DecodedOcrImage, decode_image_to_rgb8_with_security_limits, validate_encoded_image_input};

const DENOMINATORS: [u8; 3] = [2, 4, 8];
const COEFFICIENT_BYTES: u64 = 64 * 2;
const RGB_CHANNELS: u64 = 3;

/// One reduced-decode choice.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub(crate) struct ReducedScale {
    pub width: u32,
    pub height: u32,
    pub denominator: u8,
}

#[derive(Clone, Copy, PartialEq, Eq)]
enum Process {
    Sequential,
    Progressive,
    Other,
}

struct Frame {
    width: u32,
    height: u32,
    process: Process,
    /// Horizontal and vertical sampling factor of each component.
    sampling: Vec<(u8, u8)>,
}

/// Decode a JPEG for OCR. A JPEG whose full RGB raster does not fit the limit is decoded at
/// the finest IDCT scale that fits.
pub(crate) fn load_image_for_ocr_with_scale(image_bytes: &[u8], limits: &SecurityLimits) -> Result<DecodedOcrImage> {
    if let Some(decoded) = decode_oversized_jpeg(image_bytes, limits)? {
        return Ok(decoded);
    }
    let image = decode_image_to_rgb8_with_security_limits(image_bytes, limits)?;
    Ok(DecodedOcrImage {
        original_width: image.width(),
        original_height: image.height(),
        image,
        scale_denominator: 1,
    })
}

/// Decode a JPEG at a reduced size when its full RGB raster does not fit the limit; `None` for
/// any other input, which the full decoder then takes.
pub(crate) fn decode_oversized_jpeg(bytes: &[u8], limits: &SecurityLimits) -> Result<Option<DecodedOcrImage>> {
    if !bytes.starts_with(&[0xff, 0xd8]) {
        return Ok(None);
    }
    validate_encoded_image_input(bytes, limits)?;
    decode_if_needed(bytes, limits)
}

fn decode_if_needed(bytes: &[u8], limits: &SecurityLimits) -> Result<Option<DecodedOcrImage>> {
    let Some(frame) = read_frame(bytes) else {
        return Ok(None);
    };
    let encoded = u64::try_from(bytes.len()).unwrap_or(u64::MAX);
    let maximum = u64::try_from(limits.max_content_size).unwrap_or(u64::MAX);
    let full_peak = full_decode_peak(&frame, encoded)?;
    if full_peak <= maximum {
        return Ok(None);
    }
    let too_large = || image_dimension_error(frame.width, frame.height, full_peak, maximum);
    if frame.process == Process::Other || !matches!(frame.sampling.len(), 1 | 3) {
        return Err(too_large());
    }
    let Some(scale) = DENOMINATORS
        .iter()
        .map(|&denominator| reduced_scale(&frame, denominator))
        .find(|&scale| reduced_peak(&frame, scale, encoded).is_some_and(|peak| peak <= maximum))
    else {
        return Err(too_large());
    };
    decode(bytes, &frame, scale).map(Some)
}

fn full_decode_peak(frame: &Frame, encoded: u64) -> Result<u64> {
    // The full decoder holds the decoded raster and, for grayscale, its RGB conversion. ~keep
    let channels = if frame.sampling.len() == 3 {
        RGB_CHANNELS
    } else {
        frame.sampling.len() as u64 + RGB_CHANNELS
    };
    decoded_byte_count(frame.width, frame.height, channels)?
        .checked_add(encoded)
        .ok_or_else(|| image_dimension_error(frame.width, frame.height, u64::MAX, u64::MAX))
}

fn reduced_scale(frame: &Frame, denominator: u8) -> ReducedScale {
    let d = u32::from(denominator);
    ReducedScale {
        width: frame.width.div_ceil(d),
        height: frame.height.div_ceil(d),
        denominator,
    }
}

/// Upper bound on the live bytes of a `jpeg-decoder` decode at `scale`, following its
/// component layout (`update_component_sizes`) and immediate worker.
fn reduced_peak(frame: &Frame, scale: ReducedScale, encoded: u64) -> Option<u64> {
    let h_max = u64::from(frame.sampling.iter().map(|s| s.0).max()?);
    let v_max = u64::from(frame.sampling.iter().map(|s| s.1).max()?);
    let mcu_columns = u64::from(frame.width).div_ceil(h_max * 8);
    let mcu_rows = u64::from(frame.height).div_ceil(v_max * 8);
    let idct = 8 / u64::from(scale.denominator);
    let mut planes = 0u64;
    let mut coefficients = 0u64;
    let mut largest_component = 0u64;
    for &(h, v) in &frame.sampling {
        let blocks_wide = mcu_columns.checked_mul(u64::from(h))?;
        let blocks_high = mcu_rows.checked_mul(u64::from(v))?;
        let blocks = blocks_wide.checked_mul(blocks_high)?;
        planes = planes.checked_add(blocks.checked_mul(idct * idct)?)?;
        let component = if frame.process == Process::Progressive {
            blocks.checked_mul(COEFFICIENT_BYTES)?
        } else {
            blocks_wide
                .checked_mul(u64::from(v))?
                .checked_mul(2 * COEFFICIENT_BYTES)?
        };
        largest_component = largest_component.max(component);
        coefficients = coefficients.checked_add(component)?;
    }
    if frame.process == Process::Progressive {
        coefficients = coefficients.checked_add(largest_component)?;
    }
    let pixels = u64::from(scale.width).checked_mul(u64::from(scale.height))?;
    let output = pixels.checked_mul(frame.sampling.len() as u64)?;
    let decoding = planes.checked_add(coefficients)?.checked_add(output)?;
    let converting = if frame.sampling.len() == 1 {
        output.checked_add(pixels.checked_mul(RGB_CHANNELS)?)?
    } else {
        output
    };
    decoding.max(converting).checked_add(encoded.checked_mul(2)?)
}

fn decode(bytes: &[u8], frame: &Frame, scale: ReducedScale) -> Result<DecodedOcrImage> {
    let failed =
        |error: jpeg_decoder::Error| XbergError::parsing(format!("The JPEG image could not be decoded: {error}"));
    let mut decoder = jpeg_decoder::Decoder::new(bytes);
    let requested = |value: u32| {
        u16::try_from(value).map_err(|_| image_dimension_error(frame.width, frame.height, u64::MAX, u64::MAX))
    };
    let actual = decoder
        .scale(requested(scale.width)?, requested(scale.height)?)
        .map_err(failed)?;
    if (u32::from(actual.0), u32::from(actual.1)) != (scale.width, scale.height) {
        return Err(XbergError::parsing(
            "The JPEG image could not be decoded at the admitted reduced size",
        ));
    }
    let output_bytes = usize::try_from(u64::from(scale.width) * u64::from(scale.height) * frame.sampling.len() as u64)
        .map_err(|_| image_dimension_error(scale.width, scale.height, u64::MAX, u64::MAX))?;
    decoder.set_max_decoding_buffer_size(output_bytes);
    let pixels = decoder.decode().map_err(failed)?;
    let image = match decoder.info().map(|info| info.pixel_format) {
        Some(jpeg_decoder::PixelFormat::RGB24) => image::RgbImage::from_raw(scale.width, scale.height, pixels),
        Some(jpeg_decoder::PixelFormat::L8) => image::GrayImage::from_raw(scale.width, scale.height, pixels)
            .map(|gray| image::DynamicImage::ImageLuma8(gray).into_rgb8()),
        _ => None,
    }
    .ok_or_else(|| XbergError::parsing("The JPEG image decoded to an unexpected pixel layout"))?;
    Ok(DecodedOcrImage {
        image,
        original_width: frame.width,
        original_height: frame.height,
        scale_denominator: scale.denominator,
    })
}

/// Read the frame header (dimensions, coding process, sampling factors). `None` when the
/// bytes have no readable frame header; the full decoder then reports the problem.
fn read_frame(bytes: &[u8]) -> Option<Frame> {
    let mut position = 2;
    loop {
        if *bytes.get(position)? != 0xff {
            return None;
        }
        while bytes.get(position) == Some(&0xff) {
            position += 1;
        }
        let marker = *bytes.get(position)?;
        position += 1;
        if matches!(marker, 0x01 | 0xd0..=0xd7) {
            continue;
        }
        if matches!(marker, 0xd9 | 0xda) {
            return None;
        }
        let length = usize::from(u16::from_be_bytes([*bytes.get(position)?, *bytes.get(position + 1)?]));
        let payload = bytes.get(position + 2..position.checked_add(length)?)?;
        if matches!(marker, 0xc0..=0xc3 | 0xc5..=0xc7 | 0xc9..=0xcb | 0xcd..=0xcf) {
            let height = u32::from(u16::from_be_bytes([*payload.get(1)?, *payload.get(2)?]));
            let width = u32::from(u16::from_be_bytes([*payload.get(3)?, *payload.get(4)?]));
            let count = usize::from(*payload.get(5)?);
            let sampling = (0..count)
                .map(|index| payload.get(7 + index * 3).map(|factors| (factors >> 4, factors & 0x0f)))
                .collect::<Option<Vec<_>>>()?;
            if width == 0
                || height == 0
                || sampling.is_empty()
                || sampling
                    .iter()
                    .any(|&(h, v)| !(1..=4).contains(&h) || !(1..=4).contains(&v))
            {
                return None;
            }
            let process = match marker {
                0xc0 | 0xc1 => Process::Sequential,
                0xc2 => Process::Progressive,
                _ => Process::Other,
            };
            return Some(Frame {
                width,
                height,
                process,
                sampling,
            });
        }
        position = position.checked_add(length)?;
    }
}
