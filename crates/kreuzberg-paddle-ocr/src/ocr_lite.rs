use std::collections::HashMap;

use image::ImageBuffer;
use ort::session::builder::SessionBuilder;

use crate::{
    angle_net::AngleNet,
    base_net::BaseNet,
    crnn_net::CrnnNet,
    db_net::DbNet,
    ocr_error::OcrError,
    ocr_result::{OcrResult, Point, TextBlock},
    ocr_utils::OcrUtils,
    scale_param::ScaleParam,
};

#[derive(Debug)]
pub struct OcrLite {
    db_net: DbNet,
    angle_net: AngleNet,
    crnn_net: CrnnNet,
}

impl Default for OcrLite {
    fn default() -> Self {
        Self::new()
    }
}

impl OcrLite {
    pub fn new() -> Self {
        Self {
            db_net: DbNet::new(),
            angle_net: AngleNet::new(),
            crnn_net: CrnnNet::new(),
        }
    }

    pub fn init_models(
        &mut self,
        det_path: &str,
        cls_path: &str,
        rec_path: &str,
        num_thread: usize,
    ) -> Result<(), OcrError> {
        self.db_net.init_model(det_path, num_thread, None)?;
        self.angle_net.init_model(cls_path, num_thread, None)?;
        self.crnn_net.init_model(rec_path, num_thread, None)?;
        Ok(())
    }

    pub fn init_models_with_dict(
        &mut self,
        det_path: &str,
        cls_path: &str,
        rec_path: &str,
        dict_path: &str,
        num_thread: usize,
    ) -> Result<(), OcrError> {
        self.db_net.init_model(det_path, num_thread, None)?;
        self.angle_net.init_model(cls_path, num_thread, None)?;
        self.crnn_net
            .init_model_dict_file(rec_path, num_thread, None, dict_path)?;
        Ok(())
    }

    pub fn init_models_custom(
        &mut self,
        det_path: &str,
        cls_path: &str,
        rec_path: &str,
        builder_fn: fn(SessionBuilder) -> Result<SessionBuilder, ort::Error>,
    ) -> Result<(), OcrError> {
        self.db_net.init_model(det_path, 0, Some(builder_fn))?;
        self.angle_net.init_model(cls_path, 0, Some(builder_fn))?;
        self.crnn_net.init_model(rec_path, 0, Some(builder_fn))?;
        Ok(())
    }

    pub fn init_models_from_memory(
        &mut self,
        det_bytes: &[u8],
        cls_bytes: &[u8],
        rec_bytes: &[u8],
        num_thread: usize,
    ) -> Result<(), OcrError> {
        self.db_net.init_model_from_memory(det_bytes, num_thread, None)?;
        self.angle_net.init_model_from_memory(cls_bytes, num_thread, None)?;
        self.crnn_net.init_model_from_memory(rec_bytes, num_thread, None)?;
        Ok(())
    }

    pub fn init_models_from_memory_custom(
        &mut self,
        det_bytes: &[u8],
        cls_bytes: &[u8],
        rec_bytes: &[u8],
        builder_fn: fn(SessionBuilder) -> Result<SessionBuilder, ort::Error>,
    ) -> Result<(), OcrError> {
        self.db_net.init_model_from_memory(det_bytes, 0, Some(builder_fn))?;
        self.angle_net.init_model_from_memory(cls_bytes, 0, Some(builder_fn))?;
        self.crnn_net.init_model_from_memory(rec_bytes, 0, Some(builder_fn))?;
        Ok(())
    }

    fn detect_base(
        &mut self,
        img_src: &image::RgbImage,
        padding: u32,
        max_side_len: u32,
        box_score_thresh: f32,
        box_thresh: f32,
        un_clip_ratio: f32,
        do_angle: bool,
        most_angle: bool,
        angle_rollback: bool,
        angle_rollback_threshold: f32,
    ) -> Result<OcrResult, OcrError> {
        let origin_max_side = img_src.width().max(img_src.height());
        let mut resize;
        if max_side_len == 0 || max_side_len > origin_max_side {
            resize = origin_max_side;
        } else {
            resize = max_side_len;
        }
        resize += 2 * padding;

        let padding_src = OcrUtils::make_padding(img_src, padding)?;

        let scale = ScaleParam::get_scale_param(&padding_src, resize);

        self.detect_once(
            &padding_src,
            &scale,
            padding,
            box_score_thresh,
            box_thresh,
            un_clip_ratio,
            do_angle,
            most_angle,
            angle_rollback,
            angle_rollback_threshold,
        )
    }

    /// Detect text in image
    ///
    /// # Arguments
    ///
    /// - `img_src` - Input image
    /// - `padding` - Padding width added during image transformation (improves detection)
    /// - `max_side_len` - Maximum side length after transformation (larger images will be scaled down)
    /// - `box_score_thresh` - Score threshold for text region detection
    /// - `box_thresh` - Box threshold
    /// - `un_clip_ratio` - Unclip ratio
    /// - `do_angle` - Whether to perform angle detection
    /// - `most_angle` - Use most common angle for all text regions
    pub fn detect(
        &mut self,
        img_src: &image::RgbImage,
        padding: u32,
        max_side_len: u32,
        box_score_thresh: f32,
        box_thresh: f32,
        un_clip_ratio: f32,
        do_angle: bool,
        most_angle: bool,
    ) -> Result<OcrResult, OcrError> {
        self.detect_base(
            img_src,
            padding,
            max_side_len,
            box_score_thresh,
            box_thresh,
            un_clip_ratio,
            do_angle,
            most_angle,
            false,
            0.0,
        )
    }

    /// Detect text with angle rollback support
    ///
    /// When `do_angle` is true, if the image was angle-corrected but recognition
    /// result is poor, the angle correction will be reverted.
    ///
    /// # Arguments
    ///
    /// - `img_src` - Input image
    /// - `padding` - Padding width added during image transformation
    /// - `max_side_len` - Maximum side length after transformation
    /// - `box_score_thresh` - Score threshold for text region detection
    /// - `box_thresh` - Box threshold
    /// - `un_clip_ratio` - Unclip ratio
    /// - `do_angle` - Whether to perform angle detection
    /// - `most_angle` - Use most common angle
    /// - `angle_rollback_threshold` - If text score is below this value (or NaN), angle correction is reverted
    pub fn detect_angle_rollback(
        &mut self,
        img_src: &image::RgbImage,
        padding: u32,
        max_side_len: u32,
        box_score_thresh: f32,
        box_thresh: f32,
        un_clip_ratio: f32,
        do_angle: bool,
        most_angle: bool,
        angle_rollback_threshold: f32,
    ) -> Result<OcrResult, OcrError> {
        self.detect_base(
            img_src,
            padding,
            max_side_len,
            box_score_thresh,
            box_thresh,
            un_clip_ratio,
            do_angle,
            most_angle,
            true,
            angle_rollback_threshold,
        )
    }

    pub fn detect_from_path(
        &mut self,
        img_path: &str,
        padding: u32,
        max_side_len: u32,
        box_score_thresh: f32,
        box_thresh: f32,
        un_clip_ratio: f32,
        do_angle: bool,
        most_angle: bool,
    ) -> Result<OcrResult, OcrError> {
        let img_src = image::open(img_path)?.to_rgb8();

        self.detect(
            &img_src,
            padding,
            max_side_len,
            box_score_thresh,
            box_thresh,
            un_clip_ratio,
            do_angle,
            most_angle,
        )
    }

    fn detect_once(
        &mut self,
        img_src: &image::RgbImage,
        scale: &ScaleParam,
        padding: u32,
        box_score_thresh: f32,
        box_thresh: f32,
        un_clip_ratio: f32,
        do_angle: bool,
        most_angle: bool,
        angle_rollback: bool,
        angle_rollback_threshold: f32,
    ) -> Result<OcrResult, OcrError> {
        let text_boxes = self
            .db_net
            .get_text_boxes(img_src, scale, box_score_thresh, box_thresh, un_clip_ratio)?;

        let part_images = OcrUtils::get_part_images(img_src, &text_boxes);

        let angles = self.angle_net.get_angles(&part_images, do_angle, most_angle)?;

        let mut rotated_images: Vec<image::RgbImage> = Vec::with_capacity(part_images.len());

        // Angle correction rollback
        let mut angle_rollback_records = HashMap::<usize, ImageBuffer<image::Rgb<u8>, Vec<u8>>>::new();

        for (index, (angle, mut part_image)) in angles.iter().zip(part_images.into_iter()).enumerate() {
            if angle.index == 1 {
                if angle_rollback {
                    // Keep original copy
                    angle_rollback_records.insert(index, part_image.clone());
                }

                OcrUtils::mat_rotate_clock_wise_180(&mut part_image);
            }
            rotated_images.push(part_image);
        }

        let text_lines =
            self.crnn_net
                .get_text_lines(&rotated_images, &angle_rollback_records, angle_rollback_threshold)?;

        let mut text_blocks = Vec::with_capacity(text_lines.len());
        for i in 0..text_lines.len() {
            text_blocks.push(TextBlock {
                box_points: text_boxes[i]
                    .points
                    .iter()
                    .map(|p| Point {
                        x: ((p.x as f32) - padding as f32) as u32,
                        y: ((p.y as f32) - padding as f32) as u32,
                    })
                    .collect(),
                box_score: text_boxes[i].score,
                angle_index: angles[i].index,
                angle_score: angles[i].score,
                text: text_lines[i].text.clone(),
                text_score: text_lines[i].text_score,
            });
        }

        Ok(OcrResult { text_blocks })
    }
}
