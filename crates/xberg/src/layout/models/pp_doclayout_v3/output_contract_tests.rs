use std::sync::Arc;
use std::sync::atomic::{AtomicUsize, Ordering};

use image::RgbImage;
use ndarray::{Array, IxDyn, arr1, arr2};

use super::PpDocLayoutV3Model;
use crate::inference::{InferenceError, InferenceSession, InferenceTensor};
use crate::layout::error::LayoutError;
use crate::layout::types::LayoutDetection;

const ROWS: usize = 4;
const MASK_SIDE: usize = 200;
const MASK: usize = 0;
const COUNT: usize = 1;
const BOXES: usize = 2;

struct SyntheticOutputSession {
    names: Vec<String>,
    outputs: Vec<(String, InferenceTensor)>,
    calls: Arc<AtomicUsize>,
}

impl InferenceSession for SyntheticOutputSession {
    fn run(&self, _inputs: Vec<(String, InferenceTensor)>) -> Result<Vec<(String, InferenceTensor)>, InferenceError> {
        self.calls.fetch_add(1, Ordering::SeqCst);
        Ok(self.outputs.clone())
    }

    fn input_names(&self) -> &[String] {
        &self.names
    }
}

fn outputs() -> Vec<(String, InferenceTensor)> {
    let mut masks = Array::zeros((ROWS, MASK_SIDE, MASK_SIDE));
    for row in 0..ROWS {
        masks[[row, row, row * 2]] = 1;
    }
    vec![
        ("fetch_name_2".to_owned(), InferenceTensor::I32(masks.into_dyn())),
        (
            "fetch_name_1".to_owned(),
            InferenceTensor::I32(arr1(&[ROWS as i32]).into_dyn()),
        ),
        (
            "fetch_name_0".to_owned(),
            InferenceTensor::F32(
                arr2(&[
                    [22.0, 0.9, 40.0, 2.0, 50.0, 10.0, 42.0],
                    [22.0, 0.6, 10.0, 2.0, 20.0, 10.0, 7.0],
                    [22.0, 0.4, 20.0, 2.0, 30.0, 10.0, 2.0],
                    [22.0, 0.8, 30.0, 2.0, 40.0, 10.0, 7.0],
                ])
                .into_dyn(),
            ),
        ),
    ]
}

fn infer(outputs: Vec<(String, InferenceTensor)>) -> Result<Vec<LayoutDetection>, LayoutError> {
    let names: Vec<String> = ["im_shape", "image", "scale_factor"].map(str::to_owned).into();
    let calls = Arc::new(AtomicUsize::new(0));
    let session = SyntheticOutputSession {
        names: names.clone(),
        outputs,
        calls: Arc::clone(&calls),
    };
    let mut model = PpDocLayoutV3Model {
        session: Box::new(session),
        input_names: names,
    };
    let result = model.run_inference(&RgbImage::new(64, 80), 0.5);
    assert_eq!(
        calls.load(Ordering::SeqCst),
        1,
        "actual adapter must run exactly one synthetic output boundary"
    );
    result
}

fn invalid(outputs: Vec<(String, InferenceTensor)>, case: &str) {
    let result = infer(outputs);
    assert!(
        matches!(result, Err(LayoutError::InvalidOutput(_))),
        "{case}: expected schema rejection, got {result:?}"
    );
}

#[test]
fn should_accept_shuffled_named_outputs_and_preserve_legacy_confidence_order() {
    let source = outputs();
    assert_eq!(source.len(), 3);
    let detections = infer(source.clone()).unwrap();
    assert_eq!(detections.len(), 3);
    assert_eq!(
        detections
            .iter()
            .map(|detection| detection.confidence)
            .collect::<Vec<_>>(),
        [0.9, 0.8, 0.6]
    );
    assert_eq!(
        detections.iter().map(|detection| detection.bbox.x1).collect::<Vec<_>>(),
        [40.0, 30.0, 10.0]
    );
    let mut filtered = source;
    let InferenceTensor::F32(rows) = &mut filtered[BOXES].1 else {
        panic!("real F32 rows")
    };
    for row in 0..ROWS {
        rows[[row, 1]] = 0.25;
    }
    assert!(
        infer(filtered).unwrap().is_empty(),
        "valid full raw output can have zero kept detections"
    );
}

#[test]
fn should_accept_nonconsecutive_and_repeated_provided_ranks_without_deduplicating_rows() {
    let mut source = outputs();
    let InferenceTensor::F32(rows) = &mut source[BOXES].1 else {
        panic!("real F32 rows")
    };
    assert_eq!(
        (0..ROWS).map(|row| rows[[row, 6]]).collect::<Vec<_>>(),
        [42.0, 7.0, 2.0, 7.0]
    );
    for row in 0..ROWS {
        rows[[row, 1]] = 0.75;
    }
    let detections = infer(source).unwrap();
    assert_eq!(detections.len(), ROWS, "equal ranks are legitimate selected-query rows");
    assert_eq!(
        detections.iter().map(|detection| detection.bbox.x1).collect::<Vec<_>>(),
        [40.0, 10.0, 20.0, 30.0]
    );
}

#[test]
fn should_reject_missing_named_output_instead_of_succeeding_with_zero_work() {
    for index in [MASK, COUNT, BOXES] {
        let mut source = outputs();
        let name = source.remove(index).0;
        invalid(source, &format!("missing {name}"));
    }
}

#[test]
fn should_reject_duplicate_named_outputs_instead_of_last_value_winning() {
    for index in [MASK, COUNT, BOXES] {
        let mut source = outputs();
        let duplicate = source[index].clone();
        let name = duplicate.0.clone();
        source.push(duplicate);
        invalid(source, &format!("duplicate {name}"));
    }
}

#[test]
fn should_reject_wrong_output_dtypes_before_projecting_detections() {
    let cases = [
        (BOXES, InferenceTensor::I32(Array::zeros((ROWS, 7)).into_dyn())),
        (COUNT, InferenceTensor::F32(arr1(&[ROWS as f32]).into_dyn())),
        (
            MASK,
            InferenceTensor::I64(Array::zeros((ROWS, MASK_SIDE, MASK_SIDE)).into_dyn()),
        ),
    ];
    assert_eq!(cases.len(), 3);
    for (index, replacement) in cases {
        let mut source = outputs();
        source[index].1 = replacement;
        invalid(source, &format!("wrong dtype for {}", outputs()[index].0));
    }
}

#[test]
fn should_reject_wrong_row_axes_columns_and_count_axes_even_when_flat_lengths_match() {
    let cases = [
        (BOXES, vec![ROWS * 7]),
        (BOXES, vec![1, ROWS, 7]),
        (BOXES, vec![7, ROWS]),
        (COUNT, vec![1, 1]),
    ];
    assert_eq!(cases.len(), 4);
    for (index, shape) in cases {
        let mut source = outputs();
        source[index].1 = match &source[index].1 {
            InferenceTensor::F32(array) => {
                InferenceTensor::F32(array.clone().into_shape_with_order(IxDyn(&shape)).unwrap())
            }
            InferenceTensor::I32(array) => {
                InferenceTensor::I32(array.clone().into_shape_with_order(IxDyn(&shape)).unwrap())
            }
            _ => panic!("real fixture F32 rows or I32 count"),
        };
        invalid(source, &format!("wrong axes {shape:?}"));
    }
}

#[test]
fn should_reject_invalid_counts_without_clamping_defaulting_or_truncating() {
    let cases = [vec![-1], vec![0], vec![3], vec![5], vec![], vec![2, 2]];
    assert_eq!(cases.len(), 6);
    for count in cases {
        let mut source = outputs();
        source[COUNT].1 = InferenceTensor::I32(arr1(&count).into_dyn());
        invalid(source, &format!("inconsistent B1 count {count:?}"));
    }
}

#[test]
fn should_reject_nonfinite_fractional_and_negative_ranks() {
    let cases = [-1.0, 0.5, f32::NAN, f32::INFINITY, f32::NEG_INFINITY];
    assert_eq!(cases.len(), 5);
    for rank in cases {
        let mut source = outputs();
        let InferenceTensor::F32(rows) = &mut source[BOXES].1 else {
            panic!("real F32 rows")
        };
        rows[[0, 6]] = rank;
        invalid(source, &format!("invalid rank {rank:?}"));
    }
}

#[test]
fn should_reject_mask_row_count_and_grid_mismatches() {
    let cases = [
        vec![3, MASK_SIDE, MASK_SIDE],
        vec![ROWS, 100, 400],
        vec![ROWS, MASK_SIDE * MASK_SIDE],
    ];
    assert_eq!(cases.len(), 3);
    for shape in cases {
        let mut source = outputs();
        source[MASK].1 = InferenceTensor::I32(Array::zeros(IxDyn(&shape)));
        invalid(source, &format!("misaligned mask shape {shape:?}"));
    }
}

#[test]
fn should_reject_nonbinary_mask_values_without_losing_row_alignment() {
    for value in [-1, 2] {
        let mut source = outputs();
        let InferenceTensor::I32(masks) = &mut source[MASK].1 else {
            panic!("real I32 masks")
        };
        masks[[1, 23, 47]] = value;
        invalid(source, &format!("nonbinary mask value {value}"));
    }
}
