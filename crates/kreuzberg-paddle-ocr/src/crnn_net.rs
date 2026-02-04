use ort::session::Session;
use ort::value::Tensor;
use ort::{inputs, session::builder::SessionBuilder};
use std::collections::HashMap;

use crate::{base_net::BaseNet, ocr_error::OcrError, ocr_result::TextLine, ocr_utils::OcrUtils};

const CRNN_DST_HEIGHT: u32 = 48;
const MEAN_VALUES: [f32; 3] = [127.5, 127.5, 127.5];
const NORM_VALUES: [f32; 3] = [1.0 / 127.5, 1.0 / 127.5, 1.0 / 127.5];

#[derive(Debug)]
pub struct CrnnNet {
    session: Option<Session>,
    keys: Vec<String>,
    input_names: Vec<String>,
}

impl BaseNet for CrnnNet {
    fn new() -> Self {
        Self {
            session: None,
            keys: Vec::new(),
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

impl CrnnNet {
    pub fn init_model(
        &mut self,
        path: &str,
        num_thread: usize,
        builder_fn: Option<fn(SessionBuilder) -> Result<SessionBuilder, ort::Error>>,
    ) -> Result<(), OcrError> {
        BaseNet::init_model(self, path, num_thread, builder_fn)?;

        self.keys = self.get_keys()?;

        Ok(())
    }

    pub fn init_model_dict_file(
        &mut self,
        path: &str,
        num_thread: usize,
        builder_fn: Option<fn(SessionBuilder) -> Result<SessionBuilder, ort::Error>>,
        dict_file_path: &str,
    ) -> Result<(), OcrError> {
        BaseNet::init_model(self, path, num_thread, builder_fn)?;

        self.read_keys_from_file(dict_file_path)?;

        Ok(())
    }

    pub fn init_model_from_memory(
        &mut self,
        model_bytes: &[u8],
        num_thread: usize,
        builder_fn: Option<fn(SessionBuilder) -> Result<SessionBuilder, ort::Error>>,
    ) -> Result<(), OcrError> {
        BaseNet::init_model_from_memory(self, model_bytes, num_thread, builder_fn)?;

        self.keys = self.get_keys()?;

        Ok(())
    }

    fn get_keys(&mut self) -> Result<Vec<String>, OcrError> {
        let session = self.session.as_ref().expect("crnn_net session not initialized");

        let metadata = session.metadata()?;
        let model_charater_list = metadata
            .custom("character")
            .expect("crnn_net character not initialized");

        // Estimate capacity
        let mut keys = Vec::with_capacity((model_charater_list.len() as f32 / 3.9) as usize);

        keys.push("#".to_string());

        keys.extend(model_charater_list.split('\n').map(|s: &str| s.to_string()));

        keys.push(" ".to_string());

        Ok(keys)
    }

    fn read_keys_from_file(&mut self, path: &str) -> Result<(), OcrError> {
        let content = std::fs::read_to_string(path)?;
        let mut keys = Vec::new();

        keys.extend(content.split('\n').map(|s| s.to_string()));
        self.keys = keys;
        Ok(())
    }

    pub fn get_text_lines(
        &mut self,
        part_imgs: &[image::RgbImage],
        angle_rollback_records: &HashMap<usize, image::RgbImage>,
        angle_rollback_threshold: f32,
    ) -> Result<Vec<TextLine>, OcrError> {
        let mut text_lines = Vec::new();

        for (index, img) in part_imgs.iter().enumerate() {
            let mut text_line = self.get_text_line(img)?;

            if (text_line.text_score.is_nan() || text_line.text_score < angle_rollback_threshold)
                && let Some(angle_rollback_record) = angle_rollback_records.get(&index)
            {
                text_line = self.get_text_line(angle_rollback_record)?;
            }

            text_lines.push(text_line);
        }

        Ok(text_lines)
    }

    fn get_text_line(&mut self, img_src: &image::RgbImage) -> Result<TextLine, OcrError> {
        let Some(session) = &mut self.session else {
            return Err(OcrError::SessionNotInitialized);
        };

        let scale = CRNN_DST_HEIGHT as f32 / img_src.height() as f32;
        let dst_width = (img_src.width() as f32 * scale) as u32;

        let src_resize = image::imageops::resize(
            img_src,
            dst_width,
            CRNN_DST_HEIGHT,
            image::imageops::FilterType::Triangle,
        );

        let input_tensors = OcrUtils::substract_mean_normalize(&src_resize, &MEAN_VALUES, &NORM_VALUES);

        let input_tensors = Tensor::from_array(input_tensors)?;

        let outputs = session.run(inputs![self.input_names[0].clone() => input_tensors])?;

        let (_, red_data) = outputs.iter().next().unwrap();

        let (shape, src_data) = red_data.try_extract_tensor::<f32>()?;
        let dimensions = shape;
        let height = dimensions[1] as usize;
        let width = dimensions[2] as usize;
        let src_data: Vec<f32> = src_data.to_vec();

        Self::score_to_text_line(&src_data, height, width, &self.keys)
    }

    fn score_to_text_line(
        output_data: &[f32],
        height: usize,
        width: usize,
        keys: &[String],
    ) -> Result<TextLine, OcrError> {
        let mut text_line = TextLine::default();
        let mut last_index = 0;

        let mut text_score_sum = 0.0;
        let mut text_socre_count = 0;
        for i in 0..height {
            let start = i * width;
            let stop = (i + 1) * width;
            let slice = &output_data[start..stop.min(output_data.len())];

            let (max_index, max_value) =
                slice
                    .iter()
                    .enumerate()
                    .fold((0, f32::MIN), |(max_idx, max_val), (idx, &val)| {
                        if val > max_val { (idx, val) } else { (max_idx, max_val) }
                    });

            if max_index > 0 && max_index < keys.len() && !(i > 0 && max_index == last_index) {
                text_line.text.push_str(&keys[max_index]);
                text_score_sum += max_value;
                text_socre_count += 1;
            }
            last_index = max_index;
        }

        text_line.text_score = text_score_sum / text_socre_count as f32;
        Ok(text_line)
    }
}
