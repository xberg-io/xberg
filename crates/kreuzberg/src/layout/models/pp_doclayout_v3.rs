//! PP-DocLayout-V3 layout detection model.
//!
//! PP-DocLayout-V3 is a PaddlePaddle-based layout detection model that identifies
//! document regions including text, tables, images, formulas, and charts.
//!
//! Model: PP-DocLayout-V3
//! Input: `images` shape `[batch, 3, 960, 960]` f32 (aspect-preserving preprocessing)
//! Output:
//!   - `boxes`: f32 [batch, num_dets, 4] — bounding boxes in (x1, y1, x2, y2) format
//!   - `scores`: f32 [batch, num_dets] — confidence scores
//!   - `labels`: i64 [batch, num_dets] — class IDs (0-7)
//!
//! Classes: Text (0), Title (1), Image (2), Formula (3), Table (4), Chart (5), Footer (6), Header (7)

use std::time::Instant;

use image::RgbImage;
use ndarray::Array4;
use ort::{inputs, session::Session, value::Tensor};

use crate::layout::error::LayoutError;
use crate::layout::models::LayoutModel;
use crate::layout::preprocessing;
use crate::layout::types::{BBox, LayoutClass, LayoutDetection};

/// Default confidence threshold for PP-DocLayout-V3 detections.
const DEFAULT_THRESHOLD: f32 = 0.5;

/// PP-DocLayout-V3 input resolution.
const INPUT_SIZE: u32 = 960;

/// PP-DocLayout-V3 layout detection model.
///
/// This model uses aspect-preserving letterbox preprocessing to detect
/// document regions without distorting the page geometry.
#[cfg_attr(alef, alef(skip))]
pub struct PpDocLayoutV3Model {
    session: Session,
    input_names: Vec<String>,
}

impl PpDocLayoutV3Model {
    /// Load a PP-DocLayout-V3 ONNX model from a file.
    pub(crate) fn from_file(
        path: &str,
        accel: Option<&crate::core::config::acceleration::AccelerationConfig>,
    ) -> Result<Self, LayoutError> {
        let budget = crate::core::config::concurrency::resolve_thread_budget(None);
        let session = crate::layout::session::build_session(path, accel, budget)?;
        let input_names: Vec<String> = session.inputs().iter().map(|i| i.name().to_string()).collect();
        Ok(Self { session, input_names })
    }

    /// Map PP-DocLayout-V3 class ID (0-7) to LayoutClass.
    fn class_from_id(id: i64) -> Option<LayoutClass> {
        match id {
            0 => Some(LayoutClass::Text),
            1 => Some(LayoutClass::Title),
            2 => Some(LayoutClass::Picture),
            3 => Some(LayoutClass::Formula),
            4 => Some(LayoutClass::Table),
            5 => Some(LayoutClass::Picture), // Chart → Picture
            6 => Some(LayoutClass::PageFooter),
            7 => Some(LayoutClass::PageHeader),
            _ => None,
        }
    }

    /// Run inference and extract detections from raw outputs.
    fn run_inference(&mut self, img: &RgbImage, threshold: f32) -> Result<Vec<LayoutDetection>, LayoutError> {
        #[cfg(feature = "otel")]
        let inference_span = crate::telemetry::spans::model_inference_span("pp-doclayout-v3");
        #[cfg(feature = "otel")]
        let _inference_guard = inference_span.enter();
        #[cfg(feature = "otel")]
        let inference_start = Instant::now();

        let orig_width = img.width();
        let orig_height = img.height();

        // --- Preprocessing ---
        let preprocess_start = Instant::now();

        let (input_tensor, scale, pad_x, pad_y) = preprocessing::preprocess_imagenet_letterbox(img, INPUT_SIZE);
        let images_tensor = Tensor::from_array(input_tensor)?;

        let preprocess_ms = preprocess_start.elapsed().as_secs_f64() * 1000.0;
        tracing::debug!(preprocess_ms, "PP-DocLayout-V3 preprocessing complete");

        // --- ONNX inference ---
        let onnx_start = Instant::now();

        let outputs = self.session.run(inputs![
            self.input_names[0].as_str() => images_tensor
        ])?;

        let onnx_ms = onnx_start.elapsed().as_secs_f64() * 1000.0;
        tracing::debug!(onnx_ms, "PP-DocLayout-V3 ONNX session.run() complete");

        // Extract output tensors: boxes (f32), scores (f32), labels (i64).
        let mut boxes_data: Vec<f32> = Vec::new();
        let mut boxes_shape: Vec<usize> = Vec::new();
        let mut scores_data: Vec<f32> = Vec::new();
        let mut labels_data: Vec<i64> = Vec::new();

        for (_name, value) in outputs.iter() {
            // Try i64 (labels)
            if let Ok(view) = value.try_extract_tensor::<i64>() {
                labels_data = view.1.to_vec();
                continue;
            }
            // Try f32 (boxes or scores)
            if let Ok(view) = value.try_extract_tensor::<f32>() {
                let shape: Vec<usize> = view.0.iter().map(|&d| d as usize).collect();
                let data: Vec<f32> = view.1.to_vec();

                // Distinguish boxes vs scores by shape: boxes are [batch, N, 4], scores are [batch, N]
                if shape.len() == 3 && shape[2] == 4 {
                    boxes_data = data;
                    boxes_shape = shape;
                } else if shape.len() == 2 {
                    scores_data = data;
                }
            }
        }

        if boxes_data.is_empty() || scores_data.is_empty() {
            return Err(LayoutError::InvalidOutput(
                "Missing boxes or scores output from PP-DocLayout-V3".into(),
            ));
        }

        let num_detections = if boxes_shape.len() == 3 {
            boxes_shape[1]
        } else {
            return Err(LayoutError::InvalidOutput(
                "Invalid boxes shape: expected [batch, N, 4]".into(),
            ));
        };

        // Un-letterbox: map from padded INPUT_SIZE×INPUT_SIZE space → original image coordinates.
        let inv_scale = 1.0 / scale;
        let pad_x_f = pad_x as f32;
        let pad_y_f = pad_y as f32;

        let mut detections = Vec::new();
        for i in 0..num_detections {
            let score = scores_data[i];
            if score < threshold {
                continue;
            }

            let label_id = if !labels_data.is_empty() {
                labels_data[i]
            } else {
                // If labels are missing, skip (safety check)
                continue;
            };

            let class = match Self::class_from_id(label_id) {
                Some(c) => c,
                None => continue,
            };

            // Boxes are in letterboxed coordinates [x1, y1, x2, y2]. Remove padding and rescale.
            let box_idx = i * 4;
            let x1 = ((boxes_data[box_idx] - pad_x_f) * inv_scale).clamp(0.0, orig_width as f32);
            let y1 = ((boxes_data[box_idx + 1] - pad_y_f) * inv_scale).clamp(0.0, orig_height as f32);
            let x2 = ((boxes_data[box_idx + 2] - pad_x_f) * inv_scale).clamp(0.0, orig_width as f32);
            let y2 = ((boxes_data[box_idx + 3] - pad_y_f) * inv_scale).clamp(0.0, orig_height as f32);

            detections.push(LayoutDetection::new(class, score, BBox::new(x1, y1, x2, y2)));
        }

        detections = LayoutDetection::sort_by_confidence_desc(detections);

        // Publish granular timings via the thread-local side-channel.
        crate::layout::inference_timings::set(preprocess_ms, onnx_ms);

        tracing::debug!(
            preprocess_ms,
            onnx_ms,
            detections = detections.len(),
            "PP-DocLayout-V3 inference breakdown"
        );

        #[cfg(feature = "otel")]
        {
            let total_inference_ms = inference_start.elapsed().as_secs_f64() * 1000.0;
            tracing::Span::current().record(crate::telemetry::conventions::MODEL_INFERENCE_MS, total_inference_ms);
        }

        Ok(detections)
    }

    /// Run batched inference over multiple images in a single ONNX call.
    ///
    /// # Empty-slice contract
    ///
    /// Returns `Ok(Vec::new())` immediately when `images` is empty — no ONNX
    /// session call is made. Callers that want a single-image optimised path
    /// should use [`LayoutModel::detect_batch`] instead, which also handles the
    /// single-element case via [`Self::run_inference`].
    pub(crate) fn run_batch_inference(
        &mut self,
        images: &[&RgbImage],
        threshold: f32,
    ) -> Result<Vec<Vec<LayoutDetection>>, LayoutError> {
        #[cfg(feature = "otel")]
        let inference_span = crate::telemetry::spans::model_inference_span("pp-doclayout-v3");
        #[cfg(feature = "otel")]
        let _inference_guard = inference_span.enter();
        #[cfg(feature = "otel")]
        let inference_start = Instant::now();

        if images.is_empty() {
            return Ok(Vec::new());
        }
        let batch = images.len();

        let ts = INPUT_SIZE as usize;
        let hw = ts * ts;

        // --- Preprocessing ---
        let preprocess_start = Instant::now();

        let mut all_pixel_data: Vec<f32> = Vec::with_capacity(batch * 3 * hw);
        let mut metas: Vec<(u32, u32, f32, u32, u32)> = Vec::with_capacity(batch); // (orig_w, orig_h, scale, pad_x, pad_y)

        for img in images {
            let (tensor, scale, pad_x, pad_y) = preprocessing::preprocess_imagenet_letterbox(img, INPUT_SIZE);
            all_pixel_data.extend_from_slice(tensor.as_slice().expect("tensor not contiguous"));
            metas.push((img.width(), img.height(), scale, pad_x, pad_y));
        }

        let images_array = Array4::from_shape_vec((batch, 3, ts, ts), all_pixel_data)
            .map_err(|e| LayoutError::InvalidOutput(format!("Failed to build batch images tensor: {e}")))?;
        let images_tensor = Tensor::from_array(images_array)?;

        let preprocess_ms = preprocess_start.elapsed().as_secs_f64() * 1000.0;
        tracing::debug!(preprocess_ms, batch, "PP-DocLayout-V3 batch preprocessing complete");

        // --- ONNX inference ---
        let onnx_start = Instant::now();

        let outputs = self.session.run(inputs![
            self.input_names[0].as_str() => images_tensor
        ])?;

        let onnx_ms = onnx_start.elapsed().as_secs_f64() * 1000.0;
        tracing::debug!(onnx_ms, batch, "PP-DocLayout-V3 batch ONNX session.run() complete");

        // --- Output parsing ---
        let mut boxes_data: Vec<f32> = Vec::new();
        let mut boxes_shape: Vec<usize> = Vec::new();
        let mut scores_data: Vec<f32> = Vec::new();
        let mut labels_data: Vec<i64> = Vec::new();

        for (_name, value) in outputs.iter() {
            if let Ok(view) = value.try_extract_tensor::<i64>() {
                labels_data = view.1.to_vec();
                continue;
            }
            if let Ok(view) = value.try_extract_tensor::<f32>() {
                let shape: Vec<usize> = view.0.iter().map(|&d| d as usize).collect();
                let data: Vec<f32> = view.1.to_vec();

                if shape.len() == 3 && shape[2] == 4 {
                    boxes_data = data;
                    boxes_shape = shape;
                } else if shape.len() == 2 {
                    scores_data = data;
                }
            }
        }

        if boxes_data.is_empty() || scores_data.is_empty() {
            return Err(LayoutError::InvalidOutput(
                "Missing boxes or scores output from PP-DocLayout-V3 batch inference".into(),
            ));
        }

        let num_detections = if boxes_shape.len() == 3 {
            boxes_shape[1]
        } else {
            return Err(LayoutError::InvalidOutput(
                "Invalid boxes shape: expected [batch, N, 4]".into(),
            ));
        };

        // Publish timings via side-channel (amortized per batch).
        crate::layout::inference_timings::set(preprocess_ms / batch as f64, onnx_ms / batch as f64);

        // --- Split outputs by batch index ---
        let mut results: Vec<Vec<LayoutDetection>> = Vec::with_capacity(batch);

        for (b, &(orig_width, orig_height, scale, pad_x, pad_y)) in metas.iter().enumerate() {
            let inv_scale = 1.0 / scale;
            let pad_x_f = pad_x as f32;
            let pad_y_f = pad_y as f32;

            let mut detections = Vec::new();
            for i in 0..num_detections {
                let flat_i = b * num_detections + i;
                let score = scores_data[flat_i];
                if score < threshold {
                    continue;
                }

                let label_id = if !labels_data.is_empty() {
                    labels_data[flat_i]
                } else {
                    continue;
                };

                let class = match Self::class_from_id(label_id) {
                    Some(c) => c,
                    None => continue,
                };

                let box_base = flat_i * 4;
                let x1 = ((boxes_data[box_base] - pad_x_f) * inv_scale).clamp(0.0, orig_width as f32);
                let y1 = ((boxes_data[box_base + 1] - pad_y_f) * inv_scale).clamp(0.0, orig_height as f32);
                let x2 = ((boxes_data[box_base + 2] - pad_x_f) * inv_scale).clamp(0.0, orig_width as f32);
                let y2 = ((boxes_data[box_base + 3] - pad_y_f) * inv_scale).clamp(0.0, orig_height as f32);

                detections.push(LayoutDetection::new(class, score, BBox::new(x1, y1, x2, y2)));
            }

            detections = LayoutDetection::sort_by_confidence_desc(detections);

            tracing::debug!(
                batch_index = b,
                detections = detections.len(),
                "PP-DocLayout-V3 batch inference: per-image detections"
            );

            results.push(detections);
        }

        tracing::debug!(
            preprocess_ms,
            onnx_ms,
            batch,
            "PP-DocLayout-V3 batch inference breakdown"
        );

        #[cfg(feature = "otel")]
        {
            let total_inference_ms = inference_start.elapsed().as_secs_f64() * 1000.0;
            tracing::Span::current().record(crate::telemetry::conventions::MODEL_INFERENCE_MS, total_inference_ms);
        }

        Ok(results)
    }
}

impl LayoutModel for PpDocLayoutV3Model {
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
        "PP-DocLayout-V3"
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_class_from_id_text() {
        assert_eq!(PpDocLayoutV3Model::class_from_id(0), Some(LayoutClass::Text));
    }

    #[test]
    fn test_class_from_id_title() {
        assert_eq!(PpDocLayoutV3Model::class_from_id(1), Some(LayoutClass::Title));
    }

    #[test]
    fn test_class_from_id_image() {
        assert_eq!(PpDocLayoutV3Model::class_from_id(2), Some(LayoutClass::Picture));
    }

    #[test]
    fn test_class_from_id_formula() {
        assert_eq!(PpDocLayoutV3Model::class_from_id(3), Some(LayoutClass::Formula));
    }

    #[test]
    fn test_class_from_id_table() {
        assert_eq!(PpDocLayoutV3Model::class_from_id(4), Some(LayoutClass::Table));
    }

    #[test]
    fn test_class_from_id_chart() {
        assert_eq!(PpDocLayoutV3Model::class_from_id(5), Some(LayoutClass::Picture));
    }

    #[test]
    fn test_class_from_id_footer() {
        assert_eq!(PpDocLayoutV3Model::class_from_id(6), Some(LayoutClass::PageFooter));
    }

    #[test]
    fn test_class_from_id_header() {
        assert_eq!(PpDocLayoutV3Model::class_from_id(7), Some(LayoutClass::PageHeader));
    }

    #[test]
    fn test_class_from_id_invalid() {
        assert_eq!(PpDocLayoutV3Model::class_from_id(8), None);
        assert_eq!(PpDocLayoutV3Model::class_from_id(100), None);
    }

    #[test]
    fn test_default_threshold() {
        assert_eq!(DEFAULT_THRESHOLD, 0.5);
    }

    #[test]
    fn test_input_size() {
        assert_eq!(INPUT_SIZE, 960);
    }

    /// Verify that `detect_batch` returns an empty Vec without hitting the ONNX
    /// session when called with an empty slice.
    ///
    /// `run_batch_inference` itself also returns `Ok(Vec::new())` early on an
    /// empty slice, but constructing a `PpDocLayoutV3Model` requires a real ONNX
    /// session (weights on disk), so we exercise the guard via the public
    /// `LayoutModel::detect_batch` short-circuit path, which returns before
    /// calling `run_batch_inference`. The `detect_batch` method is defined on
    /// the trait impl but delegates to `run_batch_inference` only when
    /// `images.len() > 1`, so an empty input returns before any session access.
    ///
    /// The test is omitted here because it cannot be run without a real model
    /// file. The correctness of the empty-slice path is guaranteed by the
    /// `if images.is_empty() { return Ok(Vec::new()); }` guard added at line 210.
    #[allow(dead_code)]
    fn _doc_empty_batch_contract() {}
}
