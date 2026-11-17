//! Safe Leptonica Pix wrapper for image preprocessing before OCR.
//!
//! Provides a safe Rust wrapper around the Leptonica image-processing library.
//! `Pix` is the core Leptonica image type. The wrapper releases its native
//! reference through `Drop`.
//!
//! ## Pixel format
//!
//! Leptonica's 32 bpp format stores each pixel as a native 32-bit integer
//! with the logical layout (MSB→LSB): `R G B A`, i.e.
//! `(r << 24) | (g << 16) | (b << 8) | alpha`.  Leptonica accesses
//! individual channels via bit-shift on the integer value, not via
//! byte-addressed pointer arithmetic, so the packing is identical on both
//! big- and little-endian hosts.  Do **not** call `pixEndianByteSwap` after
//! writing pixels this way — doing so inverts the channel order.
//!
//! ## Deskew and skew estimation
//!
//! Leptonica binarizes as needed during `pixDeskew`. Skew estimation through
//! `pixFindSkew` requires a 1 bpp image.

use crate::error::{Result, TesseractError};
use std::ffi::c_void;

#[cfg(any(feature = "build-tesseract", feature = "build-tesseract-wasm"))]
ffi_extern! {
    /// Allocates a new Pix with the given dimensions and bit depth.
    fn pixCreate(width: i32, height: i32, depth: i32) -> *mut c_void;

    /// Releases one reference to a Pix and clears the caller's pointer slot.
    ///
    /// Leptonica frees the allocation when the final reference is released.
    fn pixDestroy(ppix: *mut *mut c_void);

    /// Sets the horizontal and vertical resolution (DPI) on a Pix.
    ///
    /// Returns 0 on success, non-zero on error.
    fn pixSetResolution(pix: *mut c_void, xres: i32, yres: i32) -> i32;

    /// Returns the width of the Pix in pixels.
    fn pixGetWidth(pix: *const c_void) -> i32;

    /// Returns the height of the Pix in pixels.
    fn pixGetHeight(pix: *const c_void) -> i32;

    /// Returns the bit depth of the Pix (1, 2, 4, 8, 16, or 32).
    fn pixGetDepth(pix: *const c_void) -> i32;

    /// Returns the number of 32-bit words per row (words-per-line).
    fn pixGetWpl(pix: *const c_void) -> i32;

    /// Returns a mutable pointer to the start of the pixel data array.
    ///
    /// The data is stored as rows of 32-bit words; each word covers 32/depth pixels.
    fn pixGetData(pix: *mut c_void) -> *mut u32;

    /// Deskews an image using a sweep-and-search algorithm.
    ///
    /// Pass 0 for the default search reduction. A non-null result owns a
    /// reference, which may point to the input when no rotation is performed.
    fn pixDeskew(pixs: *mut c_void, redsearch: i32) -> *mut c_void;

    /// Estimates the skew angle and confidence for a 1 bpp image.
    ///
    /// Writes the angle (degrees, positive = counter-clockwise) into `*pangle`
    /// and a confidence ratio into `*pconf`. Returns 0 on success.
    fn pixFindSkew(pixs: *mut c_void, pangle: *mut f32, pconf: *mut f32) -> i32;

    /// Applies Otsu adaptive thresholding to produce a binarised Pix.
    ///
    /// `sx`/`sy` are the tile dimensions; `smoothx`/`smoothy` are half-widths
    /// for smoothing the threshold map; `scorefract` controls threshold acceptance
    /// (typical value: 0.1). `ppixth` (optional) receives the threshold image;
    /// `ppixd` receives the binarised output.
    fn pixOtsuAdaptiveThreshold(
        pixs: *mut c_void,
        sx: i32,
        sy: i32,
        smoothx: i32,
        smoothy: i32,
        scorefract: f32,
        ppixth: *mut *mut c_void,
        ppixd: *mut *mut c_void,
    ) -> i32;

    /// Applies a grayscale rank filter. A rank of 0.5 is a median filter.
    fn pixRankFilterGray(pixs: *mut c_void, width: i32, height: i32, rank: f32) -> *mut c_void;

    /// Applies a gamma transfer curve while stretching the selected input range to 0–255.
    fn pixGammaTRC(
        pixd: *mut c_void,
        pixs: *mut c_void,
        gamma: f32,
        min_value: i32,
        max_value: i32,
    ) -> *mut c_void;

    /// Applies tiled Sauvola local thresholding to an 8 bpp grayscale image.
    fn pixSauvolaBinarizeTiled(
        pixs: *mut c_void,
        window_half_size: i32,
        factor: f32,
        tile_columns: i32,
        tile_rows: i32,
        ppixth: *mut *mut c_void,
        ppixd: *mut *mut c_void,
    ) -> i32;

    /// Normalises the background of a grayscale image using morphological operations.
    ///
    /// `reduction` is the subsampling factor (e.g. 4), `size` is the morphological
    /// structuring-element half-size (e.g. 15), and `bgval` is the target background
    /// value (e.g. 200). Returns a new normalised Pix, or null on failure.
    fn pixBackgroundNormMorph(
        pixs: *mut c_void,
        pixim: *mut c_void,
        reduction: i32,
        size: i32,
        bgval: i32,
    ) -> *mut c_void;

    /// Applies unsharp masking to sharpen a grayscale or colour Pix.
    ///
    /// `halfwidth` is the half-size of the blur kernel; `fract` controls the
    /// sharpening strength (0.0–1.0 typical). A non-null result owns a
    /// reference, which Leptonica may take to the input for a no-op.
    fn pixUnsharpMasking(pixs: *mut c_void, halfwidth: i32, fract: f32) -> *mut c_void;

    /// Scales a Pix by independent x and y factors using the best available method.
    ///
    /// Returns a new scaled Pix, or null on failure. The input Pix is **not** consumed.
    fn pixScale(pixs: *mut c_void, scalex: f32, scaley: f32) -> *mut c_void;

    /// Converts an RGB (32 bpp) Pix to 8 bpp grayscale.
    ///
    /// `rwt`, `gwt`, `bwt` are the red, green, and blue channel weights; pass
    /// 0.0 for all three to use Leptonica's default weights. Returns a new
    /// 8 bpp Pix, or null on failure.
    fn pixConvertRGBToGray(pixs: *mut c_void, rwt: f32, gwt: f32, bwt: f32) -> *mut c_void;

    /// Inverts the pixel values of a Pix (bitwise complement).
    ///
    /// Pass null for `pixd` to allocate a new inverted Pix. The input Pix is
    /// **not** consumed. Returns the inverted Pix, or null on failure.
    fn pixInvert(pixd: *mut c_void, pixs: *mut c_void) -> *mut c_void;

    /// Reads a single pixel value from a Pix.
    ///
    /// Writes the pixel's raw value (for 8 bpp images, the grayscale intensity
    /// 0–255) into `*pval`. Returns 0 on success, non-zero on error (e.g. `x`/`y`
    /// out of bounds).
    fn pixGetPixel(pix: *mut c_void, x: i32, y: i32, pval: *mut u32) -> i32;

    /// Writes a single pixel value into a Pix.
    ///
    /// `val` is the raw pixel value (for 8 bpp images, the grayscale intensity
    /// 0-255). Returns 0 on success, non-zero on error (e.g. `x`/`y` out of bounds).
    fn pixSetPixel(pix: *mut c_void, x: i32, y: i32, val: u32) -> i32;

    /// Creates a Leptonica BOX with the given coordinates.
    fn boxCreate(x: i32, y: i32, w: i32, h: i32) -> *mut c_void;

    /// Frees a Leptonica BOX.
    fn boxDestroy(pbox: *mut *mut c_void);

    /// Clips a rectangular region from a Pix.
    ///
    /// Returns a new Pix containing the clipped region, or null on failure.
    /// `pboxc` (optional) receives the actual clipped box; pass null to ignore.
    fn pixClipRectangle(pixs: *mut c_void, box_: *mut c_void, pboxc: *mut *mut c_void) -> *mut c_void;

    /// Counts connected components in a 1 bpp image.
    ///
    /// `connectivity` is 4 or 8. Writes the count to `*pcount`.
    /// Returns 0 on success.
    fn pixCountConnComp(pix: *mut c_void, connectivity: i32, pcount: *mut i32) -> i32;

    /// Retrieves the horizontal and vertical resolution (DPI) from a Pix.
    ///
    /// Writes the x-resolution into `*pxres` and y-resolution into `*pyres`.
    /// Returns 0 on success, non-zero on error.
    fn pixGetResolution(pix: *const c_void, pxres: *mut i32, pyres: *mut i32) -> i32;

}

/// Safe wrapper around a Leptonica `PIX *` image object.
///
/// Owns one reference that `Drop` releases with `pixDestroy`. Some Leptonica
/// transforms can return a second reference to the same allocation.
///
/// # Thread safety
///
/// Leptonica uses a non-atomic reference count and can share one allocation
/// between wrappers. The current wrapper does not enforce thread isolation for
/// these aliases.
#[cfg(any(feature = "build-tesseract", feature = "build-tesseract-wasm"))]
pub struct Pix {
    ptr: *mut c_void,
}

#[cfg(any(feature = "build-tesseract", feature = "build-tesseract-wasm"))]
impl std::fmt::Debug for Pix {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("Pix").field("ptr", &self.ptr).finish()
    }
}

#[cfg(any(feature = "build-tesseract", feature = "build-tesseract-wasm"))]
// The Send invariant requires independent native allocations across threads.
// The wrapper does not currently guarantee this for references from pixClone. ~keep
unsafe impl Send for Pix {}

#[cfg(any(feature = "build-tesseract", feature = "build-tesseract-wasm"))]
impl Pix {
    const ADAPTIVE_OTSU_SCORE_FRACTION: f32 = 0.1;
    const ADAPTIVE_OTSU_SMOOTHING_HALF_WIDTH: i32 = 1;
    const MEDIAN_FILTER_RANK: f32 = 0.5;
    const MINIMUM_OTSU_TILE_DIMENSION: i32 = 16;
    const STANDARD_OTSU_SCORE_FRACTION: f32 = 0.0;

    /// Adopts one caller-owned reference to a live Leptonica Pix, released by
    /// `Drop` with `pixDestroy`. The null check does not establish ownership.
    /// Other owners may release their own references independently.
    pub(crate) fn from_owned_ptr(ptr: *mut c_void) -> Result<Pix> {
        if ptr.is_null() {
            return Err(TesseractError::NullPointerError);
        }
        Ok(Pix { ptr })
    }

    /// Creates a 32 bpp Leptonica Pix from a packed RGB byte slice.
    ///
    /// `data` must contain exactly `width * height * 3` bytes in left-to-right,
    /// top-to-bottom, `R G B` interleaved order.
    ///
    /// The DPI is set to 300 × 300 which is a sensible default for OCR input.
    ///
    /// # Errors
    ///
    /// Returns `TesseractError::InvalidImageData` if `data` length does not
    /// match `width * height * 3`, if either dimension is zero, or if
    /// Leptonica's `pixCreate` returns null.
    ///
    /// # Examples
    ///
    /// ```rust,no_run
    /// # use xberg_tesseract::Pix;
    /// let rgb = vec![255u8; 4 * 4 * 3]; // 4×4 white image
    /// let pix = Pix::from_raw_rgb(&rgb, 4, 4).unwrap();
    /// assert_eq!(pix.width(), 4);
    /// assert_eq!(pix.height(), 4);
    /// assert_eq!(pix.depth(), 32);
    /// ```
    pub fn from_raw_rgb(data: &[u8], width: u32, height: u32) -> Result<Pix> {
        let expected = (width as usize)
            .checked_mul(height as usize)
            .and_then(|n| n.checked_mul(3))
            .ok_or(TesseractError::InvalidImageData)?;

        if data.len() != expected || width == 0 || height == 0 {
            return Err(TesseractError::InvalidImageData);
        }

        // SAFETY: only scalar arguments cross FFI. Leptonica rejects nonpositive signed
        // dimensions, including u32 values that cast to negative, and allocates a 32 bpp Pix. ~keep
        let pix_ptr = unsafe { pixCreate(width as i32, height as i32, 32) };
        if pix_ptr.is_null() {
            return Err(TesseractError::NullPointerError);
        }

        // SAFETY: the checked pixCreate result is live. This pointer borrows its pixel
        // allocation and stays local to initialization. ~keep
        let data_ptr = unsafe { pixGetData(pix_ptr) };
        if data_ptr.is_null() {
            let mut ptr = pix_ptr;
            // SAFETY: this local slot holds the sole new Pix reference and is writable. ~keep
            unsafe { pixDestroy(&mut ptr) };
            return Err(TesseractError::NullPointerError);
        }

        // SAFETY: this live Pix was created at 32 bpp, with one u32 word per pixel.
        // pixGetWpl supplies the stride in words for its Leptonica allocation. ~keep
        let wpl = unsafe { pixGetWpl(pix_ptr) } as usize;

        for row in 0..(height as usize) {
            for col in 0..(width as usize) {
                let src = (row * width as usize + col) * 3;
                let r = data[src] as u32;
                let g = data[src + 1] as u32;
                let b = data[src + 2] as u32;
                let word: u32 = (r << 24) | (g << 16) | (b << 8) | 0xFF;
                // SAFETY: the fresh Pix owns aligned storage of height * wpl words. Row and
                // column are bounded by the accepted dimensions, and width <= wpl at 32 bpp.
                // No aliases have been published, and word is packed in Leptonica RGBA order. ~keep
                unsafe {
                    *data_ptr.add(row * wpl + col) = word;
                }
            }
        }

        // SAFETY: pix_ptr holds a live, unpublished Pix, and the setter updates its DPI fields. ~keep
        unsafe { pixSetResolution(pix_ptr, 300, 300) };

        Ok(Pix { ptr: pix_ptr })
    }

    /// Creates an 8 bpp grayscale Leptonica Pix from a byte-per-pixel buffer.
    ///
    /// `data` must contain exactly `width * height` bytes, one grayscale sample
    /// (0-255) per pixel in left-to-right, top-to-bottom order. Writes go through
    /// `pixSetPixel` one pixel at a time (like [`Pix::grayscale_bytes`]'s reads),
    /// rather than packing Leptonica's 8 bpp word layout directly, to avoid
    /// depending on undocumented byte-packing order. The DPI is set to 300 x 300;
    /// call [`Pix::set_resolution`] afterwards to match the source image.
    ///
    /// # Errors
    ///
    /// Returns `TesseractError::InvalidImageData` if `data.len()` does not equal
    /// `width * height`, if either dimension is zero, if Leptonica's `pixCreate`
    /// returns null, or if any `pixSetPixel` call fails.
    pub fn from_grayscale_bytes(data: &[u8], width: u32, height: u32) -> Result<Pix> {
        let expected = (width as usize)
            .checked_mul(height as usize)
            .ok_or(TesseractError::InvalidImageData)?;

        if data.len() != expected || width == 0 || height == 0 {
            return Err(TesseractError::InvalidImageData);
        }

        // SAFETY: only scalar arguments cross FFI. Leptonica rejects nonpositive signed
        // dimensions, including u32 values that cast to negative, and allocates an 8 bpp Pix. ~keep
        let pix_ptr = unsafe { pixCreate(width as i32, height as i32, 8) };
        if pix_ptr.is_null() {
            return Err(TesseractError::NullPointerError);
        }

        for row in 0..(height as i32) {
            for col in 0..(width as i32) {
                let value = data[(row as usize) * (width as usize) + col as usize] as u32;
                // SAFETY: row and column are in bounds for this new 8 bpp Pix; the u8 sample
                // fits its depth. Leptonica handles packed storage and row stride. ~keep
                let status = unsafe { pixSetPixel(pix_ptr, col, row, value) };
                if status != 0 {
                    let mut ptr = pix_ptr;
                    // SAFETY: the writable slot holds the sole reference to the unfinished Pix. ~keep
                    unsafe { pixDestroy(&mut ptr) };
                    return Err(TesseractError::InvalidImageData);
                }
            }
        }

        // SAFETY: pix_ptr holds a live, unpublished Pix, and the setter updates its DPI fields. ~keep
        unsafe { pixSetResolution(pix_ptr, 300, 300) };

        Ok(Pix { ptr: pix_ptr })
    }

    /// Deskews this image, binarizing internally for skew estimation as needed.
    /// The result may share the input allocation when no rotation is applied.
    ///
    /// # Errors
    ///
    /// Returns `TesseractError::NullPointerError` if Leptonica returns null.
    ///
    /// # Examples
    ///
    /// ```rust,no_run
    /// # use xberg_tesseract::Pix;
    /// # let rgb = vec![0u8; 100 * 100 * 3];
    /// # let pix = Pix::from_raw_rgb(&rgb, 100, 100).unwrap();
    /// let gray = pix.to_grayscale().unwrap();
    /// let binary = gray.adaptive_threshold(32, 32).unwrap();
    /// let deskewed = binary.deskew().unwrap();
    /// ```
    pub fn deskew(&self) -> Result<Pix> {
        // SAFETY: self keeps the input reference alive. A non-null result owns one reference
        // even when Leptonica returns pixClone(self.ptr). ~keep
        let result = unsafe { pixDeskew(self.ptr, 0) };
        if result.is_null() {
            Err(TesseractError::NullPointerError)
        } else {
            Ok(Pix { ptr: result })
        }
    }

    /// Estimates the skew angle in degrees and the confidence ratio.
    ///
    /// A positive angle indicates counter-clockwise skew. This function requires
    /// a 1 bpp image; Leptonica checks the depth.
    ///
    /// # Errors
    ///
    /// Returns `TesseractError::OcrError` if `pixFindSkew` returns a non-zero
    /// status (e.g. insufficient contrast or wrong bit depth).
    ///
    /// # Examples
    ///
    /// ```rust,no_run
    /// # use xberg_tesseract::Pix;
    /// # let rgb = vec![0u8; 100 * 100 * 3];
    /// # let pix = Pix::from_raw_rgb(&rgb, 100, 100).unwrap();
    /// let gray = pix.to_grayscale().unwrap();
    /// let binary = gray.adaptive_threshold(32, 32).unwrap();
    /// let (angle, confidence) = binary.find_skew().unwrap();
    /// println!("Skew: {angle:.2}° (confidence {confidence:.2})");
    /// ```
    pub fn find_skew(&self) -> Result<(f32, f32)> {
        let mut angle: f32 = 0.0;
        let mut conf: f32 = 0.0;
        // SAFETY: self keeps the Pix alive and both outputs are distinct writable f32
        // locals. Leptonica rejects unsupported depth before estimating skew. ~keep
        let status = unsafe { pixFindSkew(self.ptr, &mut angle, &mut conf) };
        if status != 0 {
            Err(TesseractError::OcrError)
        } else {
            Ok((angle, conf))
        }
    }

    /// Binarises this image using Otsu adaptive thresholding.
    ///
    /// `tile_width` and `tile_height` control the size of the local regions
    /// used to compute the threshold. Values around 16–64 work well for typical
    /// document images; smaller tiles follow local contrast more closely.
    ///
    /// # Errors
    ///
    /// Returns `TesseractError::NullPointerError` if Leptonica returns null, or
    /// `TesseractError::OcrError` if `pixOtsuAdaptiveThreshold` returns a
    /// non-zero status.
    ///
    /// # Examples
    ///
    /// ```rust,no_run
    /// # use xberg_tesseract::Pix;
    /// # let rgb = vec![128u8; 64 * 64 * 3];
    /// # let pix = Pix::from_raw_rgb(&rgb, 64, 64).unwrap();
    /// let gray = pix.to_grayscale().unwrap();
    /// let binary = gray.adaptive_threshold(32, 32).unwrap();
    /// assert_eq!(binary.depth(), 1);
    /// ```
    pub fn adaptive_threshold(&self, tile_width: i32, tile_height: i32) -> Result<Pix> {
        if tile_width < Self::MINIMUM_OTSU_TILE_DIMENSION || tile_height < Self::MINIMUM_OTSU_TILE_DIMENSION {
            return Err(TesseractError::InvalidParameterError);
        }
        self.otsu_threshold_with_options(
            tile_width,
            tile_height,
            Self::ADAPTIVE_OTSU_SMOOTHING_HALF_WIDTH,
            Self::ADAPTIVE_OTSU_SMOOTHING_HALF_WIDTH,
            Self::ADAPTIVE_OTSU_SCORE_FRACTION,
        )
    }

    /// Binarises this image using one global standard Otsu threshold.
    ///
    /// # Errors
    ///
    /// Returns `TesseractError::InvalidParameterError` for non-grayscale input,
    /// or an OCR/null-pointer error if Leptonica cannot create the binary image.
    pub fn otsu_threshold(&self) -> Result<Pix> {
        self.otsu_threshold_with_options(
            self.width().max(Self::MINIMUM_OTSU_TILE_DIMENSION),
            self.height().max(Self::MINIMUM_OTSU_TILE_DIMENSION),
            0,
            0,
            Self::STANDARD_OTSU_SCORE_FRACTION,
        )
    }

    fn otsu_threshold_with_options(
        &self,
        tile_width: i32,
        tile_height: i32,
        smooth_x: i32,
        smooth_y: i32,
        score_fraction: f32,
    ) -> Result<Pix> {
        self.require_grayscale()?;
        let resolution = self.get_resolution()?;
        let mut result: *mut c_void = std::ptr::null_mut();
        // SAFETY: `self.ptr` is an owned non-null 8 bpp Pix, all scalar options are validated by
        // the public callers, the threshold output is intentionally null, and `result` lives for
        // the call and is adopted or freed exactly once below.
        let status = unsafe {
            pixOtsuAdaptiveThreshold(
                self.ptr,
                tile_width,
                tile_height,
                smooth_x,
                smooth_y,
                score_fraction,
                std::ptr::null_mut(),
                &mut result,
            )
        };
        if status != 0 {
            Self::discard_transformed_ptr(result);
            return Err(TesseractError::OcrError);
        }
        if result.is_null() {
            return Err(TesseractError::NullPointerError);
        }
        Self::from_transformed_ptr(result, resolution)
    }

    /// Removes impulse noise from an 8 bpp image with a median rank filter.
    ///
    /// # Errors
    ///
    /// Returns `TesseractError::InvalidParameterError` unless the input is
    /// grayscale and both filter dimensions are positive.
    pub fn median_filter(&self, width: i32, height: i32) -> Result<Pix> {
        self.require_grayscale()?;
        if width <= 0 || height <= 0 {
            return Err(TesseractError::InvalidParameterError);
        }
        let resolution = self.get_resolution()?;
        // SAFETY: `self.ptr` is an owned non-null 8 bpp Pix and the positive kernel dimensions
        // were validated above; the returned allocation is adopted exactly once below.
        let result = unsafe { pixRankFilterGray(self.ptr, width, height, Self::MEDIAN_FILTER_RANK) };
        Self::from_transformed_ptr(result, resolution)
    }

    /// Stretches a grayscale intensity range to 0–255 with gamma correction.
    ///
    /// # Errors
    ///
    /// Returns `TesseractError::InvalidParameterError` unless `gamma` is finite
    /// and positive and `0 <= min_value < max_value <= 255`.
    pub fn contrast_stretch(&self, gamma: f32, min_value: i32, max_value: i32) -> Result<Pix> {
        self.require_grayscale()?;
        if !gamma.is_finite() || gamma <= 0.0 || min_value < 0 || max_value > 255 || min_value >= max_value {
            return Err(TesseractError::InvalidParameterError);
        }
        let resolution = self.get_resolution()?;
        // SAFETY: `self.ptr` is an owned non-null 8 bpp Pix, the transfer parameters were
        // validated above, and a null destination requests a new allocation adopted below.
        let result = unsafe { pixGammaTRC(std::ptr::null_mut(), self.ptr, gamma, min_value, max_value) };
        Self::from_transformed_ptr(result, resolution)
    }

    /// Binarises an 8 bpp image using tiled Sauvola local thresholds.
    ///
    /// # Errors
    ///
    /// Returns `TesseractError::InvalidParameterError` for invalid window,
    /// factor, tile-count, or input-depth values. Returns an OCR/null-pointer
    /// error if Leptonica cannot create the binary output.
    pub fn sauvola_threshold(
        &self,
        window_half_size: i32,
        factor: f32,
        tile_columns: i32,
        tile_rows: i32,
    ) -> Result<Pix> {
        self.require_grayscale()?;
        let minimum_dimension = self.width().min(self.height());
        let window_fits = window_half_size
            .checked_mul(2)
            .and_then(|diameter| diameter.checked_add(3))
            .is_some_and(|minimum_required| minimum_dimension >= minimum_required);
        if window_half_size < 2
            || !window_fits
            || !factor.is_finite()
            || factor < 0.0
            || tile_columns < 1
            || tile_rows < 1
        {
            return Err(TesseractError::InvalidParameterError);
        }

        let resolution = self.get_resolution()?;
        let mut result: *mut c_void = std::ptr::null_mut();
        // SAFETY: `self.ptr` is an owned non-null 8 bpp Pix, the window/factor/tile values were
        // validated above, the threshold output is intentionally null, and `result` lives for the
        // call and is adopted or freed exactly once below.
        let status = unsafe {
            pixSauvolaBinarizeTiled(
                self.ptr,
                window_half_size,
                factor,
                tile_columns,
                tile_rows,
                std::ptr::null_mut(),
                &mut result,
            )
        };
        if status != 0 {
            Self::discard_transformed_ptr(result);
            return Err(TesseractError::OcrError);
        }
        Self::from_transformed_ptr(result, resolution)
    }

    fn require_grayscale(&self) -> Result<()> {
        if self.depth() == 8 {
            Ok(())
        } else {
            Err(TesseractError::InvalidParameterError)
        }
    }

    fn from_transformed_ptr(ptr: *mut c_void, resolution: (i32, i32)) -> Result<Pix> {
        if ptr.is_null() {
            return Err(TesseractError::NullPointerError);
        }
        let mut pix = Pix { ptr };
        pix.set_resolution(resolution.0, resolution.1)?;
        Ok(pix)
    }

    fn discard_transformed_ptr(ptr: *mut c_void) {
        drop(Pix { ptr });
    }

    /// Returns the horizontal and vertical resolution (DPI) of this image.
    ///
    /// # Errors
    ///
    /// Returns `TesseractError::OcrError` if `pixGetResolution` fails.
    pub fn get_resolution(&self) -> Result<(i32, i32)> {
        let mut xres: i32 = 0;
        let mut yres: i32 = 0;
        // SAFETY: self holds a live reference and both outputs are writable i32 locals. ~keep
        let status = unsafe { pixGetResolution(self.ptr, &mut xres, &mut yres) };
        if status != 0 {
            Err(TesseractError::OcrError)
        } else {
            Ok((xres, yres))
        }
    }

    /// Sets the horizontal and vertical resolution (DPI) on this image.
    ///
    /// # Errors
    ///
    /// Returns `TesseractError::OcrError` if `pixSetResolution` fails.
    pub fn set_resolution(&mut self, xres: i32, yres: i32) -> Result<()> {
        // SAFETY: self holds a live reference and the setter only updates DPI fields.
        // The mutable borrow excludes other Rust access through this wrapper. ~keep
        let status = unsafe { pixSetResolution(self.ptr, xres, yres) };
        if status != 0 {
            Err(TesseractError::OcrError)
        } else {
            Ok(())
        }
    }

    /// Ensures the image has a valid (non-zero) DPI resolution.
    ///
    /// If either x or y resolution is zero, sets both to 72 DPI as a
    /// safe fallback. This prevents Leptonica operations that depend on
    /// resolution metadata from producing incorrect results.
    fn ensure_valid_resolution(&self) {
        if let Ok((xres, yres)) = self.get_resolution()
            && (xres == 0 || yres == 0)
        {
            // SAFETY: self keeps the Pix alive and the setter updates only DPI fields.
            // The shared receiver does not establish exclusivity across native aliases. ~keep
            unsafe { pixSetResolution(self.ptr, 72, 72) };
        }
    }

    /// Normalises the background of this image using morphological operations.
    ///
    /// Useful as a preprocessing step when the document has uneven illumination
    /// or a non-white background. Returns a new normalised Pix.
    ///
    /// # Errors
    ///
    /// Returns `TesseractError::NullPointerError` if `pixBackgroundNormMorph`
    /// returns null.
    ///
    /// # Examples
    ///
    /// ```rust,no_run
    /// # use xberg_tesseract::Pix;
    /// # let rgb = vec![200u8; 100 * 100 * 3];
    /// # let pix = Pix::from_raw_rgb(&rgb, 100, 100).unwrap();
    /// let gray = pix.to_grayscale().unwrap();
    /// let normalised = gray.background_normalize().unwrap();
    /// ```
    pub fn background_normalize(&self) -> Result<Pix> {
        self.ensure_valid_resolution();
        // SAFETY: self keeps the input alive; null selects no optional image mask.
        // Leptonica checks 8/32 bpp and returns a caller-owned output on success. ~keep
        let result = unsafe { pixBackgroundNormMorph(self.ptr, std::ptr::null_mut(), 4, 15, 200) };
        if result.is_null() {
            Err(TesseractError::NullPointerError)
        } else {
            Ok(Pix { ptr: result })
        }
    }

    /// Applies unsharp masking to sharpen this image.
    ///
    /// `halfwidth` is the half-size of the blur kernel (e.g. 1–5).
    /// `fract` is the sharpening fraction in the range 0.0–1.0; values
    /// around 0.3–0.5 produce visible sharpening without artefacts.
    ///
    /// # Errors
    ///
    /// Returns `TesseractError::NullPointerError` if `pixUnsharpMasking`
    /// returns null.
    ///
    /// # Examples
    ///
    /// ```rust,no_run
    /// # use xberg_tesseract::Pix;
    /// # let rgb = vec![128u8; 64 * 64 * 3];
    /// # let pix = Pix::from_raw_rgb(&rgb, 64, 64).unwrap();
    /// let sharpened = pix.unsharp_mask(2, 0.4).unwrap();
    /// ```
    pub fn unsharp_mask(&self, halfwidth: i32, fract: f32) -> Result<Pix> {
        self.ensure_valid_resolution();
        // SAFETY: self keeps the input reference alive. Leptonica can return a retained
        // reference to the input for a no-op; the returned wrapper releases that reference. ~keep
        let result = unsafe { pixUnsharpMasking(self.ptr, halfwidth, fract) };
        if result.is_null() {
            Err(TesseractError::NullPointerError)
        } else {
            Ok(Pix { ptr: result })
        }
    }

    /// Scales this image by independent x and y factors.
    ///
    /// Leptonica automatically chooses the best scaling algorithm based on
    /// the scale factors and bit depth (area mapping for downscaling,
    /// linear interpolation for upscaling).
    ///
    /// # Errors
    ///
    /// Returns `TesseractError::NullPointerError` if `pixScale` returns null.
    ///
    /// # Examples
    ///
    /// ```rust,no_run
    /// # use xberg_tesseract::Pix;
    /// # let rgb = vec![255u8; 40 * 40 * 3];
    /// # let pix = Pix::from_raw_rgb(&rgb, 40, 40).unwrap();
    /// let upscaled = pix.scale(2.0, 2.0).unwrap();
    /// assert_eq!(upscaled.width(), 80);
    /// assert_eq!(upscaled.height(), 80);
    /// ```
    pub fn scale(&self, sx: f32, sy: f32) -> Result<Pix> {
        // SAFETY: self keeps the source live and a non-null output is caller-owned.
        // The wrapper does not validate the scale factors or representability of the scaled dimensions. ~keep
        let result = unsafe { pixScale(self.ptr, sx, sy) };
        if result.is_null() {
            Err(TesseractError::NullPointerError)
        } else {
            Ok(Pix { ptr: result })
        }
    }

    /// Clips a rectangular sub-region from this image.
    ///
    /// Returns a new Pix containing only the pixels within the given rectangle.
    /// Coordinates are in pixel space: (x, y) is the top-left corner.
    ///
    /// # Errors
    ///
    /// Returns `TesseractError::NullPointerError` if the crop fails.
    pub fn clip_rectangle(&self, x: i32, y: i32, w: i32, h: i32) -> Result<Pix> {
        // SAFETY: boxCreate receives only integer coordinates and returns an owned Box
        // reference or null. ~keep
        let box_ = unsafe { boxCreate(x, y, w, h) };
        if box_.is_null() {
            return Err(TesseractError::NullPointerError);
        }
        // SAFETY: both input references stay alive, and null omits the optional Box output.
        // The wrapper does not validate that coordinate-plus-extent sums fit in C's signed type. ~keep
        let result = unsafe { pixClipRectangle(self.ptr, box_, std::ptr::null_mut()) };
        let mut box_mut = box_;
        // SAFETY: box_mut is a writable slot holding the owned Box reference. pixClipRectangle
        // borrows the input Box and leaves its release to this call. ~keep
        unsafe { boxDestroy(&mut box_mut) };
        if result.is_null() {
            Err(TesseractError::NullPointerError)
        } else {
            Ok(Pix { ptr: result })
        }
    }

    /// Counts connected components in a 1 bpp (binary) image.
    ///
    /// `connectivity` should be 4 or 8.
    ///
    /// # Errors
    ///
    /// Returns `TesseractError::OcrError` if `pixCountConnComp` fails
    /// (e.g., wrong bit depth — image must be 1 bpp).
    pub fn count_connected_components(&self, connectivity: i32) -> Result<i32> {
        let mut count: i32 = 0;
        // SAFETY: self owns a live Pix reference and count is a writable i32 local.
        // Leptonica checks source depth and connectivity and reports failure via status. ~keep
        let status = unsafe { pixCountConnComp(self.ptr, connectivity, &mut count) };
        if status != 0 {
            Err(TesseractError::OcrError)
        } else {
            Ok(count)
        }
    }

    /// Converts this 32 bpp RGB image to an 8 bpp grayscale Pix.
    ///
    /// Passing 0.0 for all weight parameters instructs Leptonica to use its
    /// default perceptual weights (approx. 0.299 R, 0.587 G, 0.114 B).
    ///
    /// # Errors
    ///
    /// Returns `TesseractError::NullPointerError` if `pixConvertRGBToGray`
    /// returns null (e.g. the source is not 32 bpp).
    ///
    /// # Examples
    ///
    /// ```rust,no_run
    /// # use xberg_tesseract::Pix;
    /// # let rgb = vec![100u8, 150u8, 200u8].repeat(10 * 10);
    /// # let pix = Pix::from_raw_rgb(&rgb, 10, 10).unwrap();
    /// let gray = pix.to_grayscale().unwrap();
    /// assert_eq!(gray.depth(), 8);
    /// ```
    pub fn to_grayscale(&self) -> Result<Pix> {
        self.ensure_valid_resolution();
        // SAFETY: self keeps the input alive. Zero weights select Leptonica's defaults;
        // Leptonica checks the depth and gives the caller a new output on success. ~keep
        let result = unsafe { pixConvertRGBToGray(self.ptr, 0.0, 0.0, 0.0) };
        if result.is_null() {
            Err(TesseractError::NullPointerError)
        } else {
            Ok(Pix { ptr: result })
        }
    }

    /// Inverts the pixel values of this image (bitwise complement).
    ///
    /// For an 8 bpp grayscale image this swaps light and dark: a pixel value
    /// of `v` becomes `255 - v`. Useful for correcting light-text-on-dark-
    /// background scans before Tesseract's binarizer runs, since Tesseract
    /// assumes dark text on a light background. Returns a new Pix; the
    /// receiver is unchanged.
    ///
    /// # Errors
    ///
    /// Returns `TesseractError::NullPointerError` if `pixInvert` returns null.
    ///
    /// # Examples
    ///
    /// ```rust,no_run
    /// # use xberg_tesseract::Pix;
    /// # let rgb = vec![30u8; 16 * 16 * 3];
    /// # let pix = Pix::from_raw_rgb(&rgb, 16, 16).unwrap();
    /// let gray = pix.to_grayscale().unwrap();
    /// let inverted = gray.invert().unwrap();
    /// let back = inverted.invert().unwrap();
    /// ```
    pub fn invert(&self) -> Result<Pix> {
        // SAFETY: self keeps the source alive. Passing a null destination requests a new
        // owned output rather than modifying the input in place. ~keep
        let result = unsafe { pixInvert(std::ptr::null_mut(), self.ptr) };
        if result.is_null() {
            Err(TesseractError::NullPointerError)
        } else {
            Ok(Pix { ptr: result })
        }
    }

    /// Returns the raw Leptonica `PIX *` pointer.
    ///
    /// Intended for passing this image to `TesseractAPI::set_image_2`.
    ///
    /// # Safety
    ///
    /// The pointer borrows the Pix reference; the caller must keep the wrapper
    /// alive for the call and exclude concurrent access if the callee mutates
    /// the image. `TessBaseAPISetImage2` takes an internal copy synchronously,
    /// and it can modify the input while handling PNG alpha.
    ///
    /// This pointer carries no owned reference. `Pix::drop` releases the
    /// reference owned by this wrapper.
    pub fn as_ptr(&self) -> *mut c_void {
        self.ptr
    }

    /// Returns the width of the image in pixels.
    ///
    /// # Examples
    ///
    /// ```rust,no_run
    /// # use xberg_tesseract::Pix;
    /// # let pix = Pix::from_raw_rgb(&vec![0u8; 8 * 6 * 3], 8, 6).unwrap();
    /// assert_eq!(pix.width(), 8);
    /// ```
    pub fn width(&self) -> i32 {
        // SAFETY: self keeps the Pix alive while Leptonica reads its metadata. ~keep
        unsafe { pixGetWidth(self.ptr) }
    }

    /// Returns the height of the image in pixels.
    ///
    /// # Examples
    ///
    /// ```rust,no_run
    /// # use xberg_tesseract::Pix;
    /// # let pix = Pix::from_raw_rgb(&vec![0u8; 8 * 6 * 3], 8, 6).unwrap();
    /// assert_eq!(pix.height(), 6);
    /// ```
    pub fn height(&self) -> i32 {
        // SAFETY: self keeps the Pix alive while Leptonica reads its metadata. ~keep
        unsafe { pixGetHeight(self.ptr) }
    }

    /// Returns the bit depth of the image (1, 8, or 32 for this module's usage).
    ///
    /// # Examples
    ///
    /// ```rust,no_run
    /// # use xberg_tesseract::Pix;
    /// # let pix = Pix::from_raw_rgb(&vec![0u8; 4 * 4 * 3], 4, 4).unwrap();
    /// assert_eq!(pix.depth(), 32);
    /// ```
    pub fn depth(&self) -> i32 {
        // SAFETY: self keeps the Pix alive while Leptonica reads its metadata. ~keep
        unsafe { pixGetDepth(self.ptr) }
    }
}

#[cfg(any(feature = "build-tesseract", feature = "build-tesseract-wasm"))]
impl Drop for Pix {
    fn drop(&mut self) {
        if !self.ptr.is_null() {
            // SAFETY: this writable field holds one owned Pix reference. pixDestroy releases
            // that reference and clears the field; other retained references are counted by C. ~keep
            unsafe { pixDestroy(&mut self.ptr) };
        }
    }
}

#[cfg(any(feature = "build-tesseract", feature = "build-tesseract-wasm"))]
#[path = "leptonica/grayscale.rs"]
mod grayscale;

#[cfg(test)]
#[cfg(any(feature = "build-tesseract", feature = "build-tesseract-wasm"))]
#[path = "leptonica/tests.rs"]
mod tests;
