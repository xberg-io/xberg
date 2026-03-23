use crate::{
    base_net::BaseNet,
    constants::{IMAGENET_MEAN_VALUES, IMAGENET_NORM_VALUES},
    ocr_error::OcrError,
    ocr_result::Angle,
    ocr_utils::OcrUtils,
};

use ort::{
    inputs,
    session::{Session, SessionOutputs},
    value::Tensor,
};

// PP-LCNet_x1_0_textline_ori preprocessing (ImageNet normalization).
// Input: resize to 160×80, normalize with ImageNet mean/std.
// Formula in substract_mean_normalize: (pixel - MEAN) * NORM
// For ImageNet: (pixel/255 - mean) / std = (pixel - mean*255) * (1/(std*255))
// PP-OCR angle classifier expects [3, 48, 192] input (cls_image_shape in PaddleOCR Python).
const ANGLE_DST_WIDTH: u32 = 192;
const ANGLE_DST_HEIGHT: u32 = 48;
const ANGLE_COLS: usize = 2;

#[derive(Debug)]
pub struct AngleNet {
    session: Option<Session>,
    input_names: Vec<String>,
}

impl BaseNet for AngleNet {
    fn new() -> Self {
        Self {
            session: None,
            input_names: Vec::new(),
        }
    }

    fn set_input_names(&mut self, input_names: Vec<String>) {
        self.input_names = input_names;
    }

    fn set_session(&mut self, session: Option<Session>) {
        self.session = session;
    }
}

impl AngleNet {
    pub fn get_angles(
        &self,
        part_imgs: &[image::RgbImage],
        do_angle: bool,
        most_angle: bool,
        cls_thresh: f32,
    ) -> Result<Vec<Angle>, OcrError> {
        // Pre-allocate — we know exact count upfront.
        let mut angles = Vec::with_capacity(part_imgs.len());

        if do_angle {
            for img in part_imgs {
                let angle = self.get_angle(img, cls_thresh)?;
                angles.push(angle);
            }
        } else {
            angles.extend(part_imgs.iter().map(|_| Angle::default()));
        }

        if do_angle && most_angle {
            let sum: i32 = angles.iter().map(|x| x.index).sum();
            let half_percent = angles.len() as f32 / 2.0;
            let most_angle_index = if (sum as f32) < half_percent { 0 } else { 1 };

            for angle in angles.iter_mut() {
                angle.index = most_angle_index;
            }
        }

        Ok(angles)
    }

    fn get_angle(&self, img_src: &image::RgbImage, cls_thresh: f32) -> Result<Angle, OcrError> {
        let Some(session) = &self.session else {
            return Err(OcrError::SessionNotInitialized);
        };

        let angle_img = image::imageops::resize(
            img_src,
            ANGLE_DST_WIDTH,
            ANGLE_DST_HEIGHT,
            image::imageops::FilterType::Triangle,
        );

        let input_tensors =
            OcrUtils::substract_mean_normalize(&angle_img, &IMAGENET_MEAN_VALUES, &IMAGENET_NORM_VALUES);

        let input_tensors = Tensor::from_array(input_tensors)?;

        // SAFETY: ONNX Runtime C API is thread-safe for concurrent inference.
        #[allow(unsafe_code)]
        let outputs = unsafe {
            let session_ptr = session as *const Session as *mut Session;
            (*session_ptr).run(inputs![self.input_names[0].as_str() => input_tensors])?
        };

        let mut angle = Self::score_to_angle(&outputs, ANGLE_COLS)?;

        // Only apply rotation if confidence exceeds threshold (matches PaddleOCR's cls_thresh=0.9)
        if angle.score < cls_thresh {
            angle.index = 0; // Keep original orientation when confidence is low
        }

        Ok(angle)
    }

    fn score_to_angle(output_tensor: &SessionOutputs, angle_cols: usize) -> Result<Angle, OcrError> {
        let (_, red_data) = output_tensor.iter().next().ok_or_else(|| {
            OcrError::Io(std::io::Error::new(
                std::io::ErrorKind::InvalidData,
                "No output tensors found in angle classification session output",
            ))
        })?;

        let src_data: Vec<f32> = red_data.try_extract_tensor::<f32>()?.1.to_vec();

        let mut angle = Angle::default();
        let mut max_value = f32::MIN;
        let mut angle_index = 0;

        for (i, value) in src_data.iter().take(angle_cols).enumerate() {
            if *value > max_value {
                max_value = *value;
                angle_index = i as i32;
            }
        }

        angle.index = angle_index;
        angle.score = max_value;
        Ok(angle)
    }
}
