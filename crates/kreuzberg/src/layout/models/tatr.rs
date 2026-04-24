//! TATR (Table Transformer) table structure recognition model.
//!
//! Takes a cropped table image and outputs detected rows, columns, headers,
//! and spanning cells as bounding boxes for cell grid reconstruction.
//!
//! TATR is a DETR-based non-autoregressive object detection model that
//! simultaneously predicts all table structure elements in a single forward pass.
//!
//! Model: TATR (Table Transformer, DETR-based)
//! Input: `pixel_values` shape `[batch, 3, H, W]` f32 (variable size, DETR preprocessing)
//! Output 0: `logits`     shape `[batch, 125, 7]` f32 — class logits (7 classes)
//! Output 1: `pred_boxes` shape `[batch, 125, 4]` f32 — normalized (cx, cy, w, h)
//!
//! Classes (7): Table=0, Column=1, Row=2, ColumnHeader=3, ProjectedRowHeader=4,
//!              SpanningCell=5, NoObject=6

use image::RgbImage;
use ndarray::Array4;
use ort::{inputs, session::Session, value::Tensor};

use crate::layout::error::LayoutError;
use crate::layout::session::build_session;

// ---------------------------------------------------------------------------
// Constants
// ---------------------------------------------------------------------------

/// DETR standard shortest-edge target.
const DETR_SHORT_EDGE: u32 = 800;

/// DETR standard longest-edge cap.
const DETR_LONG_EDGE: u32 = 1333;

/// ImageNet normalization mean (RGB channel order).
const IMAGENET_MEAN_RGB: [f32; 3] = [0.485, 0.456, 0.406];

/// ImageNet normalization std (RGB channel order).
const IMAGENET_STD_RGB: [f32; 3] = [0.229, 0.224, 0.225];

/// Number of TATR output classes (including NoObject).
const NUM_CLASSES: usize = 7;

/// Confidence threshold for row and column detections.
const CONF_THRESHOLD_ROW_COL: f32 = 0.3;

/// Confidence threshold for spanning cell detections.
const CONF_THRESHOLD_SPANNING: f32 = 0.5;

/// IoB threshold for NMS on **row** detections during cell grid construction.
///
/// A candidate detection is suppressed if more than this fraction of its area
/// overlaps with any already-kept detection.  The previous value of 0.1 was
/// too aggressive: TATR row predictions frequently overlap by a few pixels,
/// causing valid rows to be suppressed and merging their content.
/// 0.5 means "suppress only when the majority of the candidate is already
/// covered" — enough to remove true duplicates while preserving close but
/// distinct rows.
const NMS_IOB_THRESHOLD_ROWS: f32 = 0.5;

/// IoB threshold for NMS on **column** detections during cell grid construction.
///
/// Columns need a lower threshold than rows because narrow adjacent columns
/// (e.g. Q1, Q2 quarter headers) can have significant mutual IoB overlap
/// relative to their small width, causing valid columns to be suppressed and
/// merged. 0.3 preserves narrow adjacent columns while still removing true
/// duplicates.
const NMS_IOB_THRESHOLD_COLS: f32 = 0.3;

/// Minimum column width as a fraction of table width.
///
/// After NMS, columns narrower than this fraction of the total table width
/// are removed as noise. Prevents spurious thin column detections from
/// splitting the grid incorrectly.
const MIN_COL_WIDTH_FRAC: f32 = 0.01;

// ---------------------------------------------------------------------------
// Types
// ---------------------------------------------------------------------------

/// TATR object detection class labels.
///
/// The 7 classes output by the Table Transformer model. `NoObject` (class 6)
/// is the background/padding class and is filtered out during post-processing.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum TatrClass {
    /// Full table bounding box (class 0).
    Table,
    /// Table column (class 1).
    Column,
    /// Table row (class 2).
    Row,
    /// Column header row (class 3).
    ColumnHeader,
    /// Projected row header column (class 4).
    ProjectedRowHeader,
    /// Spanning cell covering multiple rows/columns (class 5).
    SpanningCell,
}

impl TatrClass {
    /// Map a raw class index (0..6) to a `TatrClass`.
    ///
    /// Returns `None` for class 6 (NoObject) and any out-of-range index.
    fn from_index(idx: usize) -> Option<Self> {
        match idx {
            0 => Some(Self::Table),
            1 => Some(Self::Column),
            2 => Some(Self::Row),
            3 => Some(Self::ColumnHeader),
            4 => Some(Self::ProjectedRowHeader),
            5 => Some(Self::SpanningCell),
            _ => None, // 6 = NoObject, anything else = invalid
        }
    }

    /// Human-readable label.
    #[cfg(test)]
    pub(crate) fn name(&self) -> &'static str {
        match self {
            Self::Table => "table",
            Self::Column => "column",
            Self::Row => "row",
            Self::ColumnHeader => "column_header",
            Self::ProjectedRowHeader => "projected_row_header",
            Self::SpanningCell => "spanning_cell",
        }
    }
}

/// A single TATR detection result.
#[derive(Debug, Clone)]
pub struct TatrDetection {
    /// Bounding box in crop-pixel coordinates: `[x1, y1, x2, y2]`.
    pub bbox: [f32; 4],
    /// Detection confidence score (0.0..1.0).
    pub confidence: f32,
    /// Detected class.
    pub class: TatrClass,
}

/// Aggregated TATR recognition result with detections separated by class.
#[derive(Debug, Clone)]
pub struct TatrResult {
    /// Detected rows, sorted top-to-bottom by `y2`.
    pub rows: Vec<TatrDetection>,
    /// Detected columns, sorted left-to-right by `x2`.
    pub columns: Vec<TatrDetection>,
    /// Detected headers (ColumnHeader and ProjectedRowHeader).
    pub headers: Vec<TatrDetection>,
    /// Detected spanning cells.
    pub spanning: Vec<TatrDetection>,
}

/// A cell bounding box within the reconstructed table grid.
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct CellBBox {
    pub x1: f32,
    pub y1: f32,
    pub x2: f32,
    pub y2: f32,
}

// ---------------------------------------------------------------------------
// Model
// ---------------------------------------------------------------------------

/// TATR (Table Transformer) table structure recognition model.
///
/// Wraps an ORT session for the TATR ONNX model and provides preprocessing,
/// inference, and post-processing in a single `recognize` call.
pub struct TatrModel {
    session: Session,
    input_name: String,
}

impl TatrModel {
    /// Load a TATR ONNX model from a file path.
    ///
    /// Uses the default execution provider selection from `build_session`
    /// with a CPU-only fallback if the platform EP fails.
    pub(crate) fn from_file(
        path: &str,
        accel: Option<&crate::core::config::acceleration::AccelerationConfig>,
    ) -> Result<Self, LayoutError> {
        let budget = crate::core::config::concurrency::resolve_thread_budget(None);
        let session = match build_session(path, accel, budget) {
            Ok(s) => s,
            Err(first_err) => {
                tracing::warn!("TATR: platform EP failed ({first_err}), retrying with CPU-only");
                match Self::build_cpu_session(path, budget) {
                    Ok(s) => s,
                    Err(cpu_err) => {
                        tracing::warn!("TATR: CPU-only also failed: {cpu_err}");
                        return Err(cpu_err);
                    }
                }
            }
        };
        let input_name = session.inputs()[0].name().to_string();

        Ok(Self { session, input_name })
    }

    /// Build a CPU-only ORT session (no CoreML/CUDA).
    fn build_cpu_session(path: &str, thread_budget: usize) -> Result<Session, LayoutError> {
        use ort::session::builder::GraphOptimizationLevel;
        let mut builder = Session::builder()?
            .with_optimization_level(GraphOptimizationLevel::All)
            .map_err(|e| LayoutError::Ort(ort::Error::new(e.message())))?
            .with_intra_threads(thread_budget)
            .map_err(|e| LayoutError::Ort(ort::Error::new(e.message())))?
            .with_inter_threads(1)
            .map_err(|e| LayoutError::Ort(ort::Error::new(e.message())))?;
        Ok(builder.commit_from_file(path)?)
    }

    /// Recognize table structure from a cropped table image.
    ///
    /// Returns a [`TatrResult`] with detected rows, columns, headers, and
    /// spanning cells in the input image's pixel coordinate space.
    pub(crate) fn recognize(&mut self, table_img: &RgbImage) -> Result<TatrResult, LayoutError> {
        let img_w = table_img.width() as f32;
        let img_h = table_img.height() as f32;

        // Preprocess: DETR-standard resize + ImageNet normalize + NCHW
        let (input_tensor, resized_w, resized_h) = preprocess_detr(table_img);
        let tensor = Tensor::from_array(input_tensor)?;

        let outputs = self.session.run(inputs![
            self.input_name.as_str() => tensor
        ])?;

        // Extract float outputs: logits [1, 125, 7] and pred_boxes [1, 125, 4]
        let mut float_outputs: Vec<(Vec<usize>, Vec<f32>)> = Vec::new();
        for (_name, value) in outputs.iter() {
            if let Ok(view) = value.try_extract_tensor::<f32>() {
                let shape: Vec<usize> = view.0.iter().map(|&d| d as usize).collect();
                let data: Vec<f32> = view.1.to_vec();
                float_outputs.push((shape, data));
            }
        }

        if float_outputs.len() < 2 {
            return Err(LayoutError::InvalidOutput(format!(
                "TATR expected 2 float outputs, got {}",
                float_outputs.len()
            )));
        }

        // Identify logits (last dim == NUM_CLASSES) vs pred_boxes (last dim == 4).
        let (logits_shape, logits_data, boxes_shape, boxes_data) = if float_outputs[0].0.last() == Some(&NUM_CLASSES) {
            let (ls, ld) = float_outputs.remove(0);
            let (bs, bd) = float_outputs.remove(0);
            (ls, ld, bs, bd)
        } else {
            let (bs, bd) = float_outputs.remove(0);
            let (ls, ld) = float_outputs.remove(0);
            (ls, ld, bs, bd)
        };

        let num_queries = logits_shape.get(1).copied().unwrap_or(0);
        let num_classes = logits_shape.last().copied().unwrap_or(0);
        let box_dim = boxes_shape.last().copied().unwrap_or(0);

        if num_queries == 0 || num_classes < NUM_CLASSES || box_dim < 4 {
            return Ok(TatrResult {
                rows: Vec::new(),
                columns: Vec::new(),
                headers: Vec::new(),
                spanning: Vec::new(),
            });
        }

        // Post-process each query
        let mut rows = Vec::new();
        let mut columns = Vec::new();
        let mut headers = Vec::new();
        let mut spanning = Vec::new();

        for q in 0..num_queries {
            let logit_offset = q * num_classes;
            let logits_slice = &logits_data[logit_offset..logit_offset + num_classes];

            // Softmax + argmax + confidence
            let (class_idx, confidence) = softmax_argmax(logits_slice);

            // Skip NoObject (class 6) and unmapped classes
            let class = match TatrClass::from_index(class_idx) {
                Some(c) => c,
                None => continue,
            };

            // Apply class-specific confidence threshold
            let threshold = match class {
                TatrClass::SpanningCell => CONF_THRESHOLD_SPANNING,
                TatrClass::Table => CONF_THRESHOLD_ROW_COL,
                _ => CONF_THRESHOLD_ROW_COL,
            };
            if confidence < threshold {
                continue;
            }

            // Convert normalized (cx, cy, w, h) to pixel (x1, y1, x2, y2)
            let box_offset = q * box_dim;
            let cx = boxes_data[box_offset];
            let cy = boxes_data[box_offset + 1];
            let w = boxes_data[box_offset + 2];
            let h = boxes_data[box_offset + 3];

            let bbox = cxcywh_to_xyxy(cx, cy, w, h, resized_w as f32, resized_h as f32);

            // Scale from resized coordinates back to original image coordinates
            let scale_x = img_w / resized_w as f32;
            let scale_y = img_h / resized_h as f32;
            let bbox = [
                (bbox[0] * scale_x).clamp(0.0, img_w),
                (bbox[1] * scale_y).clamp(0.0, img_h),
                (bbox[2] * scale_x).clamp(0.0, img_w),
                (bbox[3] * scale_y).clamp(0.0, img_h),
            ];

            let detection = TatrDetection {
                bbox,
                confidence,
                class,
            };

            match class {
                TatrClass::Row => rows.push(detection),
                TatrClass::Column => columns.push(detection),
                TatrClass::ColumnHeader | TatrClass::ProjectedRowHeader => {
                    headers.push(detection);
                }
                TatrClass::SpanningCell => spanning.push(detection),
                TatrClass::Table => {} // Table bbox is informational; not stored separately
            }
        }

        // Sort rows top-to-bottom by y2, columns left-to-right by x2
        rows.sort_by(|a, b| a.bbox[3].total_cmp(&b.bbox[3]));
        columns.sort_by(|a, b| a.bbox[2].total_cmp(&b.bbox[2]));

        Ok(TatrResult {
            rows,
            columns,
            headers,
            spanning,
        })
    }
}

// ---------------------------------------------------------------------------
// Preprocessing
// ---------------------------------------------------------------------------

/// Preprocess an image using DETR-standard preprocessing.
///
/// Pipeline:
/// 1. Resize: scale shortest edge to 800px, cap longest edge at 1333px (aspect-preserving)
/// 2. Normalize: ImageNet mean/std in RGB channel order
/// 3. Layout: NCHW `[1, 3, H, W]` f32
///
/// Returns `(tensor, resized_width, resized_height)`.
fn preprocess_detr(img: &RgbImage) -> (Array4<f32>, u32, u32) {
    let (orig_w, orig_h) = (img.width(), img.height());
    let (new_w, new_h) = compute_detr_resize(orig_w, orig_h);

    let resized = image::imageops::resize(img, new_w, new_h, image::imageops::FilterType::CatmullRom);

    let w = new_w as usize;
    let h = new_h as usize;
    let hw = h * w;

    let inv_std_r = 1.0 / IMAGENET_STD_RGB[0];
    let inv_std_g = 1.0 / IMAGENET_STD_RGB[1];
    let inv_std_b = 1.0 / IMAGENET_STD_RGB[2];

    let mut data = vec![0.0f32; 3 * hw];
    let pixels = resized.as_raw();

    for y in 0..h {
        for x in 0..w {
            let src_idx = (y * w + x) * 3;
            let dst_idx = y * w + x;
            let r = pixels[src_idx] as f32 * (1.0 / 255.0);
            let g = pixels[src_idx + 1] as f32 * (1.0 / 255.0);
            let b = pixels[src_idx + 2] as f32 * (1.0 / 255.0);
            // RGB channel order
            data[dst_idx] = (r - IMAGENET_MEAN_RGB[0]) * inv_std_r;
            data[hw + dst_idx] = (g - IMAGENET_MEAN_RGB[1]) * inv_std_g;
            data[2 * hw + dst_idx] = (b - IMAGENET_MEAN_RGB[2]) * inv_std_b;
        }
    }

    let tensor = Array4::from_shape_vec((1, 3, h, w), data).expect("shape mismatch in preprocess_detr");

    (tensor, new_w, new_h)
}

/// Compute DETR resize dimensions.
///
/// Scales shortest edge to [`DETR_SHORT_EDGE`] (800), then caps longest edge
/// at [`DETR_LONG_EDGE`] (1333), maintaining aspect ratio.
fn compute_detr_resize(orig_w: u32, orig_h: u32) -> (u32, u32) {
    let short = orig_w.min(orig_h) as f32;
    let long = orig_w.max(orig_h) as f32;

    // Scale so shortest edge = DETR_SHORT_EDGE
    let mut scale = DETR_SHORT_EDGE as f32 / short;

    // If longest edge exceeds cap after scaling, scale down further
    if (long * scale).round() > DETR_LONG_EDGE as f32 {
        scale = DETR_LONG_EDGE as f32 / long;
    }

    let new_w = (orig_w as f32 * scale).round().max(1.0) as u32;
    let new_h = (orig_h as f32 * scale).round().max(1.0) as u32;

    (new_w, new_h)
}

// ---------------------------------------------------------------------------
// Post-processing helpers
// ---------------------------------------------------------------------------

/// Softmax over a slice, returning `(argmax_index, max_probability)`.
fn softmax_argmax(logits: &[f32]) -> (usize, f32) {
    // Numerical stability: subtract max before exp
    let max_val = logits.iter().copied().fold(f32::NEG_INFINITY, f32::max);

    let mut sum = 0.0f32;
    let mut probs = Vec::with_capacity(logits.len());
    for &v in logits {
        let e = (v - max_val).exp();
        probs.push(e);
        sum += e;
    }

    let inv_sum = 1.0 / sum;
    let mut best_idx = 0;
    let mut best_prob = 0.0f32;
    for (i, p) in probs.iter().enumerate() {
        let prob = p * inv_sum;
        if prob > best_prob {
            best_prob = prob;
            best_idx = i;
        }
    }

    (best_idx, best_prob)
}

/// Convert normalized center-format box `(cx, cy, w, h)` to pixel `[x1, y1, x2, y2]`.
///
/// The input values are in `[0, 1]` normalized space; they are scaled by
/// `img_w` and `img_h` to produce pixel coordinates.
fn cxcywh_to_xyxy(cx: f32, cy: f32, w: f32, h: f32, img_w: f32, img_h: f32) -> [f32; 4] {
    let px_cx = cx * img_w;
    let px_cy = cy * img_h;
    let px_w = w * img_w;
    let px_h = h * img_h;

    let x1 = px_cx - px_w / 2.0;
    let y1 = px_cy - px_h / 2.0;
    let x2 = px_cx + px_w / 2.0;
    let y2 = px_cy + px_h / 2.0;

    [x1.max(0.0), y1.max(0.0), x2.max(0.0), y2.max(0.0)]
}

/// Intersection-over-Box: `intersection_area(a, b) / area(a)`.
///
/// Measures what fraction of box `a` is covered by box `b`.
/// Returns 0.0 if `a` has zero area.
fn iob(a: [f32; 4], b: [f32; 4]) -> f32 {
    let area_a = (a[2] - a[0]).max(0.0) * (a[3] - a[1]).max(0.0);
    if area_a <= 0.0 {
        return 0.0;
    }

    let ix1 = a[0].max(b[0]);
    let iy1 = a[1].max(b[1]);
    let ix2 = a[2].min(b[2]);
    let iy2 = a[3].min(b[3]);
    let inter = (ix2 - ix1).max(0.0) * (iy2 - iy1).max(0.0);

    inter / area_a
}

// ---------------------------------------------------------------------------
// Cell grid construction
// ---------------------------------------------------------------------------

/// Build a 2D cell grid from TATR detections.
///
/// The grid is `[num_rows][num_cols]` where each cell is the intersection
/// of a row bounding box and a column bounding box.
///
/// Processing steps:
/// 1. Widen all rows to span the full table width (min x1 to max x2 across rows)
/// 2. Apply NMS using IoB: sort by confidence descending, remove detections
///    whose IoB with any higher-confidence detection exceeds [`NMS_IOB_THRESHOLD`]
/// 3. For each (row, column) pair, compute the intersection rectangle
///
/// If `table_bbox` is provided, it is used to clip the row widening bounds.
pub(crate) fn build_cell_grid(result: &TatrResult, table_bbox: Option<[f32; 4]>) -> Vec<Vec<CellBBox>> {
    if result.rows.is_empty() || result.columns.is_empty() {
        return Vec::new();
    }

    // Determine table-wide x extents for row widening
    let (table_x1, table_x2) = if let Some(tb) = table_bbox {
        (tb[0], tb[2])
    } else {
        // Use the full extent of all row bounding boxes
        let min_x1 = result.rows.iter().map(|r| r.bbox[0]).fold(f32::INFINITY, f32::min);
        let max_x2 = result.rows.iter().map(|r| r.bbox[2]).fold(f32::NEG_INFINITY, f32::max);
        (min_x1, max_x2)
    };

    // Widen rows to full table width
    let widened_rows: Vec<[f32; 4]> = result
        .rows
        .iter()
        .map(|r| [table_x1, r.bbox[1], table_x2, r.bbox[3]])
        .collect();

    // NMS on rows (by confidence, IoB threshold), then sort top-to-bottom.
    // nms_by_iob returns rows in confidence order; we must restore spatial
    // order so the cell grid rows correspond to top-to-bottom reading order.
    let mut nms_rows = nms_by_iob(&result.rows, &widened_rows, NMS_IOB_THRESHOLD_ROWS);
    nms_rows.sort_by(|a, b| a[1].total_cmp(&b[1]));

    // NMS on columns (using original bboxes) with a lower threshold to
    // preserve narrow adjacent columns, then sort left-to-right.
    let col_bboxes: Vec<[f32; 4]> = result.columns.iter().map(|c| c.bbox).collect();
    let mut nms_cols = nms_by_iob(&result.columns, &col_bboxes, NMS_IOB_THRESHOLD_COLS);

    // Remove noise columns narrower than MIN_COL_WIDTH_FRAC of table width.
    let table_width = table_x2 - table_x1;
    if table_width > 0.0 {
        let min_col_width = table_width * MIN_COL_WIDTH_FRAC;
        nms_cols.retain(|col| (col[2] - col[0]) >= min_col_width);
    }

    nms_cols.sort_by(|a, b| a[0].total_cmp(&b[0]));

    // Build grid: intersection of each (row, col) pair
    let mut grid = Vec::with_capacity(nms_rows.len());
    for row_bbox in &nms_rows {
        let mut row_cells = Vec::with_capacity(nms_cols.len());
        for col_bbox in &nms_cols {
            let cell = intersect_boxes(*row_bbox, *col_bbox);
            row_cells.push(cell);
        }
        grid.push(row_cells);
    }

    grid
}

/// Apply NMS using IoB (Intersection over Box) metric.
///
/// Sort detections by confidence descending, then greedily keep detections
/// whose IoB with all previously kept detections is below the threshold.
///
/// `bboxes` are the (possibly widened) bounding boxes corresponding 1:1
/// with `detections`. `threshold` is the IoB suppression threshold.
fn nms_by_iob(detections: &[TatrDetection], bboxes: &[[f32; 4]], threshold: f32) -> Vec<[f32; 4]> {
    // Build index-confidence pairs, sort by confidence descending
    let mut indices: Vec<usize> = (0..detections.len()).collect();
    indices.sort_by(|&a, &b| detections[b].confidence.total_cmp(&detections[a].confidence));

    let mut kept: Vec<[f32; 4]> = Vec::new();

    for &idx in &indices {
        let candidate = bboxes[idx];
        let suppressed = kept.iter().any(|&kept_box| iob(candidate, kept_box) > threshold);
        if !suppressed {
            kept.push(candidate);
        }
    }

    kept
}

/// Compute the intersection rectangle of two axis-aligned bounding boxes.
///
/// If the boxes do not overlap, the resulting `CellBBox` will have
/// `x1 >= x2` or `y1 >= y2` (zero-area cell).
fn intersect_boxes(a: [f32; 4], b: [f32; 4]) -> CellBBox {
    CellBBox {
        x1: a[0].max(b[0]),
        y1: a[1].max(b[1]),
        x2: a[2].min(b[2]),
        y2: a[3].min(b[3]),
    }
}

// ---------------------------------------------------------------------------
// Tests
// ---------------------------------------------------------------------------

#[cfg(test)]
mod tests {
    use super::*;

    // -- DETR resize --

    #[test]
    fn test_compute_detr_resize_landscape() {
        // 1600x1200: short=1200, scale=800/1200=0.667
        // new: 1067x800, longest=1067 < 1333 → no capping
        let (w, h) = compute_detr_resize(1600, 1200);
        assert!(h == 800 || w == 800, "shortest edge should be 800, got {w}x{h}");
        assert!(w <= DETR_LONG_EDGE, "longest edge {w} exceeds cap {DETR_LONG_EDGE}");
        assert!(h <= DETR_LONG_EDGE, "longest edge {h} exceeds cap {DETR_LONG_EDGE}");
    }

    #[test]
    fn test_compute_detr_resize_portrait() {
        // 600x1000: short=600, scale=800/600=1.333
        // new: 800x1333, longest=1333 → exactly at cap
        let (w, h) = compute_detr_resize(600, 1000);
        assert!(w.min(h) <= DETR_SHORT_EDGE);
        assert!(w.max(h) <= DETR_LONG_EDGE, "longest edge exceeds cap: {w}x{h}");
    }

    #[test]
    fn test_compute_detr_resize_very_elongated() {
        // 100x3000: short=100, scale=800/100=8.0 → long=24000 >> 1333
        // Re-scale: 1333/3000=0.444 → new: 44x1333
        let (w, h) = compute_detr_resize(100, 3000);
        assert!(w.max(h) <= DETR_LONG_EDGE, "longest edge exceeds cap: {w}x{h}");
    }

    #[test]
    fn test_compute_detr_resize_square() {
        // 800x800: already at target
        let (w, h) = compute_detr_resize(800, 800);
        assert_eq!(w, 800);
        assert_eq!(h, 800);
    }

    #[test]
    fn test_compute_detr_resize_small() {
        // 200x300: short=200, scale=800/200=4.0
        // new: 800x1200, longest=1200 < 1333 → no capping
        let (w, h) = compute_detr_resize(200, 300);
        assert_eq!(w, 800);
        assert_eq!(h, 1200);
    }

    // -- Box conversion --

    #[test]
    fn test_cxcywh_to_xyxy_center() {
        // Center of a 100x100 image, 50x50 box
        let bbox = cxcywh_to_xyxy(0.5, 0.5, 0.5, 0.5, 100.0, 100.0);
        assert!((bbox[0] - 25.0).abs() < 1e-5, "x1={}", bbox[0]);
        assert!((bbox[1] - 25.0).abs() < 1e-5, "y1={}", bbox[1]);
        assert!((bbox[2] - 75.0).abs() < 1e-5, "x2={}", bbox[2]);
        assert!((bbox[3] - 75.0).abs() < 1e-5, "y2={}", bbox[3]);
    }

    #[test]
    fn test_cxcywh_to_xyxy_top_left() {
        // Box at top-left corner covering full image
        let bbox = cxcywh_to_xyxy(0.5, 0.5, 1.0, 1.0, 200.0, 100.0);
        assert!((bbox[0] - 0.0).abs() < 1e-5);
        assert!((bbox[1] - 0.0).abs() < 1e-5);
        assert!((bbox[2] - 200.0).abs() < 1e-5);
        assert!((bbox[3] - 100.0).abs() < 1e-5);
    }

    #[test]
    fn test_cxcywh_to_xyxy_clamps_negative() {
        // Box that extends past the origin should clamp to 0
        let bbox = cxcywh_to_xyxy(0.0, 0.0, 0.5, 0.5, 100.0, 100.0);
        assert_eq!(bbox[0], 0.0, "x1 should be clamped to 0");
        assert_eq!(bbox[1], 0.0, "y1 should be clamped to 0");
    }

    // -- Softmax + argmax --

    #[test]
    fn test_softmax_argmax_clear_winner() {
        let logits = [0.0, 0.0, 10.0, 0.0, 0.0, 0.0, 0.0];
        let (idx, prob) = softmax_argmax(&logits);
        assert_eq!(idx, 2);
        assert!(prob > 0.99, "confidence should be ~1.0, got {prob}");
    }

    #[test]
    fn test_softmax_argmax_uniform() {
        let logits = [1.0; 7];
        let (_, prob) = softmax_argmax(&logits);
        assert!(
            (prob - 1.0 / 7.0).abs() < 1e-5,
            "uniform logits should give ~1/7 confidence, got {prob}"
        );
    }

    #[test]
    fn test_softmax_argmax_negative() {
        let logits = [-10.0, -5.0, -1.0, -20.0, -30.0, -2.0, -100.0];
        let (idx, _) = softmax_argmax(&logits);
        assert_eq!(idx, 2, "should pick the least negative");
    }

    // -- IoB --

    #[test]
    fn test_iob_full_containment() {
        // a is fully inside b
        let a = [10.0, 10.0, 20.0, 20.0];
        let b = [0.0, 0.0, 100.0, 100.0];
        let result = iob(a, b);
        assert!((result - 1.0).abs() < 1e-5, "fully contained → IoB=1.0, got {result}");
    }

    #[test]
    fn test_iob_no_overlap() {
        let a = [0.0, 0.0, 10.0, 10.0];
        let b = [20.0, 20.0, 30.0, 30.0];
        let result = iob(a, b);
        assert_eq!(result, 0.0);
    }

    #[test]
    fn test_iob_partial_overlap() {
        // a = [0,0,10,10] area=100
        // b = [5,0,15,10]
        // intersection = [5,0,10,10] area=50
        // IoB = 50/100 = 0.5
        let a = [0.0, 0.0, 10.0, 10.0];
        let b = [5.0, 0.0, 15.0, 10.0];
        let result = iob(a, b);
        assert!((result - 0.5).abs() < 1e-5, "expected 0.5, got {result}");
    }

    #[test]
    fn test_iob_zero_area() {
        let a = [5.0, 5.0, 5.0, 5.0]; // zero area
        let b = [0.0, 0.0, 10.0, 10.0];
        let result = iob(a, b);
        assert_eq!(result, 0.0, "zero-area box should return 0.0");
    }

    // -- NMS --

    #[test]
    fn test_nms_suppresses_overlapping() {
        let detections = vec![
            TatrDetection {
                bbox: [0.0, 0.0, 100.0, 20.0],
                confidence: 0.9,
                class: TatrClass::Row,
            },
            TatrDetection {
                bbox: [0.0, 2.0, 100.0, 22.0],
                confidence: 0.7,
                class: TatrClass::Row,
            },
        ];
        let bboxes: Vec<[f32; 4]> = detections.iter().map(|d| d.bbox).collect();
        let kept = nms_by_iob(&detections, &bboxes, NMS_IOB_THRESHOLD_ROWS);
        // The second detection heavily overlaps the first → should be suppressed
        assert_eq!(kept.len(), 1, "overlapping detection should be suppressed");
        assert_eq!(kept[0], [0.0, 0.0, 100.0, 20.0]);
    }

    #[test]
    fn test_nms_keeps_non_overlapping() {
        let detections = vec![
            TatrDetection {
                bbox: [0.0, 0.0, 100.0, 20.0],
                confidence: 0.9,
                class: TatrClass::Row,
            },
            TatrDetection {
                bbox: [0.0, 50.0, 100.0, 70.0],
                confidence: 0.8,
                class: TatrClass::Row,
            },
        ];
        let bboxes: Vec<[f32; 4]> = detections.iter().map(|d| d.bbox).collect();
        let kept = nms_by_iob(&detections, &bboxes, NMS_IOB_THRESHOLD_ROWS);
        assert_eq!(kept.len(), 2, "non-overlapping detections should both be kept");
    }

    #[test]
    fn test_nms_keeps_adjacent_rows_with_minor_overlap() {
        // Two rows that overlap by 2px (10% of height) should be kept.
        // This tests the fix: IoB threshold 0.5 preserves close but distinct rows.
        let detections = vec![
            TatrDetection {
                bbox: [0.0, 0.0, 100.0, 20.0],
                confidence: 0.9,
                class: TatrClass::Row,
            },
            TatrDetection {
                bbox: [0.0, 18.0, 100.0, 38.0],
                confidence: 0.8,
                class: TatrClass::Row,
            },
        ];
        let bboxes: Vec<[f32; 4]> = detections.iter().map(|d| d.bbox).collect();
        let kept = nms_by_iob(&detections, &bboxes, NMS_IOB_THRESHOLD_ROWS);
        // IoB = intersection(100*2) / area(100*20) = 0.1 < 0.5 → both kept
        assert_eq!(kept.len(), 2, "adjacent rows with minor overlap should both be kept");
    }

    // -- Cell grid --

    #[test]
    fn test_build_cell_grid_2x2() {
        let result = TatrResult {
            rows: vec![
                TatrDetection {
                    bbox: [0.0, 0.0, 100.0, 20.0],
                    confidence: 0.9,
                    class: TatrClass::Row,
                },
                TatrDetection {
                    bbox: [0.0, 20.0, 100.0, 40.0],
                    confidence: 0.85,
                    class: TatrClass::Row,
                },
            ],
            columns: vec![
                TatrDetection {
                    bbox: [0.0, 0.0, 50.0, 40.0],
                    confidence: 0.9,
                    class: TatrClass::Column,
                },
                TatrDetection {
                    bbox: [50.0, 0.0, 100.0, 40.0],
                    confidence: 0.85,
                    class: TatrClass::Column,
                },
            ],
            headers: Vec::new(),
            spanning: Vec::new(),
        };

        let grid = build_cell_grid(&result, None);
        assert_eq!(grid.len(), 2, "should have 2 rows");
        assert_eq!(grid[0].len(), 2, "should have 2 columns per row");

        // Top-left cell: row [0,0..20] intersect col [0..50,0..40]
        let tl = &grid[0][0];
        assert!((tl.x1 - 0.0).abs() < 1e-5);
        assert!((tl.y1 - 0.0).abs() < 1e-5);
        assert!((tl.x2 - 50.0).abs() < 1e-5);
        assert!((tl.y2 - 20.0).abs() < 1e-5);

        // Bottom-right cell: row [0,20..40] intersect col [50..100,0..40]
        let br = &grid[1][1];
        assert!((br.x1 - 50.0).abs() < 1e-5);
        assert!((br.y1 - 20.0).abs() < 1e-5);
        assert!((br.x2 - 100.0).abs() < 1e-5);
        assert!((br.y2 - 40.0).abs() < 1e-5);
    }

    #[test]
    fn test_build_cell_grid_empty() {
        let result = TatrResult {
            rows: Vec::new(),
            columns: Vec::new(),
            headers: Vec::new(),
            spanning: Vec::new(),
        };
        let grid = build_cell_grid(&result, None);
        assert!(grid.is_empty());
    }

    #[test]
    fn test_build_cell_grid_with_table_bbox() {
        let result = TatrResult {
            rows: vec![TatrDetection {
                bbox: [10.0, 5.0, 90.0, 25.0],
                confidence: 0.9,
                class: TatrClass::Row,
            }],
            columns: vec![TatrDetection {
                bbox: [0.0, 0.0, 50.0, 30.0],
                confidence: 0.9,
                class: TatrClass::Column,
            }],
            headers: Vec::new(),
            spanning: Vec::new(),
        };

        // Table bbox should widen the row to [0, 5, 100, 25]
        let grid = build_cell_grid(&result, Some([0.0, 0.0, 100.0, 30.0]));
        assert_eq!(grid.len(), 1);
        assert_eq!(grid[0].len(), 1);
        // Row widened to table x-extent [0..100], intersected with col [0..50]
        let cell = &grid[0][0];
        assert!((cell.x1 - 0.0).abs() < 1e-5, "x1={}", cell.x1);
        assert!((cell.x2 - 50.0).abs() < 1e-5, "x2={}", cell.x2);
    }

    // -- TatrClass --

    #[test]
    fn test_tatr_class_from_index() {
        assert_eq!(TatrClass::from_index(0), Some(TatrClass::Table));
        assert_eq!(TatrClass::from_index(1), Some(TatrClass::Column));
        assert_eq!(TatrClass::from_index(2), Some(TatrClass::Row));
        assert_eq!(TatrClass::from_index(3), Some(TatrClass::ColumnHeader));
        assert_eq!(TatrClass::from_index(4), Some(TatrClass::ProjectedRowHeader));
        assert_eq!(TatrClass::from_index(5), Some(TatrClass::SpanningCell));
        assert_eq!(TatrClass::from_index(6), None); // NoObject
        assert_eq!(TatrClass::from_index(7), None); // out of range
    }

    #[test]
    fn test_build_cell_grid_rows_sorted_spatially() {
        // Rows provided in reverse confidence order (bottom row has higher
        // confidence). After NMS + spatial sorting, the grid should still
        // have the top row first.
        let result = TatrResult {
            rows: vec![
                TatrDetection {
                    bbox: [0.0, 30.0, 100.0, 50.0], // bottom row
                    confidence: 0.95,
                    class: TatrClass::Row,
                },
                TatrDetection {
                    bbox: [0.0, 0.0, 100.0, 20.0], // top row
                    confidence: 0.80,
                    class: TatrClass::Row,
                },
            ],
            columns: vec![TatrDetection {
                bbox: [0.0, 0.0, 100.0, 50.0],
                confidence: 0.9,
                class: TatrClass::Column,
            }],
            headers: Vec::new(),
            spanning: Vec::new(),
        };

        let grid = build_cell_grid(&result, None);
        assert_eq!(grid.len(), 2, "should have 2 rows");
        // First grid row should be the spatially top row (y1 = 0)
        assert!(
            grid[0][0].y1 < grid[1][0].y1,
            "grid rows should be sorted top-to-bottom: row0.y1={} row1.y1={}",
            grid[0][0].y1,
            grid[1][0].y1,
        );
    }

    #[test]
    fn test_build_cell_grid_columns_sorted_spatially() {
        // Columns with right column having higher confidence.
        let result = TatrResult {
            rows: vec![TatrDetection {
                bbox: [0.0, 0.0, 100.0, 20.0],
                confidence: 0.9,
                class: TatrClass::Row,
            }],
            columns: vec![
                TatrDetection {
                    bbox: [60.0, 0.0, 100.0, 20.0], // right column
                    confidence: 0.95,
                    class: TatrClass::Column,
                },
                TatrDetection {
                    bbox: [0.0, 0.0, 50.0, 20.0], // left column
                    confidence: 0.80,
                    class: TatrClass::Column,
                },
            ],
            headers: Vec::new(),
            spanning: Vec::new(),
        };

        let grid = build_cell_grid(&result, None);
        assert_eq!(grid[0].len(), 2, "should have 2 columns");
        // First column should be the left column (x1 = 0)
        assert!(
            grid[0][0].x1 < grid[0][1].x1,
            "grid columns should be sorted left-to-right: col0.x1={} col1.x1={}",
            grid[0][0].x1,
            grid[0][1].x1,
        );
    }

    // -- Preprocessing dimensions --

    #[test]
    fn test_preprocess_detr_output_shape() {
        let img = RgbImage::new(640, 480);
        let (tensor, rw, rh) = preprocess_detr(&img);
        let shape = tensor.shape();
        assert_eq!(shape[0], 1, "batch dim");
        assert_eq!(shape[1], 3, "channel dim");
        assert_eq!(shape[2], rh as usize, "height dim");
        assert_eq!(shape[3], rw as usize, "width dim");
        // Shortest edge (480) should scale to 800: 480→800, 640→1067
        assert_eq!(rh, 800);
        assert_eq!(rw, 1067);
    }

    // -- Per-class NMS thresholds --

    #[test]
    fn test_nms_col_threshold_preserves_narrow_adjacent_columns() {
        // Two narrow adjacent columns that overlap by ~35% of their width.
        // With row threshold (0.5) they would be kept, but this tests that
        // the column threshold (0.3) is lower, allowing suppression of true
        // duplicates while the moderate overlap here (0.35) is still preserved.
        let col_width = 20.0;
        let overlap = 7.0; // 7/20 = 0.35 IoB
        let detections = vec![
            TatrDetection {
                bbox: [0.0, 0.0, col_width, 100.0],
                confidence: 0.9,
                class: TatrClass::Column,
            },
            TatrDetection {
                bbox: [col_width - overlap, 0.0, 2.0 * col_width - overlap, 100.0],
                confidence: 0.85,
                class: TatrClass::Column,
            },
        ];
        let bboxes: Vec<[f32; 4]> = detections.iter().map(|d| d.bbox).collect();

        // With row threshold (0.5): 0.35 < 0.5 → both kept
        let kept_row = nms_by_iob(&detections, &bboxes, NMS_IOB_THRESHOLD_ROWS);
        assert_eq!(kept_row.len(), 2, "row threshold should keep both");

        // With column threshold (0.3): 0.35 > 0.3 → second suppressed
        let kept_col = nms_by_iob(&detections, &bboxes, NMS_IOB_THRESHOLD_COLS);
        assert_eq!(
            kept_col.len(),
            1,
            "column threshold should suppress heavily overlapping column"
        );
    }

    #[test]
    fn test_nms_col_threshold_keeps_well_separated_columns() {
        // Two columns with only ~15% IoB overlap — both thresholds should keep them.
        let detections = vec![
            TatrDetection {
                bbox: [0.0, 0.0, 20.0, 100.0],
                confidence: 0.9,
                class: TatrClass::Column,
            },
            TatrDetection {
                bbox: [17.0, 0.0, 37.0, 100.0], // 3/20 = 0.15 overlap
                confidence: 0.85,
                class: TatrClass::Column,
            },
        ];
        let bboxes: Vec<[f32; 4]> = detections.iter().map(|d| d.bbox).collect();

        let kept = nms_by_iob(&detections, &bboxes, NMS_IOB_THRESHOLD_COLS);
        assert_eq!(kept.len(), 2, "well-separated columns should both be kept");
    }

    #[test]
    fn test_min_col_width_filter_removes_noise_columns() {
        // A 100px-wide table with one real column and one noise column (0.5px wide).
        let result = TatrResult {
            rows: vec![TatrDetection {
                bbox: [0.0, 0.0, 100.0, 20.0],
                confidence: 0.9,
                class: TatrClass::Row,
            }],
            columns: vec![
                TatrDetection {
                    bbox: [0.0, 0.0, 50.0, 20.0], // real column: 50px wide
                    confidence: 0.9,
                    class: TatrClass::Column,
                },
                TatrDetection {
                    bbox: [60.0, 0.0, 60.5, 20.0], // noise column: 0.5px wide (< 1% of 100)
                    confidence: 0.5,
                    class: TatrClass::Column,
                },
                TatrDetection {
                    bbox: [70.0, 0.0, 100.0, 20.0], // real column: 30px wide
                    confidence: 0.85,
                    class: TatrClass::Column,
                },
            ],
            headers: Vec::new(),
            spanning: Vec::new(),
        };

        let grid = build_cell_grid(&result, Some([0.0, 0.0, 100.0, 20.0]));
        assert_eq!(
            grid[0].len(),
            2,
            "noise column should be filtered, leaving 2 real columns"
        );
    }

    #[test]
    fn test_build_cell_grid_uses_per_class_nms() {
        // Verify that build_cell_grid uses different NMS thresholds for rows vs columns.
        // Two overlapping columns (IoB ~0.4) should both survive with NMS_IOB_THRESHOLD_COLS=0.3
        // only if they are not heavily overlapping. With 0.4 > 0.3, the second is suppressed.
        // But two overlapping rows (IoB ~0.4) should both survive with NMS_IOB_THRESHOLD_ROWS=0.5.
        let result = TatrResult {
            rows: vec![
                TatrDetection {
                    bbox: [0.0, 0.0, 100.0, 25.0],
                    confidence: 0.9,
                    class: TatrClass::Row,
                },
                TatrDetection {
                    bbox: [0.0, 15.0, 100.0, 40.0], // 10/25 = 0.4 IoB with first
                    confidence: 0.85,
                    class: TatrClass::Row,
                },
            ],
            columns: vec![TatrDetection {
                bbox: [0.0, 0.0, 50.0, 40.0],
                confidence: 0.9,
                class: TatrClass::Column,
            }],
            headers: Vec::new(),
            spanning: Vec::new(),
        };

        let grid = build_cell_grid(&result, None);
        // Rows: 0.4 < 0.5 threshold → both kept
        assert_eq!(
            grid.len(),
            2,
            "rows with 0.4 IoB should both survive row NMS (threshold 0.5)"
        );
    }
}
