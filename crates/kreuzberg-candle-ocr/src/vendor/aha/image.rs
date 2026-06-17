// Vendored from jhqxxx/aha (Apache-2.0). See repo-root ATTRIBUTIONS.md § jhqxxx/aha.

//! Subset of `aha::utils::{img_utils, video_utils, interpolate}` covering image loading,
//! smart-resize, and bilinear interpolation used by Hunyuan-OCR / DeepSeek-OCR /
//! PaddleOCR-VL 1.5 processors.
//!
//! Changes from upstream:
//! - URL-fetch path removed from `get_image` (no HTTP in backend image loading).
//! - `assert_eq!` in `crop_img` replaced with a checked `Result` return.
//! - `anyhow::Result` replaced with `crate::error::Result` / `CandleOcrError`.
//! - `num::integer::lcm` inlined to avoid pulling in the `num` crate.
//! - `extract_images` / `extract_image_url` / `float_tensor_to_dynamic_image` dropped
//!   (depend on aha's chat types or are unused by the three VLM processors).
//! - `img_transform_with_resize` retained as a convenience combinator used by callers.

#![cfg(not(target_arch = "wasm32"))]

use std::{collections::HashSet, io::Cursor, path::PathBuf};

use candle_core::{DType, Device, Tensor};
use image::{DynamicImage, ImageBuffer, ImageReader, Rgb, RgbImage, imageops};

use crate::error::{CandleOcrError, Result};

// `url` is not a workspace dep; the file:// handler uses a manual strip instead.

// ---------------------------------------------------------------------------
// Factor arithmetic helpers (inlined from aha::utils)
// ---------------------------------------------------------------------------

/// Round `num` to the nearest multiple of `factor`.
fn round_by_factor(num: u32, factor: u32) -> u32 {
    let round = (num as f32 / factor as f32).round() as u32;
    round * factor
}

/// Round `num` down to the nearest multiple of `factor`.
fn floor_by_factor(num: f32, factor: u32) -> u32 {
    let floor = (num / factor as f32).floor() as u32;
    floor * factor
}

/// Round `num` up to the nearest multiple of `factor`.
fn ceil_by_factor(num: f32, factor: u32) -> u32 {
    let ceil = (num / factor as f32).ceil() as u32;
    ceil * factor
}

/// Least-common multiple of two `u32`s.  Avoids pulling in the `num` crate.
fn lcm(a: u32, b: u32) -> u32 {
    fn gcd(mut a: u32, mut b: u32) -> u32 {
        while b != 0 {
            let tmp = b;
            b = a % b;
            a = tmp;
        }
        a
    }
    if a == 0 || b == 0 { 0 } else { a / gcd(a, b) * b }
}

// ---------------------------------------------------------------------------
// Image loading (aha::utils::img_utils)
// ---------------------------------------------------------------------------

/// Decode an image from a base-64 string.
///
/// Accepts the raw base-64 payload (everything after `base64,` in a data-URI).
pub fn load_image_from_base64(base64_data: &str) -> Result<DynamicImage> {
    use base64::{Engine, engine::general_purpose};
    let image_data = general_purpose::STANDARD
        .decode(base64_data)
        .map_err(|e| CandleOcrError::InferenceFailed(format!("base64 decode: {e}")))?;
    let cursor = Cursor::new(image_data);
    let img = ImageReader::new(cursor)
        .with_guessed_format()
        .map_err(CandleOcrError::Io)?
        .decode()
        .map_err(CandleOcrError::Image)?;
    Ok(img)
}

/// Load an image from a local file path or a `file://` URI or a data-URI.
///
/// Unlike upstream aha, HTTP/HTTPS URLs are **not** supported: kreuzberg loads
/// OCR inputs from the local file system only.  Pass raw bytes through
/// [`image::load_from_memory`] if you already have them in memory.
///
/// Accepted forms:
/// - Bare filesystem path (`/tmp/doc.png`, `C:\docs\scan.jpg`)
/// - `file:///absolute/path`
/// - `data:image/<subtype>;base64,<payload>`
pub fn get_image(file: &str) -> Result<DynamicImage> {
    // data-URI with embedded base64 image
    if file.starts_with("data:image") && file.contains("base64,") {
        let payload = file
            .split_once("base64,")
            .map(|x| x.1)
            .ok_or_else(|| CandleOcrError::InferenceFailed("malformed data-URI".into()))?;
        return load_image_from_base64(payload);
    }

    // file:// URI → strip the scheme prefix and treat the rest as a local path.
    // On POSIX `file:///abs/path` → `/abs/path`; on Windows `file:///C:/...` → `C:/...`.
    if file.starts_with("file://") {
        let path = PathBuf::from(file.trim_start_matches("file://"));
        return ImageReader::open(path)
            .map_err(CandleOcrError::Io)?
            .decode()
            .map_err(CandleOcrError::Image);
    }

    // Bare filesystem path
    ImageReader::open(file)
        .map_err(CandleOcrError::Io)?
        .decode()
        .map_err(CandleOcrError::Image)
}

// ---------------------------------------------------------------------------
// Patch-grid helpers (aha::utils::img_utils)
// ---------------------------------------------------------------------------

/// Generate all `(grid_width, grid_height)` pairs whose product lies in
/// `[min_num, max_num]`, sorted by ascending product.
///
/// Used by [`dynamic_preprocess`] for InternVL-style tiling.
pub fn generate_target_ratios_sorted(min_num: u32, max_num: u32) -> Vec<(u32, u32)> {
    let mut target_ratios = HashSet::new();
    for n in min_num..=max_num {
        for i in 1..=n {
            for j in 1..=n {
                let product = i * j;
                if product <= max_num && product >= min_num {
                    target_ratios.insert((i, j));
                }
            }
        }
    }
    let mut sorted: Vec<(u32, u32)> = target_ratios.into_iter().collect();
    sorted.sort_by_key(|&(i, j)| i * j);
    sorted
}

/// Pick the `(grid_width, grid_height)` ratio from `target_ratios` that is
/// closest to the image's actual aspect ratio, breaking ties by area.
///
/// Returns `(grid_width, grid_height)`.
pub fn find_closest_aspect_ratio(
    aspect_ratio: f64,
    target_ratios: &[(u32, u32)],
    width: u32,
    height: u32,
    image_size: u32,
) -> (u32, u32) {
    let mut best_ratio_diff = f64::INFINITY;
    let mut best_ratio = (1u32, 1u32);
    let area = width * height;

    for &ratio in target_ratios {
        let target_aspect_ratio = ratio.0 as f64 / ratio.1 as f64;
        let ratio_diff = (aspect_ratio - target_aspect_ratio).abs();

        if ratio_diff < best_ratio_diff {
            best_ratio_diff = ratio_diff;
            best_ratio = ratio;
        } else if (ratio_diff - best_ratio_diff).abs() < 1e-10 {
            let target_area = 0.5 * (image_size as f64).powi(2) * (ratio.0 * ratio.1) as f64;
            if area as f64 > target_area {
                best_ratio = ratio;
            }
        }
    }
    best_ratio
}

/// Resize the image to `grid_width × image_size` by `grid_height × image_size`
/// and return the `grid_width * grid_height` individual crops.
///
/// # Errors
///
/// Returns [`CandleOcrError::InferenceFailed`] if the number of cropped tiles
/// does not match the expected `grid_width * grid_height` (guards against logic
/// bugs in the calling code).
pub fn crop_img(image: &DynamicImage, grid_height: u32, grid_width: u32, image_size: u32) -> Result<Vec<DynamicImage>> {
    let target_width = image_size * grid_width;
    let target_height = image_size * grid_height;
    let blocks = grid_width * grid_height;
    let mut resized_img = image.resize_exact(target_width, target_height, imageops::FilterType::CatmullRom);
    let mut processed_images = Vec::with_capacity(blocks as usize);
    for i in 0..blocks {
        let x1 = (i % grid_width) * image_size;
        let y1 = (i / grid_width) * image_size;
        let split_img = resized_img.crop(x1, y1, image_size, image_size);
        processed_images.push(split_img);
    }
    if processed_images.len() as u32 != blocks {
        return Err(CandleOcrError::InferenceFailed(format!(
            "crop_img: expected {blocks} tiles, got {}",
            processed_images.len()
        )));
    }
    Ok(processed_images)
}

/// Split an image into a grid of tiles, optionally appending a thumbnail.
///
/// Returns `(tiles, (grid_width, grid_height))`.
pub fn dynamic_preprocess(
    image: &DynamicImage,
    min_num: u32,
    max_num: u32,
    image_size: u32,
    use_thumbnail: bool,
) -> Result<(Vec<DynamicImage>, (u32, u32))> {
    let orig_width = image.width();
    let orig_height = image.height();
    let aspect_ratio = orig_width as f64 / orig_height as f64;
    let target_ratios = generate_target_ratios_sorted(min_num, max_num);
    let target_aspect_ratio =
        find_closest_aspect_ratio(aspect_ratio, &target_ratios, orig_width, orig_height, image_size);
    let mut processed_images = crop_img(image, target_aspect_ratio.1, target_aspect_ratio.0, image_size)?;
    if use_thumbnail && processed_images.len() != 1 {
        let thumbnail_img = image.resize_exact(image_size, image_size, imageops::FilterType::CatmullRom);
        processed_images.push(thumbnail_img);
    }
    Ok((processed_images, target_aspect_ratio))
}

/// Resize an image to exactly `(width, height)`, padding with `color` if the
/// original aspect ratio does not match.
pub fn resize_with_edge_padding(img: &DynamicImage, width: u32, height: u32, color: [u8; 3]) -> DynamicImage {
    let mut img = img.resize(width, height, imageops::FilterType::CatmullRom);
    if img.height() != height || img.width() != width {
        let (img_h, img_w) = (img.height(), img.width());
        let img_buffer = img.to_rgb8();
        let mut canvas: ImageBuffer<Rgb<u8>, Vec<u8>> = RgbImage::from_pixel(width, height, Rgb(color));
        let x_offset = (width - img_w) / 2;
        let y_offset = (height - img_h) / 2;
        imageops::overlay(&mut canvas, &img_buffer, x_offset as i64, y_offset as i64);
        img = DynamicImage::ImageRgb8(canvas);
    }
    img
}

// ---------------------------------------------------------------------------
// Tensor normalisation (aha::utils::img_utils)
// ---------------------------------------------------------------------------

/// Convert a [`DynamicImage`] to a normalised `(C, H, W)` tensor.
///
/// The image is rescaled to `[0, 1]` and then z-score normalised with the
/// provided per-channel `mean` and `std` tensors (shape `(3, 1, 1)`).
pub fn img_transform(img: &DynamicImage, mean: &Tensor, std: &Tensor, device: &Device, dtype: DType) -> Result<Tensor> {
    let img_h = img.height();
    let img_w = img.width();
    let img_vec = img.to_rgb8().into_raw();
    // (H, W, C) → (C, H, W)
    let img_tensor = Tensor::from_slice(&img_vec, (img_h as usize, img_w as usize, 3), device)?
        .permute((2, 0, 1))?
        .to_dtype(DType::F32)?;
    // Rescale 0–255 → 0–1
    let img_tensor = img_tensor.affine(1.0 / 255.0, 0.0)?;
    // Normalise
    let img_tensor = img_tensor
        .broadcast_sub(&mean.to_dtype(DType::F32)?)?
        .broadcast_div(&std.to_dtype(DType::F32)?)?
        .to_dtype(dtype)?;
    Ok(img_tensor)
}

/// Resize the image to `(w, h)` then call [`img_transform`].
pub fn img_transform_with_resize(
    img: &DynamicImage,
    h: u32,
    w: u32,
    mean: &Tensor,
    std: &Tensor,
    device: &Device,
    dtype: DType,
) -> Result<Tensor> {
    let img_resize = img.resize_exact(w, h, imageops::FilterType::CatmullRom);
    img_transform(&img_resize, mean, std, device, dtype)
}

// ---------------------------------------------------------------------------
// Smart-resize (aha::utils::img_utils)
// ---------------------------------------------------------------------------

/// Compute the target `(height, width)` for an image such that:
///
/// - Both dimensions are multiples of `factor`.
/// - The total pixel count lies in `[min_pixels, max_pixels]`.
/// - The absolute aspect ratio stays below 200:1.
///
/// Returns `(height, width)`.
pub fn img_smart_resize(img_h: u32, img_w: u32, factor: u32, min_pixels: u32, max_pixels: u32) -> Result<(u32, u32)> {
    if std::cmp::max(img_h, img_w) / std::cmp::min(img_h, img_w) > 200 {
        return Err(CandleOcrError::UnsupportedConfig(format!(
            "absolute aspect ratio must be smaller than 200, got {}",
            std::cmp::max(img_h, img_w) / std::cmp::min(img_h, img_w)
        )));
    }
    let image_factor = factor;
    let mut h_bar = std::cmp::max(image_factor, round_by_factor(img_h, image_factor));
    let mut w_bar = std::cmp::max(image_factor, round_by_factor(img_w, image_factor));

    if h_bar * w_bar > max_pixels {
        let beta = ((img_h * img_w) as f32 / max_pixels as f32).sqrt();
        h_bar = std::cmp::max(image_factor, floor_by_factor(img_h as f32 / beta, image_factor));
        w_bar = std::cmp::max(image_factor, floor_by_factor(img_w as f32 / beta, image_factor));
    } else if h_bar * w_bar < min_pixels {
        let beta = (min_pixels as f32 / (img_h * img_w) as f32).sqrt();
        h_bar = ceil_by_factor(img_h as f32 * beta, image_factor);
        w_bar = ceil_by_factor(img_w as f32 * beta, image_factor);
    }
    Ok((h_bar, w_bar))
}

// ---------------------------------------------------------------------------
// Video/temporal smart-resize (aha::utils::video_utils)
// ---------------------------------------------------------------------------

/// Compute the target spatial `(height, width)` for a video frame sequence
/// such that the spatial dimensions are multiples of `factor` (and
/// `lcm(factor, video_ratio)` when `video_ratio` is provided) and the total
/// pixel budget `num_frames * H * W` lies in `[min_pixels, max_pixels]`.
///
/// Used by GLM-OCR's processor where `num_frames = temporal_patch_size`.
/// Returns `(height, width)`.
pub fn video_smart_resize(
    num_frames: u32,
    height: u32,
    width: u32,
    temporal_factor: u32,
    factor: u32,
    min_pixels: u32,
    max_pixels: u32,
    video_ratio: Option<u32>,
) -> Result<(u32, u32)> {
    if num_frames < temporal_factor {
        return Err(CandleOcrError::UnsupportedConfig(format!(
            "{num_frames} must be >= temporal_factor {temporal_factor}"
        )));
    }
    if height < factor || width < factor {
        return Err(CandleOcrError::UnsupportedConfig(format!(
            "height:{height} or width:{width} must be >= factor:{factor}"
        )));
    }
    if std::cmp::max(height, width) / std::cmp::min(height, width) > 200 {
        return Err(CandleOcrError::UnsupportedConfig(format!(
            "absolute aspect ratio must be smaller than 200, got {}",
            std::cmp::max(height, width) / std::cmp::min(height, width)
        )));
    }
    let image_factor = if let Some(ratio) = video_ratio {
        lcm(factor, ratio)
    } else {
        factor
    };
    let mut h_bar = round_by_factor(height, image_factor);
    let mut w_bar = round_by_factor(width, image_factor);
    let t_bar = round_by_factor(num_frames, temporal_factor);

    if t_bar * h_bar * w_bar > max_pixels {
        let beta = ((num_frames * height * width) as f32 / max_pixels as f32).sqrt();
        h_bar = std::cmp::max(image_factor, floor_by_factor(height as f32 / beta, image_factor));
        w_bar = std::cmp::max(image_factor, floor_by_factor(width as f32 / beta, image_factor));
    } else if t_bar * h_bar * w_bar < min_pixels {
        let beta = (min_pixels as f32 / (num_frames * height * width) as f32).sqrt();
        h_bar = ceil_by_factor(height as f32 * beta, image_factor);
        w_bar = ceil_by_factor(width as f32 * beta, image_factor);
    }
    Ok((h_bar, w_bar))
}

// ---------------------------------------------------------------------------
// Interpolation (aha::utils::interpolate — bilinear subset)
// ---------------------------------------------------------------------------

/// Compute the scale factor for coordinate mapping.
fn compute_scale(input_size: usize, output_size: usize, align_corners: bool) -> f32 {
    if align_corners && output_size > 1 {
        (input_size - 1) as f32 / (output_size - 1) as f32
    } else {
        input_size as f32 / output_size as f32
    }
}

/// Compute the source-space coordinates for an up/down-sampling operation along
/// one spatial dimension.
pub fn compute_1d_coords(input_size: usize, output_size: usize, align_corner: Option<bool>) -> Result<Vec<f32>> {
    if input_size == 0 {
        return Err(CandleOcrError::InferenceFailed(
            "compute_1d_coords: input_size must be > 0".into(),
        ));
    }
    if output_size == 0 {
        return Err(CandleOcrError::InferenceFailed(
            "compute_1d_coords: output_size must be > 0".into(),
        ));
    }
    if input_size == 1 {
        return Ok(vec![0f32; output_size]);
    }
    let align_corners = align_corner.unwrap_or(false);
    let scale = compute_scale(input_size, output_size, align_corners);
    if align_corners {
        Ok((0..output_size).map(|i| i as f32 * scale).collect())
    } else {
        Ok((0..output_size)
            .map(|i| {
                let coord = (i as f32 + 0.5) * scale - 0.5;
                coord.clamp(0.0, (input_size - 1) as f32)
            })
            .collect())
    }
}

/// Bilinear interpolation without anti-aliasing (standard PyTorch behaviour).
///
/// Input layout: `[N, C, H, W]`.  Returns a tensor of shape
/// `[N, C, target_height, target_width]` in the same dtype and on the same
/// device as `input`.
pub fn interpolate_bilinear_standard(
    input: &Tensor,
    target_size: (usize, usize),
    align_corner: Option<bool>,
) -> Result<Tensor> {
    let (bs, channels, input_height, input_width) = input.dims4()?;
    let (target_height, target_width) = target_size;

    let coords_h = compute_1d_coords(input_height, target_height, align_corner)?;
    let coords_w = compute_1d_coords(input_width, target_width, align_corner)?;

    let dim0 = bs * channels;
    let input_3dim = input.reshape((dim0, input_height, input_width))?;
    let input_data = input_3dim.to_dtype(DType::F32)?.to_vec3::<f32>()?;
    let mut output_data = vec![vec![vec![0.0f32; target_width]; target_height]; dim0];

    for c in 0..dim0 {
        for (i, &coord_h) in coords_h.iter().enumerate() {
            let coord_h = coord_h.clamp(0.0, (input_height - 1) as f32);
            let y0 = coord_h.floor() as usize;
            let y1 = (y0 + 1).min(input_height - 1);
            let dy = coord_h - y0 as f32;
            for (j, &coord_w) in coords_w.iter().enumerate() {
                let coord_w = coord_w.clamp(0.0, (input_width - 1) as f32);
                let x0 = coord_w.floor() as usize;
                let x1 = (x0 + 1).min(input_width - 1);
                let dx = coord_w - x0 as f32;

                let q00 = input_data[c][y0][x0];
                let q01 = input_data[c][y0][x1];
                let q10 = input_data[c][y1][x0];
                let q11 = input_data[c][y1][x1];
                output_data[c][i][j] =
                    q00 * (1.0 - dx) * (1.0 - dy) + q01 * dx * (1.0 - dy) + q10 * (1.0 - dx) * dy + q11 * dx * dy;
            }
        }
    }
    let output = Tensor::new(output_data, input.device())?
        .reshape((bs, channels, target_height, target_width))?
        .to_dtype(input.dtype())?
        .contiguous()?;
    Ok(output)
}

fn antialias_filter(x: f32) -> f32 {
    let x = x.abs();
    if x < 1.0 { 1.0 - x } else { 0.0 }
}

/// Bilinear interpolation with a tent anti-aliasing filter applied when
/// down-sampling.
///
/// Input layout: `[N, C, H, W]`.
// The inner loops use indices into a 3-D Vec at non-sequential positions derived
// from floating-point center coordinates; the needless_range_loop lint's
// suggested iterator form would obscure the bilinear math without benefit.
#[allow(clippy::needless_range_loop)]
pub fn interpolate_bilinear_antialias(input: &Tensor, target_size: (usize, usize)) -> Result<Tensor> {
    let (bs, channels, input_height, input_width) = input.dims4()?;
    let (target_height, target_width) = target_size;

    let scale_h = input_height as f32 / target_height as f32;
    let scale_w = input_width as f32 / target_width as f32;

    let dim0 = bs * channels;
    let input_3dim = input.reshape((dim0, input_height, input_width))?;
    let input_data = input_3dim.to_dtype(DType::F32)?.to_vec3::<f32>()?;
    let mut output_data = vec![vec![vec![0.0f32; target_width]; target_height]; dim0];

    let support_size = scale_h.max(scale_w);
    for c in 0..dim0 {
        for out_y in 0..target_height {
            let center_y = (out_y as f32 + 0.5) * scale_h - 0.5;
            let start_y = (center_y - support_size).max(0.0) as usize;
            let end_y = (center_y + support_size).min(input_height as f32 - 1.0) as usize;
            for out_x in 0..target_width {
                let center_x = (out_x as f32 + 0.5) * scale_w - 0.5;
                let start_x = (center_x - support_size).max(0.0) as usize;
                let end_x = (center_x + support_size).min(input_width as f32 - 1.0) as usize;
                let mut total_weight = 0.0f32;
                let mut weighted_sum = 0.0f32;

                for src_y in start_y..=end_y {
                    for src_x in start_x..=end_x {
                        let dist_x = (src_x as f32 - center_x).abs();
                        let dist_y = (src_y as f32 - center_y).abs();
                        let weight_x = antialias_filter(dist_x / scale_w);
                        let weight_y = antialias_filter(dist_y / scale_h);
                        let weight = weight_x * weight_y;
                        weighted_sum += input_data[c][src_y][src_x] * weight;
                        total_weight += weight;
                    }
                }
                output_data[c][out_y][out_x] = if total_weight > 0.0 {
                    weighted_sum / total_weight
                } else {
                    let y = center_y.round().clamp(0.0, (input_height - 1) as f32) as usize;
                    let x = center_x.round().clamp(0.0, (input_width - 1) as f32) as usize;
                    input_data[c][y][x]
                };
            }
        }
    }
    let output = Tensor::new(output_data, input.device())?
        .reshape((bs, channels, target_height, target_width))?
        .to_dtype(input.dtype())?
        .contiguous()?;
    Ok(output)
}

/// Bilinear spatial interpolation of a 4-D tensor `[N, C, H, W]`.
///
/// - `align_corner`: when `Some(true)` aligns corners (PyTorch
///   `align_corners=True`); defaults to `false`.
/// - `antialias`: when `Some(true)` and the output is smaller than the input,
///   a tent filter is applied to reduce aliasing.
///
/// Returns a tensor of the same dtype and device, shaped
/// `[N, C, target_height, target_width]`.
pub fn interpolate_bilinear(
    input: &Tensor,
    target_size: (usize, usize),
    align_corner: Option<bool>,
    antialias: Option<bool>,
) -> Result<Tensor> {
    if input.rank() != 4 {
        return Err(CandleOcrError::InferenceFailed(format!(
            "interpolate_bilinear: expected rank 4 [N,C,H,W], got {}",
            input.rank()
        )));
    }
    let (_, _, input_height, input_width) = input.dims4()?;
    let (target_height, target_width) = target_size;

    if input_height == target_height && input_width == target_width {
        return Ok(input.clone());
    }

    let output = if antialias.unwrap_or(false) && (target_height < input_height || target_width < input_width) {
        interpolate_bilinear_antialias(input, target_size)?
    } else {
        interpolate_bilinear_standard(input, target_size, align_corner)?
    };
    Ok(output.to_dtype(input.dtype())?.to_device(input.device())?)
}

// ---------------------------------------------------------------------------
// Unit tests
// ---------------------------------------------------------------------------

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn round_by_factor_basic() {
        assert_eq!(round_by_factor(17, 8), 16);
        assert_eq!(round_by_factor(20, 8), 24); // 20/8 = 2.5 → rounds to 3
        assert_eq!(round_by_factor(16, 16), 16);
    }

    #[test]
    fn floor_by_factor_basic() {
        assert_eq!(floor_by_factor(17.9, 8), 16);
        assert_eq!(floor_by_factor(16.0, 8), 16);
    }

    #[test]
    fn ceil_by_factor_basic() {
        assert_eq!(ceil_by_factor(17.1, 8), 24);
        assert_eq!(ceil_by_factor(16.0, 8), 16);
    }

    #[test]
    fn lcm_basic() {
        assert_eq!(lcm(4, 6), 12);
        assert_eq!(lcm(14, 28), 28);
        assert_eq!(lcm(0, 5), 0);
    }

    #[test]
    fn img_smart_resize_within_budget() {
        // 480×640 with factor=32, min=1024, max=4096000
        let (h, w) = img_smart_resize(480, 640, 32, 1024, 4_096_000).unwrap();
        assert_eq!(h % 32, 0);
        assert_eq!(w % 32, 0);
    }

    #[test]
    fn img_smart_resize_rejects_extreme_aspect() {
        assert!(img_smart_resize(1, 300, 1, 1, 10_000).is_err());
    }

    #[test]
    fn video_smart_resize_basic() {
        let (h, w) = video_smart_resize(2, 480, 640, 2, 28, 12_544, 9_633_792, None).unwrap();
        assert_eq!(h % 28, 0);
        assert_eq!(w % 28, 0);
    }

    #[test]
    fn video_smart_resize_rejects_too_few_frames() {
        assert!(video_smart_resize(1, 480, 640, 2, 28, 12_544, 9_633_792, None).is_err());
    }

    #[test]
    fn generate_target_ratios_sorted_count() {
        let ratios = generate_target_ratios_sorted(1, 4);
        // All (i,j) with 1 ≤ i*j ≤ 4: (1,1),(1,2),(2,1),(1,3),(3,1),(2,2),(1,4),(4,1),(2,3? no: 6>4)
        // Actually: products 1,2,2,3,3,4,4 unique pairs = 7
        // Sorted ascending: (1,1)=1,(1,2)=2,(2,1)=2,(1,3)=3,(3,1)=3,(1,4)=4,(2,2)=4,(4,1)=4
        assert!(!ratios.is_empty());
        for pair in &ratios {
            assert!(pair.0 * pair.1 <= 4);
            assert!(pair.0 * pair.1 >= 1);
        }
        // Verify sort order: product is non-decreasing
        for w in ratios.windows(2) {
            assert!(w[0].0 * w[0].1 <= w[1].0 * w[1].1);
        }
    }

    #[test]
    fn crop_img_produces_correct_tile_count() {
        let img = DynamicImage::new_rgb8(64, 64);
        let tiles = crop_img(&img, 2, 2, 32).unwrap();
        assert_eq!(tiles.len(), 4);
        for t in &tiles {
            assert_eq!(t.width(), 32);
            assert_eq!(t.height(), 32);
        }
    }

    #[test]
    fn compute_1d_coords_identity() {
        let coords = compute_1d_coords(4, 4, None).unwrap();
        for (i, &c) in coords.iter().enumerate() {
            assert!((c - i as f32).abs() < 1e-5, "coord[{i}] = {c}, expected {i}");
        }
    }
}
