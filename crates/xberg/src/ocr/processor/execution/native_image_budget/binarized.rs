use super::pix_bytes;

const FLOAT_BYTES: u64 = size_of::<f32>() as u64;
const POINTER_BYTES: u64 = size_of::<usize>() as u64;
const NUMA_HEADER_BYTES: u64 = (5 * 4 + POINTER_BYTES).next_multiple_of(POINTER_BYTES);
const TILING_HEADER_BYTES: u64 = (7 * 4 + POINTER_BYTES).next_multiple_of(POINTER_BYTES);
const BOX_BYTES: u64 = 5 * 4;
const GRAY_LEVELS: u64 = 256;

pub(super) fn plain_binarization_peak(
    width: u32,
    height: u32,
    options: &crate::types::ImagePreprocessingConfig,
) -> Option<u64> {
    let gray = pix_bytes(width, height, 8)?;
    let (map_width, map_height) = threshold_map_dimensions(width, height, options);
    let map = pix_bytes(map_width, map_height, 8)?;
    // binarize.c retains current gray, tile, sampling and conversion copies;
    // inverted polarity also retains original gray. Full-size bounds cover tiles
    // and source32+gray conversion for both global and adaptive Otsu. ~keep
    let histogram = (GRAY_LEVELS * FLOAT_BYTES + NUMA_HEADER_BYTES) * 4;
    let threshold = gray
        .checked_mul(5)?
        .checked_add(map)?
        .checked_add(histogram + TILING_HEADER_BYTES + 2 * BOX_BYTES)?;
    // pixBlockconvGray retains the old and new threshold maps alongside a 32-bpp
    // map-sized accumulator. The tiling source and inverted gray remain live.
    // This also bounds the no-op copy used for narrow maps. ~keep
    let smoothing = gray
        .checked_mul(2)?
        .checked_add(map.checked_mul(2)?)?
        .checked_add(pix_bytes(map_width, map_height, 32)?)?
        .checked_add(TILING_HEADER_BYTES)?;
    let threshold = threshold.max(smoothing);
    if !options.deskew {
        return Some(threshold);
    }
    Some(threshold.max(deskew_peak(width, height, gray)?))
}

fn threshold_map_dimensions(width: u32, height: u32, options: &crate::types::ImagePreprocessingConfig) -> (u32, u32) {
    let tile = crate::ocr::preprocessing::ADAPTIVE_TILE_SIZE as u32;
    if options.binarization_method.eq_ignore_ascii_case("adaptive") && width.min(height) >= tile {
        ((width / tile).max(1), (height / tile).max(1))
    } else {
        (1, 1)
    }
}

fn deskew_peak(width: u32, height: u32, gray: u64) -> Option<u64> {
    // skew.c keeps the binary input, two reduced rasters and their two templates;
    // bounding each at full size also covers the three-Pix shear rotation peak.
    // An inverted source gray remains live until preprocess_pix returns. ~keep
    let rasters = pix_bytes(width, height, 1)?.checked_mul(5)?.checked_add(gray)?;
    // The default sweep and search each use at most15 samples (three initial
    // points plus two for each of six halvings from0.5 to0.01 degrees).
    // Bound row NUMA growth with old+new storage, rotation row pointers, the
    //256-byte rank-reduction table and256-entry pixel-sum table. ~keep
    let rows = u64::from(height).checked_mul(3 * FLOAT_BYTES + POINTER_BYTES)?;
    let score_arrays = 2 * 15 * FLOAT_BYTES;
    let tables = GRAY_LEVELS * (1 + size_of::<i32>() as u64);
    let scores = rows.checked_add(score_arrays + 3 * NUMA_HEADER_BYTES + tables + 5 * FLOAT_BYTES)?;
    rasters.checked_add(scores)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn adaptive_map_matches_native_floor_tiles_and_small_image_fallback() {
        let options = crate::types::ImagePreprocessingConfig {
            binarization_method: "adaptive".into(),
            ..Default::default()
        };
        assert_eq!(threshold_map_dimensions(65, 97, &options), (2, 3));
        assert_eq!(threshold_map_dimensions(31, 97, &options), (1, 1));
        assert_eq!(threshold_map_dimensions(32, 32, &options), (1, 1));
        assert!(plain_binarization_peak(u32::MAX, u32::MAX, &options).is_none());
    }
}
