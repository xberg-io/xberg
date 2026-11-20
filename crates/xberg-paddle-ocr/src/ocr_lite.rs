use std::collections::HashMap;

use image::ImageBuffer;
use imageproc::geometric_transformations::Projection;
#[cfg(feature = "ort")]
use ort::session::builder::SessionBuilder;
use tracing;

use crate::{
    angle_net::AngleNet,
    base_net::BaseNet,
    crnn_net::CrnnNet,
    db_net::DbNet,
    ocr_error::OcrError,
    ocr_result::{
        Angle, DetailedOcrResult, DetailedTextBlock, DetailedTextLine, OcrResult, Point, RecognizedWord, TextBlock,
        TextBox, WordBlock,
    },
    ocr_utils::OcrUtils,
    scale_param::ScaleParam,
};

const VISUAL_LINE_Y_TOLERANCE_PX: u32 = 10;

#[derive(Debug, Clone, Copy)]
struct CropProjectionMetadata {
    crop_to_source: Projection,
    crop_width: f32,
    crop_height: f32,
    vertical_rotation_applied: bool,
}

/// Text boxes, their cropped images, and the projection metadata for each crop, in the order
/// detection produced them. Named so the detection helper's signature stays readable. ~keep
type DetectedRegions = (Vec<TextBox>, Vec<image::RgbImage>, Vec<Option<CropProjectionMetadata>>);

/// Crops in recognition order, paired with the pre-rotation copy of every crop the angle
/// classifier turned, keyed by its index. ~keep
type RotatedRegions = (
    Vec<image::RgbImage>,
    HashMap<usize, ImageBuffer<image::Rgb<u8>, Vec<u8>>>,
);

impl CropProjectionMetadata {
    fn new(box_points: &[Point], cropped_image: &image::RgbImage) -> Option<Self> {
        let points = box_points.get(..4)?;
        let edge_length = |first: Point, second: Point| {
            let dx = first.x as f32 - second.x as f32;
            let dy = first.y as f32 - second.y as f32;
            (dx * dx + dy * dy).sqrt()
        };
        let crop_width = edge_length(points[0], points[1]).max(edge_length(points[2], points[3])) as u32;
        let crop_height = edge_length(points[0], points[3]).max(edge_length(points[1], points[2])) as u32;
        if crop_width == 0 || crop_height == 0 {
            return None;
        }

        let vertical_rotation_applied = if cropped_image.dimensions() == (crop_width, crop_height) {
            false
        } else if cropped_image.dimensions() == (crop_height, crop_width) {
            true
        } else {
            return None;
        };
        let source_points = [
            (points[0].x as f32, points[0].y as f32),
            (points[1].x as f32, points[1].y as f32),
            (points[2].x as f32, points[2].y as f32),
            (points[3].x as f32, points[3].y as f32),
        ];
        let crop_points = [
            (0.0, 0.0),
            (crop_width as f32, 0.0),
            (crop_width as f32, crop_height as f32),
            (0.0, crop_height as f32),
        ];
        let crop_to_source = Projection::from_control_points(source_points, crop_points)?.invert();

        Some(Self {
            crop_to_source,
            crop_width: crop_width as f32,
            crop_height: crop_height as f32,
            vertical_rotation_applied,
        })
    }

    fn recognition_width(self) -> f32 {
        if self.vertical_rotation_applied {
            self.crop_height
        } else {
            self.crop_width
        }
    }
}

/// Page padding and pre-padding pixel dimensions, bundled so word-geometry projection
/// functions stay under the workspace parameter-count limit. ~keep
#[derive(Debug, Clone, Copy)]
struct PageGeometry {
    padding: u32,
    dimensions: (u32, u32),
}

/// Detection, classification and recognition options threaded through one detection pass.
/// Grouped so the private `detect_base_detailed`/`detect_once_detailed` helpers stay under the
/// workspace parameter-count limit; the public `detect*` methods below keep their existing
/// individual-argument signatures unchanged. ~keep
#[derive(Debug, Clone, Copy)]
struct DetectionOptions {
    padding: u32,
    box_score_thresh: f32,
    box_thresh: f32,
    un_clip_ratio: f32,
    do_angle: bool,
    most_angle: bool,
    angle_rollback: bool,
    angle_rollback_threshold: f32,
    cls_thresh: f32,
    rec_batch_size: u32,
}

#[derive(Debug)]
pub struct PaddleOcrEngine {
    db_net: DbNet,
    angle_net: AngleNet,
    crnn_net: CrnnNet,
}

impl Default for PaddleOcrEngine {
    fn default() -> Self {
        Self::new()
    }
}

impl PaddleOcrEngine {
    pub fn new() -> Self {
        Self {
            db_net: DbNet::new(),
            angle_net: AngleNet::new(),
            crnn_net: CrnnNet::new(),
        }
    }

    /// Recognize a source crop with the loaded recognizer and no detector or angle classifier.
    #[doc(hidden)]
    pub fn recognize_source_crop(&self, image: &image::RgbImage) -> Result<String, OcrError> {
        const HEIGHT: u32 = 48;
        let width = image.width().saturating_mul(HEIGHT) / image.height().max(1);
        let resized = image::imageops::resize(image, width.max(1), HEIGHT, image::imageops::FilterType::Triangle);
        self.crnn_net
            .get_text_lines(&[resized], &HashMap::new(), 0.0, 1)
            .map(|mut lines| lines.pop().map_or_else(String::new, |line| line.text))
    }

    pub fn init_models(
        &mut self,
        det_path: &str,
        cls_path: &str,
        rec_path: &str,
        num_thread: usize,
    ) -> Result<(), OcrError> {
        self.db_net.init_model(det_path, num_thread)?;
        self.angle_net.init_model(cls_path, num_thread)?;
        self.crnn_net.init_model(rec_path, num_thread)?;
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
        self.init_models_with_dict_on(
            crate::inference::default_backend(),
            det_path,
            cls_path,
            rec_path,
            dict_path,
            num_thread,
        )
    }

    /// Initialize all three models on an explicitly chosen inference backend.
    ///
    /// [`crate::inference::default_backend`] resolves at compile time and prefers `ort`
    /// whenever the `ort` feature is on. A build that compiles both engines — the native
    /// cross-engine parity configuration — therefore cannot reach `tract` through
    /// [`Self::init_models_with_dict`]; it would silently construct an `ort` engine. Callers
    /// that mean a specific engine must say so here.
    ///
    /// Detection needs no shape configuration on either engine: `DbNet` builds whatever plan
    /// the chosen backend needs for the extent each page resizes to (see
    /// [`DbNet::get_text_boxes`]).
    pub fn init_models_with_dict_on(
        &mut self,
        backend: crate::inference::Backend,
        det_path: &str,
        cls_path: &str,
        rec_path: &str,
        dict_path: &str,
        num_thread: usize,
    ) -> Result<(), OcrError> {
        self.db_net.init_model_on(backend, det_path, num_thread)?;
        self.angle_net.init_model_on(backend, cls_path, num_thread)?;
        self.crnn_net
            .init_model_dict_file_on(backend, rec_path, num_thread, dict_path)?;
        Ok(())
    }

    #[cfg(feature = "ort")]
    pub fn init_models_custom(
        &mut self,
        det_path: &str,
        cls_path: &str,
        rec_path: &str,
        builder_fn: fn(SessionBuilder) -> Result<SessionBuilder, ort::Error>,
    ) -> Result<(), OcrError> {
        self.db_net.init_model_with_ort_builder(det_path, 0, Some(builder_fn))?;
        self.angle_net
            .init_model_with_ort_builder(cls_path, 0, Some(builder_fn))?;
        self.crnn_net
            .init_model_with_ort_builder(rec_path, 0, Some(builder_fn))?;
        Ok(())
    }

    /// Initialize models with dictionary file and custom session builder.
    ///
    /// Combines `init_models_with_dict` and `init_models_custom`: loads the
    /// dictionary for the recognition model while applying a custom ORT
    /// session builder (e.g. for GPU execution providers).
    #[cfg(feature = "ort")]
    pub fn init_models_with_dict_custom(
        &mut self,
        det_path: &str,
        cls_path: &str,
        rec_path: &str,
        dict_path: &str,
        num_thread: usize,
        builder_fn: Option<fn(SessionBuilder) -> Result<SessionBuilder, ort::Error>>,
    ) -> Result<(), OcrError> {
        self.db_net
            .init_model_with_ort_builder(det_path, num_thread, builder_fn)?;
        self.angle_net
            .init_model_with_ort_builder(cls_path, num_thread, builder_fn)?;
        self.crnn_net
            .init_model_dict_file_with_ort_builder(rec_path, num_thread, builder_fn, dict_path)?;
        Ok(())
    }

    pub fn init_models_from_memory(
        &mut self,
        det_bytes: &[u8],
        cls_bytes: &[u8],
        rec_bytes: &[u8],
        num_thread: usize,
    ) -> Result<(), OcrError> {
        self.db_net.init_model_from_memory(det_bytes, num_thread)?;
        self.angle_net.init_model_from_memory(cls_bytes, num_thread)?;
        self.crnn_net.init_model_from_memory(rec_bytes, num_thread)?;
        Ok(())
    }

    #[cfg(feature = "ort")]
    pub fn init_models_from_memory_custom(
        &mut self,
        det_bytes: &[u8],
        cls_bytes: &[u8],
        rec_bytes: &[u8],
        builder_fn: fn(SessionBuilder) -> Result<SessionBuilder, ort::Error>,
    ) -> Result<(), OcrError> {
        self.db_net
            .init_model_from_memory_with_ort_builder(det_bytes, 0, Some(builder_fn))?;
        self.angle_net
            .init_model_from_memory_with_ort_builder(cls_bytes, 0, Some(builder_fn))?;
        self.crnn_net
            .init_model_from_memory_with_ort_builder(rec_bytes, 0, Some(builder_fn))?;
        Ok(())
    }

    fn detect_base_detailed(
        &self,
        img_src: &image::RgbImage,
        max_side_len: u32,
        options: DetectionOptions,
    ) -> Result<DetailedOcrResult, OcrError> {
        tracing::debug!(
            width = img_src.width(),
            height = img_src.height(),
            padding = options.padding,
            max_side_len = max_side_len,
            do_angle = options.do_angle,
            angle_rollback = options.angle_rollback,
            "PaddleOCR: starting detection"
        );

        let origin_max_side = img_src.width().max(img_src.height());
        let mut resize;
        if max_side_len == 0 || max_side_len > origin_max_side {
            resize = origin_max_side;
        } else {
            resize = max_side_len;
        }
        resize += 2 * options.padding;

        let padding_src = OcrUtils::make_padding(img_src, options.padding)?;

        let scale = ScaleParam::get_scale_param(&padding_src, resize);
        tracing::debug!(
            resize_width = scale.dst_width,
            resize_height = scale.dst_height,
            "PaddleOCR: image resized"
        );

        self.detect_once_detailed(&padding_src, &scale, options)
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
    const DEFAULT_CLS_THRESH: f32 = 0.9;
    const DEFAULT_REC_BATCH_SIZE: u32 = 6;

    pub fn detect(
        &self,
        img_src: &image::RgbImage,
        padding: u32,
        max_side_len: u32,
        box_score_thresh: f32,
        box_thresh: f32,
        un_clip_ratio: f32,
        do_angle: bool,
        most_angle: bool,
    ) -> Result<OcrResult, OcrError> {
        self.detect_detailed(
            img_src,
            padding,
            max_side_len,
            box_score_thresh,
            box_thresh,
            un_clip_ratio,
            do_angle,
            most_angle,
        )
        .map(Into::into)
    }

    /// Detect text and retain word-level recognition geometry.
    pub fn detect_detailed(
        &self,
        img_src: &image::RgbImage,
        padding: u32,
        max_side_len: u32,
        box_score_thresh: f32,
        box_thresh: f32,
        un_clip_ratio: f32,
        do_angle: bool,
        most_angle: bool,
    ) -> Result<DetailedOcrResult, OcrError> {
        self.detect_base_detailed(
            img_src,
            max_side_len,
            DetectionOptions {
                padding,
                box_score_thresh,
                box_thresh,
                un_clip_ratio,
                do_angle,
                most_angle,
                angle_rollback: false,
                angle_rollback_threshold: 0.0,
                cls_thresh: Self::DEFAULT_CLS_THRESH,
                rec_batch_size: Self::DEFAULT_REC_BATCH_SIZE,
            },
        )
    }

    /// Detect text with a configurable recognition batch size.
    #[allow(clippy::too_many_arguments)]
    pub fn detect_with_rec_batch_size(
        &self,
        img_src: &image::RgbImage,
        padding: u32,
        max_side_len: u32,
        box_score_thresh: f32,
        box_thresh: f32,
        un_clip_ratio: f32,
        do_angle: bool,
        most_angle: bool,
        rec_batch_size: u32,
    ) -> Result<OcrResult, OcrError> {
        self.detect_detailed_with_rec_batch_size(
            img_src,
            padding,
            max_side_len,
            box_score_thresh,
            box_thresh,
            un_clip_ratio,
            do_angle,
            most_angle,
            rec_batch_size,
        )
        .map(Into::into)
    }

    /// Detect text with detailed word geometry and a configurable recognition batch size.
    #[allow(clippy::too_many_arguments)]
    pub fn detect_detailed_with_rec_batch_size(
        &self,
        img_src: &image::RgbImage,
        padding: u32,
        max_side_len: u32,
        box_score_thresh: f32,
        box_thresh: f32,
        un_clip_ratio: f32,
        do_angle: bool,
        most_angle: bool,
        rec_batch_size: u32,
    ) -> Result<DetailedOcrResult, OcrError> {
        self.detect_base_detailed(
            img_src,
            max_side_len,
            DetectionOptions {
                padding,
                box_score_thresh,
                box_thresh,
                un_clip_ratio,
                do_angle,
                most_angle,
                angle_rollback: false,
                angle_rollback_threshold: 0.0,
                cls_thresh: Self::DEFAULT_CLS_THRESH,
                rec_batch_size,
            },
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
        &self,
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
        self.detect_detailed_angle_rollback(
            img_src,
            padding,
            max_side_len,
            box_score_thresh,
            box_thresh,
            un_clip_ratio,
            do_angle,
            most_angle,
            angle_rollback_threshold,
        )
        .map(Into::into)
    }

    /// Detect detailed word geometry with angle rollback support.
    #[allow(clippy::too_many_arguments)]
    pub fn detect_detailed_angle_rollback(
        &self,
        img_src: &image::RgbImage,
        padding: u32,
        max_side_len: u32,
        box_score_thresh: f32,
        box_thresh: f32,
        un_clip_ratio: f32,
        do_angle: bool,
        most_angle: bool,
        angle_rollback_threshold: f32,
    ) -> Result<DetailedOcrResult, OcrError> {
        self.detect_base_detailed(
            img_src,
            max_side_len,
            DetectionOptions {
                padding,
                box_score_thresh,
                box_thresh,
                un_clip_ratio,
                do_angle,
                most_angle,
                angle_rollback: true,
                angle_rollback_threshold,
                cls_thresh: Self::DEFAULT_CLS_THRESH,
                rec_batch_size: Self::DEFAULT_REC_BATCH_SIZE,
            },
        )
    }

    pub fn detect_from_path(
        &self,
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

    /// Sort text boxes in reading order: top-to-bottom, left-to-right.
    ///
    /// Matches PaddleOCR's stable primary ordering and visual-line correction. ~keep
    fn sort_text_boxes(text_boxes: &mut [TextBox]) {
        text_boxes.sort_by(|a, b| {
            let ay = a.points.first().map_or(0, |p| p.y);
            let ax = a.points.first().map_or(0, |p| p.x);
            let by = b.points.first().map_or(0, |p| p.y);
            let bx = b.points.first().map_or(0, |p| p.x);
            (ay, ax).cmp(&(by, bx))
        });

        for current in 1..text_boxes.len() {
            let mut index = current;
            while index > 0 {
                let previous_point = text_boxes[index - 1].points.first();
                let current_point = text_boxes[index].points.first();
                let should_swap = previous_point.zip(current_point).is_some_and(|(previous, current)| {
                    previous.y.abs_diff(current.y) < VISUAL_LINE_Y_TOLERANCE_PX && current.x < previous.x
                });
                if !should_swap {
                    break;
                }
                text_boxes.swap(index - 1, index);
                index -= 1;
            }
        }
    }

    /// Run DB-net detection, sort boxes into reading order, and precompute each region's crop
    /// geometry for later word-bound projection. ~keep
    fn detect_and_project(
        &self,
        img_src: &image::RgbImage,
        scale: &ScaleParam,
        box_score_thresh: f32,
        box_thresh: f32,
        un_clip_ratio: f32,
    ) -> Result<DetectedRegions, OcrError> {
        tracing::debug!("PaddleOCR: running DB-net text detection");
        let mut text_boxes = self
            .db_net
            .get_text_boxes(img_src, scale, box_score_thresh, box_thresh, un_clip_ratio)?;

        tracing::debug!(
            num_boxes = text_boxes.len(),
            min_score = text_boxes.iter().map(|b| b.score).fold(f32::INFINITY, f32::min),
            max_score = text_boxes.iter().map(|b| b.score).fold(f32::NEG_INFINITY, f32::max),
            "PaddleOCR: detection complete"
        );

        Self::sort_text_boxes(&mut text_boxes);

        let part_images = OcrUtils::get_part_images(img_src, &text_boxes);
        let crop_projections = text_boxes
            .iter()
            .zip(&part_images)
            .map(|(text_box, part_image)| CropProjectionMetadata::new(&text_box.points, part_image))
            .collect::<Vec<_>>();

        Ok((text_boxes, part_images, crop_projections))
    }

    fn detect_once_detailed(
        &self,
        img_src: &image::RgbImage,
        scale: &ScaleParam,
        options: DetectionOptions,
    ) -> Result<DetailedOcrResult, OcrError> {
        let DetectionOptions {
            padding,
            box_score_thresh,
            box_thresh,
            un_clip_ratio,
            do_angle,
            most_angle,
            angle_rollback,
            angle_rollback_threshold,
            cls_thresh,
            rec_batch_size,
        } = options;

        let (text_boxes, part_images, crop_projections) =
            self.detect_and_project(img_src, scale, box_score_thresh, box_thresh, un_clip_ratio)?;

        if do_angle {
            tracing::debug!(num_regions = part_images.len(), "PaddleOCR: running angle classifier");
        }

        let angles = self
            .angle_net
            .get_angles(&part_images, do_angle, most_angle, cls_thresh)?;

        if do_angle {
            let rotated_count = angles.iter().filter(|a| a.index != 0).count();
            tracing::debug!(
                rotated_regions = rotated_count,
                "PaddleOCR: angle classification complete"
            );
        }

        let (rotated_images, angle_rollback_records) =
            Self::rotate_for_recognition(&angles, part_images, angle_rollback);

        tracing::debug!(
            num_regions = rotated_images.len(),
            "PaddleOCR: running CRNN text recognition"
        );
        let (text_lines, rollback_applied) = self.recognize_detailed_with_rollbacks(
            &rotated_images,
            &angle_rollback_records,
            angle_rollback_threshold,
            rec_batch_size,
        )?;

        if text_lines.is_empty() {
            tracing::warn!("PaddleOCR: no text recognized");
        } else {
            let avg_score = text_lines.iter().map(|l| l.line.text_score).sum::<f32>() / text_lines.len() as f32;
            let empty_count = text_lines.iter().filter(|l| l.line.text.trim().is_empty()).count();
            tracing::debug!(
                num_lines = text_lines.len(),
                avg_score = avg_score,
                empty_lines = empty_count,
                "PaddleOCR: recognition complete"
            );
        }

        let text_blocks = Self::build_text_blocks(
            &text_boxes,
            &angles,
            text_lines,
            &rollback_applied,
            &crop_projections,
            PageGeometry {
                padding,
                dimensions: img_src.dimensions(),
            },
        );

        Ok(DetailedOcrResult { text_blocks })
    }

    /// Rotate 180° any region the angle classifier flagged upside-down, recording a
    /// pre-rotation copy for `angle_rollback` (see [`Self::recognize_detailed_with_rollbacks`])
    /// before doing so. ~keep
    fn rotate_for_recognition(
        angles: &[Angle],
        part_images: Vec<image::RgbImage>,
        angle_rollback: bool,
    ) -> RotatedRegions {
        let mut rotated_images: Vec<image::RgbImage> = Vec::with_capacity(part_images.len());
        let mut angle_rollback_records = HashMap::<usize, ImageBuffer<image::Rgb<u8>, Vec<u8>>>::new();

        for (index, (angle, mut part_image)) in angles.iter().zip(part_images).enumerate() {
            if angle.index == 1 {
                if angle_rollback {
                    angle_rollback_records.insert(index, part_image.clone());
                }

                OcrUtils::mat_rotate_clock_wise_180(&mut part_image);
            }
            rotated_images.push(part_image);
        }

        (rotated_images, angle_rollback_records)
    }

    /// Assemble the final [`DetailedTextBlock`]s from one detection pass's per-region outputs;
    /// `text_boxes`, `angles`, `text_lines` and `crop_projections` are all aligned to the same
    /// box/line index. ~keep
    fn build_text_blocks(
        text_boxes: &[TextBox],
        angles: &[Angle],
        text_lines: Vec<DetailedTextLine>,
        rollback_applied: &[bool],
        crop_projections: &[Option<CropProjectionMetadata>],
        page: PageGeometry,
    ) -> Vec<DetailedTextBlock> {
        let mut text_blocks = Vec::with_capacity(text_lines.len());
        for (i, text_line) in text_lines.into_iter().enumerate() {
            let rotation_retained = Self::rotation_retained(angles[i].index, rollback_applied[i]);
            let DetailedTextLine {
                line,
                words: recognized_words,
                line_column_count,
            } = text_line;
            let words = Self::build_word_blocks(
                &line,
                recognized_words,
                line_column_count,
                crop_projections[i],
                rotation_retained,
                page,
            );
            let block = TextBlock {
                box_points: text_boxes[i]
                    .points
                    .iter()
                    .map(|p| Point {
                        x: ((p.x as f32) - page.padding as f32) as u32,
                        y: ((p.y as f32) - page.padding as f32) as u32,
                    })
                    .collect(),
                box_score: text_boxes[i].score,
                angle_index: angles[i].index,
                angle_score: angles[i].score,
                text: line.text,
                text_score: line.text_score,
            };
            text_blocks.push(DetailedTextBlock {
                block,
                words,
                line_column_count,
                rotation_retained,
            });
        }
        text_blocks
    }

    fn recognize_detailed_with_rollbacks(
        &self,
        rotated_images: &[image::RgbImage],
        rollback_records: &HashMap<usize, image::RgbImage>,
        rollback_threshold: f32,
        batch_size: u32,
    ) -> Result<(Vec<DetailedTextLine>, Vec<bool>), OcrError> {
        let no_rollback_records = HashMap::new();
        let mut lines = self.crnn_net.get_detailed_text_lines(
            rotated_images,
            &no_rollback_records,
            rollback_threshold,
            batch_size,
        )?;
        let rollback_indices = lines
            .iter()
            .enumerate()
            .filter_map(|(index, line)| {
                let should_rollback = line.line.text_score.is_nan() || line.line.text_score < rollback_threshold;
                should_rollback
                    .then(|| rollback_records.get(&index).map(|image| (index, image.clone())))
                    .flatten()
            })
            .collect::<Vec<_>>();
        let mut rollback_applied = vec![false; lines.len()];
        if rollback_indices.is_empty() {
            return Ok((lines, rollback_applied));
        }

        let images = rollback_indices
            .iter()
            .map(|(_, image)| image.clone())
            .collect::<Vec<_>>();
        let replacements =
            self.crnn_net
                .get_detailed_text_lines(&images, &no_rollback_records, rollback_threshold, batch_size)?;
        for ((index, _), replacement) in rollback_indices.into_iter().zip(replacements) {
            lines[index] = replacement;
            rollback_applied[index] = true;
        }
        Ok((lines, rollback_applied))
    }

    fn rotation_retained(angle_index: i32, rollback_applied: bool) -> bool {
        angle_index == 1 && !rollback_applied
    }

    fn build_word_blocks(
        line: &crate::ocr_result::TextLine,
        words: Vec<RecognizedWord>,
        line_column_count: f32,
        projection: Option<CropProjectionMetadata>,
        rotation_retained: bool,
        page: PageGeometry,
    ) -> Vec<WordBlock> {
        let Some(projection) = projection else {
            return words.into_iter().map(Into::into).collect();
        };
        let recognition_width = projection.recognition_width();
        let Some(average_character_width) =
            Self::average_character_width(line, &words, line_column_count, recognition_width)
        else {
            return words.into_iter().map(Into::into).collect();
        };
        let column_width = recognition_width / line_column_count;
        let mut bounds = words
            .iter()
            .map(|word| Self::word_horizontal_bounds(word, line_column_count, column_width, average_character_width))
            .collect::<Vec<_>>();
        Self::adjust_word_bound_overlaps(&mut bounds);

        words
            .into_iter()
            .zip(bounds)
            .map(|(word, bounds)| {
                let box_points = bounds
                    .and_then(|bounds| {
                        Self::project_word_bounds(projection, bounds, rotation_retained, page.padding, page.dimensions)
                    })
                    .unwrap_or_default();
                WordBlock { word, box_points }
            })
            .collect()
    }

    fn adjust_word_bound_overlaps(bounds: &mut [Option<(f32, f32)>]) {
        for index in 0..bounds.len().saturating_sub(1) {
            let (previous, next) = bounds.split_at_mut(index + 1);
            let (Some((_, previous_right)), Some((next_left, _))) = (&mut previous[index], &mut next[0]) else {
                continue;
            };
            if *previous_right > *next_left {
                let midpoint = (*previous_right + *next_left) / 2.0;
                *previous_right = midpoint;
                *next_left = midpoint;
            }
        }
    }

    fn average_character_width(
        line: &crate::ocr_result::TextLine,
        words: &[RecognizedWord],
        line_column_count: f32,
        recognition_width: f32,
    ) -> Option<f32> {
        if !line_column_count.is_finite()
            || line_column_count <= 0.0
            || !recognition_width.is_finite()
            || recognition_width <= 0.0
        {
            return None;
        }
        let column_width = recognition_width / line_column_count;
        let widths = words.iter().filter_map(|word| {
            let (first, last) = Self::effective_word_column_span(word, line_column_count)?;
            (word.columns.len() > 1 && last > first)
                .then_some((last - first) * column_width / (word.columns.len() - 1) as f32)
        });
        let (width_sum, width_count) = widths.fold((0.0, 0usize), |(sum, count), width| (sum + width, count + 1));
        let average_width = if width_count > 0 {
            width_sum / width_count as f32
        } else {
            recognition_width / line.text.chars().count() as f32
        };
        (average_width.is_finite() && average_width > 0.0).then_some(average_width)
    }

    fn word_horizontal_bounds(
        word: &RecognizedWord,
        line_column_count: f32,
        column_width: f32,
        average_character_width: f32,
    ) -> Option<(f32, f32)> {
        let (first, last) = Self::effective_word_column_span(word, line_column_count)?;
        if last < first || !column_width.is_finite() || !average_character_width.is_finite() {
            return None;
        }
        let half_width = average_character_width / 2.0;
        let left = (((first + 0.5) * column_width - half_width).trunc()).max(0.0);
        let right = ((last + 0.5) * column_width + half_width).trunc();
        (right > left).then_some((left, right))
    }

    fn effective_word_column_span(word: &RecognizedWord, line_column_count: f32) -> Option<(f32, f32)> {
        let (&first, &last) = word.columns.first().zip(word.columns.last())?;
        if !line_column_count.is_finite() || line_column_count <= 0.0 {
            return None;
        }
        let max_column = (line_column_count.ceil() - 1.0).max(0.0);
        Some(((first as f32).min(max_column), (last as f32).min(max_column)))
    }

    fn project_word_bounds(
        projection: CropProjectionMetadata,
        bounds: (f32, f32),
        rotation_retained: bool,
        padding: u32,
        padded_dimensions: (u32, u32),
    ) -> Option<Vec<Point>> {
        let recognition_width = projection.recognition_width();
        let (mut left, mut right) = bounds;
        right = right.min(recognition_width);
        if rotation_retained {
            (left, right) = (recognition_width - right, recognition_width - left);
        }
        if !left.is_finite() || !right.is_finite() || left < 0.0 || right <= left {
            return None;
        }

        let crop_points = if projection.vertical_rotation_applied {
            [
                (0.0, left),
                (projection.crop_width, left),
                (projection.crop_width, right),
                (0.0, right),
            ]
        } else {
            [
                (left, 0.0),
                (right, 0.0),
                (right, projection.crop_height),
                (left, projection.crop_height),
            ]
        };
        let original_width = padded_dimensions.0.saturating_sub(padding.saturating_mul(2));
        let original_height = padded_dimensions.1.saturating_sub(padding.saturating_mul(2));
        let points = crop_points
            .into_iter()
            .map(|point| projection.crop_to_source * point)
            .map(|(x, y)| Self::unpad_projected_point(x, y, padding, original_width, original_height))
            .collect::<Option<Vec<_>>>()?;

        (Self::quadrilateral_area(&points) > 0.0).then_some(points)
    }

    fn unpad_projected_point(x: f32, y: f32, padding: u32, original_width: u32, original_height: u32) -> Option<Point> {
        if !x.is_finite() || !y.is_finite() {
            return None;
        }
        let x = (x - padding as f32).clamp(0.0, original_width as f32).trunc() as u32;
        let y = (y - padding as f32).clamp(0.0, original_height as f32).trunc() as u32;
        Some(Point { x, y })
    }

    fn quadrilateral_area(points: &[Point]) -> f32 {
        if points.len() != 4 {
            return 0.0;
        }
        let mut twice_area = 0.0;
        for index in 0..points.len() {
            let current = points[index];
            let next = points[(index + 1) % points.len()];
            twice_area += current.x as f32 * next.y as f32 - next.x as f32 * current.y as f32;
        }
        twice_area.abs() / 2.0
    }
}

/// Backward-compatible name for [`PaddleOcrEngine`].
#[deprecated(note = "use PaddleOcrEngine")]
pub type OcrLite = PaddleOcrEngine;

#[cfg(test)]
#[path = "ocr_lite/tests.rs"]
mod tests;
