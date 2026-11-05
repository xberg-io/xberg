//! Disjoint OCR strip geometry using blank bands certified against the source image.

use std::ops::Range;

#[derive(Clone, Copy)]
pub(super) struct StripLimits {
    pub(super) max_dimension: u32,
    pub(super) max_lookback: u32,
    pub(super) max_tiles: usize,
    pub(super) max_tile_pixels: u64,
    pub(super) max_total_pixels: u64,
}

#[derive(Debug, PartialEq, Eq)]
pub(super) enum Refusal {
    Unneeded,
    Bounds,
    Budget,
    Ambiguous,
    Cancelled,
}

#[derive(Debug, PartialEq, Eq)]
pub(super) struct StripPlan {
    pub(super) width: u32,
    pub(super) rows: Vec<Range<u32>>,
    pub(super) total_pixels: u64,
}

/// Blank bands must be certified against the source pixels before planning.
/// Without a certified cut within the tile bounds, leave the page on its existing route.
pub(super) fn plan_strips(
    width: u32,
    height: u32,
    limits: StripLimits,
    blank_bands: &[Range<u32>],
    is_cancelled: impl Fn() -> bool,
) -> Result<StripPlan, Refusal> {
    if is_cancelled() {
        return Err(Refusal::Cancelled);
    }
    if width == 0 || height == 0 || width > limits.max_dimension {
        return Err(Refusal::Bounds);
    }
    if height <= limits.max_dimension {
        return Err(Refusal::Unneeded);
    }
    let max_height = (limits.max_tile_pixels / u64::from(width)).min(u64::from(limits.max_dimension)) as u32;
    let total_pixels = u64::from(width) * u64::from(height);
    if max_height == 0
        || height.div_ceil(max_height) as usize > limits.max_tiles
        || total_pixels > limits.max_total_pixels
    {
        return Err(Refusal::Budget);
    }
    validate_blank_bands(blank_bands, height, &is_cancelled)?;
    let mut rows = Vec::new();
    let mut start = 0_u32;
    while start < height {
        if is_cancelled() {
            return Err(Refusal::Cancelled);
        }
        if rows.len() >= limits.max_tiles {
            return Err(Refusal::Budget);
        }
        let nominal_end = start.saturating_add(max_height).min(height);
        let end = if nominal_end == height {
            height
        } else {
            closest_certified_cut(blank_bands, start, nominal_end, limits.max_lookback).ok_or(Refusal::Ambiguous)?
        };
        rows.push(start..end);
        start = end;
    }
    Ok(StripPlan {
        width,
        rows,
        total_pixels,
    })
}

fn validate_blank_bands(bands: &[Range<u32>], height: u32, is_cancelled: &impl Fn() -> bool) -> Result<(), Refusal> {
    let mut previous_end = 0;
    for band in bands {
        if is_cancelled() {
            return Err(Refusal::Cancelled);
        }
        if band.start < previous_end || band.end > height || band.end.saturating_sub(band.start) < 2 {
            return Err(Refusal::Bounds);
        }
        previous_end = band.end;
    }
    Ok(())
}

fn gap_midpoint(band: &Range<u32>) -> u32 {
    band.start + (band.end - band.start) / 2
}

fn closest_certified_cut(bands: &[Range<u32>], start: u32, end: u32, max_lookback: u32) -> Option<u32> {
    let earliest = end.saturating_sub(max_lookback).max(start + 1);
    let after_end = bands.partition_point(|band| gap_midpoint(band) <= end);
    let cut = gap_midpoint(bands.get(after_end.checked_sub(1)?)?);
    (cut >= earliest).then_some(cut)
}

#[cfg(test)]
#[path = "tall_geometry/tests.rs"]
mod tests;
