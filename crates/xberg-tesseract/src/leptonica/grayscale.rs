//! Read grayscale pixel values from Leptonica images.

use super::{Pix, pixGetPixel};
use crate::error::{Result, TesseractError};

impl Pix {
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
