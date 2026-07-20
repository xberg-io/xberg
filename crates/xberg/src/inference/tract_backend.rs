//! tract implementation of the inference seam — the pure-Rust engine used on
//! no-ORT targets (WASM, Android x86_64).
//!
//! [`TractBackend`] loads an `.onnx` artifact with `tract-onnx`, optimizes it,
//! and builds a runnable plan. Unlike ONNX Runtime, tract is CPU-only: the
//! `accel` argument is accepted for a uniform [`InferenceBackend`] signature but
//! ignored (there is no execution-provider selection). [`TractSession`] holds the
//! plan and converts tensors at the boundary via engine-neutral `(shape, data)`,
//! so no `ndarray` version needs to be shared between xberg and tract.
//!
//! tract resolves symbolic dimensions (batch, sequence, image height/width) from
//! the concrete input at run time, so one plan serves every input size — the plan
//! is built once at load. Models whose ONNX graph carries symbols tract cannot
//! reconcile (e.g. the quantized TATR export's symbolic scale tensors) stay on
//! ONNX Runtime; see the Phase-0 coverage matrix.
//!
//! Since v5.0.0 (issue #1275).

use std::path::Path;
use std::sync::Arc;

use tract_onnx::prelude::{
    Datum, DatumType, Framework, InferenceModelExt, IntoRunnable, TValue, Tensor, TractResult, TypedRunnableModel,
};

use crate::core::config::acceleration::AccelerationConfig;

use super::backend::{InferenceBackend, InferenceError, InferenceSession};
use super::tensor::InferenceTensor;

/// The pure-Rust tract inference backend.
///
/// Zero-sized; construct with [`TractBackend::new`].
pub struct TractBackend;

impl TractBackend {
    /// Create a new tract backend.
    pub fn new() -> Self {
        Self
    }
}

impl Default for TractBackend {
    fn default() -> Self {
        Self::new()
    }
}

impl InferenceBackend for TractBackend {
    fn load(
        &self,
        model_path: &Path,
        _accel: Option<&AccelerationConfig>,
    ) -> Result<Box<dyn InferenceSession>, InferenceError> {
        let plan = (|| -> TractResult<Arc<TypedRunnableModel>> {
            tract_onnx::onnx()
                .model_for_path(model_path)?
                .into_optimized()?
                .into_runnable()
        })()
        .map_err(|e| InferenceError::Load(e.to_string()))?;
        Ok(session_from_plan(plan))
    }

    fn load_from_memory(
        &self,
        model_bytes: &[u8],
        _accel: Option<&AccelerationConfig>,
    ) -> Result<Box<dyn InferenceSession>, InferenceError> {
        let plan = (|| -> TractResult<Arc<TypedRunnableModel>> {
            tract_onnx::onnx()
                .model_for_read(&mut std::io::Cursor::new(model_bytes))?
                .into_optimized()?
                .into_runnable()
        })()
        .map_err(|e| InferenceError::Load(e.to_string()))?;
        Ok(session_from_plan(plan))
    }
}

/// Wrap a runnable plan behind the neutral session trait, reading input/output
/// names from the graph in declared order (tract runs positionally).
fn session_from_plan(plan: Arc<TypedRunnableModel>) -> Box<dyn InferenceSession> {
    let graph = plan.model();
    let input_names = graph.inputs.iter().map(|o| graph.node(o.node).name.clone()).collect();
    let output_names = graph.outputs.iter().map(|o| graph.node(o.node).name.clone()).collect();
    Box::new(TractSession {
        plan,
        input_names,
        output_names,
    })
}

/// A tract runnable plan behind the neutral [`InferenceSession`] trait.
pub struct TractSession {
    plan: Arc<TypedRunnableModel>,
    input_names: Vec<String>,
    output_names: Vec<String>,
}

impl InferenceSession for TractSession {
    fn run(&self, inputs: Vec<(String, InferenceTensor)>) -> Result<Vec<(String, InferenceTensor)>, InferenceError> {
        // tract runs positionally in the graph's declared input order; reorder the
        // named inputs to match, converting each across the boundary.
        let mut ordered = Vec::with_capacity(self.input_names.len());
        for name in &self.input_names {
            let tensor = inputs
                .iter()
                .find(|(n, _)| n == name)
                .map(|(_, t)| t)
                .ok_or_else(|| InferenceError::Run(format!("missing input '{name}'")))?;
            let tract_tensor = tensor_to_tract(tensor)?;
            ordered.push(TValue::from(tract_tensor));
        }

        let outputs = self
            .plan
            .run(ordered.into_iter().collect())
            .map_err(|e| InferenceError::Run(e.to_string()))?;

        let mut result = Vec::with_capacity(outputs.len());
        for (index, value) in outputs.iter().enumerate() {
            let name = self
                .output_names
                .get(index)
                .cloned()
                .unwrap_or_else(|| index.to_string());
            result.push((name, tract_to_tensor(value)?));
        }
        Ok(result)
    }

    fn input_names(&self) -> &[String] {
        &self.input_names
    }
}

/// Convert a neutral input tensor into a tract tensor via a contiguous slice.
fn tensor_to_tract(tensor: &InferenceTensor) -> Result<Tensor, InferenceError> {
    fn build<T: Datum + Copy>(array: &ndarray::ArrayD<T>) -> Result<Tensor, InferenceError> {
        let standard = array.as_standard_layout();
        // `as_standard_layout()` yields a C-contiguous array, so `as_slice()` is
        // `Some` here; guard rather than panic to keep the seam fully `Result`-based.
        let slice = standard
            .as_slice()
            .ok_or_else(|| InferenceError::Tensor("standard-layout array is not contiguous".to_string()))?;
        Tensor::from_shape(standard.shape(), slice).map_err(|e| InferenceError::Tensor(e.to_string()))
    }
    match tensor {
        InferenceTensor::F32(array) => build(array),
        InferenceTensor::I64(array) => build(array),
        InferenceTensor::I32(array) => build(array),
        InferenceTensor::U8(array) => build(array),
        InferenceTensor::Bool(array) => build(array),
    }
}

/// Convert a tract output tensor into a neutral tensor, copying via `(shape, data)`.
fn tract_to_tensor(tensor: &Tensor) -> Result<InferenceTensor, InferenceError> {
    fn extract<T: Datum + Copy>(tensor: &Tensor) -> Result<ndarray::ArrayD<T>, InferenceError> {
        let view = tensor
            .to_plain_array_view::<T>()
            .map_err(|e| InferenceError::Tensor(e.to_string()))?;
        let shape = view.shape().to_vec();
        let data: Vec<T> = view.iter().copied().collect();
        ndarray::ArrayD::from_shape_vec(shape, data).map_err(|e| InferenceError::Tensor(e.to_string()))
    }
    let tensor = match tensor.datum_type() {
        DatumType::F32 => InferenceTensor::F32(extract(tensor)?),
        DatumType::I64 => InferenceTensor::I64(extract(tensor)?),
        DatumType::I32 => InferenceTensor::I32(extract(tensor)?),
        DatumType::U8 => InferenceTensor::U8(extract(tensor)?),
        DatumType::Bool => InferenceTensor::Bool(extract(tensor)?),
        other => {
            return Err(InferenceError::Tensor(format!(
                "unsupported output datum type {other:?}"
            )));
        }
    };
    Ok(tensor)
}

#[cfg(all(test, inference_ort))]
mod tests {
    use super::*;
    use crate::inference::ort_backend::OrtBackend;

    /// HF repository holding the PP-LCNet classifiers compared here.
    const PARITY_REPO: &str = "xberg-io/paddleocr-onnx-models";

    /// Whether the parity comparison is mandatory. CI sets
    /// `XBERG_REQUIRE_TRACT_PARITY=1` so a missing model is a hard failure, not a
    /// silent skip — otherwise these tests could pass by comparing nothing.
    fn parity_required() -> bool {
        std::env::var_os("XBERG_REQUIRE_TRACT_PARITY").is_some()
    }

    /// Locate a cached ONNX model by the tail of its path in the HuggingFace hub
    /// cache, returning `None` when it is not present.
    fn cached_model(suffix: &str) -> Option<std::path::PathBuf> {
        let home = std::env::var("HOME").ok()?;
        let root = std::path::Path::new(&home).join(".cache/huggingface/hub");
        let mut stack = vec![root];
        let mut best: Option<std::path::PathBuf> = None;
        while let Some(dir) = stack.pop() {
            let Ok(entries) = std::fs::read_dir(&dir) else { continue };
            for entry in entries.flatten() {
                let path = entry.path();
                if path.is_dir() {
                    stack.push(path);
                } else if path.to_string_lossy().ends_with(suffix) {
                    best = Some(path);
                }
            }
        }
        best
    }

    /// Resolve a parity model to a local path. Uses the HF cache when present;
    /// otherwise, when parity is required (CI), downloads it via the production
    /// path and fails loudly on error. Returns `None` only for a local run where
    /// the model is absent and parity is not required — the self-skip case that
    /// keeps the suite runnable offline without ever passing vacuously in CI.
    fn resolve_model(filename: &str) -> Option<std::path::PathBuf> {
        if let Some(path) = cached_model(filename) {
            return Some(path);
        }
        if parity_required() {
            return Some(download_parity_model(filename));
        }
        None
    }

    #[cfg(any(
        feature = "paddle-ocr",
        feature = "layout-detection",
        auto_rotate,
        feature = "ner-onnx",
        feature = "candle-paddleocr-vl"
    ))]
    fn download_parity_model(filename: &str) -> std::path::PathBuf {
        crate::model_download::hf_download(PARITY_REPO, filename)
            .expect("XBERG_REQUIRE_TRACT_PARITY is set but the parity model download failed")
    }

    #[cfg(not(any(
        feature = "paddle-ocr",
        feature = "layout-detection",
        auto_rotate,
        feature = "ner-onnx",
        feature = "candle-paddleocr-vl"
    )))]
    fn download_parity_model(_filename: &str) -> std::path::PathBuf {
        panic!(
            "XBERG_REQUIRE_TRACT_PARITY is set but no model-download feature is enabled; \
             build the parity tests with `--features full,tract`"
        );
    }

    /// The two PP-LCNet CNN classifiers migrated onto the seam in Phase 1. Both are
    /// fixed 224×224 NCHW; tract and ORT must agree within a tight tolerance.
    #[test]
    fn tract_matches_ort_on_cnn_classifiers() {
        let cases: &[(&str, [usize; 4], usize)] = &[
            ("v2/classifiers/PP-LCNet_x1_0_table_cls.onnx", [1, 3, 224, 224], 2),
            ("v2/classifiers/PP-LCNet_x1_0_doc_ori.onnx", [1, 3, 224, 224], 4),
        ];

        let mut ran = 0;
        for (suffix, shape, out_len) in cases {
            let Some(path) = resolve_model(suffix) else {
                eprintln!("skip: {suffix} not in HF cache");
                continue;
            };
            ran += 1;

            // Deterministic pseudo-image so both engines see identical input.
            let count: usize = shape.iter().product();
            let data: Vec<f32> = (0..count).map(|i| ((i % 255) as f32) / 255.0 - 0.5).collect();
            let input = ndarray::ArrayD::from_shape_vec(shape.to_vec(), data).unwrap();
            let named = |session: &dyn InferenceSession| {
                let input_name = session.input_names().first().cloned().unwrap();
                session
                    .run(vec![(input_name, InferenceTensor::F32(input.clone()))])
                    .unwrap()
            };

            let ort = OrtBackend::new().load(&path, None).unwrap();
            let tract = TractBackend::new().load(&path, None).unwrap();

            let ort_out = named(ort.as_ref());
            let tract_out = named(tract.as_ref());

            let ort_logits = ort_out[0].1.as_f32().unwrap().as_slice().unwrap();
            let tract_logits = tract_out[0].1.as_f32().unwrap().as_slice().unwrap();
            assert_eq!(ort_logits.len(), *out_len, "{suffix}: output length");
            assert_eq!(tract_logits.len(), *out_len, "{suffix}: output length");

            let max_abs_diff = ort_logits
                .iter()
                .zip(tract_logits)
                .map(|(a, b)| (a - b).abs())
                .fold(0.0f32, f32::max);
            assert!(
                max_abs_diff < 1e-3,
                "{suffix}: tract vs ORT max |Δ| = {max_abs_diff} exceeds 1e-3\n ort={ort_logits:?}\n tract={tract_logits:?}"
            );
        }

        if ran == 0 {
            assert!(
                !parity_required(),
                "XBERG_REQUIRE_TRACT_PARITY is set but no parity models were compared"
            );
            eprintln!("tract_matches_ort_on_cnn_classifiers: no models cached, nothing compared");
        }
    }

    /// `load_from_memory` must produce a session equivalent to `load` for the same
    /// artifact — covered on both engines (ORT `commit_from_memory`, tract
    /// `model_for_read`). Self-skips when the model is not cached.
    #[test]
    fn load_from_memory_matches_load_on_both_engines() {
        let suffix = "v2/classifiers/PP-LCNet_x1_0_doc_ori.onnx";
        let Some(path) = resolve_model(suffix) else {
            eprintln!("skip: {suffix} not in HF cache");
            return;
        };
        let bytes = std::fs::read(&path).unwrap();

        let shape = [1usize, 3, 224, 224];
        let count: usize = shape.iter().product();
        let data: Vec<f32> = (0..count).map(|i| ((i % 255) as f32) / 255.0 - 0.5).collect();
        let input = ndarray::ArrayD::from_shape_vec(shape.to_vec(), data).unwrap();

        let run = |session: &dyn InferenceSession| -> Vec<f32> {
            let input_name = session.input_names().first().cloned().unwrap();
            let out = session
                .run(vec![(input_name, InferenceTensor::F32(input.clone()))])
                .unwrap();
            out[0].1.as_f32().unwrap().as_slice().unwrap().to_vec()
        };

        for backend in [
            Box::new(OrtBackend::new()) as Box<dyn InferenceBackend>,
            Box::new(TractBackend::new()) as Box<dyn InferenceBackend>,
        ] {
            let from_path = run(backend.load(&path, None).unwrap().as_ref());
            let from_memory = run(backend.load_from_memory(&bytes, None).unwrap().as_ref());
            assert_eq!(from_path, from_memory, "load vs load_from_memory diverged");
        }
    }
}
