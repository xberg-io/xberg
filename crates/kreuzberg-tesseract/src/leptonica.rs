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

// ---------------------------------------------------------------------------
// Raw Leptonica FFI declarations
// ---------------------------------------------------------------------------

#[cfg(any(feature = "build-tesseract", feature = "build-tesseract-wasm"))]
unsafe extern "C-unwind" {
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

// ---------------------------------------------------------------------------
// Safe Pix wrapper
// ---------------------------------------------------------------------------

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

// SAFETY: A Pix owns a uniquely heap-allocated Leptonica PIX. There is no
// interior mutability shared across thread boundaries, so transferring
// ownership to another thread is safe.
#[cfg(any(feature = "build-tesseract", feature = "build-tesseract-wasm"))]
unsafe impl Send for Pix {}

#[cfg(any(feature = "build-tesseract", feature = "build-tesseract-wasm"))]
impl Pix {
    // -----------------------------------------------------------------------
    // Construction
    // -----------------------------------------------------------------------

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
    /// # use kreuzberg_tesseract::Pix;
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

        // SAFETY: pixCreate() allocates a new PIX with the requested dimensions.
        // It is safe because:
        // 1. width, height, and depth (32) are valid positive integers.
        // 2. pixCreate() documents that it returns null only on allocation
        //    failure, which we check immediately below.
        let pix_ptr = unsafe { pixCreate(width as i32, height as i32, 32) };
        if pix_ptr.is_null() {
            return Err(TesseractError::NullPointerError);
        }

        // SAFETY: pixGetData() returns a mutable pointer into the allocated pixel
        // buffer that is valid for the lifetime of the Pix. We own pix_ptr
        // exclusively at this point and have not exposed it to any other code.
        let data_ptr = unsafe { pixGetData(pix_ptr) };
        if data_ptr.is_null() {
            // Clean up before returning the error.
            // SAFETY: pix_ptr is a valid non-null allocation from pixCreate().
            // Passing &mut pix_ptr satisfies the double-pointer convention; after
            // this call pix_ptr is set to null by Leptonica.
            let mut ptr = pix_ptr;
            unsafe { pixDestroy(&mut ptr) };
            return Err(TesseractError::NullPointerError);
        }

        // SAFETY: pixGetWpl() is a pure read of the Pix header that is always
        // valid for a correctly-allocated Pix.
        // For a 32 bpp image, each pixel occupies exactly one 32-bit word, so
        // wpl == width (no padding bytes). The loop below uses `row * wpl + col`
        // to index into the pixel data, which is within bounds because col < width <= wpl.
        let wpl = unsafe { pixGetWpl(pix_ptr) } as usize;

        // Write RGB pixels into the Leptonica data buffer.
        //
        // Leptonica's 32 bpp pixel format stores each pixel as a native
        // 32-bit integer word with the logical layout (MSB→LSB): R G B A,
        // i.e. `(r << 24) | (g << 16) | (b << 8) | alpha`.  This is the
        // same bit pattern regardless of host endianness — Leptonica treats
        // the data as an array of 32-bit integers and accesses individual
        // bytes via bit-shift, not via byte-addressed pointer arithmetic.
        //
        // Therefore we pack directly as `(r << 24) | (g << 16) | (b << 8) | 0xFF`
        // and write the resulting u32 without any byte-swapping.  Calling
        // `pixEndianByteSwap` would invert the channel order, producing
        // A B G R instead of R G B A.
        for row in 0..(height as usize) {
            for col in 0..(width as usize) {
                let src = (row * width as usize + col) * 3;
                let r = data[src] as u32;
                let g = data[src + 1] as u32;
                let b = data[src + 2] as u32;
                // Pack channels as (MSB) R G B A (LSB) in the 32-bit integer.
                let word: u32 = (r << 24) | (g << 16) | (b << 8) | 0xFF;
                // SAFETY: data_ptr is a valid writable pointer into the Leptonica
                // pixel buffer. The offset `row * wpl + col` is within bounds because:
                // 1. wpl >= width (Leptonica pads rows to 32-bit word boundaries).
                // 2. row < height and col < width by loop invariants.
                unsafe {
                    *data_ptr.add(row * wpl + col) = word;
                }
            }
        }

        // Set a sensible default DPI for OCR processing.
        // SAFETY: pix_ptr is valid and non-null. pixSetResolution only writes
        // two integer fields in the Pix header.
        unsafe { pixSetResolution(pix_ptr, 300, 300) };

        Ok(Pix { ptr: pix_ptr })
    }

    // -----------------------------------------------------------------------
    // Image processing operations
    // -----------------------------------------------------------------------

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
    /// # use kreuzberg_tesseract::Pix;
    /// # let rgb = vec![0u8; 100 * 100 * 3];
    /// # let pix = Pix::from_raw_rgb(&rgb, 100, 100).unwrap();
    /// let gray = pix.to_grayscale().unwrap();
    /// let binary = gray.adaptive_threshold(32, 32).unwrap();
    /// let deskewed = binary.deskew().unwrap();
    /// ```
    pub fn deskew(&self) -> Result<Pix> {
        // SAFETY: self.ptr is a valid non-null Pix we own. pixDeskew() does
        // not take ownership; it creates and returns a new Pix allocation.
        // We check for null to handle the case where the operation fails
        // (e.g. input is not 1 bpp).
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
    /// # use kreuzberg_tesseract::Pix;
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
        // SAFETY: self.ptr is valid and non-null. We pass pointers to local
        // stack-allocated f32 values, which are valid write targets for the
        // duration of this call. pixFindSkew() writes into them and returns
        // an integer status code.
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
    /// # use kreuzberg_tesseract::Pix;
    /// # let rgb = vec![128u8; 64 * 64 * 3];
    /// # let pix = Pix::from_raw_rgb(&rgb, 64, 64).unwrap();
    /// let gray = pix.to_grayscale().unwrap();
    /// let binary = gray.adaptive_threshold(32, 32).unwrap();
    /// assert_eq!(binary.depth(), 1);
    /// ```
    pub fn adaptive_threshold(&self, tile_width: i32, tile_height: i32) -> Result<Pix> {
        let mut result: *mut c_void = std::ptr::null_mut();
        // SAFETY: self.ptr is a valid non-null Pix. We pass null for ppixth
        // because we do not need the intermediate threshold image. result is a
        // local pointer that will be written by pixOtsuAdaptiveThreshold(); we
        // check it for null before wrapping in a Pix.
        let status = unsafe {
            pixOtsuAdaptiveThreshold(
                self.ptr,
                tile_width,
                tile_height,
                0,                    // smoothx: no smoothing
                0,                    // smoothy: no smoothing
                0.1,                  // scorefract: Leptonica-recommended default
                std::ptr::null_mut(), // ppixth: we don't need the threshold map
                &mut result,
            )
        };
        if status != 0 {
            return Err(TesseractError::OcrError);
        }
        if result.is_null() {
            return Err(TesseractError::NullPointerError);
        }
        Ok(Pix { ptr: result })
    }

    /// Returns the horizontal and vertical resolution (DPI) of this image.
    ///
    /// # Errors
    ///
    /// Returns `TesseractError::OcrError` if `pixGetResolution` fails.
    pub fn get_resolution(&self) -> Result<(i32, i32)> {
        let mut xres: i32 = 0;
        let mut yres: i32 = 0;
        // SAFETY: self.ptr is a valid non-null Pix. xres and yres are valid
        // stack-allocated i32 values. pixGetResolution reads the Pix header.
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
        // SAFETY: self.ptr is a valid non-null Pix. pixSetResolution only
        // writes two integer fields in the Pix header.
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
            // SAFETY: self.ptr is valid. We set a safe default DPI.
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
    /// # use kreuzberg_tesseract::Pix;
    /// # let rgb = vec![200u8; 100 * 100 * 3];
    /// # let pix = Pix::from_raw_rgb(&rgb, 100, 100).unwrap();
    /// let gray = pix.to_grayscale().unwrap();
    /// let normalised = gray.background_normalize().unwrap();
    /// ```
    pub fn background_normalize(&self) -> Result<Pix> {
        self.ensure_valid_resolution();
        // SAFETY: self.ptr is a valid non-null Pix. We pass null for pixim
        // (no mask image). pixBackgroundNormMorph() returns a newly allocated
        // Pix or null on failure.
        let result = unsafe {
            pixBackgroundNormMorph(
                self.ptr,
                std::ptr::null_mut(), // pixim: no mask
                4,                    // reduction: 4x subsampling
                15,                   // size: morphological SE half-size
                200,                  // bgval: target background value
            )
        };
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
    /// # use kreuzberg_tesseract::Pix;
    /// # let rgb = vec![128u8; 64 * 64 * 3];
    /// # let pix = Pix::from_raw_rgb(&rgb, 64, 64).unwrap();
    /// let sharpened = pix.unsharp_mask(2, 0.4).unwrap();
    /// ```
    pub fn unsharp_mask(&self, halfwidth: i32, fract: f32) -> Result<Pix> {
        self.ensure_valid_resolution();
        // SAFETY: self.ptr is valid and non-null. pixUnsharpMasking() returns
        // a new Pix without modifying or taking ownership of the source.
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
    /// # use kreuzberg_tesseract::Pix;
    /// # let rgb = vec![255u8; 40 * 40 * 3];
    /// # let pix = Pix::from_raw_rgb(&rgb, 40, 40).unwrap();
    /// let upscaled = pix.scale(2.0, 2.0).unwrap();
    /// assert_eq!(upscaled.width(), 80);
    /// assert_eq!(upscaled.height(), 80);
    /// ```
    pub fn scale(&self, sx: f32, sy: f32) -> Result<Pix> {
        // SAFETY: self.ptr is valid and non-null. pixScale() creates a new Pix
        // and does not modify the source.
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
        // SAFETY: boxCreate allocates a new BOX on the heap.
        let box_ = unsafe { boxCreate(x, y, w, h) };
        if box_.is_null() {
            return Err(TesseractError::NullPointerError);
        }
        // SAFETY: pixClipRectangle returns a new Pix clipped to the BOX region.
        // We pass null for pboxc (we don't need the clipped box coordinates back).
        let result = unsafe { pixClipRectangle(self.ptr, box_, std::ptr::null_mut()) };
        // SAFETY: Free the BOX we allocated.
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
        // SAFETY: self.ptr is a valid Pix. count is a valid stack local.
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
    /// # use kreuzberg_tesseract::Pix;
    /// # let rgb = vec![100u8, 150u8, 200u8].repeat(10 * 10);
    /// # let pix = Pix::from_raw_rgb(&rgb, 10, 10).unwrap();
    /// let gray = pix.to_grayscale().unwrap();
    /// assert_eq!(gray.depth(), 8);
    /// ```
    pub fn to_grayscale(&self) -> Result<Pix> {
        self.ensure_valid_resolution();
        // SAFETY: self.ptr is valid and non-null. pixConvertRGBToGray() returns
        // a new 8 bpp Pix; the source is not modified.
        let result = unsafe { pixConvertRGBToGray(self.ptr, 0.0, 0.0, 0.0) };
        if result.is_null() {
            Err(TesseractError::NullPointerError)
        } else {
            Ok(Pix { ptr: result })
        }
    }

    // -----------------------------------------------------------------------
    // Accessors
    // -----------------------------------------------------------------------

    /// Returns the raw Leptonica `PIX *` pointer.
    ///
    /// Intended for passing this image to `TesseractAPI::set_image_2`.
    ///
    /// # Safety
    ///
    /// The caller must ensure the `Pix` outlives any use of the returned
    /// pointer.  `TessBaseAPISetImage2` **borrows** the pointer — it does not
    /// take ownership — so the `Pix` must remain alive until after
    /// `TessBaseAPIRecognize` (or any other Tesseract call that consumes the
    /// image data) has completed.  Dropping the `Pix` while Tesseract holds
    /// the pointer will result in a use-after-free.
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
    /// # use kreuzberg_tesseract::Pix;
    /// # let pix = Pix::from_raw_rgb(&vec![0u8; 8 * 6 * 3], 8, 6).unwrap();
    /// assert_eq!(pix.width(), 8);
    /// ```
    pub fn width(&self) -> i32 {
        // SAFETY: self.ptr is a valid non-null Pix. pixGetWidth() is a pure
        // read of the Pix header struct; it does not mutate any state.
        unsafe { pixGetWidth(self.ptr) }
    }

    /// Returns the height of the image in pixels.
    ///
    /// # Examples
    ///
    /// ```rust,no_run
    /// # use kreuzberg_tesseract::Pix;
    /// # let pix = Pix::from_raw_rgb(&vec![0u8; 8 * 6 * 3], 8, 6).unwrap();
    /// assert_eq!(pix.height(), 6);
    /// ```
    pub fn height(&self) -> i32 {
        // SAFETY: self.ptr is a valid non-null Pix. pixGetHeight() is a pure
        // read of the Pix header struct.
        unsafe { pixGetHeight(self.ptr) }
    }

    /// Returns the bit depth of the image (1, 8, or 32 for this module's usage).
    ///
    /// # Examples
    ///
    /// ```rust,no_run
    /// # use kreuzberg_tesseract::Pix;
    /// # let pix = Pix::from_raw_rgb(&vec![0u8; 4 * 4 * 3], 4, 4).unwrap();
    /// assert_eq!(pix.depth(), 32);
    /// ```
    pub fn depth(&self) -> i32 {
        // SAFETY: self.ptr is a valid non-null Pix. pixGetDepth() is a pure
        // read of the Pix header struct.
        unsafe { pixGetDepth(self.ptr) }
    }
}

// ---------------------------------------------------------------------------
// Drop implementation
// ---------------------------------------------------------------------------

#[cfg(any(feature = "build-tesseract", feature = "build-tesseract-wasm"))]
impl Drop for Pix {
    fn drop(&mut self) {
        if !self.ptr.is_null() {
            // SAFETY: self.ptr is a non-null Leptonica PIX that we allocated and
            // own exclusively. pixDestroy() takes a double pointer, sets *ppix to
            // null after freeing, and is safe to call exactly once per allocation.
            // After this call self.ptr is null (Leptonica sets it), preventing
            // any double-free if drop() were somehow called again.
            unsafe { pixDestroy(&mut self.ptr) };
        }
    }
}

// ---------------------------------------------------------------------------
// Tests
// ---------------------------------------------------------------------------

#[cfg(test)]
#[cfg(any(feature = "build-tesseract", feature = "build-tesseract-wasm"))]
mod tests {
    use super::*;

    fn make_rgb_pix(width: u32, height: u32, fill: u8) -> Pix {
        let data = vec![fill; (width * height * 3) as usize];
        Pix::from_raw_rgb(&data, width, height).expect("from_raw_rgb failed")
    }

    #[test]
    fn test_from_raw_rgb_dimensions() {
        let pix = make_rgb_pix(16, 8, 200);
        assert_eq!(pix.width(), 16);
        assert_eq!(pix.height(), 8);
        assert_eq!(pix.depth(), 32);
    }

    #[test]
    fn test_from_raw_rgb_wrong_length() {
        let data = vec![0u8; 10]; // too short for 4×4
        let err = Pix::from_raw_rgb(&data, 4, 4).unwrap_err();
        assert!(matches!(err, TesseractError::InvalidImageData));
    }

    #[test]
    fn test_from_raw_rgb_zero_dimensions() {
        let err = Pix::from_raw_rgb(&[], 0, 4).unwrap_err();
        assert!(matches!(err, TesseractError::InvalidImageData));

        let err = Pix::from_raw_rgb(&[], 4, 0).unwrap_err();
        assert!(matches!(err, TesseractError::InvalidImageData));
    }

    #[test]
    fn test_as_ptr_is_non_null() {
        let pix = make_rgb_pix(8, 8, 128);
        assert!(!pix.as_ptr().is_null());
    }

    #[test]
    fn test_to_grayscale() {
        let pix = make_rgb_pix(32, 32, 150);
        let gray = pix.to_grayscale().expect("to_grayscale failed");
        assert_eq!(gray.width(), 32);
        assert_eq!(gray.height(), 32);
        assert_eq!(gray.depth(), 8);
    }

    #[test]
    fn test_scale_up() {
        let pix = make_rgb_pix(20, 10, 100);
        let scaled = pix.scale(2.0, 2.0).expect("scale failed");
        assert_eq!(scaled.width(), 40);
        assert_eq!(scaled.height(), 20);
    }

    #[test]
    fn test_unsharp_mask_returns_same_dimensions() {
        let pix = make_rgb_pix(32, 32, 200);
        let sharpened = pix.unsharp_mask(2, 0.4).expect("unsharp_mask failed");
        assert_eq!(sharpened.width(), 32);
        assert_eq!(sharpened.height(), 32);
    }

    #[test]
    fn test_adaptive_threshold_produces_1bpp() {
        let pix = make_rgb_pix(64, 64, 180);
        let gray = pix.to_grayscale().expect("to_grayscale failed");
        let binary = gray.adaptive_threshold(32, 32).expect("adaptive_threshold failed");
        assert_eq!(binary.depth(), 1);
    }
}
