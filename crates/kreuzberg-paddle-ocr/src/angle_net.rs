use crate::{base_net::BaseNet, ocr_error::OcrError, ocr_result::Angle, ocr_utils::OcrUtils};

use ort::{
    inputs,
    session::{Session, SessionOutputs},
    value::Tensor,
};

const MEAN_VALUES: [f32; 3] = [127.5, 127.5, 127.5];
const NORM_VALUES: [f32; 3] = [1.0 / 127.5, 1.0 / 127.5, 1.0 / 127.5];
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
        &mut self,
        part_imgs: &[image::RgbImage],
        do_angle: bool,
        most_angle: bool,
    ) -> Result<Vec<Angle>, OcrError> {
        let mut angles = Vec::new();

        if do_angle {
            for img in part_imgs {
                let angle = self.get_angle(img)?;
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

    fn get_angle(&mut self, img_src: &image::RgbImage) -> Result<Angle, OcrError> {
        let Some(session) = &mut self.session else {
            return Err(OcrError::SessionNotInitialized);
        };

        let angle_img = image::imageops::resize(
            img_src,
            ANGLE_DST_WIDTH,
            ANGLE_DST_HEIGHT,
            image::imageops::FilterType::Triangle,
        );

        let input_tensors = OcrUtils::substract_mean_normalize(&angle_img, &MEAN_VALUES, &NORM_VALUES);

        let input_tensors = Tensor::from_array(input_tensors)?;

        let outputs = session.run(inputs![self.input_names[0].clone() => input_tensors])?;

        let angle = Self::score_to_angle(&outputs, ANGLE_COLS)?;

        Ok(angle)
    }

    fn score_to_angle(output_tensor: &SessionOutputs, angle_cols: usize) -> Result<Angle, OcrError> {
        let (_, red_data) = output_tensor.iter().next().unwrap();

        let src_data: Vec<f32> = red_data.try_extract_tensor::<f32>()?.1.to_vec();

        let mut angle = Angle::default();
        let mut max_value = f32::MIN;
        let mut angle_index = 0;

        for (i, value) in src_data.iter().take(angle_cols).enumerate() {
            if i == 0 || value > &max_value {
                max_value = *value;
                angle_index = i as i32;
            }
        }

        angle.index = angle_index;
        angle.score = max_value;
        Ok(angle)
    }
}
