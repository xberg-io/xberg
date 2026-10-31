use super::*;

#[test]
fn corner_rows_preserve_threshold_boundary_unknown_class_filter_and_coordinate_scales() {
    let rows = [
        1.0, 2.0, 3.0, 4.0, 0.5, 0.0, 1.0, 2.0, 3.0, 4.0, 0.49, 0.0, 1.0, 2.0, 3.0, 4.0, 0.9, 99.0, 1.0, 2.0, 3.0, 4.0,
        0.75, 1.0,
    ];
    let detections = decode_yolov10_corners(YoloVariant::DocStructBench, &rows, 4, 0.5, (2.0, 3.0));
    assert_eq!(detections.len(), 2);
    assert_eq!(detections[0].class_name, LayoutClass::Title);
    assert_eq!(detections[0].confidence, 0.5);
    assert_eq!(detections[0].bbox, BBox::new(2.0, 6.0, 6.0, 12.0));
    assert_eq!(detections[1].class_name, LayoutClass::Text);
    assert_eq!(detections[1].confidence, 0.75);
}

#[test]
fn center_rows_keep_first_class_on_ties_and_filter_scores_below_threshold() {
    let rows = [
        10.0, 20.0, 4.0, 8.0, 0.5, 0.5, 0.25, 10.0, 20.0, 4.0, 8.0, 0.49, 0.1, 0.2,
    ];
    let detections = decode_yolov10_centers(YoloVariant::DocLayNet, &rows, 2, 7, 0.5, (2.0, 3.0));
    assert_eq!(detections.len(), 1);
    assert_eq!(detections[0].class_name, LayoutClass::Caption);
    assert_eq!(detections[0].confidence, 0.5);
    assert_eq!(detections[0].bbox, BBox::new(16.0, 48.0, 24.0, 72.0));
}

#[test]
fn yolox_grid_preserves_stride_and_row_major_anchor_order() {
    let grid = build_yolox_grid(32, 32, 21).expect("three stride levels");
    assert_eq!(grid.len(), 21);
    assert_eq!(grid[0], (0.0, 0.0, 8.0));
    assert_eq!(grid[3], (3.0, 0.0, 8.0));
    assert_eq!(grid[4], (0.0, 1.0, 8.0));
    assert_eq!(grid[15], (3.0, 3.0, 8.0));
    assert_eq!(grid[16], (0.0, 0.0, 16.0));
    assert_eq!(grid[19], (1.0, 1.0, 16.0));
    assert_eq!(grid[20], (0.0, 0.0, 32.0));
    let error = build_yolox_grid(32, 32, 22).expect_err("model shape must match the grid");
    assert_eq!(
        error.to_string(),
        "Invalid model output: Grid anchor count mismatch: expected 21 from strides, got 22 from model output"
    );
}

#[test]
fn yolox_rows_apply_objectness_stride_and_letterbox_scale_before_filtering() {
    let rows = [0.0, 0.0, 0.0, 0.0, 0.5, 0.5, 0.5, 0.0, 0.0, 0.0, 0.0, 0.5, 0.49, 0.1];
    let grids = [(1.0, 2.0, 8.0), (2.0, 2.0, 16.0)];
    let detections = decode_yolox_anchors(YoloVariant::Yolox, &rows, (7, 2), &grids, 0.25, 0.5);
    assert_eq!(detections.len(), 1);
    assert_eq!(detections[0].class_name, LayoutClass::Caption);
    assert_eq!(detections[0].confidence, 0.25);
    assert_eq!(detections[0].bbox, BBox::new(8.0, 24.0, 24.0, 40.0));
}

#[test]
fn yolov10_output_dimensions_preserve_batched_and_unbatched_shapes() {
    assert_eq!(yolov10_output_dimensions(&[1, 2, 6]).expect("batched"), (2, 6));
    assert_eq!(yolov10_output_dimensions(&[2, 6]).expect("unbatched"), (2, 6));
    let error = yolov10_output_dimensions(&[2]).expect_err("unsupported rank");
    assert_eq!(error.to_string(), "Invalid model output: Unexpected output shape: [2]");
}
