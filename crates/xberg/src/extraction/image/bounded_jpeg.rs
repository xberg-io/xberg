mod budget;
mod headers;

#[cfg(test)]
mod tests;

use oxiarc_jpeg::{ColorSpace, DecodeLimits, DecodeOptions, Scale, decode_abbreviated_into};

use crate::error::{Result, XbergError};
use crate::extraction::image_decode::{decoded_byte_count, image_dimension_error};
use crate::extractors::security::SecurityLimits;

use super::{DecodedOcrImage, decode_image_to_rgb8_with_security_limits, validate_encoded_image_input};

pub(crate) fn load_image_for_ocr_with_scale(image_bytes: &[u8], limits: &SecurityLimits) -> Result<DecodedOcrImage> {
    if image_bytes.starts_with(&[0xff, 0xd8]) {
        validate_encoded_image_input(image_bytes, limits)?;
        if let Some(decoded) = decode_if_needed(image_bytes, limits)? {
            return Ok(decoded);
        }
    }
    let image = decode_image_to_rgb8_with_security_limits(image_bytes, limits)?;
    Ok(DecodedOcrImage {
        original_width: image.width(),
        original_height: image.height(),
        image,
        scale_denominator: 1,
    })
}

fn decode_if_needed(bytes: &[u8], limits: &SecurityLimits) -> Result<Option<DecodedOcrImage>> {
    let Some((width, height)) = headers::dimensions(bytes) else {
        return Ok(None);
    };
    let encoded_bytes = u64::try_from(bytes.len()).unwrap_or(u64::MAX);
    let maximum = u64::try_from(limits.max_content_size).unwrap_or(u64::MAX);
    let full_rgb = decoded_byte_count(width, height, 3)?.checked_add(encoded_bytes);
    if full_rgb.is_some_and(|peak| peak <= maximum) {
        return Ok(None);
    }
    let header = headers::inspect(bytes)?;
    let plan = budget::select(&header, encoded_bytes, maximum)
        .ok_or_else(|| image_dimension_error(width, height, budget::minimum_peak(&header, encoded_bytes), maximum))?;
    decode(bytes, &header, &plan).map(Some)
}

fn decode(bytes: &[u8], header: &headers::Header, plan: &budget::Plan) -> Result<DecodedOcrImage> {
    let options = DecodeOptions {
        scale: match plan.denominator {
            2 => Scale::ONE_HALF,
            4 => Scale::ONE_QUARTER,
            _ => Scale::ONE_EIGHTH,
        },
        output_color_space: Some(ColorSpace::Rgb),
        limits: DecodeLimits {
            max_width: header.width,
            max_height: header.height,
            max_pixels: u64::from(header.width) * u64::from(header.height),
            max_components: 3,
            max_scans: 1,
            max_coefficient_bytes: 0,
            max_output_bytes: plan.plane_bytes.max(plan.output_bytes as u64),
            max_input_bytes: bytes.len() as u64,
        },
        ..DecodeOptions::default()
    };
    let mut pixels = Vec::new();
    pixels
        .try_reserve_exact(plan.output_bytes)
        .map_err(|error| XbergError::parsing(format!("Cannot reserve reduced JPEG output: {error}")))?;
    pixels.resize(plan.output_bytes, 0);
    let info = decode_abbreviated_into(None, bytes, &options, &mut pixels)
        .map_err(|error| XbergError::parsing(format!("Reduced JPEG decoding failed: {error}")))?;
    if (info.scaled_width, info.scaled_height) != (plan.width, plan.height) {
        return Err(headers::invalid("decoded dimensions differ from the admitted output"));
    }
    let image = image::RgbImage::from_raw(plan.width, plan.height, pixels)
        .ok_or_else(|| headers::invalid("decoded RGB length differs from its admitted dimensions"))?;
    Ok(DecodedOcrImage {
        image,
        original_width: header.width,
        original_height: header.height,
        scale_denominator: plan.denominator,
    })
}
