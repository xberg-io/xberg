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
        DatumType::TDim => {
            let cast = tensor
                .cast_to::<i64>()
                .map_err(|e| InferenceError::Tensor(e.to_string()))?;
            InferenceTensor::I64(extract(cast.as_ref())?)
        }
        other => {
            return Err(InferenceError::Tensor(format!(
                "unsupported output datum type {other:?}"
            )));
        }
    };
    Ok(tensor)
}

/// Model-free coverage of the tract boundary conversions. Needs neither ONNX
/// Runtime nor a downloaded model, so it runs offline wherever the tract engine
/// is compiled — unlike the model-gated parity suite below, it never self-skips.
#[cfg(test)]
mod conversion_tests {
    use super::*;
    use ndarray::ArrayD;

    #[test]
    fn tensor_conversions_roundtrip_every_dtype() {
        let cases = [
            InferenceTensor::F32(ArrayD::from_shape_vec(vec![2, 2], vec![1.0f32, -2.0, 3.5, 4.0]).unwrap()),
            InferenceTensor::I64(ArrayD::from_shape_vec(vec![3], vec![10i64, -20, 30]).unwrap()),
            InferenceTensor::I32(ArrayD::from_shape_vec(vec![2, 1], vec![7i32, -8]).unwrap()),
            InferenceTensor::U8(ArrayD::from_shape_vec(vec![4], vec![0u8, 1, 254, 255]).unwrap()),
            InferenceTensor::Bool(ArrayD::from_shape_vec(vec![2], vec![true, false]).unwrap()),
        ];
        for original in cases {
            let tract = tensor_to_tract(&original).unwrap();
            let back = tract_to_tensor(&tract).unwrap();
            assert_eq!(back, original, "tract dtype round-trip diverged for {original:?}");
        }
    }

    #[test]
    fn non_standard_layout_input_is_copied_and_preserves_values() {
        let original = InferenceTensor::F32(
            ArrayD::from_shape_vec(vec![2, 3], vec![1.0f32, 2.0, 3.0, 4.0, 5.0, 6.0])
                .unwrap()
                .reversed_axes(),
        );
        let back = tract_to_tensor(&tensor_to_tract(&original).unwrap()).unwrap();
        assert_eq!(back, original);
    }
}

/// HF repository holding the PP-LCNet classifiers compared here. Referenced by the
/// orientation byte-entry test and the ORT-parity `mod tests`, so it is only live
/// when one of those compiles.
#[cfg(all(test, any(auto_rotate, inference_ort)))]
const PARITY_REPO: &str = "xberg-io/paddleocr-onnx-models";

/// HF repository holding the layout detectors (RT-DETR, PP-DocLayout-V3). Referenced by
/// the RT-DETR byte-entry test and the ORT-parity `mod tests`, so it is only live when
/// one of those compiles.
#[cfg(all(test, any(layout_detection, inference_ort)))]
const LAYOUT_REPO: &str = "xberg-io/layout-models";

/// Whether model-backed coverage is mandatory. CI sets `XBERG_REQUIRE_TRACT_PARITY=1`
/// so a missing model is a hard failure, not a silent skip — otherwise these tests
/// could pass by comparing (or exercising) nothing.
#[cfg(test)]
fn parity_required() -> bool {
    std::env::var_os("XBERG_REQUIRE_TRACT_PARITY").is_some()
}

/// Locate a cached ONNX model by the tail of its path in the HuggingFace hub
/// cache, returning `None` when it is not present.
#[cfg(test)]
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

/// Resolve a model to a local path. Uses the HF cache when present; otherwise,
/// when coverage is required (CI), downloads it via the production path and fails
/// loudly on error. Returns `None` only for a local run where the model is absent
/// and coverage is not required — the self-skip case that keeps the suite runnable
/// offline without ever passing vacuously in CI.
#[cfg(test)]
fn resolve_model(repo: &str, filename: &str) -> Option<std::path::PathBuf> {
    if let Some(path) = cached_model(filename) {
        return Some(path);
    }
    if parity_required() {
        return Some(download_parity_model(repo, filename));
    }
    None
}

#[cfg(all(
    test,
    any(
        feature = "paddle-ocr",
        feature = "layout-detection",
        auto_rotate,
        feature = "ner-onnx",
        feature = "candle-paddleocr-vl"
    )
))]
fn download_parity_model(repo: &str, filename: &str) -> std::path::PathBuf {
    crate::model_download::hf_download(repo, filename)
        .expect("XBERG_REQUIRE_TRACT_PARITY is set but the parity model download failed")
}

#[cfg(all(
    test,
    not(any(
        feature = "paddle-ocr",
        feature = "layout-detection",
        auto_rotate,
        feature = "ner-onnx",
        feature = "candle-paddleocr-vl"
    ))
))]
fn download_parity_model(_repo: &str, _filename: &str) -> std::path::PathBuf {
    panic!(
        "XBERG_REQUIRE_TRACT_PARITY is set but no model-download feature is enabled; \
         build the parity tests with `--features full,tract`"
    );
}

/// End-to-end smoke tests for the byte-based model entry points the WASM bridge
/// calls. Unlike the cross-engine parity suite below (which needs `OrtBackend`, so
/// is `inference_ort`-only), these run on whichever backend the build links — ORT
/// on native, pure-Rust tract on no-ORT targets — so the exact code path
/// `xberg-wasm` depends on is exercised on the no-ORT engine too. Each self-skips
/// when its model is absent, guarded by `parity_required` to stay non-vacuous.
#[cfg(test)]
mod byte_entry_tests {
    use super::*;

    /// Encode a deterministic gradient RGB image as PNG bytes — a valid input for
    /// the byte-based entry points' `image::load_from_memory` decode step.
    #[cfg(any(layout_detection, auto_rotate))]
    fn png_bytes(width: u32, height: u32) -> Vec<u8> {
        let rgb = image::RgbImage::from_fn(width, height, |x, y| {
            image::Rgb([(x % 256) as u8, (y % 256) as u8, 128])
        });
        let mut buf: Vec<u8> = Vec::new();
        image::DynamicImage::ImageRgb8(rgb)
            .write_to(&mut std::io::Cursor::new(&mut buf), image::ImageFormat::Png)
            .expect("PNG encode of an in-memory RGB image cannot fail");
        buf
    }

    /// Exercises the WASM bridge's byte-based layout path end to end:
    /// [`crate::layout::LayoutEngine::from_rtdetr_bytes`] (model bytes, no file path)
    /// then `detect_image_bytes` (encoded image → decode → detect). These are the
    /// exact methods `xberg-wasm`'s `detectLayout` calls and their only production
    /// callers — the seam parity tests stop below this preprocess/postprocess layer.
    #[cfg(layout_detection)]
    #[test]
    fn rtdetr_byte_entry_points_run_end_to_end() {
        let suffix = "rtdetr/model.onnx";
        let Some(path) = resolve_model(LAYOUT_REPO, suffix) else {
            eprintln!("skip: {suffix} not in HF cache");
            assert!(
                !parity_required(),
                "XBERG_REQUIRE_TRACT_PARITY is set but the RT-DETR byte path was not exercised"
            );
            return;
        };
        let model_bytes = std::fs::read(&path).unwrap();
        let mut engine = crate::layout::LayoutEngine::from_rtdetr_bytes(&model_bytes, None)
            .expect("from_rtdetr_bytes should load RT-DETR from memory");
        let result = engine
            .detect_image_bytes(&png_bytes(640, 640))
            .expect("detect_image_bytes should decode the image and run detection");
        assert!(
            result.page_width > 0 && result.page_height > 0,
            "page dimensions should be populated: {}x{}",
            result.page_width,
            result.page_height
        );
    }

    /// Exercises the WASM bridge's byte-based orientation path end to end:
    /// [`crate::doc_orientation::DocOrientationDetector::from_bytes`] then
    /// `detect_image_bytes` — the exact methods `xberg-wasm`'s `detectOrientation`
    /// calls.
    #[cfg(auto_rotate)]
    #[test]
    fn doc_orientation_byte_entry_points_run_end_to_end() {
        let suffix = "v2/classifiers/PP-LCNet_x1_0_doc_ori.onnx";
        let Some(path) = resolve_model(PARITY_REPO, suffix) else {
            eprintln!("skip: {suffix} not in HF cache");
            assert!(
                !parity_required(),
                "XBERG_REQUIRE_TRACT_PARITY is set but the orientation byte path was not exercised"
            );
            return;
        };
        let model_bytes = std::fs::read(&path).unwrap();
        let detector = crate::doc_orientation::DocOrientationDetector::from_bytes(model_bytes, None);
        let result = detector
            .detect_image_bytes(&png_bytes(224, 224))
            .expect("detect_image_bytes should decode the image and run orientation detection");
        assert!(
            matches!(result.degrees, 0 | 90 | 180 | 270),
            "orientation must be a valid rotation, got {}",
            result.degrees
        );
        assert!(
            (0.0..=1.0).contains(&result.confidence),
            "confidence must be in [0, 1], got {}",
            result.confidence
        );
    }
}

#[cfg(all(test, inference_ort))]
mod tests {
    use super::*;
    use crate::inference::ort_backend::OrtBackend;

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
            let Some(path) = resolve_model(PARITY_REPO, suffix) else {
                eprintln!("skip: {suffix} not in HF cache");
                continue;
            };
            ran += 1;

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
        let Some(path) = resolve_model(PARITY_REPO, suffix) else {
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

    /// RT-DETR (NMS-free layout detector) migrated onto the seam in Phase 4. Two
    /// inputs (image + `orig_target_sizes`), three outputs (labels/boxes/scores).
    /// tract runs it as-is — no input-fact pinning — and must track ORT within
    /// tolerance. Box coordinates live in the 0..640 pixel range while scores are
    /// in 0..1, so f32 outputs are compared with a magnitude-normalized relative
    /// tolerance rather than the single absolute logit tolerance used for the CNN
    /// classifiers.
    ///
    /// PP-DocLayout-V3 is also seam-migrated (ORT path is byte-identical), but it
    /// stays ORT-only under tract permanently: pinning all three input facts
    /// (`im_shape`/`image`/`scale_factor`) clears the earlier symbolic-shape wall
    /// noted in Phase 0, but tract 0.23.4's `LayerNormalization` op translator then
    /// fails on the DETR decoder's norm layer with a genuine shape-inference bug
    /// (`Output mismatch after rewiring expansion for output #0: expected
    /// 1,300,1,..,F32 got 1,300,256,F32`, node `LayerNormalization.3`) — reproduced
    /// even at the bare `into_typed()` translation stage, before any
    /// declutter/optimize pass runs. Not a mechanical fix; see the model-coverage
    /// matrix in `docs-site/src/content/docs/concepts/tract-inference.md`
    /// (`pp_doclayout_v3` = ORT-only) — so it is not compared here.
    #[test]
    fn tract_matches_ort_on_rtdetr_layout() {
        let suffix = "rtdetr/model.onnx";
        let Some(path) = resolve_model(LAYOUT_REPO, suffix) else {
            eprintln!("skip: {suffix} not in HF cache");
            assert!(
                !parity_required(),
                "XBERG_REQUIRE_TRACT_PARITY is set but RT-DETR was not compared"
            );
            return;
        };

        const SIZE: usize = 640;
        let img_count = 3 * SIZE * SIZE;
        let img_data: Vec<f32> = (0..img_count).map(|i| ((i % 255) as f32) / 255.0).collect();
        let image = ndarray::ArrayD::from_shape_vec(vec![1, 3, SIZE, SIZE], img_data).unwrap();
        let sizes = ndarray::ArrayD::from_shape_vec(vec![1, 2], vec![SIZE as i64, SIZE as i64]).unwrap();

        let run = |session: &dyn InferenceSession| {
            let names = session.input_names().to_vec();
            session
                .run(vec![
                    (names[0].clone(), InferenceTensor::F32(image.clone())),
                    (names[1].clone(), InferenceTensor::I64(sizes.clone())),
                ])
                .unwrap()
        };

        let ort = OrtBackend::new().load(&path, None).unwrap();
        let tract = TractBackend::new().load(&path, None).unwrap();
        let ort_out = run(ort.as_ref());
        let tract_out = run(tract.as_ref());

        assert_eq!(ort_out.len(), tract_out.len(), "RT-DETR output count");
        let mut compared_f32 = 0;
        for (index, ((_, oval), (_, tval))) in ort_out.iter().zip(&tract_out).enumerate() {
            match (oval, tval) {
                (InferenceTensor::F32(a), InferenceTensor::F32(b)) => {
                    assert_eq!(a.shape(), b.shape(), "output {index}: RT-DETR f32 shape");
                    let max_rel_diff = a
                        .iter()
                        .zip(b)
                        .map(|(x, y)| (x - y).abs() / x.abs().max(1.0))
                        .fold(0.0f32, f32::max);
                    assert!(
                        max_rel_diff < 5e-3,
                        "output {index}: RT-DETR tract vs ORT max relative |Δ| = {max_rel_diff} exceeds 5e-3"
                    );
                    compared_f32 += 1;
                }
                (InferenceTensor::I64(a), InferenceTensor::I64(b)) => {
                    assert_eq!(a, b, "output {index}: RT-DETR class labels diverged between engines");
                }
                _ => panic!("output {index}: RT-DETR output dtype mismatch between engines"),
            }
        }
        assert!(compared_f32 >= 2, "expected boxes + scores f32 outputs to be compared");
    }

    /// Number of warm-up `run()` calls before recording latency samples.
    const LATENCY_WARMUP: usize = 2;

    /// Number of recorded `run()` samples per engine; the reported latency is the
    /// minimum (best-of-N), which isolates steady-state execution cost from
    /// scheduler/allocator noise.
    const LATENCY_RUNS: usize = 8;

    /// Deterministic pseudo-image input shared by both engines, identical to the
    /// one used by [`tract_matches_ort_on_cnn_classifiers`].
    fn pseudo_input(shape: &[usize]) -> ndarray::ArrayD<f32> {
        let count: usize = shape.iter().product();
        let data: Vec<f32> = (0..count).map(|i| ((i % 255) as f32) / 255.0 - 0.5).collect();
        ndarray::ArrayD::from_shape_vec(shape.to_vec(), data).unwrap()
    }

    /// Load a model once (timed) and run it `LATENCY_WARMUP + LATENCY_RUNS`
    /// times, returning `(load_ms, run_ms_samples)`. `make_inputs` builds fresh
    /// named inputs from the session's declared input order on every call, since
    /// RT-DETR needs two distinct inputs in position order.
    ///
    /// Each backend runs exactly as xberg ships it: `OrtBackend` with its default
    /// intra-op thread pool (up to `min(8, cores)`), `TractBackend` single-threaded
    /// (the seam configures no tract thread pool). The reported tract/ORT ratio is
    /// thus an as-shipped, wall-clock comparison — an upper bound on the per-core
    /// kernel gap, not a thread-matched one. See the "Latency" note in the
    /// Pure-Rust Inference concept doc.
    fn time_engine(
        backend: &dyn InferenceBackend,
        path: &std::path::Path,
        make_inputs: impl Fn(&[String]) -> Vec<(String, InferenceTensor)>,
    ) -> (f64, Vec<f64>) {
        let load_start = std::time::Instant::now();
        let session = backend.load(path, None).unwrap();
        let load_ms = load_start.elapsed().as_secs_f64() * 1000.0;

        let names = session.input_names().to_vec();
        for _ in 0..LATENCY_WARMUP {
            session.run(make_inputs(&names)).unwrap();
        }
        let mut samples = Vec::with_capacity(LATENCY_RUNS);
        for _ in 0..LATENCY_RUNS {
            let inputs = make_inputs(&names);
            let start = std::time::Instant::now();
            session.run(inputs).unwrap();
            samples.push(start.elapsed().as_secs_f64() * 1000.0);
        }
        (load_ms, samples)
    }

    /// tract-vs-ORT latency report for the three models on the seam (issue
    /// #1275, S5-BENCH). Not a correctness assertion — prints a markdown table
    /// consumed by `tools/benchmark-harness/README.md`. `#[ignore]`d so ordinary
    /// `cargo test` runs skip it; invoke explicitly with `--ignored --nocapture`.
    /// Self-skips per model when the weight is not in the local HF cache,
    /// matching the parity tests above — never fabricates a number for a model
    /// it could not load.
    #[test]
    #[ignore = "prints a timing report; run explicitly with --ignored --nocapture"]
    fn tract_vs_ort_latency_report() {
        struct Case {
            label: &'static str,
            repo: &'static str,
            suffix: &'static str,
            make_inputs: fn(&[String]) -> Vec<(String, InferenceTensor)>,
        }

        fn cnn_inputs(names: &[String]) -> Vec<(String, InferenceTensor)> {
            vec![(names[0].clone(), InferenceTensor::F32(pseudo_input(&[1, 3, 224, 224])))]
        }

        fn rtdetr_inputs(names: &[String]) -> Vec<(String, InferenceTensor)> {
            let image = InferenceTensor::F32(pseudo_input(&[1, 3, 640, 640]));
            let sizes = InferenceTensor::I64(ndarray::ArrayD::from_shape_vec(vec![1, 2], vec![640i64, 640]).unwrap());
            vec![(names[0].clone(), image), (names[1].clone(), sizes)]
        }

        let cases = [
            Case {
                label: "RT-DETR layout detector",
                repo: LAYOUT_REPO,
                suffix: "rtdetr/model.onnx",
                make_inputs: rtdetr_inputs,
            },
            Case {
                label: "PP-LCNet table classifier",
                repo: PARITY_REPO,
                suffix: "v2/classifiers/PP-LCNet_x1_0_table_cls.onnx",
                make_inputs: cnn_inputs,
            },
            Case {
                label: "PP-LCNet doc-orientation",
                repo: PARITY_REPO,
                suffix: "v2/classifiers/PP-LCNet_x1_0_doc_ori.onnx",
                make_inputs: cnn_inputs,
            },
        ];

        println!(
            "\n| model | tract load (ms) | ORT load (ms) | tract run (ms, best-of-{LATENCY_RUNS}) \
             | ORT run (ms, best-of-{LATENCY_RUNS}) | tract/ORT run ratio |"
        );
        println!("|---|---|---|---|---|---|");

        let mut ran = 0;
        for case in &cases {
            let Some(path) = resolve_model(case.repo, case.suffix) else {
                eprintln!("skip: {} ({}) not in HF cache", case.label, case.suffix);
                continue;
            };
            ran += 1;

            let (ort_load_ms, ort_runs) = time_engine(&OrtBackend::new(), &path, case.make_inputs);
            let (tract_load_ms, tract_runs) = time_engine(&TractBackend::new(), &path, case.make_inputs);

            let ort_best = ort_runs.iter().copied().fold(f64::INFINITY, f64::min);
            let tract_best = tract_runs.iter().copied().fold(f64::INFINITY, f64::min);
            let ratio = tract_best / ort_best;

            println!(
                "| {} | {tract_load_ms:.2} | {ort_load_ms:.2} | {tract_best:.3} | {ort_best:.3} | {ratio:.2}x |",
                case.label
            );
        }

        if ran == 0 {
            assert!(
                !parity_required(),
                "XBERG_REQUIRE_TRACT_PARITY is set but no latency models were compared"
            );
            eprintln!("tract_vs_ort_latency_report: no models cached, nothing measured");
        }
    }
}
