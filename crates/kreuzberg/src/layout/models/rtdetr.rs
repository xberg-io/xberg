use std::time::Instant;

use image::RgbImage;
use ndarray::{Array, Array2, Array4, s};
use ort::{inputs, session::Session, value::Tensor};

use crate::layout::error::LayoutError;
use crate::layout::models::LayoutModel;
use crate::layout::preprocessing;
use crate::layout::types::{BBox, LayoutClass, LayoutDetection};

/// Default confidence threshold for RT-DETR detections.
const DEFAULT_THRESHOLD: f32 = 0.3;

/// RT-DETR input resolution.
const INPUT_SIZE: u32 = 640;

/// Docling RT-DETR v2 layout detection model.
///
/// This model is NMS-free (transformer-based end-to-end detection).
///
/// Input tensors:
///   - `images`:            f32 [batch, 3, 640, 640]  (preprocessed pixel data)
///   - `orig_target_sizes`: i64 [batch, 2]            ([height, width] of original image)
///
/// Output tensors:
///   - `labels`: i64 [batch, num_queries]   (class IDs, 0-16)
///   - `boxes`:  f32 [batch, num_queries, 4] (bounding boxes in original image coordinates)
///   - `scores`: f32 [batch, num_queries]   (confidence scores)
pub struct RtDetrModel {
    session: Session,
    input_names: Vec<String>,
}

impl RtDetrModel {
    /// Load a Docling RT-DETR ONNX model from a file.
    pub fn from_file(path: &str) -> Result<Self, LayoutError> {
        let session = crate::layout::session::build_session(path)?;
        let input_names: Vec<String> = session.inputs().iter().map(|i| i.name().to_string()).collect();
        Ok(Self { session, input_names })
    }

    /// Run inference and extract detections from raw outputs.
    ///
    /// Uses aspect-preserving letterbox preprocessing (Lanczos3) to avoid
    /// distorting the page geometry. The model sees a properly proportioned
    /// image, which produces more accurate bounding box coordinates.
    fn run_inference(&mut self, img: &RgbImage, threshold: f32) -> Result<Vec<LayoutDetection>, LayoutError> {
        let orig_width = img.width();
        let orig_height = img.height();

        // --- Preprocessing timing ---
        let preprocess_start = Instant::now();

        // Letterbox preprocessing: resize preserving aspect ratio, pad to 640×640.
        let (input_tensor, scale, pad_x, pad_y) = preprocessing::preprocess_imagenet_letterbox(img, INPUT_SIZE);
        let images_tensor = Tensor::from_array(input_tensor)?;

        // Tell the model the "original" size is 640×640 (the letterboxed size).
        // The model maps output boxes to this coordinate space; we un-letterbox below.
        let sizes = Array::from_shape_vec((1, 2), vec![INPUT_SIZE as i64, INPUT_SIZE as i64])
            .map_err(|e| LayoutError::InvalidOutput(format!("Failed to create sizes tensor: {e}")))?;
        let sizes_tensor = Tensor::from_array(sizes)?;

        let preprocess_ms = preprocess_start.elapsed().as_secs_f64() * 1000.0;
        tracing::debug!(preprocess_ms, "RT-DETR preprocessing complete");

        // --- ONNX inference timing ---
        let onnx_start = Instant::now();

        let outputs = self.session.run(inputs![
            self.input_names[0].as_str() => images_tensor,
            self.input_names[1].as_str() => sizes_tensor
        ])?;

        let onnx_ms = onnx_start.elapsed().as_secs_f64() * 1000.0;
        tracing::debug!(onnx_ms, "RT-DETR ONNX session.run() complete");

        // Extract output tensors: try i64 labels first, then f32 boxes/scores.
        let mut float_data: Vec<Vec<f32>> = Vec::new();
        let mut float_shapes: Vec<Vec<usize>> = Vec::new();
        let mut label_data: Vec<i64> = Vec::new();

        for (_name, value) in outputs.iter() {
            if let Ok(view) = value.try_extract_tensor::<i64>() {
                label_data = view.1.to_vec();
                continue;
            }
            if let Ok(view) = value.try_extract_tensor::<f32>() {
                let shape: Vec<usize> = view.0.iter().map(|&d| d as usize).collect();
                let data: Vec<f32> = view.1.to_vec();
                float_shapes.push(shape);
                float_data.push(data);
            }
        }

        // If labels came as f32 instead of i64, convert the last float output.
        if label_data.is_empty() && float_data.len() >= 3 {
            label_data = float_data.last().unwrap().iter().map(|&v| v as i64).collect();
            float_data.pop();
            float_shapes.pop();
        }

        if float_data.len() < 2 {
            return Err(LayoutError::InvalidOutput(format!(
                "Expected at least 2 float output tensors, got {}",
                float_data.len()
            )));
        }

        let boxes = &float_data[0];
        let scores = &float_data[1];
        let box_shape = &float_shapes[0];
        let num_detections = if box_shape.len() == 3 {
            box_shape[1]
        } else {
            box_shape[0]
        };

        // Un-letterbox: map from padded 640×640 space → original image coordinates.
        let inv_scale = 1.0 / scale;
        let pad_x_f = pad_x as f32;
        let pad_y_f = pad_y as f32;

        let mut detections = Vec::new();
        for i in 0..num_detections {
            let score = scores[i];
            if score < threshold {
                continue;
            }

            let label_id = label_data[i];
            let class = match LayoutClass::from_docling_id(label_id) {
                Some(c) => c,
                None => continue,
            };

            // Boxes are in 640×640 letterboxed coordinates. Remove padding and rescale.
            let x1 = ((boxes[i * 4] - pad_x_f) * inv_scale).clamp(0.0, orig_width as f32);
            let y1 = ((boxes[i * 4 + 1] - pad_y_f) * inv_scale).clamp(0.0, orig_height as f32);
            let x2 = ((boxes[i * 4 + 2] - pad_x_f) * inv_scale).clamp(0.0, orig_width as f32);
            let y2 = ((boxes[i * 4 + 3] - pad_y_f) * inv_scale).clamp(0.0, orig_height as f32);

            detections.push(LayoutDetection::new(class, score, BBox::new(x1, y1, x2, y2)));
        }

        LayoutDetection::sort_by_confidence_desc(&mut detections);

        // Publish granular timings via the thread-local side-channel so that
        // LayoutEngine::detect_timed() can populate PageTiming without changing
        // the LayoutModel trait signature.
        crate::layout::inference_timings::set(preprocess_ms, onnx_ms);

        tracing::debug!(
            preprocess_ms,
            onnx_ms,
            detections = detections.len(),
            "RT-DETR inference breakdown"
        );

        Ok(detections)
    }

    /// Run batched inference over multiple images in a single ONNX call.
    ///
    /// Stacks per-image tensors into `[N, 3, 640, 640]` and `[N, 2]` inputs,
    /// executes a single `session.run()`, then splits outputs by batch index.
    ///
    /// Returns one `Vec<LayoutDetection>` per input image, in the same order.
    pub(crate) fn run_batch_inference(
        &mut self,
        images: &[&RgbImage],
        threshold: f32,
    ) -> Result<Vec<Vec<LayoutDetection>>, LayoutError> {
        let batch = images.len();
        assert!(!images.is_empty(), "run_batch_inference called with empty slice");

        let ts = INPUT_SIZE as usize;
        let hw = ts * ts;

        // --- Preprocessing ---
        let preprocess_start = Instant::now();

        // Preprocess every image and collect per-image metadata needed for un-letterboxing.
        let mut all_pixel_data: Vec<f32> = Vec::with_capacity(batch * 3 * hw);
        let mut metas: Vec<(u32, u32, f32, u32, u32)> = Vec::with_capacity(batch); // (orig_w, orig_h, scale, pad_x, pad_y)

        for img in images {
            let (tensor, scale, pad_x, pad_y) = preprocessing::preprocess_imagenet_letterbox(img, INPUT_SIZE);
            // tensor shape is [1, 3, ts, ts]; extract flat data
            all_pixel_data.extend_from_slice(tensor.as_slice().expect("tensor not contiguous"));
            metas.push((img.width(), img.height(), scale, pad_x, pad_y));
        }

        // Build batched [N, 3, 640, 640] images tensor.
        let images_array = Array4::from_shape_vec((batch, 3, ts, ts), all_pixel_data)
            .map_err(|e| LayoutError::InvalidOutput(format!("Failed to build batch images tensor: {e}")))?;
        let images_tensor = Tensor::from_array(images_array)?;

        // Build [N, 2] orig_target_sizes tensor — each row is [640, 640].
        let sizes_flat: Vec<i64> = std::iter::repeat([INPUT_SIZE as i64, INPUT_SIZE as i64])
            .take(batch)
            .flatten()
            .collect();
        let sizes_array = Array2::from_shape_vec((batch, 2), sizes_flat)
            .map_err(|e| LayoutError::InvalidOutput(format!("Failed to build batch sizes tensor: {e}")))?;
        let sizes_tensor = Tensor::from_array(sizes_array)?;

        let preprocess_ms = preprocess_start.elapsed().as_secs_f64() * 1000.0;
        tracing::debug!(preprocess_ms, batch, "RT-DETR batch preprocessing complete");

        // --- ONNX inference (single call for the whole batch) ---
        let onnx_start = Instant::now();

        let outputs = self.session.run(inputs![
            self.input_names[0].as_str() => images_tensor,
            self.input_names[1].as_str() => sizes_tensor
        ])?;

        let onnx_ms = onnx_start.elapsed().as_secs_f64() * 1000.0;
        tracing::debug!(onnx_ms, batch, "RT-DETR batch ONNX session.run() complete");

        // --- Output parsing ---
        // Same tensor layout as single inference, but the leading dimension is N.
        let mut float_data: Vec<Vec<f32>> = Vec::new();
        let mut float_shapes: Vec<Vec<usize>> = Vec::new();
        let mut label_data: Vec<i64> = Vec::new();

        for (_name, value) in outputs.iter() {
            if let Ok(view) = value.try_extract_tensor::<i64>() {
                label_data = view.1.to_vec();
                continue;
            }
            if let Ok(view) = value.try_extract_tensor::<f32>() {
                let shape: Vec<usize> = view.0.iter().map(|&d| d as usize).collect();
                let data: Vec<f32> = view.1.to_vec();
                float_shapes.push(shape);
                float_data.push(data);
            }
        }

        if label_data.is_empty() && float_data.len() >= 3 {
            label_data = float_data.last().unwrap().iter().map(|&v| v as i64).collect();
            float_data.pop();
            float_shapes.pop();
        }

        if float_data.len() < 2 {
            return Err(LayoutError::InvalidOutput(format!(
                "Expected at least 2 float output tensors, got {}",
                float_data.len()
            )));
        }

        let boxes = &float_data[0]; // [N * num_queries * 4]
        let scores = &float_data[1]; // [N * num_queries]
        let box_shape = &float_shapes[0];

        // box_shape is [N, num_queries, 4]; resolve num_queries.
        let num_queries = if box_shape.len() == 3 {
            box_shape[1]
        } else {
            box_shape[0]
        };

        // Publish timings via side-channel (amortized preprocess per batch).
        crate::layout::inference_timings::set(preprocess_ms / batch as f64, onnx_ms / batch as f64);

        // --- Split outputs by batch index ---
        let mut results: Vec<Vec<LayoutDetection>> = Vec::with_capacity(batch);

        for b in 0..batch {
            let (orig_width, orig_height, scale, pad_x, pad_y) = metas[b];
            let inv_scale = 1.0 / scale;
            let pad_x_f = pad_x as f32;
            let pad_y_f = pad_y as f32;

            let mut detections = Vec::new();
            for i in 0..num_queries {
                let flat_i = b * num_queries + i;
                let score = scores[flat_i];
                if score < threshold {
                    continue;
                }

                let label_id = label_data[flat_i];
                let class = match LayoutClass::from_docling_id(label_id) {
                    Some(c) => c,
                    None => continue,
                };

                let box_base = flat_i * 4;
                let x1 = ((boxes[box_base] - pad_x_f) * inv_scale).clamp(0.0, orig_width as f32);
                let y1 = ((boxes[box_base + 1] - pad_y_f) * inv_scale).clamp(0.0, orig_height as f32);
                let x2 = ((boxes[box_base + 2] - pad_x_f) * inv_scale).clamp(0.0, orig_width as f32);
                let y2 = ((boxes[box_base + 3] - pad_y_f) * inv_scale).clamp(0.0, orig_height as f32);

                detections.push(LayoutDetection::new(class, score, BBox::new(x1, y1, x2, y2)));
            }

            LayoutDetection::sort_by_confidence_desc(&mut detections);

            tracing::debug!(
                batch_index = b,
                detections = detections.len(),
                "RT-DETR batch inference: per-image detections"
            );

            results.push(detections);
        }

        tracing::debug!(preprocess_ms, onnx_ms, batch, "RT-DETR batch inference breakdown");

        Ok(results)
    }
}

impl LayoutModel for RtDetrModel {
    fn detect(&mut self, img: &RgbImage) -> Result<Vec<LayoutDetection>, LayoutError> {
        self.run_inference(img, DEFAULT_THRESHOLD)
    }

    fn detect_with_threshold(&mut self, img: &RgbImage, threshold: f32) -> Result<Vec<LayoutDetection>, LayoutError> {
        self.run_inference(img, threshold)
    }

    fn detect_batch(
        &mut self,
        images: &[&RgbImage],
        threshold: Option<f32>,
    ) -> Result<Vec<Vec<LayoutDetection>>, LayoutError> {
        if images.is_empty() {
            return Ok(Vec::new());
        }
        let t = threshold.unwrap_or(DEFAULT_THRESHOLD);
        // Single-image case: use the regular inference path (no tensor stacking overhead).
        if images.len() == 1 {
            return self.run_inference(images[0], t).map(|d| vec![d]);
        }
        self.run_batch_inference(images, t)
    }

    fn name(&self) -> &str {
        "Docling RT-DETR v2"
    }
}
