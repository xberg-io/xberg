//! Borrowed singleton outputs keep raw ranks and masks paired in source-row order.

use ndarray::{ArrayView1, ArrayView2, ArrayView3, Ix1, Ix2, Ix3};

use crate::inference::InferenceTensor;
use crate::layout::error::LayoutError;

use super::DET_ROW_COLS;

const MASK_SIDE: usize = 200;
const COL_RANK: usize = 6;

pub(super) struct ValidatedOutput<'a> {
    boxes: ArrayView2<'a, f32>,
    masks: ArrayView3<'a, i32>,
    count: usize,
}

fn invalid(message: impl Into<String>) -> LayoutError {
    LayoutError::InvalidOutput(message.into())
}

fn named<'a>(outputs: &'a [(String, InferenceTensor)], name: &str) -> Result<&'a InferenceTensor, LayoutError> {
    let mut matching = outputs.iter().filter(|(key, _)| key == name);
    let value = matching
        .next()
        .ok_or_else(|| invalid(format!("PP-DocLayout-V3 is missing {name}")))?;
    if matching.next().is_some() {
        return Err(invalid(format!("PP-DocLayout-V3 has duplicate {name}")));
    }
    Ok(&value.1)
}

impl<'a> ValidatedOutput<'a> {
    pub(super) fn from_outputs(outputs: &'a [(String, InferenceTensor)]) -> Result<Self, LayoutError> {
        let InferenceTensor::F32(boxes) = named(outputs, "fetch_name_0")? else {
            return Err(invalid("fetch_name_0 must be F32[N,7]"));
        };
        let InferenceTensor::I32(count) = named(outputs, "fetch_name_1")? else {
            return Err(invalid("fetch_name_1 must be I32[1]"));
        };
        let InferenceTensor::I32(masks) = named(outputs, "fetch_name_2")? else {
            return Err(invalid("fetch_name_2 must be I32[N,200,200]"));
        };
        if outputs.len() != 3 {
            return Err(invalid("PP-DocLayout-V3 requires exactly three named outputs"));
        }
        let boxes = boxes
            .view()
            .into_dimensionality::<Ix2>()
            .map_err(|_| invalid("fetch_name_0 must be rank two"))?;
        let count = count
            .view()
            .into_dimensionality::<Ix1>()
            .map_err(|_| invalid("fetch_name_1 must be rank one"))?;
        let masks = masks
            .view()
            .into_dimensionality::<Ix3>()
            .map_err(|_| invalid("fetch_name_2 must be rank three"))?;
        if boxes.ncols() != DET_ROW_COLS || boxes.nrows() == 0 {
            return Err(invalid("fetch_name_0 must contain nonempty seven-column rows"));
        }
        if count.len() != 1 {
            return Err(invalid("fetch_name_1 must contain exactly one B1 count"));
        }
        let count = usize::try_from(count[0]).map_err(|_| invalid("fetch_name_1 contains a negative count"))?;
        if count != boxes.nrows() || masks.shape() != [count, MASK_SIDE, MASK_SIDE] {
            return Err(invalid("PP-DocLayout-V3 row/count/mask dimensions disagree"));
        }
        Self::validate_values(boxes, masks)?;
        Ok(Self { boxes, masks, count })
    }

    fn validate_values(boxes: ArrayView2<'_, f32>, masks: ArrayView3<'_, i32>) -> Result<(), LayoutError> {
        if boxes
            .column(COL_RANK)
            .iter()
            .any(|rank| !rank.is_finite() || *rank < 0.0 || rank.fract() != 0.0)
        {
            return Err(invalid("PP-DocLayout-V3 raw ranks must be finite nonnegative integers"));
        }
        if masks.iter().any(|value| !matches!(*value, 0 | 1)) {
            return Err(invalid("PP-DocLayout-V3 masks must be binary I32 values"));
        }
        Ok(())
    }

    pub(super) fn count(&self) -> usize {
        self.count
    }

    pub(super) fn rows(&self) -> impl Iterator<Item = (usize, ArrayView1<'_, f32>, ArrayView2<'_, i32>)> {
        self.boxes
            .outer_iter()
            .zip(self.masks.outer_iter())
            .enumerate()
            .map(|(source_row, (row, mask))| (source_row, row, mask))
    }
}
