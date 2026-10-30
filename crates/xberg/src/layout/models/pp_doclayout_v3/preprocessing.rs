use fast_image_resize::{Filter, FilterType, ResizeAlg, ResizeOptions, Resizer, images::TypedImage, pixels::F32x3};
use image::RgbImage;
use ndarray::Array4;

use crate::layout::error::LayoutError;

const CUBIC_PARAMETER: f64 = -0.75;
const CUBIC_BORDER: u32 = 2;
const RGB_CHANNELS: usize = 3;
const U8_MAX: f32 = 255.0;

pub(super) fn preprocess_cubic(img: &RgbImage, target: u32) -> Result<Array4<f32>, LayoutError> {
    if img.width() == 0 || img.height() == 0 {
        return Err(preprocess_error("cannot resize an empty image"));
    }
    let resized = resize_cubic(img, target)?;
    let size = target as usize;
    Ok(Array4::from_shape_fn(
        (1, RGB_CHANNELS, size, size),
        |(_, channel, y, x)| {
            let value = resized.pixels()[y * size + x].0[channel];
            value.round().clamp(0.0, U8_MAX) * (1.0 / U8_MAX)
        },
    ))
}

fn resize_cubic(img: &RgbImage, target: u32) -> Result<TypedImage<'static, F32x3>, LayoutError> {
    let source = replicate_borders(img)?;
    let mut resized = TypedImage::<F32x3>::new(target, target);
    let filter = Filter::new("OpenCV Keys cubic", cubic_weight, f64::from(CUBIC_BORDER))
        .map_err(|error| preprocess_error(error.to_string()))?;
    let options = ResizeOptions::new()
        .resize_alg(ResizeAlg::Interpolation(FilterType::Custom(filter)))
        .crop(
            f64::from(CUBIC_BORDER),
            f64::from(CUBIC_BORDER),
            f64::from(img.width()),
            f64::from(img.height()),
        )
        .use_alpha(false);
    // Float intermediates retain cubic overshoot until the final u8 rounding. ~keep
    Resizer::new()
        .resize_typed(&source, &mut resized, Some(&options))
        .map_err(|error| preprocess_error(error.to_string()))?;
    Ok(resized)
}

fn replicate_borders(img: &RgbImage) -> Result<TypedImage<'static, F32x3>, LayoutError> {
    let width = padded_dimension(img.width())?;
    let height = padded_dimension(img.height())?;
    let count = (width as usize)
        .checked_mul(height as usize)
        .ok_or_else(|| preprocess_error("replicated image dimensions overflow"))?;
    let mut pixels = Vec::new();
    pixels
        .try_reserve_exact(count)
        .map_err(|error| preprocess_error(format!("cannot allocate replicated image: {error}")))?;
    // Padding keeps every four-tap window intact: truncating and renormalizing
    // border taps changes OpenCV's replicated-edge interpolation. ~keep
    for y in 0..height {
        let source_y = y.saturating_sub(CUBIC_BORDER).min(img.height() - 1);
        for x in 0..width {
            let source_x = x.saturating_sub(CUBIC_BORDER).min(img.width() - 1);
            pixels.push(F32x3::new(img.get_pixel(source_x, source_y).0.map(f32::from)));
        }
    }
    TypedImage::from_pixels(width, height, pixels).map_err(|error| preprocess_error(error.to_string()))
}

fn padded_dimension(size: u32) -> Result<u32, LayoutError> {
    size.checked_add(CUBIC_BORDER * 2)
        .ok_or_else(|| preprocess_error("source dimension cannot fit a replicated cubic border"))
}

fn cubic_weight(distance: f64) -> f64 {
    let x = distance.abs();
    let a = CUBIC_PARAMETER;
    if x < 1.0 {
        ((a + 2.0) * x - (a + 3.0)) * x * x + 1.0
    } else if x < f64::from(CUBIC_BORDER) {
        ((a * x - 5.0 * a) * x + 8.0 * a) * x - 4.0 * a
    } else {
        0.0
    }
}

fn preprocess_error(message: impl Into<String>) -> LayoutError {
    LayoutError::Inference(format!("PP-DocLayoutV3 preprocessing: {}", message.into()))
}
