//! Pixel-capped image decoding to prevent OOM on crafted inputs.
//!
//! All four image-decode call sites that accept attacker-controlled bytes
//! must go through [`decode_with_pixel_cap`] instead of calling
//! `image::load_from_memory` directly.

use std::io::Cursor;

use crate::error::{KreuzbergError, Result};

/// Maximum number of pixels accepted from a single image decode.
///
/// 64 MP = 8000 × 8000 at ~3 bytes/px ≈ 192 MB decoded; above this an
/// attacker can trigger multi-GB allocations with a crafted image header.
pub(crate) const MAX_DECODE_PIXELS: u64 = 64_000_000;

/// Decode image bytes with a pixel-dimension cap.
///
/// Probes the image header first (cheap, no pixel allocation) and rejects
/// images whose `width × height` exceeds [`MAX_DECODE_PIXELS`].  Images
/// within the cap are decoded identically to `image::load_from_memory`.
///
/// # Errors
///
/// Returns `KreuzbergError::ImageProcessing` when:
/// - the format cannot be detected from the header,
/// - the dimensions cannot be probed, or
/// - the pixel count exceeds the cap.
///
/// Returns `KreuzbergError::Parsing` when the image data cannot be decoded.
pub(crate) fn decode_with_pixel_cap(bytes: &[u8]) -> Result<image::DynamicImage> {
    // --- header probe (no pixel allocation) ---
    let probe = image::ImageReader::new(Cursor::new(bytes))
        .with_guessed_format()
        .map_err(|e| KreuzbergError::image_processing(format!("image format probe failed: {e}")))?;

    let (w, h) = probe
        .into_dimensions()
        .map_err(|e| KreuzbergError::image_processing(format!("image dimension probe failed: {e}")))?;

    if (w as u64) * (h as u64) > MAX_DECODE_PIXELS {
        return Err(KreuzbergError::image_processing(format!(
            "image decode rejected: {w}x{h} ({} pixels) exceeds {} pixel cap (DoS guard)",
            (w as u64) * (h as u64),
            MAX_DECODE_PIXELS,
        )));
    }

    // --- full decode ---
    let reader = image::ImageReader::new(Cursor::new(bytes))
        .with_guessed_format()
        .map_err(|e| KreuzbergError::image_processing(format!("image format probe failed: {e}")))?;

    reader
        .decode()
        .map_err(|e| KreuzbergError::parsing(format!("image decode failed: {e}")))
}

#[cfg(test)]
mod tests {
    use std::io::Cursor;

    use image::{DynamicImage, ImageFormat, RgbImage};

    use super::*;

    fn encode_png(image: DynamicImage) -> Vec<u8> {
        let mut buf = Vec::new();
        image
            .write_to(&mut Cursor::new(&mut buf), ImageFormat::Png)
            .expect("PNG encode failed in test");
        buf
    }

    #[test]
    fn decode_with_pixel_cap_accepts_small_image() {
        let img = DynamicImage::ImageRgb8(RgbImage::new(100, 100));
        let png = encode_png(img);
        let result = decode_with_pixel_cap(&png);
        assert!(result.is_ok(), "expected Ok, got: {:?}", result.err());
        let decoded = result.unwrap();
        assert_eq!(decoded.width(), 100);
        assert_eq!(decoded.height(), 100);
    }

    #[test]
    fn decode_with_pixel_cap_rejects_oversized_image() {
        // 8000 × 9000 = 72 MP, above the 64 MP cap
        let img = DynamicImage::ImageRgb8(RgbImage::new(8000, 9000));
        let png = encode_png(img);
        let result = decode_with_pixel_cap(&png);
        assert!(result.is_err(), "expected Err for oversized image");
        let msg = result.unwrap_err().to_string();
        assert!(
            msg.contains("DoS guard") || msg.contains("pixel cap"),
            "error message should mention DoS guard or pixel cap, got: {msg}"
        );
    }
}
