//! Global Otsu binarization from one pass over the gray raster.

use super::{Pix, pixGetData, pixGetWpl};
use crate::error::Result;
use std::ffi::c_void;

ffi_extern! {
    /// Builds a NUMA from `size` floats; `copyflag` 1 (`L_COPY`) copies them.
    fn numaCreateFromFArray(farray: *mut f32, size: i32, copyflag: i32) -> *mut c_void;

    /// Releases a NUMA and clears the caller's pointer.
    fn numaDestroy(pna: *mut *mut c_void);

    /// Leptonica's Otsu split of a histogram: the index of the best two-class split.
    ///
    /// Returns 0 on success, non-zero on error (the split index is then 0).
    fn numaSplitDistribution(
        na: *mut c_void,
        scorefract: f32,
        psplitindex: *mut i32,
        pave1: *mut f32,
        pave2: *mut f32,
        pnum1: *mut f32,
        pnum2: *mut f32,
        pnascore: *mut *mut c_void,
    ) -> i32;

    /// A 1 bpp Pix with each 8 bpp pixel below `thresh` set.
    fn pixThresholdToBinary(pixs: *mut c_void, thresh: i32) -> *mut c_void;
}

/// Leptonica's `L_COPY` copy flag (`pix.h`).
const L_COPY: i32 = 1;

/// The largest count a Leptonica histogram bin reaches: it adds `1.0` per pixel in `f32`, and
/// past 2^24 that addition rounds back to the same value.
const F32_EXACT_COUNT_LIMIT: u64 = 1 << 24;

impl Pix {
    /// The global Otsu binarization `pixOtsuAdaptiveThreshold` makes with one tile covering
    /// the image and no smoothing, bit for bit, without its three full-image copies (the tile,
    /// its sampled 8 bpp version and the tile again for thresholding) or its per-pixel
    /// histogram accessor.
    pub(super) fn global_otsu_threshold(&self, score_fraction: f32) -> Result<Pix> {
        self.require_grayscale()?;
        let resolution = self.get_resolution()?;
        let mut histogram = self.leptonica_gray_histogram();
        let threshold = otsu_split_index(&mut histogram, score_fraction);
        // SAFETY: `self.ptr` is an owned non-null 8 bpp Pix without a colormap and the split
        // index of a 256-bin histogram lies in 0..=255; the result is adopted exactly once.
        let result = unsafe { pixThresholdToBinary(self.ptr, threshold) };
        self.finish_transform_with_resolution(result, resolution)
    }

    /// `pixGetGrayHistogram(self, 1)` of this 8 bpp Pix, as the `f32` bins Leptonica accumulates.
    fn leptonica_gray_histogram(&self) -> [f32; 256] {
        let width = usize::try_from(self.width()).unwrap_or(0);
        let height = usize::try_from(self.height()).unwrap_or(0);
        // SAFETY: self keeps the Pix live; the calls only read its layout. ~keep
        let (data, words_per_line) = unsafe { (pixGetData(self.ptr), pixGetWpl(self.ptr)) };
        let row_bytes = usize::try_from(words_per_line).unwrap_or(0) * 4;
        let mut lanes = [[0u64; 256]; 4];
        if !data.is_null() && row_bytes >= width {
            for row in 0..height {
                // SAFETY: Leptonica allocates `wpl` words for each of the `height` rows, so the
                // row's bytes are initialized and stay live, unaliased by writes, while self is
                // borrowed. ~keep
                let line = unsafe { std::slice::from_raw_parts(data.cast::<u8>().add(row * row_bytes), row_bytes) };
                count_row(line, width, &mut lanes);
            }
        }
        let mut histogram = [0f32; 256];
        for (value, bin) in histogram.iter_mut().enumerate() {
            let count: u64 = lanes.iter().map(|lane| lane[value]).sum();
            *bin = count.min(F32_EXACT_COUNT_LIMIT) as f32;
        }
        histogram
    }
}

/// Counts the `width` pixels of one 8 bpp row. Leptonica packs pixels big-endian within 32-bit
/// words, so on a little-endian host pixel `j` is the byte at `j ^ 3`; a whole word holds four
/// pixels in any order, and only the last, partial word needs the mapping.
fn count_row(line: &[u8], width: usize, lanes: &mut [[u64; 256]; 4]) {
    let whole = width & !3;
    for word in line[..whole].as_chunks::<4>().0 {
        lanes[0][usize::from(word[0])] += 1;
        lanes[1][usize::from(word[1])] += 1;
        lanes[2][usize::from(word[2])] += 1;
        lanes[3][usize::from(word[3])] += 1;
    }
    for pixel in 0..width - whole {
        let offset = if cfg!(target_endian = "little") {
            pixel ^ 3
        } else {
            pixel
        };
        lanes[0][usize::from(line[whole + offset])] += 1;
    }
}

/// `numaSplitDistribution`'s split index for a 256-bin histogram; 0 when Leptonica refuses the
/// histogram (an empty image), as `pixSplitDistributionFgBg` leaves it.
fn otsu_split_index(histogram: &mut [f32; 256], score_fraction: f32) -> i32 {
    // SAFETY: the array outlives the call and `L_COPY` makes Leptonica copy its 256 floats.
    let mut numa = unsafe { numaCreateFromFArray(histogram.as_mut_ptr(), 256, L_COPY) };
    if numa.is_null() {
        return 0;
    }
    let mut split_index = 0;
    // SAFETY: `numa` is a live NUMA, the index output is a writable local and every other output
    // is null, which Leptonica skips. ~keep
    unsafe {
        numaSplitDistribution(
            numa,
            score_fraction,
            &mut split_index,
            std::ptr::null_mut(),
            std::ptr::null_mut(),
            std::ptr::null_mut(),
            std::ptr::null_mut(),
            std::ptr::null_mut(),
        );
        numaDestroy(&mut numa);
    }
    split_index
}
