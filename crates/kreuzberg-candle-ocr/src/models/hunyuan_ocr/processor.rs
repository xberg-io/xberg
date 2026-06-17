// Vendored from jhqxxx/aha (Apache-2.0). See repo-root ATTRIBUTIONS.md § jhqxxx/aha.

//! Image preprocessing for Hunyuan-VL.

use candle_core::{DType, Device, IndexOp, Shape, Tensor};
use image::DynamicImage;

use crate::error::{CandleOcrError, Result};
use crate::models::hunyuan_ocr::config::HunyuanOCRPreprocessorConfig;
use crate::vendor::aha::image::{img_smart_resize, img_transform};

/// Processed multimodal data: input IDs, position IDs, image embeddings, and masks.
pub struct HunyuanData {
    /// Token IDs for the text input.
    pub input_ids: Tensor,
    /// Position IDs for XD-RoPE (4D: t, h, w, and text positions).
    pub position_ids: Tensor,
    /// Mask indicating which positions correspond to image tokens.
    pub image_mask: Tensor,
    /// Pixel values (visual features) if images are present.
    pub pixel_values: Option<Tensor>,
    /// Image grid dimensions (T, H, W) per image.
    pub image_grid_thw: Option<Tensor>,
}

/// Processor for Hunyuan-OCR: handles image and text preprocessing.
pub struct HunyuanVLProcessor {
    image_token_id: u32,
    image_token: String,
    placeholder_token: String,
    pub process_cfg: HunyuanOCRPreprocessorConfig,
    device: Device,
    dtype: DType,
}

impl HunyuanVLProcessor {
    /// Create a new Hunyuan-VL processor.
    ///
    /// # Arguments
    /// * `path` - Path to the model directory containing `preprocessor_config.json`
    /// * `device` - Candle device (CPU or GPU)
    /// * `dtype` - Data type for tensor operations
    pub fn new(path: &str, device: &Device, dtype: DType) -> Result<Self> {
        let path = path.to_string();

        if !std::path::Path::new(&path).exists() {
            return Err(CandleOcrError::ModelLoadFailed(format!(
                "Model path does not exist: {}",
                path
            )));
        }

        let process_cfg_file = format!("{}/preprocessor_config.json", path);

        if !std::path::Path::new(&process_cfg_file).exists() {
            return Err(CandleOcrError::ModelLoadFailed(
                "preprocessor_config.json not found in model path".to_string(),
            ));
        }

        let config_bytes = std::fs::read(&process_cfg_file)?;

        let process_cfg: HunyuanOCRPreprocessorConfig = serde_json::from_slice(&config_bytes)
            .map_err(|e| CandleOcrError::ModelLoadFailed(format!("Parse preprocessor_config.json: {}", e)))?;

        let image_token_id = 120120u32;
        let image_token = "<｜hy_place▁holder▁no▁102｜>".to_string();
        let placeholder_token = "<｜hy_place▁holder▁no▁799｜>".to_string();

        Ok(Self {
            image_token_id,
            image_token,
            placeholder_token,
            process_cfg,
            device: device.clone(),
            dtype,
        })
    }

    /// Preprocess a single image: resize, normalize, and return tensor.
    pub fn process_img(&self, img: &DynamicImage, img_mean: &Tensor, img_std: &Tensor) -> Result<Tensor> {
        let img_h = img.height();
        let img_w = img.width();

        let (resize_h, resize_w) = img_smart_resize(
            img_h,
            img_w,
            (self.process_cfg.patch_size * self.process_cfg.merge_size) as u32,
            self.process_cfg.min_pixels as u32,
            self.process_cfg.max_pixels as u32,
        )
        .map_err(|e| CandleOcrError::InferenceFailed(format!("Smart resize: {}", e)))?;

        let img = img.resize_exact(resize_w, resize_h, image::imageops::FilterType::CatmullRom);

        let img_tensor = img_transform(&img, img_mean, img_std, &self.device, self.dtype)
            .map_err(|e| CandleOcrError::InferenceFailed(format!("Transform: {}", e)))?;

        let img_tensor = img_tensor
            .unsqueeze(0)
            .map_err(|e| CandleOcrError::InferenceFailed(format!("Unsqueeze: {}", e)))?;

        Ok(img_tensor)
    }

    /// Process a single image tensor into patches and grid information.
    pub fn process_vision_tensor(&self, img_tensor: &Tensor) -> Result<(Tensor, Tensor)> {
        let channel = img_tensor
            .dim(1)
            .map_err(|e| CandleOcrError::InferenceFailed(format!("Dim 1: {}", e)))?;

        let grid_t = img_tensor
            .dim(0)
            .map_err(|e| CandleOcrError::InferenceFailed(format!("Dim 0: {}", e)))?
            / self.process_cfg.temporal_patch_size;

        let grid_h = img_tensor
            .dim(2)
            .map_err(|e| CandleOcrError::InferenceFailed(format!("Dim 2: {}", e)))?
            / self.process_cfg.patch_size;

        let grid_w = img_tensor
            .dim(3)
            .map_err(|e| CandleOcrError::InferenceFailed(format!("Dim 3: {}", e)))?
            / self.process_cfg.patch_size;

        let shape = Shape::from(vec![
            grid_t,
            channel,
            grid_h / self.process_cfg.merge_size,
            self.process_cfg.merge_size,
            self.process_cfg.patch_size,
            grid_w / self.process_cfg.merge_size,
            self.process_cfg.merge_size,
            self.process_cfg.patch_size,
        ]);

        let img_tensor = img_tensor
            .reshape(shape)
            .map_err(|e| CandleOcrError::InferenceFailed(format!("Reshape: {}", e)))?;

        let img_tensor = img_tensor
            .permute(vec![0, 2, 3, 5, 6, 1, 4, 7])
            .map_err(|e| CandleOcrError::InferenceFailed(format!("Permute: {}", e)))?;

        let img_tensor = img_tensor
            .reshape((
                grid_t * grid_h * grid_w,
                channel * self.process_cfg.patch_size * self.process_cfg.patch_size,
            ))
            .map_err(|e| CandleOcrError::InferenceFailed(format!("Reshape flat: {}", e)))?
            .contiguous()
            .map_err(|e| CandleOcrError::InferenceFailed(format!("Contiguous: {}", e)))?;

        let grid_thw = Tensor::from_vec(vec![grid_t as u32, grid_h as u32, grid_w as u32], (1, 3), &self.device)
            .map_err(|e| CandleOcrError::InferenceFailed(format!("Grid tensor: {}", e)))?;

        Ok((img_tensor, grid_thw))
    }

    /// Preprocess multiple images and combine into batch tensors.
    pub fn process_images(
        &self,
        imgs: &[DynamicImage],
        img_mean: &Tensor,
        img_std: &Tensor,
    ) -> Result<(Tensor, Tensor)> {
        let mut pixel_values_vec = Vec::new();
        let mut vision_grid_thws_vec = Vec::new();

        for img in imgs {
            let img_tensor = self.process_img(img, img_mean, img_std)?;
            let (img_tensor, grid_thw) = self.process_vision_tensor(&img_tensor)?;
            pixel_values_vec.push(img_tensor);
            vision_grid_thws_vec.push(grid_thw);
        }

        let pixel_values = Tensor::cat(&pixel_values_vec, 0)
            .map_err(|e| CandleOcrError::InferenceFailed(format!("Cat pixels: {}", e)))?;

        let vision_grid_thws = Tensor::cat(&vision_grid_thws_vec, 0)
            .map_err(|e| CandleOcrError::InferenceFailed(format!("Cat grids: {}", e)))?;

        Ok((pixel_values, vision_grid_thws))
    }

    /// Process images and text into multimodal data.
    ///
    /// This function:
    /// 1. Preprocesses images into pixel values and grid info
    /// 2. Tokenizes text with image placeholders
    /// 3. Builds position IDs for XD-RoPE
    /// 4. Creates image mask
    ///
    /// Note: Tokenization is not performed here; callers must provide pre-tokenized `input_ids`.
    pub fn process_images_and_text(&self, imgs: &[DynamicImage], input_ids: Tensor, text: &str) -> Result<HunyuanData> {
        let img_mean = Tensor::from_slice(&self.process_cfg.image_mean, (3, 1, 1), &self.device)
            .map_err(|e| CandleOcrError::InferenceFailed(format!("Mean tensor: {}", e)))?
            .to_dtype(self.dtype)
            .map_err(|e| CandleOcrError::InferenceFailed(format!("Mean dtype: {}", e)))?;

        let img_std = Tensor::from_slice(&self.process_cfg.image_std, (3, 1, 1), &self.device)
            .map_err(|e| CandleOcrError::InferenceFailed(format!("Std tensor: {}", e)))?
            .to_dtype(self.dtype)
            .map_err(|e| CandleOcrError::InferenceFailed(format!("Std dtype: {}", e)))?;

        let (pixel_values, image_grid_thw) = if !imgs.is_empty() {
            let (pixel_values, image_grid_thw) = self.process_images(imgs, &img_mean, &img_std)?;
            (Some(pixel_values), Some(image_grid_thw))
        } else {
            (None, None)
        };

        let mut image_tokens_cumsum = vec![0];
        let mut text = text.to_string();

        if !imgs.is_empty()
            && let Some(grid_thw) = image_grid_thw.as_ref()
        {
            let mut index = 0;
            while text.contains(&self.image_token) {
                let grid_i = grid_thw
                    .i(index)
                    .map_err(|e| CandleOcrError::InferenceFailed(format!("Grid index: {}", e)))?;

                let grid_h = grid_i
                    .i(1)
                    .map_err(|e| CandleOcrError::InferenceFailed(format!("Grid h: {}", e)))?
                    .to_scalar::<u32>()
                    .map_err(|e| CandleOcrError::InferenceFailed(format!("H scalar: {}", e)))?;

                let grid_w = grid_i
                    .i(2)
                    .map_err(|e| CandleOcrError::InferenceFailed(format!("Grid w: {}", e)))?
                    .to_scalar::<u32>()
                    .map_err(|e| CandleOcrError::InferenceFailed(format!("W scalar: {}", e)))?;

                let patch_h = grid_h / self.process_cfg.merge_size as u32;
                let patch_w = grid_w / self.process_cfg.merge_size as u32;
                let num_image_tokens = patch_h * (patch_w + 1) + 2;
                let num_id = image_tokens_cumsum[image_tokens_cumsum.len() - 1] + num_image_tokens;
                image_tokens_cumsum.push(num_id);

                let replace = self.placeholder_token.repeat(num_image_tokens as usize);
                text = text.replacen(&self.image_token, &replace, 1);
                index += 1;
            }
        }

        let _text = text.replace(&self.placeholder_token, &self.image_token);

        let seq_len = input_ids
            .dim(1)
            .map_err(|e| CandleOcrError::InferenceFailed(format!("Seq len: {}", e)))?;

        let position_ids = Tensor::arrange(0, seq_len as u32, &self.device)
            .map_err(|e| CandleOcrError::InferenceFailed(format!("Position IDs: {}", e)))?;

        let mut position_ids_w = Tensor::arrange(0, seq_len as u32, &self.device)
            .map_err(|e| CandleOcrError::InferenceFailed(format!("Position IDs W: {}", e)))?;

        let mut position_ids_h = Tensor::arrange(0, seq_len as u32, &self.device)
            .map_err(|e| CandleOcrError::InferenceFailed(format!("Position IDs H: {}", e)))?;

        let mut position_ids_t = Tensor::arrange(0, seq_len as u32, &self.device)
            .map_err(|e| CandleOcrError::InferenceFailed(format!("Position IDs T: {}", e)))?;

        if !imgs.is_empty()
            && let Some(grid_thw) = image_grid_thw.as_ref()
        {
            let image_token_pos_indices = get_eq_indices(
                &input_ids
                    .i(0)
                    .map_err(|e| CandleOcrError::InferenceFailed(format!("Input IDs [0]: {}", e)))?,
                self.image_token_id,
            )
            .map_err(|e| CandleOcrError::InferenceFailed(format!("Get indices: {}", e)))?;

            #[allow(clippy::needless_range_loop)]
            for i in 0..grid_thw
                .dim(0)
                .map_err(|e| CandleOcrError::InferenceFailed(format!("Grid dim: {}", e)))?
            {
                let grid_i = grid_thw
                    .i(i)
                    .map_err(|e| CandleOcrError::InferenceFailed(format!("Grid [{}]: {}", i, e)))?;

                let grid_h = grid_i
                    .i(1)
                    .map_err(|e| CandleOcrError::InferenceFailed(format!("Grid h: {}", e)))?
                    .to_scalar::<u32>()
                    .map_err(|e| CandleOcrError::InferenceFailed(format!("H scalar: {}", e)))?;

                let grid_w = grid_i
                    .i(2)
                    .map_err(|e| CandleOcrError::InferenceFailed(format!("Grid w: {}", e)))?
                    .to_scalar::<u32>()
                    .map_err(|e| CandleOcrError::InferenceFailed(format!("W scalar: {}", e)))?;

                let patch_h = grid_h / self.process_cfg.merge_size as u32;
                let patch_w = grid_w / self.process_cfg.merge_size as u32;

                let start_pos = image_token_pos_indices
                    .i(image_tokens_cumsum[i] as usize)
                    .map_err(|e| CandleOcrError::InferenceFailed(format!("Indices [{}]: {}", i, e)))?
                    .to_scalar::<u32>()
                    .map_err(|e| CandleOcrError::InferenceFailed(format!("Start pos scalar: {}", e)))?
                    as usize
                    + 1;

                let replace_num = ((patch_w + 1) * patch_h) as usize;

                #[allow(clippy::single_range_in_vec_init)]
                let pos_w: Vec<u32> = (0..patch_h).flat_map(|_| 0u32..patch_w + 1).collect();
                #[allow(clippy::single_range_in_vec_init)]
                let range = start_pos..start_pos + replace_num;
                position_ids_w = position_ids_w
                    .slice_assign(
                        &[range][..],
                        &Tensor::new(pos_w, &self.device)
                            .map_err(|e| CandleOcrError::InferenceFailed(format!("Create pos_w: {}", e)))?,
                    )
                    .map_err(|e| CandleOcrError::InferenceFailed(format!("Assign pos_w: {}", e)))?;

                let pos_h: Vec<u32> = (0..patch_h).flat_map(|h| vec![h; (patch_w + 1) as usize]).collect();
                #[allow(clippy::single_range_in_vec_init)]
                let range_h = start_pos..start_pos + replace_num;
                position_ids_h = position_ids_h
                    .slice_assign(
                        &[range_h][..],
                        &Tensor::new(pos_h, &self.device)
                            .map_err(|e| CandleOcrError::InferenceFailed(format!("Create pos_h: {}", e)))?,
                    )
                    .map_err(|e| CandleOcrError::InferenceFailed(format!("Assign pos_h: {}", e)))?;

                #[allow(clippy::single_range_in_vec_init)]
                let range_t = start_pos..start_pos + replace_num;
                position_ids_t = position_ids_t
                    .slice_assign(
                        &[range_t][..],
                        &Tensor::new(vec![0u32; replace_num], &self.device)
                            .map_err(|e| CandleOcrError::InferenceFailed(format!("Create pos_t: {}", e)))?,
                    )
                    .map_err(|e| CandleOcrError::InferenceFailed(format!("Assign pos_t: {}", e)))?;
            }
        }

        let position_ids = Tensor::stack(&[position_ids, position_ids_h, position_ids_w, position_ids_t], 0)
            .map_err(|e| CandleOcrError::InferenceFailed(format!("Stack positions: {}", e)))?
            .unsqueeze(0)
            .map_err(|e| CandleOcrError::InferenceFailed(format!("Unsqueeze positions: {}", e)))?;

        let image_mask = get_equal_mask(&input_ids, self.image_token_id)
            .map_err(|e| CandleOcrError::InferenceFailed(format!("Image mask: {}", e)))?;

        let data = HunyuanData {
            input_ids,
            position_ids,
            image_mask,
            pixel_values,
            image_grid_thw,
        };

        Ok(data)
    }
}

// ---------------------------------------------------------------------------
// Internal tensor helpers
// ---------------------------------------------------------------------------

/// Find indices where tensor equals a target value.
fn get_eq_indices(tensor: &Tensor, target: u32) -> Result<Tensor> {
    let vec = tensor
        .to_vec1::<u32>()
        .map_err(|e| CandleOcrError::InferenceFailed(format!("To vec: {}", e)))?;

    let indices: Vec<u32> = vec
        .iter()
        .enumerate()
        .filter_map(|(i, &v)| if v == target { Some(i as u32) } else { None })
        .collect();

    let len = indices.len();
    Tensor::from_vec(indices, (len,), tensor.device())
        .map_err(|e| CandleOcrError::InferenceFailed(format!("Create indices: {}", e)))
}

/// Create a binary mask where positions equal to target are 1, others are 0.
fn get_equal_mask(tensor: &Tensor, target: u32) -> Result<Tensor> {
    let vec = tensor
        .to_vec1::<u32>()
        .map_err(|e| CandleOcrError::InferenceFailed(format!("To vec: {}", e)))?;

    let mask: Vec<u32> = vec.iter().map(|&v| if v == target { 1 } else { 0 }).collect();

    Tensor::from_vec(mask, (vec.len(),), tensor.device())
        .map_err(|e| CandleOcrError::InferenceFailed(format!("Create mask: {}", e)))
}
