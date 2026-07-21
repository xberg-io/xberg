//! Engine-neutral tensor currency for the inference seam.
//!
//! [`InferenceTensor`] is the type exchanged across the [`InferenceSession`] API,
//! independent of the backing engine (ONNX Runtime today; tract on no-ORT targets
//! in a later phase). Each variant wraps an owned dynamic-dimensional
//! [`ndarray::ArrayD`] — callers build inputs with [`ndarray`] and read outputs
//! back as views or slices. Engine-specific conversions (`ort::Value` ↔ tensor)
//! live next to their backend, keeping this module pure `ndarray`.
//!
//! [`InferenceSession`]: super::InferenceSession
//!
//! Since v5.0.0 (issue #1275).

use ndarray::ArrayD;

/// A dense tensor passed across the inference seam.
///
/// The dtype set mirrors the ONNX tensor element types xberg's models actually
/// exchange. New dtypes are added here as models that need them are migrated onto
/// the seam.
#[derive(Debug, Clone, PartialEq)]
pub enum InferenceTensor {
    /// 32-bit float — the common image/logit currency.
    F32(ArrayD<f32>),
    /// 64-bit signed integer — token ids, shape inputs.
    I64(ArrayD<i64>),
    /// 32-bit signed integer — box counts and similar.
    I32(ArrayD<i32>),
    /// 8-bit unsigned integer — raw pixel inputs.
    U8(ArrayD<u8>),
    /// Boolean — attention/pad masks that ship as bool tensors.
    Bool(ArrayD<bool>),
}

impl InferenceTensor {
    /// Borrow the `f32` payload, or `None` when the tensor holds another dtype.
    #[allow(dead_code)]
    pub fn as_f32(&self) -> Option<&ArrayD<f32>> {
        match self {
            Self::F32(array) => Some(array),
            _ => None,
        }
    }
}

impl From<ArrayD<f32>> for InferenceTensor {
    fn from(array: ArrayD<f32>) -> Self {
        Self::F32(array)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn as_f32_returns_payload_for_f32_and_none_otherwise() {
        let f32_tensor: InferenceTensor = ndarray::arr1(&[1.0f32, 2.0]).into_dyn().into();
        assert_eq!(f32_tensor.as_f32().unwrap(), &ndarray::arr1(&[1.0f32, 2.0]).into_dyn());

        let i64_tensor = InferenceTensor::I64(ndarray::arr1(&[1i64, 2]).into_dyn());
        assert!(i64_tensor.as_f32().is_none());
    }
}
