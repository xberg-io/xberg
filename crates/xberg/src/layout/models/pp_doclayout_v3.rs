//! PP-DocLayout-V3 layout detection model.
//!
//! PP-DocLayout-V3 is a PaddleDetection DETR-based layout detection model that identifies
//! 25 document region classes. The ONNX is produced via the PaddleDetection inference
//! export pipeline.
//!
//! Model: PP-DocLayout-V3 (PaddleDetection DETR export)
//! Inference resolution: 800 × 800
//!
//! ## Inputs (3, all f32, named; pass by name — do not assume order)
//! - `image`        — `[batch, 3, H, W]` f32, resize to 800×800, normalise by /255 (no ImageNet
//!                    mean/std), CHW layout.
//! - `im_shape`     — `[batch, 2]` f32: **resized** input dimensions — always `[800.0, 800.0]`
//!                    (PaddleDetection convention: this is H×W of the tensor fed in, not the
//!                    original image size).
//! - `scale_factor` — `[batch, 2]` f32: `[resized_H / orig_H, resized_W / orig_W]`
//!                    (PaddleDetection convention: the model divides output box coords by these
//!                    values to map them back to original-image pixel space).
//!
//! The model uses `im_shape` + `scale_factor` to unscale boxes back to original-image
//! pixel coordinates internally — output boxes are already in original pixel space.
//!
//! ## Empirical verification
//! With `im_shape=[800,800]` and `scale_factor=[800/orig_h, 800/orig_w]`, the output
//! bounding boxes fall within the original image bounds. With `im_shape=[orig_h, orig_w]`
//! the coordinates overflow (they become ~`orig_h/scale`-sized), confirming that the
//! model expects the **resized** shape, not the original shape, in `im_shape`.
//!
//! ## Outputs (3)
//! - `fetch_name_0` — `[N_total, 7]` f32: detection rows in PaddleDetection NMS format.
//!                    Empirically confirmed column layout:
//!                    `[class_id(f32), score(f32), x1, y1, x2, y2, raw_rank]`
//!                    where class_id ∈ 0..24 (25-class taxonomy, see below) and
//!                    coordinates are in original image pixel space.
//!                    The validated private view preserves zero-based ranks and source-row identity;
//!                    the legacy public projection still returns confidence-sorted boxes.
//! - `fetch_name_1` — `[1]` i32: singleton top-k row count, before confidence filtering.
//! - `fetch_name_2` — `[N_total, 200, 200]` i32: binary full-canvas masks paired with the same rows.
//!                    Shape/value validation precedes projection; public detections omit masks.
//!
//! ## 25-class taxonomy (index → canonical name)
//! 0 abstract, 1 algorithm, 2 aside_text, 3 chart, 4 content, 5 display_formula,
//! 6 doc_title, 7 figure_title, 8 footer, 9 footer_image, 10 footnote, 11 formula_number,
//! 12 header, 13 header_image, 14 image, 15 inline_formula, 16 number, 17 paragraph_title,
//! 18 reference, 19 reference_content, 20 seal, 21 table, 22 text, 23 vertical_text,
//! 24 vision_footnote.

use std::time::Instant;

use image::RgbImage;
use ndarray::{Array2, Array4};

use crate::inference::{InferenceSession, InferenceTensor, default_backend};
use crate::layout::error::LayoutError;
use crate::layout::models::LayoutModel;
use crate::layout::types::{BBox, LayoutClass, LayoutDetection};

mod output;
mod preprocessing;

/// Default confidence threshold for PP-DocLayout-V3 detections.
const DEFAULT_THRESHOLD: f32 = 0.5;

/// PP-DocLayout-V3 input resolution (800 × 800).
pub(crate) const INPUT_SIZE: u32 = 800;

/// Number of columns in `fetch_name_0` rows (empirically confirmed: 7).
const DET_ROW_COLS: usize = 7;

/// Column index of the class ID in each detection row.
const COL_CLASS: usize = 0;
/// Column index of the confidence score in each detection row.
const COL_SCORE: usize = 1;
/// Column index of x1 (left edge) in each detection row.
const COL_X1: usize = 2;
/// Column index of y1 (top edge) in each detection row.
const COL_Y1: usize = 3;
/// Column index of x2 (right edge) in each detection row.
const COL_X2: usize = 4;
/// Column index of y2 (bottom edge) in each detection row.
const COL_Y2: usize = 5;

/// PP-DocLayout-V3 layout detection model.
///
/// Uses PaddleDetection DETR architecture with 800 × 800 input. Preprocessing
/// squash-resizes to 800 × 800 (/255 normalisation only; no ImageNet mean/std),
/// and passes `im_shape`/`scale_factor` so the model itself unscales output boxes
/// to original-image pixel coordinates.
#[cfg_attr(alef, alef(skip))]
pub struct PpDocLayoutV3Model {
    session: Box<dyn InferenceSession>,
    /// Cached input names in session order (looked up by name at runtime).
    input_names: Vec<String>,
}

impl PpDocLayoutV3Model {
    /// Load a PP-DocLayout-V3 ONNX model from a file.
    ///
    /// The session (optimization level, thread budget, execution-provider
    /// selection, and CPU-only fallback) is built by the [`crate::inference`]
    /// seam's default backend, so the model is engine-neutral.
    #[cfg(feature = "candle-glm-ocr")]
    pub(crate) fn from_file(
        path: &str,
        accel: Option<&crate::core::config::acceleration::AccelerationConfig>,
    ) -> Result<Self, LayoutError> {
        Self::from_file_with_thread_budget(
            path,
            accel,
            crate::core::config::concurrency::resolve_thread_budget(None),
        )
    }

    pub(crate) fn from_file_with_thread_budget(
        path: &str,
        accel: Option<&crate::core::config::acceleration::AccelerationConfig>,
        thread_budget: usize,
    ) -> Result<Self, LayoutError> {
        let session = default_backend()
            .load_with_thread_budget(std::path::Path::new(path), accel, thread_budget)
            .map_err(|e| LayoutError::Inference(e.to_string()))?;
        let input_names: Vec<String> = session.input_names().to_vec();
        Ok(Self { session, input_names })
    }

    /// Map PP-DocLayout-V3 class ID (0-24) to [`LayoutClass`].
    ///
    /// The 25-class taxonomy is:
    /// 0 abstract, **1 algorithm**, 2 aside_text, **3 chart**, **4 content**,
    /// **5 display_formula**, 6 doc_title, **7 figure_title**, 8 footer, 9 footer_image,
    /// **10 footnote**, **11 formula_number**, 12 header, 13 header_image, **14 image**,
    /// **15 inline_formula**, 16 number, **17 paragraph_title**, 18 reference,
    /// 19 reference_content, 20 seal, **21 table**, **22 text**, 23 vertical_text,
    /// **24 vision_footnote**.
    ///
    /// `content` (4) is PP-DocLayout-V3's table-of-contents region, so it maps to
    /// [`LayoutClass::DocumentIndex`] rather than `Text`. `figure_title` (7) is a
    /// figure/table caption, not a document title, so it maps to
    /// [`LayoutClass::Caption`] and no longer shares a bucket with `doc_title` (6)
    /// / `paragraph_title` (17). `footnote` (10) and `vision_footnote` (24) map to
    /// [`LayoutClass::Footnote`]. `algorithm` (1) is a code-like listing and maps
    /// to [`LayoutClass::Code`]. This keeps PP-DocLayout-V3 consistent with
    /// RT-DETR's mapping for the classes both taxonomies share (#178).
    fn class_from_id(id: i64) -> Option<LayoutClass> {
        match id {
            1 => Some(LayoutClass::Code),
            3 => Some(LayoutClass::Chart),
            4 => Some(LayoutClass::DocumentIndex),
            5 | 11 | 15 => Some(LayoutClass::Formula),
            6 | 17 => Some(LayoutClass::Title),
            7 => Some(LayoutClass::Caption),
            8 | 9 => Some(LayoutClass::PageFooter),
            10 | 24 => Some(LayoutClass::Footnote),
            12 | 13 => Some(LayoutClass::PageHeader),
            14 | 20 => Some(LayoutClass::Picture),
            21 => Some(LayoutClass::Table),
            0 | 2 | 16 | 18 | 19 | 22 | 23 => Some(LayoutClass::Text),
            _ => None,
        }
    }

    /// Look up a named input and return a cloned `String` for use in `inputs![]`.
    ///
    /// Returns the exact match when found; falls back to the provided positional
    /// default name so the model still works if the ONNX renames inputs.
    fn resolve_input_name(&self, canonical: &str, fallback_pos: usize) -> String {
        self.input_names
            .iter()
            .find(|n| n.as_str() == canonical)
            .cloned()
            .unwrap_or_else(|| {
                self.input_names
                    .get(fallback_pos)
                    .cloned()
                    .unwrap_or_else(|| canonical.to_owned())
            })
    }

    /// Preprocess a single image for PP-DocLayout-V3.
    ///
    /// Returns `(pixel_tensor [1,3,800,800], im_shape [1,2], scale_factor [1,2])`.
    ///
    /// Preprocessing steps:
    /// 1. Fixed-support Keys cubic resize to 800 × 800 with replicated borders,
    ///    matching PaddleX's `interp=2` and `keep_ratio=False`.
    /// 2. Convert to f32, divide by 255.0 — mean=0 std=1 (no ImageNet normalisation).
    /// 3. Permute HWC → CHW.
    ///
    /// `im_shape` is the **resized** input size `[800.0, 800.0]` (PaddleDetection convention:
    /// the model expects the tensor dimensions here, not the original image size).
    /// `scale_factor` is `[800/orig_h, 800/orig_w]`; the model divides output box coords by
    /// these values to restore original-image pixel coordinates.
    fn preprocess_single(img: &RgbImage) -> Result<(Array4<f32>, Array2<f32>, Array2<f32>), LayoutError> {
        let orig_w = img.width() as f32;
        let orig_h = img.height() as f32;
        let ts = INPUT_SIZE;

        let pixel_tensor = preprocessing::preprocess_cubic(img, ts)?;

        let scale_h = ts as f32 / orig_h;
        let scale_w = ts as f32 / orig_w;

        let im_shape = Array2::from_shape_vec((1, 2), vec![ts as f32, ts as f32]).expect("im_shape shape mismatch");
        let scale_factor = Array2::from_shape_vec((1, 2), vec![scale_h, scale_w]).expect("scale_factor shape mismatch");

        Ok((pixel_tensor, im_shape, scale_factor))
    }

    /// Parse valid detection rows from `fetch_name_0` for a single image.
    ///
    /// `rows` is the flat slice covering exactly `n_valid` rows × `DET_ROW_COLS` columns.
    /// Coordinates are already in original-image pixel space (unscaled by the model).
    #[cfg(test)]
    fn parse_detections(
        rows: &[f32],
        n_valid: usize,
        threshold: f32,
        orig_w: u32,
        orig_h: u32,
    ) -> Vec<LayoutDetection> {
        Self::project_detections(
            rows.chunks_exact(DET_ROW_COLS)
                .take(n_valid)
                .map(ndarray::ArrayView1::from),
            threshold,
            orig_w,
            orig_h,
        )
    }

    fn project_detections<'a>(
        rows: impl Iterator<Item = ndarray::ArrayView1<'a, f32>>,
        threshold: f32,
        orig_w: u32,
        orig_h: u32,
    ) -> Vec<LayoutDetection> {
        let max_w = orig_w as f32;
        let max_h = orig_h as f32;
        let mut detections = Vec::new();
        for row in rows {
            let score = row[COL_SCORE];
            if score < threshold {
                continue;
            }
            let class = match Self::class_from_id(row[COL_CLASS] as i64) {
                Some(class) => class,
                None => continue,
            };
            let x1 = row[COL_X1].clamp(0.0, max_w);
            let y1 = row[COL_Y1].clamp(0.0, max_h);
            let x2 = row[COL_X2].clamp(0.0, max_w);
            let y2 = row[COL_Y2].clamp(0.0, max_h);
            detections.push(LayoutDetection::new(class, score, BBox::new(x1, y1, x2, y2)));
        }
        LayoutDetection::sort_by_confidence_desc(detections)
    }

    /// Run single-image inference and return layout detections.
    fn run_inference(&mut self, img: &RgbImage, threshold: f32) -> Result<Vec<LayoutDetection>, LayoutError> {
        #[cfg(feature = "otel")]
        let inference_span = crate::telemetry::spans::model_inference_span("pp-doclayout-v3");
        #[cfg(feature = "otel")]
        let _inference_guard = inference_span.enter();
        #[cfg(feature = "otel")]
        let inference_start = Instant::now();

        let orig_w = img.width();
        let orig_h = img.height();

        let preprocess_start = Instant::now();

        let (pixel_array, im_shape_array, scale_factor_array) = Self::preprocess_single(img)?;

        let preprocess_ms = preprocess_start.elapsed().as_secs_f64() * 1000.0;
        tracing::debug!(preprocess_ms, "PP-DocLayout-V3 preprocessing complete");

        let onnx_start = Instant::now();

        let im_shape_name = self.resolve_input_name("im_shape", 0);
        let image_name = self.resolve_input_name("image", 1);
        let scale_factor_name = self.resolve_input_name("scale_factor", 2);

        let outputs = self
            .session
            .run(vec![
                (im_shape_name, InferenceTensor::F32(im_shape_array.into_dyn())),
                (image_name, InferenceTensor::F32(pixel_array.into_dyn())),
                (scale_factor_name, InferenceTensor::F32(scale_factor_array.into_dyn())),
            ])
            .map_err(|e| LayoutError::Inference(e.to_string()))?;

        let onnx_ms = onnx_start.elapsed().as_secs_f64() * 1000.0;
        tracing::debug!(onnx_ms, "PP-DocLayout-V3 ONNX session.run() complete");

        let validated = output::ValidatedOutput::from_outputs(&outputs)?;
        let n_valid = validated.count();
        let detections = Self::project_detections(
            validated.rows().map(|(_source_row, row, _mask)| row),
            threshold,
            orig_w,
            orig_h,
        );

        crate::layout::inference_timings::set(preprocess_ms, onnx_ms);

        tracing::debug!(
            preprocess_ms,
            onnx_ms,
            n_valid,
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

    /// Run requests as ordered single-image forwards.
    ///
    /// The upstream masked postprocessor supports B1 only; keep that contract
    /// until exported native batching is qualified separately (#2284).
    pub(crate) fn run_batch_inference(
        &mut self,
        images: &[&RgbImage],
        threshold: f32,
    ) -> Result<Vec<Vec<LayoutDetection>>, LayoutError> {
        crate::layout::inference_timings::take();
        if let Some(empty) = empty_batch_short_circuit(images) {
            return Ok(empty);
        }
        let mut results = Vec::with_capacity(images.len());
        let mut preprocess_ms = 0.0;
        let mut onnx_ms = 0.0;
        for image in images {
            let detections = self.run_inference(image, threshold);
            let (preprocess, onnx) = crate::layout::inference_timings::take();
            results.push(detections?);
            preprocess_ms += preprocess;
            onnx_ms += onnx;
        }
        let batch = images.len() as f64;
        crate::layout::inference_timings::set(preprocess_ms / batch, onnx_ms / batch);
        Ok(results)
    }
}

/// Empty-batch short-circuit for [`LayoutModel::detect_batch`].
///
/// Returns `Some(Vec::new())` for an empty input slice and `None` when
/// inference must proceed. Extracted as a pure function so the short-circuit
/// contract is unit-testable without constructing a model.
fn empty_batch_short_circuit(images: &[&RgbImage]) -> Option<Vec<Vec<LayoutDetection>>> {
    if images.is_empty() { Some(Vec::new()) } else { None }
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
        self.run_batch_inference(images, threshold.unwrap_or(DEFAULT_THRESHOLD))
    }

    fn name(&self) -> &str {
        "PP-DocLayout-V3"
    }
}

#[cfg(test)]
#[path = "pp_doclayout_v3/preprocessing_tests.rs"]
mod preprocessing_tests;

#[cfg(test)]
#[path = "pp_doclayout_v3/batch_contract_tests.rs"]
mod batch_contract_tests;

#[cfg(test)]
#[path = "pp_doclayout_v3/output_contract_tests.rs"]
mod output_contract_tests;

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn class_from_id_chart_maps_to_chart() {
        assert_eq!(PpDocLayoutV3Model::class_from_id(3), Some(LayoutClass::Chart));
    }

    #[test]
    fn class_from_id_display_formula_maps_to_formula() {
        assert_eq!(PpDocLayoutV3Model::class_from_id(5), Some(LayoutClass::Formula));
    }

    #[test]
    fn class_from_id_formula_number_maps_to_formula() {
        assert_eq!(PpDocLayoutV3Model::class_from_id(11), Some(LayoutClass::Formula));
    }

    #[test]
    fn class_from_id_inline_formula_maps_to_formula() {
        assert_eq!(PpDocLayoutV3Model::class_from_id(15), Some(LayoutClass::Formula));
    }

    #[test]
    fn class_from_id_doc_title_maps_to_title() {
        assert_eq!(PpDocLayoutV3Model::class_from_id(6), Some(LayoutClass::Title));
    }

    /// #178: `figure_title` (7) is a figure/table caption, not a document title.
    /// It must not share a bucket with `doc_title` (6) / `paragraph_title` (17).
    #[test]
    fn class_from_id_figure_title_maps_to_caption() {
        assert_eq!(PpDocLayoutV3Model::class_from_id(7), Some(LayoutClass::Caption));
    }

    #[test]
    fn class_from_id_paragraph_title_maps_to_title() {
        assert_eq!(PpDocLayoutV3Model::class_from_id(17), Some(LayoutClass::Title));
    }

    /// #178: `footnote` (10) and `vision_footnote` (24) must map to
    /// `LayoutClass::Footnote`, which exists and is consumed by
    /// `layout_hints::map_class`, rather than being collapsed into `Text`.
    #[test]
    fn class_from_id_footnote_classes_map_to_footnote() {
        for id in [10i64, 24] {
            assert_eq!(
                PpDocLayoutV3Model::class_from_id(id),
                Some(LayoutClass::Footnote),
                "class_id {id} should map to Footnote"
            );
        }
    }

    /// #178: `content` (4) is the table-of-contents region and must map to
    /// `LayoutClass::DocumentIndex`, matching RT-DETR's dedicated TOC class.
    #[test]
    fn class_from_id_content_maps_to_document_index() {
        assert_eq!(PpDocLayoutV3Model::class_from_id(4), Some(LayoutClass::DocumentIndex));
    }

    /// #178: `algorithm` (1) is a code-like listing and must map to
    /// `LayoutClass::Code`, matching RT-DETR's dedicated code class.
    #[test]
    fn class_from_id_algorithm_maps_to_code() {
        assert_eq!(PpDocLayoutV3Model::class_from_id(1), Some(LayoutClass::Code));
    }

    #[test]
    fn class_from_id_footer_maps_to_page_footer() {
        assert_eq!(PpDocLayoutV3Model::class_from_id(8), Some(LayoutClass::PageFooter));
    }

    #[test]
    fn class_from_id_footer_image_maps_to_page_footer() {
        assert_eq!(PpDocLayoutV3Model::class_from_id(9), Some(LayoutClass::PageFooter));
    }

    #[test]
    fn class_from_id_header_maps_to_page_header() {
        assert_eq!(PpDocLayoutV3Model::class_from_id(12), Some(LayoutClass::PageHeader));
    }

    #[test]
    fn class_from_id_header_image_maps_to_page_header() {
        assert_eq!(PpDocLayoutV3Model::class_from_id(13), Some(LayoutClass::PageHeader));
    }

    #[test]
    fn class_from_id_image_maps_to_picture() {
        assert_eq!(PpDocLayoutV3Model::class_from_id(14), Some(LayoutClass::Picture));
    }

    #[test]
    fn class_from_id_seal_maps_to_picture() {
        assert_eq!(PpDocLayoutV3Model::class_from_id(20), Some(LayoutClass::Picture));
    }

    #[test]
    fn class_from_id_table_maps_to_table() {
        assert_eq!(PpDocLayoutV3Model::class_from_id(21), Some(LayoutClass::Table));
    }

    #[test]
    fn class_from_id_text_classes_map_to_text() {
        for id in [0i64, 2, 16, 18, 19, 22, 23] {
            assert_eq!(
                PpDocLayoutV3Model::class_from_id(id),
                Some(LayoutClass::Text),
                "class_id {id} should map to Text"
            );
        }
    }

    #[test]
    fn class_from_id_out_of_range_returns_none() {
        assert_eq!(PpDocLayoutV3Model::class_from_id(25), None);
        assert_eq!(PpDocLayoutV3Model::class_from_id(-1), None);
        assert_eq!(PpDocLayoutV3Model::class_from_id(100), None);
    }

    #[test]
    fn default_threshold_is_half() {
        assert_eq!(DEFAULT_THRESHOLD, 0.5);
    }

    #[test]
    fn input_size_is_800() {
        assert_eq!(INPUT_SIZE, 800);
    }

    #[test]
    fn parse_detections_filters_low_confidence() {
        let row: Vec<f32> = vec![3.0, 0.3, 10.0, 20.0, 100.0, 200.0, 0.0];
        let dets = PpDocLayoutV3Model::parse_detections(&row, 1, 0.5, 640, 480);
        assert!(dets.is_empty(), "detection below threshold must be filtered");
    }

    #[test]
    fn parse_detections_accepts_above_threshold() {
        let row: Vec<f32> = vec![21.0, 0.8, 10.0, 20.0, 100.0, 200.0, 0.0];
        let dets = PpDocLayoutV3Model::parse_detections(&row, 1, 0.5, 640, 480);
        assert_eq!(dets.len(), 1);
        assert_eq!(dets[0].class_name, LayoutClass::Table);
        assert!((dets[0].confidence - 0.8).abs() < 1e-5);
    }

    #[test]
    fn parse_detections_clamps_coordinates_to_image_bounds() {
        let row: Vec<f32> = vec![22.0, 0.9, -5.0, -10.0, 700.0, 500.0, 0.0];
        let dets = PpDocLayoutV3Model::parse_detections(&row, 1, 0.5, 640, 480);
        assert_eq!(dets.len(), 1);
        assert_eq!(dets[0].bbox.x1, 0.0);
        assert_eq!(dets[0].bbox.y1, 0.0);
        assert_eq!(dets[0].bbox.x2, 640.0);
        assert_eq!(dets[0].bbox.y2, 480.0);
    }

    #[test]
    fn parse_detections_skips_unknown_class_id() {
        let row: Vec<f32> = vec![25.0, 0.9, 10.0, 20.0, 100.0, 200.0, 0.0];
        let dets = PpDocLayoutV3Model::parse_detections(&row, 1, 0.5, 640, 480);
        assert!(dets.is_empty(), "unknown class ID must be skipped");
    }

    #[test]
    fn empty_batch_short_circuits_to_empty_result() {
        let empty: Vec<&RgbImage> = vec![];
        match empty_batch_short_circuit(&empty) {
            Some(result) => assert!(result.is_empty(), "empty batch must yield an empty result"),
            None => panic!("empty batch must short-circuit"),
        }
    }

    #[test]
    fn non_empty_batch_does_not_short_circuit() {
        let img = RgbImage::new(1, 1);
        let images = [&img];
        assert!(
            empty_batch_short_circuit(&images).is_none(),
            "non-empty batch must proceed to inference"
        );
    }

    /// `im_shape` must always be `[INPUT_SIZE, INPUT_SIZE]` (the resized input dimensions),
    /// NOT the original image dimensions. The PaddleDetection model uses `im_shape` as the
    /// tensor input size and divides output coordinates by `scale_factor` to restore original
    /// pixel space. Passing original dimensions causes output boxes to be out of bounds.
    #[test]
    fn preprocess_single_im_shape_is_resized_dimensions_not_original() {
        let img = RgbImage::new(1275, 1650);
        let (_pixel_tensor, im_shape, _scale_factor) = PpDocLayoutV3Model::preprocess_single(&img).unwrap();
        assert_eq!(
            im_shape[[0, 0]],
            INPUT_SIZE as f32,
            "im_shape[H] must be INPUT_SIZE (800), not original height"
        );
        assert_eq!(
            im_shape[[0, 1]],
            INPUT_SIZE as f32,
            "im_shape[W] must be INPUT_SIZE (800), not original width"
        );
    }

    #[test]
    fn preprocess_single_scale_factor_is_resized_over_original() {
        let img = RgbImage::new(1275, 1650);
        let (_pixel_tensor, _im_shape, scale_factor) = PpDocLayoutV3Model::preprocess_single(&img).unwrap();
        let expected_scale_h = INPUT_SIZE as f32 / 1650.0;
        let expected_scale_w = INPUT_SIZE as f32 / 1275.0;
        assert!(
            (scale_factor[[0, 0]] - expected_scale_h).abs() < 1e-5,
            "scale_factor[H] must be 800/orig_h"
        );
        assert!(
            (scale_factor[[0, 1]] - expected_scale_w).abs() < 1e-5,
            "scale_factor[W] must be 800/orig_w"
        );
    }
}

#[cfg(all(test, inference_ort))]
mod production_parity;
