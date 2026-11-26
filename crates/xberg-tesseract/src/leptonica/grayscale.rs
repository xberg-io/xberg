//! Build grayscale Leptonica images and read their pixel values.

use super::{Pix, pixCreate, pixGetPixel, pixSetResolution};
use crate::error::{Result, TesseractError};
use std::ffi::c_void;

ffi_extern! {
    /// Combines the `dw` x `dh` rectangle of `pixs` at (`sx`, `sy`) into `pixd` at (`dx`, `dy`)
    /// under the raster operation `op`. Both images must have the same depth.
    ///
    /// Returns 0 on success, non-zero on error.
    fn pixRasterop(
        pixd: *mut c_void,
        dx: i32,
        dy: i32,
        dw: i32,
        dh: i32,
        op: i32,
        pixs: *mut c_void,
        sx: i32,
        sy: i32,
    ) -> i32;
}

impl Pix {
    /// Rows converted per 32 bpp band by [`Pix::gray_from_raw_rgb`].
    pub(super) const GRAY_CONVERSION_STRIP_ROWS: usize = 64;
    /// Leptonica's `PIX_SRC` raster operation (`0xc << 1` in `pix.h`): copy the source.
    const PIX_SRC: i32 = 0xc << 1;

    /// Creates the 8 bpp grayscale Pix that `Pix::from_raw_rgb(data, width, height)?.to_grayscale()`
    /// returns, without ever holding the full-page 32 bpp Pix.
    ///
    /// Each band of [`Self::GRAY_CONVERSION_STRIP_ROWS`] rows goes through that same pair of calls,
    /// so every gray value comes from Leptonica's own `pixConvertRGBToGray`, and `pixRasterop`
    /// copies the band into place. The DPI is 300 x 300, as on that conversion's output.
    ///
    /// # Errors
    ///
    /// Returns `TesseractError::InvalidImageData` for the inputs [`Pix::from_raw_rgb`] rejects or
    /// when a band cannot be copied, and `TesseractError::NullPointerError` when Leptonica fails
    /// to allocate or convert.
    pub fn gray_from_raw_rgb(data: &[u8], width: u32, height: u32) -> Result<Pix> {
        let row_bytes = (width as usize)
            .checked_mul(3)
            .ok_or(TesseractError::InvalidImageData)?;
        let expected = row_bytes
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
        let gray = Pix { ptr: pix_ptr };

        let band_bytes = row_bytes.saturating_mul(Self::GRAY_CONVERSION_STRIP_ROWS);
        for (band, rgb) in data.chunks(band_bytes).enumerate() {
            let rows = rgb.len() / row_bytes;
            let band_gray = Pix::from_raw_rgb(rgb, width, rows as u32)?.to_grayscale()?;
            let top = band * Self::GRAY_CONVERSION_STRIP_ROWS;
            // SAFETY: both Pix values are live 8 bpp images owned here. The band is `width` x
            // `rows`, and rows `top..top + rows` lie inside the destination's `height`. ~keep
            let status = unsafe {
                pixRasterop(
                    gray.ptr,
                    0,
                    top as i32,
                    width as i32,
                    rows as i32,
                    Self::PIX_SRC,
                    band_gray.ptr,
                    0,
                    0,
                )
            };
            if status != 0 {
                return Err(TesseractError::InvalidImageData);
            }
        }

        // SAFETY: gray holds a live Pix no other wrapper shares; the setter updates DPI fields. ~keep
        unsafe { pixSetResolution(gray.ptr, 300, 300) };
        Ok(gray)
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
                // SAFETY: self keeps the Pix live and value is a writable u32 local.
                // Leptonica reads the depth-specific packing and checks the coordinates. ~keep
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
    /// need the full-resolution raster, not a sampled estimate. Any other depth is
    /// refused: `pixGetPixel` returns the raw packed value, which is a palette
    /// index, a 1-bit flag or packed RGBA, and narrowing it to a byte would
    /// return meaningless intensities.
    ///
    /// # Errors
    ///
    /// Returns `TesseractError::InvalidParameterError` if the image is not 8 bpp,
    /// and `TesseractError::OcrError` if the image is zero-sized or if
    /// `pixGetPixel` fails on any pixel.
    pub fn grayscale_bytes(&self) -> Result<Vec<u8>> {
        self.require_grayscale()?;
        let width = self.width();
        let height = self.height();
        if width <= 0 || height <= 0 {
            return Err(TesseractError::OcrError);
        }

        let mut out = Vec::with_capacity((width as usize) * (height as usize));
        for y in 0..height {
            for x in 0..width {
                let mut value: u32 = 0;
                // SAFETY: self keeps the Pix live and value is a writable u32 local.
                // Leptonica reads the depth-specific packing and checks the coordinates. ~keep
                let status = unsafe { pixGetPixel(self.ptr, x, y, &mut value) };
                if status != 0 {
                    return Err(TesseractError::OcrError);
                }
                out.push(value as u8);
            }
        }
        Ok(out)
    }
}
