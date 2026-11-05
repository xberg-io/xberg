use super::*;

const LABELS: [&str; 3] = ["ALPHA FIRST", "BRAVO NEXT", "OMEGA LAST"];

fn blocks(layout: &[(u32, u32); 3]) -> Vec<xberg_paddle_ocr::DetailedTextBlock> {
    LABELS
        .iter()
        .zip(layout)
        .map(|(text, &(left, top))| xberg_paddle_ocr::DetailedTextBlock {
            block: xberg_paddle_ocr::TextBlock {
                text: (*text).into(),
                box_points: vec![
                    xberg_paddle_ocr::Point { x: left, y: top },
                    xberg_paddle_ocr::Point {
                        x: left + 20,
                        y: top + 10,
                    },
                ],
                ..Default::default()
            },
            ..Default::default()
        })
        .collect()
}

fn source_pixels() -> image::RgbImage {
    let mut upright = image::RgbImage::new(240, 120);
    for (index, &(x, y)) in [(10, 10), (100, 50), (190, 90)].iter().enumerate() {
        upright.put_pixel(x, y, image::Rgb([17 + index as u8, 31, 47]));
    }
    upright
}

fn corrected_pixels(detected: u32) -> (RotationOutcome, image::RgbImage) {
    let upright = source_pixels();
    let raw = match detected {
        90 => image::imageops::rotate90(&upright),
        180 => image::imageops::rotate180(&upright),
        270 => image::imageops::rotate270(&upright),
        _ => upright.clone(),
    };
    let outcome = rotate_for_detected_orientation(
        &raw,
        crate::doc_orientation::OrientationResult {
            degrees: detected,
            confidence: 0.9,
        },
    )
    .unwrap();
    let decoded = outcome
        .rotated_bytes
        .as_deref()
        .map(|bytes| image::load_from_memory(bytes).unwrap().to_rgb8())
        .unwrap_or(raw);
    assert_eq!(decoded, upright, "actual corrected pixels must be upright");
    (outcome, decoded)
}

fn assert_corrected_order(page_rotation: u32, detected: u32) {
    let (outcome, decoded) = corrected_pixels(detected);
    let detected = outcome.auto_rotated().then_some(outcome.orientation.unwrap().degrees);
    let residual = PaddleOcrBackend::residual_rotation_for_reorder(page_rotation, detected);
    for layout in [
        [(10, 10), (10, 50), (10, 90)],
        [(10, 10), (100, 10), (190, 10)],
        [(10, 10), (100, 50), (50, 90)],
    ] {
        let mut blocks = blocks(&layout);
        let geometry: Vec<_> = blocks.iter().map(|block| block.block.box_points.clone()).collect();
        PaddleOcrBackend::reorder_blocks_for_page_rotation(&mut blocks, residual, decoded.width(), decoded.height());
        assert_eq!(
            blocks.iter().map(|block| block.block.text.as_str()).collect::<Vec<_>>(),
            LABELS,
            "page /Rotate {page_rotation}: corrected stack, horizontal and staggered source order"
        );
        assert_eq!(
            blocks
                .iter()
                .map(|block| block.block.box_points.clone())
                .collect::<Vec<_>>(),
            geometry,
            "ordering must preserve the OCR raster geometry"
        );
    }
}

#[test]
fn corrected_rotate90_source_labels_keep_display_order() {
    assert_corrected_order(90, 270);
}

#[test]
fn corrected_rotate270_source_labels_keep_display_order() {
    assert_corrected_order(270, 90);
}

#[test]
fn corrected_rotate180_source_labels_keep_display_order() {
    assert_corrected_order(180, 180);
}

#[test]
fn upright_rotate0_source_labels_keep_display_order() {
    assert_corrected_order(0, 0);
}

#[test]
fn low_confidence_and_unsupported_angles_leave_the_raw_raster_and_page_hint() {
    let raw = source_pixels();
    for (detected, confidence) in [(90, 0.1), (180, 0.1), (270, 0.1), (45, 0.9)] {
        let outcome = rotate_for_detected_orientation(
            &raw,
            crate::doc_orientation::OrientationResult {
                degrees: detected,
                confidence,
            },
        )
        .unwrap();
        assert!(!outcome.auto_rotated());
        assert!(outcome.rotated_bytes.is_none());
        assert_eq!((outcome.processed_width, outcome.processed_height), raw.dimensions());
        let detected = outcome.auto_rotated().then_some(outcome.orientation.unwrap().degrees);
        for page in [0, 90, 180, 270] {
            assert_eq!(PaddleOcrBackend::residual_rotation_for_reorder(page, detected), page);
        }
    }
}
