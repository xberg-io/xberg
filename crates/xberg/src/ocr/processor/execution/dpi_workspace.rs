use crate::types::ImageDpiConfig;

const RGB_CHANNELS: u64 = 3;
// fast_image_resize 6.1.0 uses Lanczos3 (support3) for shrinking and
// CatmullRom (support2) otherwise. The larger support bounds both. ~keep
const MAX_FILTER_SUPPORT: f64 = 3.0;
const BOUND_BYTES: u64 = 2 * size_of::<u32>() as u64;
const NORMALIZER_CHUNK_BYTES: u64 =
    (size_of::<Vec<i16>>() + size_of::<u32>()).next_multiple_of(align_of::<Vec<i16>>()) as u64;

pub(super) fn check_known_dpi_resize(
    width: u32,
    height: u32,
    source_dpi: f64,
    config: &ImageDpiConfig,
    encoded_bytes: usize,
    limit: usize,
) -> crate::Result<()> {
    if !crate::image::preprocessing::normalization_requires_resize(width, height, config, source_dpi) {
        return Ok(());
    }
    let (target_width, target_height) =
        crate::image::preprocessing::normalized_image_dimensions(width, height, config, Some(source_dpi));
    let peak = resize_peak(width, height, target_width, target_height, encoded_bytes)
        .ok_or_else(|| crate::XbergError::validation("OCR image DPI resize workspace calculation overflowed"))?;
    if peak > limit as u64 {
        return Err(crate::XbergError::validation(format!(
            "OCR image DPI resize workspace for {width}x{height} to {target_width}x{target_height} exceeds \
             security_limits.max_content_size ({limit} bytes). Raise security_limits.max_content_size or lower target_dpi",
        )));
    }
    Ok(())
}

/// Avoid the second rounding at the dimension clamp, which can shrink the short axis as DPI increases.
pub(super) fn largest_unclamped_upscale(
    lower: i32,
    upper: i32,
    long_side: u32,
    source_dpi: f64,
    dimension_cap: i32,
) -> crate::Result<Option<i32>> {
    largest_feasible_upscale(lower, upper, |target| {
        let width = f64::from(long_side) * (f64::from(target) / source_dpi);
        Ok(width.round() <= f64::from(dimension_cap))
    })
}

/// Admission for unclamped upscales is monotone: destination buffers grow while filter support stays fixed.
pub(super) fn largest_feasible_upscale(
    mut lower: i32,
    mut upper: i32,
    mut fits: impl FnMut(i32) -> crate::Result<bool>,
) -> crate::Result<Option<i32>> {
    let mut best = None;
    while lower <= upper {
        let target = lower + (upper - lower) / 2;
        if fits(target)? {
            best = Some(target);
            let Some(next) = target.checked_add(1) else {
                break;
            };
            lower = next;
        } else {
            upper = target - 1;
        }
    }
    Ok(best)
}

pub(super) fn has_no_resize_reduction(
    width: u32,
    height: u32,
    source_dpi: f64,
    config: &ImageDpiConfig,
    current_denominator: u8,
) -> bool {
    [2_u8, 4, 8].into_iter().any(|denominator| {
        denominator <= 8 / current_denominator
            && !crate::image::preprocessing::normalization_requires_resize(
                width.div_ceil(u32::from(denominator)),
                height.div_ceil(u32::from(denominator)),
                config,
                source_dpi / f64::from(denominator),
            )
    })
}

fn resize_peak(width: u32, height: u32, target_width: u32, target_height: u32, encoded: usize) -> Option<u64> {
    let source = rgb_bytes(width, height)?;
    let destination = rgb_bytes(target_width, target_height)?;
    let horizontal = axis_workspace(width, target_width)?;
    let vertical = axis_workspace(height, target_height)?;
    // resizer.rs::do_convolution does the u8 vertical pass first. The RGB
    // intermediate has at most source_width*target_height pixels plus3 alignment bytes.
    // There is no intermediate when only one axis changes. ~keep
    let intermediate = if width != target_width && height != target_height {
        rgb_bytes(width, target_height)?.checked_add(RGB_CHANNELS)?
    } else {
        0
    };
    // vertical_u8/neon.rs keeps one i32 accumulator per output RGB channel.
    // Its output is the intermediate when both axes change, otherwise the destination. ~keep
    let accumulator = if height != target_height {
        u64::from(width.max(target_width))
            .checked_mul(RGB_CHANNELS)?
            .checked_mul(size_of::<i32>() as u64)?
    } else {
        0
    };
    source
        .checked_add(destination)?
        .checked_add(accumulator)?
        .checked_add(intermediate)?
        .checked_add(horizontal)?
        .checked_add(vertical)?
        .checked_add((encoded as u64).checked_mul(2)?)
}

fn rgb_bytes(width: u32, height: u32) -> Option<u64> {
    u64::from(width)
        .checked_mul(u64::from(height))?
        .checked_mul(RGB_CHANNELS)
}

fn axis_workspace(source: u32, target: u32) -> Option<u64> {
    if source == target {
        return Some(0);
    }
    if source == 0 || target == 0 {
        return None;
    }
    // convolution::precompute_coefficients stores f64 values and two-u32 bounds.
    // Normalizer16::new retains them while constructing i16 values and per-output
    // Vec chunks. Summing both axes also bounds the one-at-a-time normalization. ~keep
    let filter_scale = (f64::from(source) / f64::from(target)).max(1.0);
    let radius = (MAX_FILTER_SUPPORT * filter_scale).ceil() as u64;
    let window = radius.checked_mul(2)?.checked_add(1)?;
    let outputs = u64::from(target);
    let values = window.checked_mul(outputs)?;
    let value_bytes = (size_of::<f64>() + size_of::<i16>()) as u64;
    values
        .checked_mul(value_bytes)?
        .checked_add(outputs.checked_mul(BOUND_BYTES.checked_add(NORMALIZER_CHUNK_BYTES)?)?)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn resize_bound_includes_live_rgb_and_both_coefficient_representations() {
        let chunk_bytes = NORMALIZER_CHUNK_BYTES;
        let coefficients = 2 * (7 * 20 * 10 + 20 * (8 + chunk_bytes));
        assert_eq!(
            resize_peak(10, 10, 20, 20, 7),
            Some(300 + 1200 + 603 + 20 * 3 * 4 + coefficients + 14)
        );
        assert_eq!(axis_workspace(10, 5), Some(13 * 5 * 10 + 5 * (8 + chunk_bytes)));
    }

    #[test]
    fn one_axis_resize_has_no_intermediate_and_invalid_sizes_refuse() {
        let expected = 300 + 600 + axis_workspace(10, 20).unwrap();
        assert_eq!(resize_peak(10, 10, 20, 10, 0), Some(expected));
        assert!(resize_peak(u32::MAX, u32::MAX, u32::MAX - 1, u32::MAX - 1, 0).is_none());
        assert!(axis_workspace(1, 0).is_none());
    }

    #[test]
    fn dpi_search_stops_before_dimension_clamping_can_reverse_rounding() {
        let config = ImageDpiConfig {
            target_dpi: 177,
            auto_adjust_dpi: false,
            ..Default::default()
        };
        let mut next = config.clone();
        next.target_dpi = 178;
        assert_eq!(
            crate::image::preprocessing::normalized_image_dimensions(873, 1725, &config, Some(72.0)),
            (2073, 4096)
        );
        assert_eq!(
            crate::image::preprocessing::normalized_image_dimensions(873, 1725, &next, Some(72.0)),
            (2072, 4096)
        );
        assert_eq!(largest_unclamped_upscale(72, 178, 1725, 72.0, 4096).unwrap(), Some(170));
    }

    #[test]
    fn known_resize_limit_is_inclusive_and_skip_has_no_resize_workspace() {
        let config = ImageDpiConfig {
            target_dpi: 300,
            auto_adjust_dpi: false,
            ..Default::default()
        };
        let peak = resize_peak(10, 10, 20, 20, 7).unwrap() as usize;
        assert!(check_known_dpi_resize(10, 10, 150.0, &config, 7, peak).is_ok());
        assert!(check_known_dpi_resize(10, 10, 150.0, &config, 7, peak - 1).is_err());
        assert!(check_known_dpi_resize(10, 10, 300.0, &config, 7, 0).is_ok());
        let rounded = ImageDpiConfig {
            target_dpi: 330,
            ..config
        };
        assert!(check_known_dpi_resize(1, 1, 300.0, &rounded, 0, 5).is_err());
    }
}
