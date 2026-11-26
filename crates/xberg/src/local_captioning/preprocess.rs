use image::imageops::FilterType;

use super::policy::invalid;
use super::{LocalCaptioningConfig, LocalCaptioningFormat};

/// Upper bound on a preprocessed frame edge. A larger value is a malformed or hostile processor
/// config, not a legitimate export.
pub(super) const MAX_EDGE: u32 = 4_096;
/// Upper bound on the text-side image token count the prompt expands to per frame.
pub(super) const MAX_IMAGE_SEQ_LEN: u32 = 1_024;
/// Upper bound on the tiles one image is split into, excluding the global frame. The pinned
/// SmolVLM processor resizes to a 2048-pixel longest edge and splits into 512-pixel tiles, which
/// is at most a 4x4 grid; a processor config that could exceed this is rejected.
pub(super) const MAX_TILES: u32 = 16;
/// Upper bound on the image tokens one prompt expands to, across every tile and the global frame.
pub(super) const MAX_TOTAL_IMAGE_TOKENS: u32 = 2_048;
/// Absolute ceiling the pinned Idefics3 processor applies to the longest edge before splitting
/// (`MAX_IMAGE_SIZE` in the upstream module).
const ABSOLUTE_MAX_IMAGE_SIZE: u32 = 4_096;

fn validate_dimensions(edge: u32, image_seq_len: u32) -> crate::Result<()> {
    if edge == 0 || edge > MAX_EDGE {
        return Err(invalid(format!(
            "Local captioning image edge must be in 1..={MAX_EDGE}, got {edge}"
        )));
    }
    if image_seq_len == 0 || image_seq_len > MAX_IMAGE_SEQ_LEN {
        return Err(invalid(format!(
            "Local captioning image token count must be in 1..={MAX_IMAGE_SEQ_LEN}, got {image_seq_len}"
        )));
    }
    Ok(())
}

/// How the pinned processor describes input images and the text-side image token count.
#[derive(Debug)]
pub(super) struct PreprocessConfig {
    /// Per-frame edge (`max_image_size.longest_edge`): the tile and global-frame side.
    pub(super) edge: u32,
    /// Longest edge the processor resizes to before splitting (`size.longest_edge`).
    pub(super) resize_edge: u32,
    /// Whether the pinned processor splits an image into tiles (`do_image_splitting`).
    pub(super) do_image_splitting: bool,
    pub(super) image_seq_len: u32,
    pub(super) mean: [f32; 3],
    pub(super) std: [f32; 3],
    pub(super) rescale: f32,
    filter: FilterType,
}

/// A batch of normalized `[num_images, 3, edge, edge]` pixel tensors in CHW order.
pub(super) struct ImageBatch {
    pub(super) pixels: Vec<f32>,
    pub(super) num_images: usize,
    pub(super) edge: u32,
    /// Tile grid the pinned processor reported, or `0`/`0` for a single global frame.
    pub(super) rows: usize,
    pub(super) cols: usize,
}

fn value_f32(value: &serde_json::Value, key: &str, fallback: Option<f32>) -> crate::Result<f32> {
    match value.get(key).and_then(serde_json::Value::as_f64) {
        Some(number) => Ok(number as f32),
        None => fallback.ok_or_else(|| invalid(format!("Local captioning processor config lacks a numeric {key}"))),
    }
}

fn value_u32(value: &serde_json::Value, key: &str) -> Option<u32> {
    value
        .get(key)
        .and_then(serde_json::Value::as_u64)
        .and_then(|number| u32::try_from(number).ok())
}

fn value_bool(value: &serde_json::Value, key: &str, fallback: bool) -> bool {
    value.get(key).and_then(serde_json::Value::as_bool).unwrap_or(fallback)
}

fn longest_edge(value: Option<&serde_json::Value>) -> Option<u32> {
    value.and_then(|size| value_u32(size, "longest_edge"))
}

fn channel_triplet(value: &serde_json::Value, key: &str, fallback: [f32; 3]) -> crate::Result<[f32; 3]> {
    let Some(array) = value.get(key).and_then(serde_json::Value::as_array) else {
        return Ok(fallback);
    };
    let channels: Vec<f32> = array
        .iter()
        .filter_map(serde_json::Value::as_f64)
        .map(|number| number as f32)
        .collect();
    channels
        .try_into()
        .map_err(|_| invalid(format!("Local captioning processor config has a malformed {key}")))
}

fn filter(resample: Option<u32>) -> FilterType {
    match resample {
        Some(1) => FilterType::Lanczos3,
        Some(2) => FilterType::Triangle,
        Some(3) => FilterType::CatmullRom,
        _ => FilterType::Triangle,
    }
}

impl PreprocessConfig {
    pub(super) fn from_json(
        format: LocalCaptioningFormat,
        preprocessor: &serde_json::Value,
        processor: &serde_json::Value,
    ) -> crate::Result<Self> {
        let mean = channel_triplet(preprocessor, "image_mean", [0.5, 0.5, 0.5])?;
        let std = channel_triplet(preprocessor, "image_std", [0.5, 0.5, 0.5])?;
        let rescale = value_f32(preprocessor, "rescale_factor", Some(1.0 / 255.0))?;
        if !rescale.is_finite() || rescale <= 0.0 || mean.iter().chain(&std).any(|value| !value.is_finite()) {
            return Err(invalid("Local captioning preprocessing constants must be finite"));
        }
        let filter = filter(value_u32(preprocessor, "resample"));
        match format {
            LocalCaptioningFormat::SmolVlm => {
                let max_image_size = preprocessor.get("max_image_size");
                let edge = longest_edge(max_image_size)
                    .or_else(|| longest_edge(preprocessor.get("size")))
                    .ok_or_else(|| invalid("Local captioning preprocessor config lacks an image edge"))?;
                let resize_edge = longest_edge(preprocessor.get("size")).unwrap_or(edge);
                let do_image_splitting = value_bool(preprocessor, "do_image_splitting", true);
                let image_seq_len = value_u32(processor, "image_seq_len")
                    .ok_or_else(|| invalid("SmolVLM processor config lacks image_seq_len"))?;
                validate_dimensions(edge, image_seq_len)?;
                validate_tile_budget(edge, resize_edge, image_seq_len)?;
                Ok(Self {
                    edge,
                    resize_edge,
                    do_image_splitting,
                    image_seq_len,
                    mean,
                    std,
                    rescale,
                    filter,
                })
            }
            LocalCaptioningFormat::Florence2 => {
                let size = preprocessor
                    .get("size")
                    .ok_or_else(|| invalid("Florence-2 preprocessor config lacks a size"))?;
                let edge = value_u32(size, "height")
                    .ok_or_else(|| invalid("Florence-2 preprocessor config lacks a size height"))?;
                let image_seq_len = value_u32(preprocessor, "image_seq_length")
                    .ok_or_else(|| invalid("Florence-2 preprocessor config lacks image_seq_length"))?;
                validate_dimensions(edge, image_seq_len)?;
                Ok(Self {
                    edge,
                    resize_edge: edge,
                    do_image_splitting: false,
                    image_seq_len,
                    mean,
                    std,
                    rescale,
                    filter,
                })
            }
        }
    }

    fn normalize(&self, image: &image::RgbImage) -> Vec<f32> {
        let (width, height) = image.dimensions();
        let mut pixels = vec![0.0f32; 3 * width as usize * height as usize];
        let plane = width as usize * height as usize;
        for (index, pixel) in image.pixels().enumerate() {
            for channel in 0..3 {
                let value = f32::from(pixel.0[channel]) * self.rescale;
                pixels[channel * plane + index] = (value - self.mean[channel]) / self.std[channel];
            }
        }
        pixels
    }
}

/// The pinned processor always resizes the longest edge to `size.longest_edge` (upscaling a
/// smaller image), caps it at its absolute ceiling, then rounds each side up to a multiple of the
/// tile edge. So the largest grid is derivable from the two configured edges; reject a config
/// whose grid or token expansion exceeds this adapter's ceilings before any image is decoded.
fn validate_tile_budget(edge: u32, resize_edge: u32, image_seq_len: u32) -> crate::Result<()> {
    if resize_edge == 0 || resize_edge > ABSOLUTE_MAX_IMAGE_SIZE {
        return Err(invalid(format!(
            "Local captioning resize edge must be in 1..={ABSOLUTE_MAX_IMAGE_SIZE}, got {resize_edge}"
        )));
    }
    let longest = resize_edge.min(ABSOLUTE_MAX_IMAGE_SIZE);
    let side = longest.div_ceil(edge);
    let tiles = u64::from(side) * u64::from(side);
    if tiles > u64::from(MAX_TILES) {
        return Err(invalid(format!(
            "Local captioning processor could split one image into {tiles} tiles; the adapter allows {MAX_TILES}"
        )));
    }
    let total = u64::from(image_seq_len) * (tiles + 1);
    if total > u64::from(MAX_TOTAL_IMAGE_TOKENS) {
        return Err(invalid(format!(
            "Local captioning prompt could expand to {total} image tokens; the adapter allows {MAX_TOTAL_IMAGE_TOKENS}"
        )));
    }
    Ok(())
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(super) struct TileGeometry {
    /// Size the image is resized to before the vision-encoder rounding step.
    pub(super) resize: (u32, u32),
    /// Size after rounding each side to a multiple of the tile edge.
    pub(super) tiled: (u32, u32),
    pub(super) rows: u32,
    pub(super) cols: u32,
}

impl TileGeometry {
    pub(super) fn frames(self) -> u32 {
        if self.rows == 0 || self.cols == 0 {
            1
        } else {
            self.rows * self.cols + 1
        }
    }
}

/// `_resize_output_size_rescale_to_max_len` from the pinned Idefics3 processor: the longest edge
/// becomes `max_len` and the short edge keeps the aspect ratio, rounded up to an even number.
fn rescale_to_max_len(height: u32, width: u32, max_len: u32) -> (u32, u32) {
    let aspect_ratio = f64::from(width) / f64::from(height);
    let (height, width) = if width >= height {
        let width = max_len;
        let mut height = (f64::from(width) / aspect_ratio) as u32;
        if !height.is_multiple_of(2) {
            height += 1;
        }
        (height, width)
    } else {
        let height = max_len;
        let mut width = (f64::from(height) * aspect_ratio) as u32;
        if !width.is_multiple_of(2) {
            width += 1;
        }
        (height, width)
    };
    (height.max(1), width.max(1))
}

/// `_resize_output_size_scale_below_upper_bound`: only ever scales a side past `max_len` down.
fn scale_below_upper_bound(height: u32, width: u32, max_len: u32) -> (u32, u32) {
    let aspect_ratio = f64::from(width) / f64::from(height);
    let (height, width) = if width >= height && width > max_len {
        let width = max_len;
        ((f64::from(width) / aspect_ratio) as u32, width)
    } else if height > width && height > max_len {
        let height = max_len;
        (height, (f64::from(height) * aspect_ratio) as u32)
    } else {
        (height, width)
    };
    (height.max(1), width.max(1))
}

/// Reproduces the pinned processor's ladder before splitting: resize the longest edge to
/// `size.longest_edge` (capped at 4096), then round both sides up to whole tiles. The grid is the
/// rounding result, so a non-square image keeps its aspect ratio instead of being squashed.
pub(super) fn tile_geometry(width: u32, height: u32, edge: u32, resize_edge: u32) -> TileGeometry {
    let (height, width) = rescale_to_max_len(height.max(1), width.max(1), resize_edge);
    let (height, width) = scale_below_upper_bound(height, width, ABSOLUTE_MAX_IMAGE_SIZE);
    // `int()` truncation of the short side, exactly as `resize_for_vision_encoder` does.
    let aspect_ratio = f64::from(width) / f64::from(height);
    let (tiled_height, tiled_width) = if width >= height {
        let tiled_width = width.div_ceil(edge) * edge;
        let tiled_height = (f64::from(tiled_width) / aspect_ratio) as u32;
        (tiled_height.div_ceil(edge) * edge, tiled_width)
    } else {
        let tiled_height = height.div_ceil(edge) * edge;
        let tiled_width = (f64::from(tiled_height) * aspect_ratio) as u32;
        (tiled_height, tiled_width.div_ceil(edge) * edge)
    };
    let tiled_height = tiled_height.max(edge);
    let tiled_width = tiled_width.max(edge);
    let rows = if tiled_height > edge {
        tiled_height.div_ceil(edge)
    } else {
        0
    };
    let cols = if tiled_width > edge {
        tiled_width.div_ceil(edge)
    } else {
        0
    };
    TileGeometry {
        resize: (height, width),
        tiled: (tiled_height, tiled_width),
        rows,
        cols,
    }
}

fn resize_exact(image: &image::RgbImage, edge: u32, filter: FilterType) -> image::RgbImage {
    image::imageops::resize(image, edge, edge, filter)
}

fn resize_to(image: &image::RgbImage, width: u32, height: u32, filter: FilterType) -> image::RgbImage {
    image::imageops::resize(image, width, height, filter)
}

/// Splits a resized image into `rows * cols` tiles in row-major order, exactly as the pinned
/// `split_image` does: `ceil(height / rows)` by `ceil(width / cols)` crops, each clipped to the
/// image. The tiles are returned first; the caller appends the global frame last.
fn split_tiles(image: &image::RgbImage, rows: u32, cols: u32) -> Vec<image::RgbImage> {
    let (width, height) = image.dimensions();
    let optimal_height = height.div_ceil(rows);
    let optimal_width = width.div_ceil(cols);
    let mut frames = Vec::with_capacity((rows * cols) as usize);
    for row in 0..rows {
        for col in 0..cols {
            let left = col * optimal_width;
            let top = row * optimal_height;
            let right = (left + optimal_width).min(width);
            let bottom = (top + optimal_height).min(height);
            frames.push(image::imageops::crop_imm(image, left, top, right - left, bottom - top).to_image());
        }
    }
    frames
}

pub(super) fn decode_png_or_common(bytes: &[u8], max_pixels: u32) -> crate::Result<image::RgbImage> {
    crate::extraction::image_decode::decode_standard_rgb8_within_pixels(bytes, max_pixels)
        .map_err(|error| invalid(format!("Local captioning image decode: {error}")))
}

pub(super) fn preprocess(
    config: &LocalCaptioningConfig,
    format: LocalCaptioningFormat,
    preprocess: &PreprocessConfig,
    image: &image::RgbImage,
) -> crate::Result<ImageBatch> {
    if u64::from(image.width()) * u64::from(image.height()) > u64::from(config.limits.max_pixels) {
        return Err(invalid("Local captioning image exceeds the configured pixel limit"));
    }
    let mut frames = Vec::new();
    let (rows, cols) = match format {
        // The pinned Idefics3 processor splits a large image into `max_image_size` tiles and adds
        // a squashed global frame last, which is the order the vision encoder and prompt expect.
        LocalCaptioningFormat::SmolVlm if preprocess.do_image_splitting => {
            let geometry = tile_geometry(image.width(), image.height(), preprocess.edge, preprocess.resize_edge);
            if geometry.frames() > MAX_TILES + 1 {
                return Err(invalid(format!(
                    "Local captioning image needs {} tiles; the adapter allows {MAX_TILES}",
                    geometry.frames() - 1
                )));
            }
            let resized = resize_to(image, geometry.resize.1, geometry.resize.0, preprocess.filter);
            let tiled = resize_to(&resized, geometry.tiled.1, geometry.tiled.0, preprocess.filter);
            if geometry.rows == 0 || geometry.cols == 0 {
                frames.push(resize_exact(&tiled, preprocess.edge, preprocess.filter));
                (0, 0)
            } else {
                frames.extend(split_tiles(&tiled, geometry.rows, geometry.cols));
                frames.push(resize_exact(&tiled, preprocess.edge, preprocess.filter));
                (geometry.rows as usize, geometry.cols as usize)
            }
        }
        // One global square frame, matching the processor's no-splitting path.
        LocalCaptioningFormat::SmolVlm | LocalCaptioningFormat::Florence2 => {
            frames.push(resize_exact(image, preprocess.edge, preprocess.filter));
            (0, 0)
        }
    };
    let total_tokens = u64::from(preprocess.image_seq_len) * frames.len() as u64;
    if total_tokens > u64::from(MAX_TOTAL_IMAGE_TOKENS) {
        return Err(invalid(format!(
            "Local captioning prompt expands to {total_tokens} image tokens; the adapter allows {MAX_TOTAL_IMAGE_TOKENS}"
        )));
    }
    let num_images = frames.len();
    let mut pixels = Vec::with_capacity(num_images * 3 * (preprocess.edge as usize).pow(2));
    for frame in &frames {
        pixels.extend(preprocess.normalize(frame));
    }
    Ok(ImageBatch {
        pixels,
        num_images,
        edge: preprocess.edge,
        rows,
        cols,
    })
}
