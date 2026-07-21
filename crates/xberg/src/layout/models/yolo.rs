use image::RgbImage;
use ort::{inputs, session::Session, value::Tensor};

use crate::layout::error::LayoutError;
use crate::layout::models::LayoutModel;
use crate::layout::postprocessing::nms;
use crate::layout::preprocessing;
use crate::layout::types::{BBox, LayoutClass, LayoutDetection};

/// Default confidence threshold for YOLO detections.
const DEFAULT_THRESHOLD: f32 = 0.35;

/// NMS IoU threshold.
const NMS_IOU_THRESHOLD: f32 = 0.45;

/// Which YOLO variant this model represents.
#[cfg_attr(alef, alef(skip))]
#[derive(Debug, Clone, Copy)]
pub enum YoloVariant {
    /// YOLOv10/v8 trained on DocLayNet (11 classes).
    /// Output: [batch, num_dets, 6] = [x1, y1, x2, y2, score, class_id]
    DocLayNet,
    /// DocLayout-YOLO trained on DocStructBench (10 classes).
    /// Output: [batch, num_dets, 4+num_classes] center-format, or [batch, num_dets, 6] decoded.
    DocStructBench,
    /// YOLOX with letterbox preprocessing and grid decoding.
    /// Output: [batch, num_anchors, 5+num_classes] — needs grid decoding + NMS.
    /// Strides: [8, 16, 32], anchors decoded via (raw + grid_offset) * stride.
    Yolox,
}

/// YOLO-family layout detection model (YOLOv10, DocLayout-YOLO, YOLOX).
#[cfg_attr(alef, alef(skip))]
pub struct YoloModel {
    session: Session,
    input_name: String,
    variant: YoloVariant,
    input_width: u32,
    input_height: u32,
    model_name: String,
}

impl YoloModel {
    /// Load a YOLO ONNX model from a file.
    ///
    /// For square-input models (YOLOv10, DocLayout-YOLO), pass the same value for both dimensions.
    /// For YOLOX (unstructuredio), use width=768, height=1024.
    pub(crate) fn from_file(
        path: &str,
        variant: YoloVariant,
        input_width: u32,
        input_height: u32,
        model_name: &str,
        accel: Option<&crate::core::config::acceleration::AccelerationConfig>,
        thread_budget: usize,
    ) -> Result<Self, LayoutError> {
        let session = crate::layout::session::build_session(path, accel, thread_budget.max(1))?;
        let input_name = session.inputs()[0].name().to_string();
        Ok(Self {
            session,
            input_name,
            variant,
            input_width,
            input_height,
            model_name: model_name.to_string(),
        })
    }

    fn run_inference(&mut self, img: &RgbImage, threshold: f32) -> Result<Vec<LayoutDetection>, LayoutError> {
        let orig_width = img.width();
        let orig_height = img.height();

        match self.variant {
            YoloVariant::Yolox => self.run_yolox(img, threshold, orig_width, orig_height),
            YoloVariant::DocLayNet | YoloVariant::DocStructBench => {
                self.run_yolov10(img, threshold, orig_width, orig_height)
            }
        }
    }

    /// YOLOv10/v8 and DocLayout-YOLO inference.
    ///
    /// These models output decoded detections directly (post-NMS in v10, or need NMS in v8).
    /// Output shape: [1, num_dets, 6] where each detection = [x1, y1, x2, y2, score, class_id].
    fn run_yolov10(
        &mut self,
        img: &RgbImage,
        threshold: f32,
        orig_width: u32,
        orig_height: u32,
    ) -> Result<Vec<LayoutDetection>, LayoutError> {
        let variant = self.variant;
        let input_tensor = preprocessing::preprocess_rescale(img, self.input_width);
        let images_tensor = Tensor::from_array(input_tensor)?;

        let outputs = self.session.run(inputs![self.input_name.as_str() => images_tensor])?;

        let (_, output_value) = outputs
            .iter()
            .next()
            .ok_or_else(|| LayoutError::InvalidOutput("No output tensors from YOLO model".into()))?;

        let view = output_value
            .try_extract_tensor::<f32>()
            .map_err(|e| LayoutError::InvalidOutput(format!("Failed to extract f32 output: {e}")))?;

        let shape = &view.0;
        let data = view.1;

        let num_dets = if shape.len() == 3 {
            shape[1] as usize
        } else {
            shape[0] as usize
        };
        let cols = if shape.len() == 3 {
            shape[2] as usize
        } else if shape.len() == 2 {
            shape[1] as usize
        } else {
            return Err(LayoutError::InvalidOutput(format!(
                "Unexpected output shape: {shape:?}"
            )));
        };

        let scale_x = orig_width as f32 / self.input_width as f32;
        let scale_y = orig_height as f32 / self.input_height as f32;

        let mut detections = Vec::new();

        if cols == 6 {
            for i in 0..num_dets {
                let offset = i * 6;
                let score = data[offset + 4];
                if score < threshold {
                    continue;
                }
                let class_id = data[offset + 5] as i64;
                let class = match map_class_id(variant, class_id) {
                    Some(c) => c,
                    None => continue,
                };
                let bbox = BBox::new(
                    data[offset] * scale_x,
                    data[offset + 1] * scale_y,
                    data[offset + 2] * scale_x,
                    data[offset + 3] * scale_y,
                );
                detections.push(LayoutDetection::new(class, score, bbox));
            }
        } else if cols > 4 {
            let num_classes = cols - 4;
            for i in 0..num_dets {
                let offset = i * cols;
                let cx = data[offset];
                let cy = data[offset + 1];
                let w = data[offset + 2];
                let h = data[offset + 3];

                let mut best_score = 0.0f32;
                let mut best_class_idx = 0i64;
                for c in 0..num_classes {
                    let s = data[offset + 4 + c];
                    if s > best_score {
                        best_score = s;
                        best_class_idx = c as i64;
                    }
                }

                if best_score < threshold {
                    continue;
                }

                let class = match map_class_id(variant, best_class_idx) {
                    Some(c) => c,
                    None => continue,
                };

                let x1 = (cx - w / 2.0) * scale_x;
                let y1 = (cy - h / 2.0) * scale_y;
                let x2 = (cx + w / 2.0) * scale_x;
                let y2 = (cy + h / 2.0) * scale_y;
                detections.push(LayoutDetection::new(class, best_score, BBox::new(x1, y1, x2, y2)));
            }
            detections = nms::greedy_nms(detections, NMS_IOU_THRESHOLD);
        }

        detections = LayoutDetection::sort_by_confidence_desc(detections);

        Ok(detections)
    }

    /// YOLOX inference with letterbox preprocessing and grid decoding.
    ///
    /// YOLOX outputs raw anchor predictions [1, num_anchors, 5+num_classes] that must be
    /// decoded using grid offsets and stride values before boxes are usable.
    ///
    /// Grid decoding (per anchor at stride level s, grid position (gx, gy)):
    ///   decoded_cx = (raw_cx + gx) * s
    ///   decoded_cy = (raw_cy + gy) * s
    ///   decoded_w  = exp(raw_w) * s
    ///   decoded_h  = exp(raw_h) * s
    ///   confidence = objectness * max(class_scores)
    ///
    /// Reference: layoutparser-ort (Apache-2.0)
    fn run_yolox(
        &mut self,
        img: &RgbImage,
        threshold: f32,
        _orig_width: u32,
        _orig_height: u32,
    ) -> Result<Vec<LayoutDetection>, LayoutError> {
        let variant = self.variant;
        let input_w = self.input_width;
        let input_h = self.input_height;

        let (input_tensor, scale) = preprocessing::preprocess_letterbox(img, input_w, input_h);
        let images_tensor = Tensor::from_array(input_tensor)?;

        let outputs = self.session.run(inputs![self.input_name.as_str() => images_tensor])?;

        let (_, output_value) = outputs
            .iter()
            .next()
            .ok_or_else(|| LayoutError::InvalidOutput("No output tensors from YOLOX model".into()))?;

        let view = output_value
            .try_extract_tensor::<f32>()
            .map_err(|e| LayoutError::InvalidOutput(format!("Failed to extract f32 output: {e}")))?;

        let shape = &view.0;
        let data = view.1;

        let num_anchors = if shape.len() == 3 {
            shape[1] as usize
        } else {
            shape[0] as usize
        };
        let cols = if shape.len() == 3 {
            shape[2] as usize
        } else {
            shape[1] as usize
        };
        let num_classes = cols - 5;

        let strides: &[u32] = &[8, 16, 32];

        let mut grids: Vec<(f32, f32, f32)> = Vec::with_capacity(num_anchors);
        for &stride in strides {
            let h_size = input_h / stride;
            let w_size = input_w / stride;
            for y in 0..h_size {
                for x in 0..w_size {
                    grids.push((x as f32, y as f32, stride as f32));
                }
            }
        }

        if grids.len() != num_anchors {
            return Err(LayoutError::InvalidOutput(format!(
                "Grid anchor count mismatch: expected {} from strides, got {} from model output",
                grids.len(),
                num_anchors
            )));
        }

        let mut detections = Vec::new();
        for (i, &(gx, gy, s)) in grids.iter().enumerate() {
            let offset = i * cols;

            let cx = (data[offset] + gx) * s;
            let cy = (data[offset + 1] + gy) * s;
            let w = data[offset + 2].exp() * s;
            let h = data[offset + 3].exp() * s;

            let objectness = data[offset + 4];

            let mut best_class_score = 0.0f32;
            let mut best_class_idx = 0i64;
            for c in 0..num_classes {
                let cs = data[offset + 5 + c];
                if cs > best_class_score {
                    best_class_score = cs;
                    best_class_idx = c as i64;
                }
            }

            let confidence = objectness * best_class_score;
            if confidence < threshold {
                continue;
            }

            let class = match map_class_id(variant, best_class_idx) {
                Some(c) => c,
                None => continue,
            };

            let x1 = (cx - w / 2.0) / scale;
            let y1 = (cy - h / 2.0) / scale;
            let x2 = (cx + w / 2.0) / scale;
            let y2 = (cy + h / 2.0) / scale;

            detections.push(LayoutDetection::new(class, confidence, BBox::new(x1, y1, x2, y2)));
        }

        detections = nms::greedy_nms(detections, NMS_IOU_THRESHOLD);

        detections = LayoutDetection::sort_by_confidence_desc(detections);

        Ok(detections)
    }
}

fn map_class_id(variant: YoloVariant, id: i64) -> Option<LayoutClass> {
    match variant {
        YoloVariant::DocLayNet | YoloVariant::Yolox => LayoutClass::from_doclaynet_id(id),
        YoloVariant::DocStructBench => LayoutClass::from_docstructbench_id(id),
    }
}

impl LayoutModel for YoloModel {
    fn detect(&mut self, img: &RgbImage) -> Result<Vec<LayoutDetection>, LayoutError> {
        self.run_inference(img, DEFAULT_THRESHOLD)
    }

    fn detect_with_threshold(&mut self, img: &RgbImage, threshold: f32) -> Result<Vec<LayoutDetection>, LayoutError> {
        self.run_inference(img, threshold)
    }

    fn name(&self) -> &str {
        &self.model_name
    }
}
