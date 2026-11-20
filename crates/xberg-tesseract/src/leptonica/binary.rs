//! Read binary pixel values from Leptonica images.

use super::{Pix, pixGetColormap, pixGetPixel};
use crate::error::{Result, TesseractError};

impl Pix {
    /// Reads every pixel of a 1 bpp binary Pix into a byte-per-pixel buffer, in
    /// left-to-right, top-to-bottom order.
    ///
    /// Each byte is 1 where the pixel's bit is set, which Leptonica's thresholding
    /// marks as ink, and 0 for background. This is the binary counterpart of
    /// [`Pix::grayscale_bytes`]: a 1 bpp image has no intensities to narrow, and
    /// `pixGetPixel` already returns the bit itself. Any other depth, or a
    /// colormapped image whose set bit indexes a palette rather than marking ink,
    /// is refused.
    ///
    /// # Errors
    ///
    /// Returns `TesseractError::InvalidParameterError` if the image is not a 1 bpp
    /// image without a colormap, and `TesseractError::OcrError` if the image is
    /// zero-sized or if `pixGetPixel` fails on any pixel.
    pub fn binary_bytes(&self) -> Result<Vec<u8>> {
        self.require_binary()?;
        let width = self.width();
        let height = self.height();
        if width <= 0 || height <= 0 {
            return Err(TesseractError::OcrError);
        }
        let pixel_count = usize::try_from(width)
            .ok()
            .and_then(|width| {
                usize::try_from(height)
                    .ok()
                    .and_then(|height| width.checked_mul(height))
            })
            .ok_or(TesseractError::OcrError)?;

        let mut out = Vec::with_capacity(pixel_count);
        for y in 0..height {
            for x in 0..width {
                let mut value: u32 = 0;
                // SAFETY: self keeps the Pix live and value is a writable u32 local.
                // Leptonica reads the depth-specific packing and checks the coordinates. ~keep
                let status = unsafe { pixGetPixel(self.ptr, x, y, &mut value) };
                if status != 0 {
                    return Err(TesseractError::OcrError);
                }
                out.push(u8::from(value != 0));
            }
        }
        Ok(out)
    }

    fn require_binary(&self) -> Result<()> {
        // SAFETY: self keeps the Pix alive; the call only reads the colormap pointer. ~keep
        let colormapped = !unsafe { pixGetColormap(self.ptr) }.is_null();
        if self.depth() == 1 && !colormapped {
            Ok(())
        } else {
            Err(TesseractError::InvalidParameterError)
        }
    }
}
