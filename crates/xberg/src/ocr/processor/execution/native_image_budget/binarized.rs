use super::pix_bytes;

const FLOAT_BYTES: u64 = size_of::<f32>() as u64;
const POINTER_BYTES: u64 = size_of::<usize>() as u64;
const NUMA_HEADER_BYTES: u64 = (5 * 4 + POINTER_BYTES).next_multiple_of(POINTER_BYTES);
const TILING_HEADER_BYTES: u64 = (7 * 4 + POINTER_BYTES).next_multiple_of(POINTER_BYTES);
const BOX_BYTES: u64 = 5 * 4;
const GRAY_LEVELS: u64 = 256;

pub(super) fn plain_otsu_peak(width: u32, height: u32, deskew: bool) -> Option<u64> {
    let gray = pix_bytes(width, height, 8)?;
    // binarize.c global Otsu retains the current gray, full tile, sampling and
    // conversion copies. Inverted polarity additionally retains the original gray.
    // Five word-aligned gray rasters also bound source32+gray conversion. ~keep
    let histogram = (GRAY_LEVELS * FLOAT_BYTES + NUMA_HEADER_BYTES) * 4;
    let otsu = gray
        .checked_mul(5)?
        .checked_add(pix_bytes(1, 1, 8)?)?
        .checked_add(histogram + TILING_HEADER_BYTES + 2 * BOX_BYTES)?;
    if !deskew {
        return Some(otsu);
    }
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
    Some(otsu.max(rasters.checked_add(scores)?))
}
