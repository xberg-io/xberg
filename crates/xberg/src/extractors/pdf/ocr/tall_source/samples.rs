//! Exact, budgeted decoding of grayscale-equivalent source samples.

use super::{MAX_SOURCE_PIXELS, MAX_STRIPS, MAX_TILE_SIDE};
use xberg_native_pdf::geometry::Rect;
use xberg_native_pdf::object::Object;

#[derive(Clone, Copy, PartialEq, Eq)]
pub(super) enum GrayPacking {
    Byte,
    Bilevel,
    NeutralRgb,
}

pub(super) fn decode_source_gray(
    compressed: &[u8],
    width: usize,
    height: usize,
    packing: GrayPacking,
    byte_limit: usize,
) -> Option<(Vec<u8>, usize)> {
    let pixels = width.checked_mul(height)?;
    let retained_bytes = compressed.len().checked_add(pixels)?.checked_add(1)?;
    if retained_bytes > byte_limit {
        return None;
    }
    if packing == GrayPacking::Byte {
        return Some((decode_exact_gray(compressed, pixels)?, retained_bytes));
    }
    let stride = match packing {
        GrayPacking::Byte => return None,
        GrayPacking::Bilevel => width.div_ceil(8),
        GrayPacking::NeutralRgb => width.checked_mul(3)?,
    };
    let packed_size = stride.checked_mul(height)?;
    if stride == 0 || retained_bytes.checked_add(packed_size)? > byte_limit {
        return None;
    }
    let packed = decode_exact_gray(compressed, packed_size)?;
    let mut samples = Vec::new();
    samples.try_reserve_exact(pixels).ok()?;
    match packing {
        GrayPacking::Byte => return None,
        GrayPacking::Bilevel => {
            // PDF starts each packed row on a byte boundary, with the first sample in the high bit. ~keep
            for row in packed.chunks_exact(stride) {
                samples.extend((0..width).map(|x| if row[x / 8] & (0x80 >> (x % 8)) == 0 { 0 } else { 255 }));
            }
        }
        GrayPacking::NeutralRgb => {
            for &[red, green, blue] in packed.as_chunks::<3>().0 {
                if red != green || green != blue {
                    return None;
                }
                samples.push(red);
            }
        }
    }
    Some((samples, retained_bytes))
}

pub(super) fn decode_exact_gray(compressed: &[u8], pixels: usize) -> Option<Vec<u8>> {
    use flate2::{Decompress, FlushDecompress, Status};

    let capacity = pixels.checked_add(1)?;
    let mut samples = Vec::new();
    samples.try_reserve_exact(capacity).ok()?;
    samples.resize(capacity, 0);
    let mut decoder = Decompress::new(true);
    let status = decoder
        .decompress(compressed, &mut samples, FlushDecompress::Finish)
        .ok()?;
    if status != Status::StreamEnd
        || decoder.total_out() != pixels as u64
        || decoder.total_in() != compressed.len() as u64
    {
        return None;
    }
    samples.truncate(pixels);
    Some(samples)
}

pub(super) fn source_frame(
    dict: &std::collections::HashMap<String, Object>,
    frame: Rect,
    rendered_width: Option<u32>,
) -> Option<(u32, u32, f64)> {
    let width = u32::try_from(dict.get("Width")?.as_integer()?).ok()?;
    let height = u32::try_from(dict.get("Height")?.as_integer()?).ok()?;
    let pixels = (width as usize).checked_mul(height as usize)?;
    if rendered_width.is_some_and(|rendered| u64::from(rendered) * 100 >= u64::from(width) * 98)
        || width > MAX_TILE_SIDE
        || height <= MAX_TILE_SIDE
        || height > MAX_TILE_SIDE * MAX_STRIPS as u32
        || pixels > MAX_SOURCE_PIXELS
    {
        return None;
    }
    let dpi = 72.0 * f64::from(width) / f64::from(frame.width);
    let dpi_y = 72.0 * f64::from(height) / f64::from(frame.height);
    if !(150.0..=300.0).contains(&dpi) || (dpi - dpi_y).abs() > 0.01 {
        return None;
    }
    Some((width, height, dpi))
}
