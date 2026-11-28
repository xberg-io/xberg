use super::*;
use crate::ocr_result::{TextBox, TextLine};

fn make_box(x: u32, y: u32) -> TextBox {
    TextBox {
        points: vec![
            Point { x, y },
            Point { x: x + 100, y },
            Point { x: x + 100, y: y + 20 },
            Point { x, y: y + 20 },
        ],
        score: 0.9,
    }
}

fn make_detailed_line() -> DetailedTextLine {
    DetailedTextLine {
        line: TextLine {
            text: "ab".to_string(),
            text_score: 0.8,
        },
        words: vec![RecognizedWord {
            text: "ab".to_string(),
            columns: vec![2, 3],
            start_column: 2,
            end_column: 3,
            confidence: 0.8,
        }],
        line_column_count: 10.0,
    }
}

fn project_test_line(
    box_points: Vec<Point>,
    crop_dimensions: (u32, u32),
    rotation_retained: bool,
    padded_dimensions: (u32, u32),
) -> Vec<Point> {
    let crop = image::RgbImage::new(crop_dimensions.0, crop_dimensions.1);
    let projection = CropProjectionMetadata::new(&box_points, &crop);
    let line = make_detailed_line();
    PaddleOcrEngine::build_word_blocks(
        &line.line,
        line.words,
        line.line_column_count,
        projection,
        rotation_retained,
        PageGeometry {
            padding: 10,
            dimensions: padded_dimensions,
        },
    )[0]
    .box_points
    .clone()
}

#[test]
fn should_project_horizontal_word_columns_to_source_quadrilateral() {
    let points = make_box(10, 20).points;

    let projected = project_test_line(points, (100, 20), false, (120, 60));

    assert_eq!(
        projected,
        [
            Point { x: 20, y: 10 },
            Point { x: 40, y: 10 },
            Point { x: 40, y: 30 },
            Point { x: 20, y: 30 },
        ]
    );
}

#[test]
fn should_reverse_retained_180_degree_rotation_when_projecting_word() {
    let points = make_box(10, 20).points;

    let projected = project_test_line(points, (100, 20), true, (120, 60));

    assert_eq!(
        projected,
        [
            Point { x: 60, y: 10 },
            Point { x: 80, y: 10 },
            Point { x: 80, y: 30 },
            Point { x: 60, y: 30 },
        ]
    );
}

#[test]
fn should_not_reverse_180_degree_rotation_after_angle_rollback() {
    assert!(PaddleOcrEngine::rotation_retained(1, false));
    assert!(!PaddleOcrEngine::rotation_retained(1, true));

    let projected = project_test_line(make_box(10, 20).points, (100, 20), false, (120, 60));

    assert_eq!(projected[0], Point { x: 20, y: 10 });
    assert_eq!(projected[2], Point { x: 40, y: 30 });
}

#[test]
fn should_reverse_vertical_crop_rotation_when_projecting_word() {
    let points = vec![
        Point { x: 10, y: 10 },
        Point { x: 30, y: 10 },
        Point { x: 30, y: 110 },
        Point { x: 10, y: 110 },
    ];

    let projected = project_test_line(points, (100, 20), false, (40, 120));

    assert_eq!(
        projected,
        [
            Point { x: 0, y: 20 },
            Point { x: 20, y: 20 },
            Point { x: 20, y: 40 },
            Point { x: 0, y: 40 },
        ]
    );
}

#[test]
fn should_omit_geometry_for_degenerate_crop_projection() {
    let points = vec![Point { x: 10, y: 10 }; 4];

    let projected = project_test_line(points, (0, 0), false, (40, 40));

    assert!(projected.is_empty());
}

#[test]
fn should_clamp_padded_batch_columns_to_effective_timeline() {
    let crop = image::RgbImage::new(100, 20);
    let projection = CropProjectionMetadata::new(&make_box(10, 20).points, &crop);
    let mut line = make_detailed_line();
    line.words[0].columns = vec![8, 9];
    line.words[0].start_column = 8;
    line.words[0].end_column = 9;
    line.line_column_count = 5.0;

    let words = PaddleOcrEngine::build_word_blocks(
        &line.line,
        line.words,
        line.line_column_count,
        projection,
        false,
        PageGeometry {
            padding: 10,
            dimensions: (120, 60),
        },
    );

    assert_eq!(words.len(), 1);
    assert_eq!(words[0].word.columns, [8, 9]);
    assert_eq!(
        words[0].box_points,
        [
            Point { x: 65, y: 10 },
            Point { x: 100, y: 10 },
            Point { x: 100, y: 30 },
            Point { x: 65, y: 30 },
        ]
    );
}

#[test]
fn should_convert_detailed_result_to_exact_legacy_fields() {
    let block = TextBlock {
        box_points: make_box(5, 7).points,
        box_score: 0.9,
        angle_index: 1,
        angle_score: 0.8,
        text: "hello".to_string(),
        text_score: 0.7,
    };
    let detailed = DetailedOcrResult {
        text_blocks: vec![DetailedTextBlock {
            block: block.clone(),
            words: vec![WordBlock::from(RecognizedWord {
                text: "hello".to_string(),
                columns: vec![1, 2, 3],
                start_column: 1,
                end_column: 3,
                confidence: 0.7,
            })],
            line_column_count: 5.0,
            rotation_retained: true,
        }],
    };

    let legacy: OcrResult = detailed.into();

    assert_eq!(legacy.text_blocks.len(), 1);
    assert_eq!(legacy.text_blocks[0].box_points, block.box_points);
    assert_eq!(legacy.text_blocks[0].box_score, block.box_score);
    assert_eq!(legacy.text_blocks[0].angle_index, block.angle_index);
    assert_eq!(legacy.text_blocks[0].angle_score, block.angle_score);
    assert_eq!(legacy.text_blocks[0].text, block.text);
    assert_eq!(legacy.text_blocks[0].text_score, block.text_score);
}

#[test]
fn test_sort_text_boxes_top_to_bottom() {
    let mut boxes = vec![make_box(10, 100), make_box(10, 50), make_box(10, 10)];
    PaddleOcrEngine::sort_text_boxes(&mut boxes);
    assert_eq!(boxes[0].points[0].y, 10);
    assert_eq!(boxes[1].points[0].y, 50);
    assert_eq!(boxes[2].points[0].y, 100);
}

#[test]
fn test_sort_text_boxes_same_line_left_to_right() {
    let mut boxes = vec![make_box(200, 10), make_box(100, 10), make_box(50, 10)];
    PaddleOcrEngine::sort_text_boxes(&mut boxes);
    assert_eq!(boxes[0].points[0].x, 50);
    assert_eq!(boxes[1].points[0].x, 100);
    assert_eq!(boxes[2].points[0].x, 200);
}

#[test]
fn test_sort_text_boxes_visual_line_jitter_left_to_right() {
    let mut boxes = vec![make_box(200, 10), make_box(50, 19)];
    PaddleOcrEngine::sort_text_boxes(&mut boxes);
    assert_eq!(boxes.iter().map(|b| b.points[0].x).collect::<Vec<_>>(), vec![50, 200]);
}

#[test]
fn test_sort_text_boxes_tolerance_boundary_stays_top_to_bottom() {
    let mut boxes = vec![make_box(200, 10), make_box(50, 20)];
    PaddleOcrEngine::sort_text_boxes(&mut boxes);
    assert_eq!(boxes.iter().map(|b| b.points[0].x).collect::<Vec<_>>(), vec![200, 50]);
}

#[test]
fn test_sort_text_boxes_bubbles_across_visual_line() {
    let mut boxes = vec![make_box(300, 10), make_box(200, 12), make_box(50, 14)];
    PaddleOcrEngine::sort_text_boxes(&mut boxes);
    assert_eq!(
        boxes.iter().map(|b| b.points[0].x).collect::<Vec<_>>(),
        vec![50, 200, 300]
    );
}

#[test]
fn test_sort_text_boxes_does_not_transitively_merge_lines() {
    let mut boxes = vec![make_box(300, 0), make_box(200, 9), make_box(100, 18)];
    PaddleOcrEngine::sort_text_boxes(&mut boxes);
    assert_eq!(
        boxes.iter().map(|b| b.points[0].x).collect::<Vec<_>>(),
        vec![200, 300, 100]
    );
}

#[test]
fn test_sort_text_boxes_multi_line() {
    let mut boxes = vec![
        make_box(300, 50),
        make_box(100, 100),
        make_box(50, 50),
        make_box(200, 100),
    ];
    PaddleOcrEngine::sort_text_boxes(&mut boxes);

    assert_eq!(boxes[0].points[0].x, 50);
    assert_eq!(boxes[1].points[0].x, 300);
    assert_eq!(boxes[2].points[0].x, 100);
    assert_eq!(boxes[3].points[0].x, 200);
}

#[test]
fn test_sort_text_boxes_empty() {
    let mut boxes: Vec<TextBox> = vec![];
    PaddleOcrEngine::sort_text_boxes(&mut boxes);
    assert!(boxes.is_empty());
}

#[test]
fn test_sort_text_boxes_single() {
    let mut boxes = vec![make_box(10, 20)];
    PaddleOcrEngine::sort_text_boxes(&mut boxes);
    assert_eq!(boxes.len(), 1);
}

#[test]
fn an_engine_without_loaded_models_has_no_angle_classifier() {
    assert!(!PaddleOcrEngine::new().has_angle_classifier());
}
