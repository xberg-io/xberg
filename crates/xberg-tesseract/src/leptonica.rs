//! Safe Leptonica Pix wrapper for image preprocessing before OCR.
//!
//! Provides a safe Rust wrapper around the Leptonica image-processing library.
//! `Pix` is the core Leptonica image type. All methods return `Result<Pix>`,
//! and the wrapper takes care of proper memory management via `Drop`.
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
//! ## `pixDeskew` requires a binary (1 bpp) image
//!
//! Call `to_grayscale()` followed by `adaptive_threshold()` before `deskew()`.
//! `pixDeskew` internally calls `pixFindSkewSweepAndSearchScorePivot` which
//! operates on 1-bit images only; passing a colour image will return a null
//! pointer.

use crate::error::{Result, TesseractError};
use std::ffi::c_void;

#[cfg(any(feature = "build-tesseract", feature = "build-tesseract-wasm"))]
ffi_extern! {
    /// Allocates a new Pix with the given dimensions and bit depth.
    fn pixCreate(width: i32, height: i32, depth: i32) -> *mut c_void;

    /// Frees a Pix and sets the caller's pointer to null.
    ///
    /// Leptonica uses a double-pointer convention: `*ppix` is set to null
    /// after the call so that accidental double-frees are a no-op.
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

    /// Deskews a 1 bpp image using a sweep-and-search algorithm.
    ///
    /// `redsearch` is the reduction factor used during the search; pass 0 for
    /// the Leptonica default (2x reduction). Returns a new deskewed Pix on
    /// success, or null on failure. The input Pix is **not** consumed.
    fn pixDeskew(pixs: *mut c_void, redsearch: i32) -> *mut c_void;

    /// Estimates the skew angle and confidence for a 1 bpp image.
    ///
    /// Writes the angle (degrees, positive = counter-clockwise) into `*pangle`
    /// and a confidence score (0–1) into `*pconf`. Returns 0 on success.
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
    /// sharpening strength (0.0–1.0 typical). Returns a new Pix, or null on failure.
    fn pixUnsharpMasking(pixs: *mut c_void, halfwidth: i32, fract: f32) -> *mut c_void;

    /// Scales a Pix by independent x and y factors using the best available method.
    ///
    /// Returns a new scaled Pix, or null on failure. The input Pix is **not** consumed.
    fn pixScale(pixs: *mut c_void, scalex: f32, scaley: f32) -> *mut c_void;

    /// Converts an RGB (32 bpp) Pix to 8 bpp grayscale.
    ///
    /// `rwt`, `gwt`, `bwt` are the red, green, and blue channel weights; pass
    /// 0.0 for all three to use Leptonica's default equal weights. Returns a new
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
/// Owns the underlying allocation and frees it in `Drop`. All methods that
/// return a new image allocate a fresh `Pix`; the receiver is never consumed.
///
/// # Thread safety
///
/// `Pix` is `Send` because Leptonica image objects are independent heap
/// allocations with no shared mutable state. Concurrent mutation from multiple
/// threads is **not** safe (no `Sync`).
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
unsafe impl Send for Pix {}

#[cfg(any(feature = "build-tesseract", feature = "build-tesseract-wasm"))]
impl Pix {
    const ADAPTIVE_OTSU_SCORE_FRACTION: f32 = 0.1;
    const ADAPTIVE_OTSU_SMOOTHING_HALF_WIDTH: i32 = 1;
    const MEDIAN_FILTER_RANK: f32 = 0.5;
    const MINIMUM_OTSU_TILE_DIMENSION: i32 = 16;
    const STANDARD_OTSU_SCORE_FRACTION: f32 = 0.0;

    /// Takes ownership of a Pix that Leptonica or Tesseract allocated for the caller, so that
    /// `Drop` frees it with `pixDestroy`. `ptr` must not be null and must not be freed elsewhere.
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

        let pix_ptr = unsafe { pixCreate(width as i32, height as i32, 32) };
        if pix_ptr.is_null() {
            return Err(TesseractError::NullPointerError);
        }

        let data_ptr = unsafe { pixGetData(pix_ptr) };
        if data_ptr.is_null() {
            let mut ptr = pix_ptr;
            unsafe { pixDestroy(&mut ptr) };
            return Err(TesseractError::NullPointerError);
        }

        let wpl = unsafe { pixGetWpl(pix_ptr) } as usize;

        for row in 0..(height as usize) {
            for col in 0..(width as usize) {
                let src = (row * width as usize + col) * 3;
                let r = data[src] as u32;
                let g = data[src + 1] as u32;
                let b = data[src + 2] as u32;
                let word: u32 = (r << 24) | (g << 16) | (b << 8) | 0xFF;
                unsafe {
                    *data_ptr.add(row * wpl + col) = word;
                }
            }
        }

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

        let pix_ptr = unsafe { pixCreate(width as i32, height as i32, 8) };
        if pix_ptr.is_null() {
            return Err(TesseractError::NullPointerError);
        }

        for row in 0..(height as i32) {
            for col in 0..(width as i32) {
                let value = data[(row as usize) * (width as usize) + col as usize] as u32;
                let status = unsafe { pixSetPixel(pix_ptr, col, row, value) };
                if status != 0 {
                    let mut ptr = pix_ptr;
                    unsafe { pixDestroy(&mut ptr) };
                    return Err(TesseractError::InvalidImageData);
                }
            }
        }

        unsafe { pixSetResolution(pix_ptr, 300, 300) };

        Ok(Pix { ptr: pix_ptr })
    }

    /// Deskews this image, returning a new corrected Pix.
    ///
    /// **Note:** `pixDeskew` requires a 1 bpp (binary) image. Call
    /// `to_grayscale()` followed by `adaptive_threshold()` before invoking
    /// this method on a colour or grayscale Pix.
    ///
    /// # Errors
    ///
    /// Returns `TesseractError::NullPointerError` if Leptonica returns null
    /// (typically because the input is not 1 bpp or the image is too small).
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
        let result = unsafe { pixDeskew(self.ptr, 0) };
        if result.is_null() {
            Err(TesseractError::NullPointerError)
        } else {
            Ok(Pix { ptr: result })
        }
    }

    /// Estimates the skew angle (degrees) and confidence (0–1) for this image.
    ///
    /// A positive angle indicates counter-clockwise skew. Confidence near 1.0
    /// means a clear dominant skew direction was found.
    ///
    /// **Note:** Like `deskew`, this operates on 1 bpp images.
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
        let status = unsafe { pixSetResolution(self.ptr, xres, yres) };
        if status != 0 {
            Err(TesseractError::OcrError)
        } else {
            Ok(())
        }
    }

    /// Ensures the image has a valid (non-zero) DPI resolution.
    ///
    /// If both x and y resolution are zero, sets them to 72 DPI as a
    /// safe fallback. This prevents Leptonica operations that depend on
    /// resolution metadata from producing incorrect results.
    fn ensure_valid_resolution(&self) {
        if let Ok((xres, yres)) = self.get_resolution()
            && (xres == 0 || yres == 0)
        {
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
        let box_ = unsafe { boxCreate(x, y, w, h) };
        if box_.is_null() {
            return Err(TesseractError::NullPointerError);
        }
        let result = unsafe { pixClipRectangle(self.ptr, box_, std::ptr::null_mut()) };
        let mut box_mut = box_;
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
        let result = unsafe { pixInvert(std::ptr::null_mut(), self.ptr) };
        if result.is_null() {
            Err(TesseractError::NullPointerError)
        } else {
            Ok(Pix { ptr: result })
        }
    }

    /// Estimates the mean pixel value and the fraction of "light" pixels of
    /// an 8 bpp grayscale image, by sampling a regular grid in a single pass.
    ///
    /// Used to decide image polarity (light-on-dark vs. dark-on-light) before
    /// OCR. `sample_stride` controls the sampling grid spacing in pixels (e.g.
    /// 4 samples roughly every 4th pixel in each dimension); pass 1 to sample
    /// every pixel. `light_threshold` is the grayscale value (0–255) at or
    /// above which a sampled pixel counts towards the returned light
    /// fraction. Only meaningful on an 8 bpp Pix — behaviour on other bit
    /// depths is Leptonica-defined and not validated here.
    ///
    /// Returns `(mean, light_fraction)`.
    ///
    /// # Errors
    ///
    /// Returns `TesseractError::OcrError` if no pixels could be sampled
    /// (zero-sized image) or if `pixGetPixel` fails.
    pub fn grayscale_stats(&self, light_threshold: u8, sample_stride: i32) -> Result<(f64, f64)> {
        let stride = sample_stride.max(1);
        let width = self.width();
        let height = self.height();
        if width <= 0 || height <= 0 {
            return Err(TesseractError::OcrError);
        }

        let mut sum: u64 = 0;
        let mut light_count: u64 = 0;
        let mut count: u64 = 0;
        let mut y = 0;
        while y < height {
            let mut x = 0;
            while x < width {
                let mut value: u32 = 0;
                let status = unsafe { pixGetPixel(self.ptr, x, y, &mut value) };
                if status != 0 {
                    return Err(TesseractError::OcrError);
                }
                sum += value as u64;
                if value >= light_threshold as u32 {
                    light_count += 1;
                }
                count += 1;
                x += stride;
            }
            y += stride;
        }

        if count == 0 {
            return Err(TesseractError::OcrError);
        }

        Ok((sum as f64 / count as f64, light_count as f64 / count as f64))
    }

    /// Estimates the mean pixel value of an 8 bpp grayscale image by sampling
    /// a regular grid of pixels. See [`Pix::grayscale_stats`] for details.
    ///
    /// # Errors
    ///
    /// Returns `TesseractError::OcrError` if no pixels could be sampled
    /// (zero-sized image) or if `pixGetPixel` fails.
    pub fn mean_gray_value(&self, sample_stride: i32) -> Result<f64> {
        self.grayscale_stats(255, sample_stride).map(|(mean, _)| mean)
    }

    /// Reads every pixel of an 8 bpp grayscale Pix into a byte-per-pixel buffer,
    /// in left-to-right, top-to-bottom order.
    ///
    /// Unlike [`Pix::grayscale_stats`], this reads every pixel (no sampling
    /// stride), because per-region operations such as shaded-row normalization
    /// need the full-resolution raster, not a sampled estimate.
    ///
    /// # Errors
    ///
    /// Returns `TesseractError::OcrError` if the image is zero-sized or if
    /// `pixGetPixel` fails on any pixel.
    pub fn grayscale_bytes(&self) -> Result<Vec<u8>> {
        let width = self.width();
        let height = self.height();
        if width <= 0 || height <= 0 {
            return Err(TesseractError::OcrError);
        }

        let mut out = Vec::with_capacity((width as usize) * (height as usize));
        for y in 0..height {
            for x in 0..width {
                let mut value: u32 = 0;
                let status = unsafe { pixGetPixel(self.ptr, x, y, &mut value) };
                if status != 0 {
                    return Err(TesseractError::OcrError);
                }
                out.push(value as u8);
            }
        }
        Ok(out)
    }

    /// Returns the raw Leptonica `PIX *` pointer.
    ///
    /// Intended for passing this image to `TesseractAPI::set_image_2`.
    ///
    /// # Safety
    ///
    /// The caller must ensure the `Pix` remains alive while the returned
    /// pointer is used. `TessBaseAPISetImage2` synchronously deep-copies the
    /// image, so the `Pix` only needs to outlive that call.
    ///
    /// The caller must **not** free the returned pointer; `Pix::drop` is
    /// solely responsible for deallocation via `pixDestroy`.
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
        unsafe { pixGetDepth(self.ptr) }
    }
}

#[cfg(any(feature = "build-tesseract", feature = "build-tesseract-wasm"))]
impl Drop for Pix {
    fn drop(&mut self) {
        if !self.ptr.is_null() {
            unsafe { pixDestroy(&mut self.ptr) };
        }
    }
}

#[cfg(test)]
#[cfg(any(feature = "build-tesseract", feature = "build-tesseract-wasm"))]
#[path = "leptonica/tests.rs"]
mod tests;
