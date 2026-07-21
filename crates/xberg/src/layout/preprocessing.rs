use image::RgbImage;
use ndarray::Array4;

/// Preprocess with rescale only (no ImageNet normalization).
///
/// Pipeline: bilinear resize to target_size x target_size -> rescale /255
/// -> NCHW f32. This is also the preprocessing contract of the original
/// Docling Heron ONNX export.
pub(crate) fn preprocess_rescale(img: &RgbImage, target_size: u32) -> Array4<f32> {
    let resized = image::imageops::resize(img, target_size, target_size, image::imageops::FilterType::Triangle);
    let pixels = resized.as_raw();
    let ts = target_size as usize;

    Array4::from_shape_fn((1, 3, ts, ts), |(_, c, y, x)| {
        pixels[(y * ts + x) * 3 + c] as f32 * (1.0 / 255.0)
    })
}

/// Letterbox preprocessing for YOLOX-style models.
///
/// Resizes the image to fit within (target_width x target_height) while maintaining
/// aspect ratio, padding the remaining area with value 114.0 (raw pixel value).
/// No normalization — values are 0-255 as YOLOX expects.
///
/// Returns the NCHW tensor and the scale ratio (for rescaling detections back).
///
/// ORT-only: used solely by [`crate::layout::models::yolo`], which is gated out of the
/// pure-Rust `layout-tract` variant.
#[cfg(feature = "layout-detection")]
pub(crate) fn preprocess_letterbox(img: &RgbImage, target_width: u32, target_height: u32) -> (Array4<f32>, f32) {
    let (orig_w, orig_h) = (img.width() as f32, img.height() as f32);
    let scale = (target_height as f32 / orig_h).min(target_width as f32 / orig_w);
    let new_w = (orig_w * scale) as u32;
    let new_h = (orig_h * scale) as u32;

    let resized = image::imageops::resize(img, new_w, new_h, image::imageops::FilterType::Triangle);

    let tw = target_width as usize;
    let th = target_height as usize;
    let hw = th * tw;
    let mut data = vec![114.0f32; 3 * hw];

    let rw = new_w as usize;
    let rh = new_h as usize;
    let resized_pixels = resized.as_raw();

    for y in 0..rh {
        for x in 0..rw {
            let src_idx = (y * rw + x) * 3;
            let dst_idx = y * tw + x;
            data[dst_idx] = resized_pixels[src_idx] as f32;
            data[hw + dst_idx] = resized_pixels[src_idx + 1] as f32;
            data[2 * hw + dst_idx] = resized_pixels[src_idx + 2] as f32;
        }
    }

    let tensor = Array4::from_shape_vec((1, 3, th, tw), data).expect("shape mismatch in preprocess_letterbox");

    (tensor, scale)
}

#[cfg(test)]
mod tests {
    use image::{Rgb, RgbImage};

    use super::preprocess_rescale;

    #[test]
    fn preprocess_rescale_matches_heron_contract_for_portrait_input() {
        let img = RgbImage::from_pixel(320, 640, Rgb([255, 0, 0]));
        let tensor = preprocess_rescale(&img, 640);

        assert_eq!(tensor.shape(), &[1, 3, 640, 640]);
        assert_eq!(tensor[[0, 0, 0, 0]], 1.0);
        assert_eq!(tensor[[0, 1, 0, 0]], 0.0);
        assert_eq!(tensor[[0, 2, 639, 639]], 0.0);
        assert_eq!(tensor[[0, 0, 639, 639]], 1.0);
    }

    #[test]
    fn preprocess_rescale_matches_heron_contract_for_landscape_input() {
        let img = RgbImage::from_pixel(640, 320, Rgb([0, 128, 255]));
        let tensor = preprocess_rescale(&img, 640);

        assert_eq!(tensor.shape(), &[1, 3, 640, 640]);
        assert_eq!(tensor[[0, 0, 0, 0]], 0.0);
        assert!((tensor[[0, 1, 0, 0]] - 128.0 / 255.0).abs() < f32::EPSILON);
        assert_eq!(tensor[[0, 2, 639, 639]], 1.0);
        assert_eq!(tensor[[0, 1, 639, 639]], 128.0 / 255.0);
    }

    #[test]
    fn preprocess_rescale_preserves_odd_image_geometry_and_channel_order() {
        let img = RgbImage::from_fn(3, 5, |x, y| {
            Rgb([(x * 80 + y * 7) as u8, (x * 11 + y * 40) as u8, (x * 31 + y * 13) as u8])
        });
        let tensor = preprocess_rescale(&img, 5);

        assert_eq!(tensor.shape(), &[1, 3, 5, 5]);
        for (channel, expected) in [94.0, 91.0, 57.0].into_iter().enumerate() {
            let actual = tensor[[0, channel, 2, 2]];
            assert!(
                (actual - expected / 255.0).abs() < f32::EPSILON,
                "center channel {channel}: expected {expected}, got {}",
                actual * 255.0
            );
        }
        for (channel, expected) in [188.0, 182.0, 114.0].into_iter().enumerate() {
            assert!((tensor[[0, channel, 4, 4]] - expected / 255.0).abs() < f32::EPSILON);
        }
    }
}
