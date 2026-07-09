//! Vendored text embedding engine.
//!
//! Core inference pipeline for ONNX-based text embedding generation.
//! Key design: `embed()` takes `&self` instead of `&mut self`, enabling
//! concurrent inference from multiple threads without mutex contention.
//!
//! This is safe because `ort::Session::run()` takes `&mut self` purely as
//! an API constraint — its internal `run_inner()` takes `&self`, and the
//! ONNX Runtime C API (`OrtApi::Run`) is documented as thread-safe for
//! concurrent calls on the same session.
//!
//! See ATTRIBUTIONS.md for original source attribution.

use ndarray::{Array2, ArrayView, Dim, Dimension, IxDynImpl, s};
use ort::session::Session;
use ort::value::Value;
use tokenizers::Tokenizer;
/// Pooling strategy for extracting a single fixed-length vector from per-token embeddings.
#[cfg_attr(alef, alef(skip))]
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum Pooling {
    /// Use the `[CLS]` token embedding (first token).
    Cls,
    /// Mean of all token embeddings, weighted by attention mask.
    Mean,
    /// Use the last non-padding token embedding. Decoder-style embedding models
    /// (e.g. Qwen3-Embedding) place the pooled representation on the final
    /// content token rather than a prepended `[CLS]`.
    Last,
}
#[cfg_attr(alef, alef(skip))]
/// Text embedding model with thread-safe inference.
///
/// The `embed()` method takes `&self` instead of `&mut self`, allowing it to
/// be shared across threads via `Arc<EmbeddingEngine>` without mutex contention.
pub struct EmbeddingEngine {
    tokenizer: Tokenizer,
    session: Session,
    pooling: Pooling,
    need_token_type_ids: bool,
}

impl EmbeddingEngine {
    /// Create a new embedding engine from a pre-built session and tokenizer.
    pub(crate) fn new(tokenizer: Tokenizer, session: Session, pooling: Pooling) -> Self {
        let need_token_type_ids = session.inputs().iter().any(|input| input.name() == "token_type_ids");

        Self {
            tokenizer,
            session,
            pooling,
            need_token_type_ids,
        }
    }

    /// Generate embeddings for a batch of texts.
    ///
    /// This method is **thread-safe** — multiple threads can call `embed()`
    /// concurrently on the same `EmbeddingEngine` instance.
    ///
    /// # Safety note
    ///
    /// Uses an internal unsafe cast because `ort::Session::run()` takes
    /// `&mut self` despite performing no mutation (its `run_inner()` takes
    /// `&self`). The ONNX Runtime C API is documented as thread-safe for
    /// concurrent `Run()` calls on the same session.
    pub(crate) fn embed<S: AsRef<str>>(&self, texts: &[S], batch_size: usize) -> Result<Vec<Vec<f32>>, EmbedError> {
        if texts.is_empty() {
            return Ok(Vec::new());
        }

        // Defensive: callers from polyglot bindings may pass batch_size=0 when the
        // host-side `EmbeddingConfig` mirror omits the serde default. `chunks(0)`
        // would panic — fall back to the documented default rather than aborting
        // the entire interpreter.
        let batch_size = if batch_size == 0 { 32 } else { batch_size };

        let mut all_embeddings = Vec::with_capacity(texts.len());

        for batch in texts.chunks(batch_size) {
            let batch_embeddings = self.embed_batch(batch)?;
            all_embeddings.extend(batch_embeddings);
        }

        Ok(all_embeddings)
    }

    /// Embed a single batch of texts.
    fn embed_batch<S: AsRef<str>>(&self, batch: &[S]) -> Result<Vec<Vec<f32>>, EmbedError> {
        // Tokenize
        let inputs: Vec<&str> = batch.iter().map(|t| t.as_ref()).collect();
        let encodings = self
            .tokenizer
            .encode_batch(inputs, true)
            .map_err(|e| EmbedError::Tokenizer(e.to_string()))?;

        let encoding_length = encodings
            .first()
            .ok_or_else(|| EmbedError::Tokenizer("Empty encodings".to_string()))?
            .len();
        let batch_size = batch.len();
        let max_size = encoding_length * batch_size;

        // Build input tensors
        let mut ids_array = Vec::with_capacity(max_size);
        let mut mask_array = Vec::with_capacity(max_size);
        let mut type_ids_array = Vec::with_capacity(max_size);

        for encoding in &encodings {
            ids_array.extend(encoding.get_ids().iter().map(|&x| x as i64));
            mask_array.extend(encoding.get_attention_mask().iter().map(|&x| x as i64));
            type_ids_array.extend(encoding.get_type_ids().iter().map(|&x| x as i64));
        }

        let ids_tensor = ndarray::Array::from_shape_vec((batch_size, encoding_length), ids_array)
            .map_err(|e| EmbedError::Shape(e.to_string()))?;
        let type_ids_tensor = ndarray::Array::from_shape_vec((batch_size, encoding_length), type_ids_array)
            .map_err(|e| EmbedError::Shape(e.to_string()))?;

        let mask_nd = ndarray::Array::from_shape_vec((batch_size, encoding_length), mask_array)
            .map_err(|e| EmbedError::Shape(e.to_string()))?;
        // Clone mask only when mean/last pooling needs it for post-processing.
        let attention_mask_for_pooling = if matches!(self.pooling, Pooling::Mean | Pooling::Last) {
            Some(mask_nd.clone())
        } else {
            None
        };
        let mask_tensor = Value::from_array(mask_nd)?;

        let mut session_inputs = ort::inputs![
            "input_ids" => Value::from_array(ids_tensor)?,
            "attention_mask" => mask_tensor,
        ];

        if self.need_token_type_ids {
            session_inputs.push(("token_type_ids".into(), Value::from_array(type_ids_tensor)?.into()));
        }

        // Run inference — thread-safe despite &mut self signature on Session::run()
        //
        // SAFETY: ort::Session::run() takes &mut self but delegates to run_inner(&self)
        // with zero actual mutation. The ONNX Runtime C API (OrtApi::Run) is documented
        // as thread-safe for concurrent Run() calls on the same session.
        #[allow(unsafe_code)]
        let outputs = unsafe {
            let session_ptr = &self.session as *const Session as *mut Session;
            (*session_ptr).run(session_inputs)
        }
        .map_err(EmbedError::Ort)?;

        // Find the embedding output tensor
        let (_, output_value) = outputs.iter().next().ok_or(EmbedError::NoOutput)?;

        let tensor: ArrayView<f32, Dim<IxDynImpl>> = output_value.try_extract_array().map_err(EmbedError::Ort)?;

        // Pool (without normalization — caller controls normalization)
        let pooled = match self.pooling {
            Pooling::Cls => cls_pool(&tensor)?,
            Pooling::Mean => {
                let mask = attention_mask_for_pooling.ok_or_else(|| {
                    EmbedError::Shape("mean pooling requires the attention mask".to_string())
                })?;
                mean_pool(&tensor, mask)?
            }
            Pooling::Last => {
                let mask = attention_mask_for_pooling.ok_or_else(|| {
                    EmbedError::Shape("last-token pooling requires the attention mask".to_string())
                })?;
                last_pool(&tensor, mask)?
            }
        };

        let embeddings: Vec<Vec<f32>> = pooled
            .rows()
            .into_iter()
            .map(|row| row.as_slice().unwrap_or(&[]).to_vec())
            .collect();

        Ok(embeddings)
    }
}

// SAFETY: EmbeddingEngine is Send + Sync because:
// 1. Tokenizer is Send + Sync (confirmed in tokenizers crate)
// 2. Session: we only call run() which is internally thread-safe
// 3. All other fields are immutable after construction
#[allow(unsafe_code)]
unsafe impl Send for EmbeddingEngine {}
#[allow(unsafe_code)]
unsafe impl Sync for EmbeddingEngine {}

/// CLS pooling — extract the first token's embedding.
fn cls_pool(tensor: &ArrayView<f32, Dim<IxDynImpl>>) -> Result<Array2<f32>, EmbedError> {
    match tensor.dim().ndim() {
        2 => Ok(tensor.slice(s![.., ..]).to_owned()),
        3 => Ok(tensor.slice(s![.., 0, ..]).to_owned()),
        _ => Err(EmbedError::Shape(format!(
            "Expected 2D or 3D tensor, got {:?}",
            tensor.dim()
        ))),
    }
}

/// Mean pooling — average token embeddings weighted by attention mask.
fn mean_pool(tensor: &ArrayView<f32, Dim<IxDynImpl>>, attention_mask: Array2<i64>) -> Result<Array2<f32>, EmbedError> {
    if tensor.dim().ndim() == 2 {
        return Ok(tensor.slice(s![.., ..]).to_owned());
    }
    if tensor.dim().ndim() != 3 {
        return Err(EmbedError::Shape(format!(
            "Expected 2D or 3D tensor, got {:?}",
            tensor.dim()
        )));
    }

    let token_embeddings = tensor.slice(s![.., .., ..]);
    let mask_dim = attention_mask.dim();
    let mask_expanded = attention_mask
        .insert_axis(ndarray::Axis(2))
        .broadcast(token_embeddings.dim())
        .ok_or_else(|| {
            EmbedError::Shape(format!(
                "Cannot broadcast attention mask {:?} to {:?}",
                mask_dim,
                token_embeddings.dim()
            ))
        })?
        .mapv(|x| x as f32);

    let masked = &mask_expanded * &token_embeddings;
    let sum = masked.sum_axis(ndarray::Axis(1));
    let mask_sum = mask_expanded.sum_axis(ndarray::Axis(1));
    let mask_sum = mask_sum.mapv(|x| if x == 0.0 { 1.0 } else { x });

    Ok(&sum / &mask_sum)
}

/// Last-token pooling — extract the final non-padding token's embedding per row.
///
/// Decoder-style embedding models (Qwen3-Embedding) carry the pooled
/// representation on the last content token. The last real token is located via
/// the attention mask (scanning from the end for the last `1`), so this is
/// robust to either left- or right-padding. A row whose mask is entirely zero
/// falls back to the final position.
fn last_pool(tensor: &ArrayView<f32, Dim<IxDynImpl>>, attention_mask: Array2<i64>) -> Result<Array2<f32>, EmbedError> {
    if tensor.dim().ndim() == 2 {
        return Ok(tensor.slice(s![.., ..]).to_owned());
    }
    if tensor.dim().ndim() != 3 {
        return Err(EmbedError::Shape(format!(
            "Expected 2D or 3D tensor, got {:?}",
            tensor.dim()
        )));
    }

    let tensor3 = tensor.slice(s![.., .., ..]);
    let (batch, seq_len, hidden) = tensor3.dim();
    let mut pooled = Array2::<f32>::zeros((batch, hidden));
    for b in 0..batch {
        let last_index = (0..seq_len)
            .rev()
            .find(|&t| attention_mask[[b, t]] != 0)
            .unwrap_or(seq_len - 1);
        pooled.row_mut(b).assign(&tensor3.slice(s![b, last_index, ..]));
    }
    Ok(pooled)
}

/// L2-normalize a vector.
#[cfg(test)]
pub(crate) fn normalize(v: &[f32]) -> Vec<f32> {
    let norm = v.iter().map(|x| x * x).sum::<f32>().sqrt();
    if norm > f32::EPSILON {
        let inv = 1.0 / norm;
        v.iter().map(|&x| x * inv).collect()
    } else {
        v.to_vec()
    }
}
/// Errors that can occur during text embedding inference.
#[cfg_attr(alef, alef(skip))]
#[derive(Debug)]
pub enum EmbedError {
    /// Tokenization failed with the given message.
    Tokenizer(String),
    /// ONNX Runtime returned an error during inference.
    Ort(ort::Error),
    /// The model output tensor had an unexpected shape.
    Shape(String),
    /// The model produced no output tensors.
    NoOutput,
}

impl From<ort::Error> for EmbedError {
    fn from(e: ort::Error) -> Self {
        Self::Ort(e)
    }
}

impl std::fmt::Display for EmbedError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::Tokenizer(e) => write!(f, "Tokenizer error: {e}"),
            Self::Ort(e) => write!(f, "ONNX Runtime error: {e}"),
            Self::Shape(e) => write!(f, "Tensor shape error: {e}"),
            Self::NoOutput => write!(f, "Model produced no output tensors"),
        }
    }
}

impl std::error::Error for EmbedError {}

#[cfg(test)]
mod tests {
    use super::*;

    /// Test normalization of a known vector produces unit vector (L2 norm ≈ 1.0).
    #[test]
    fn test_normalize_unit_vector() {
        let v = vec![3.0, 4.0]; // 3-4-5 triangle
        let normalized = normalize(&v);

        assert_eq!(normalized.len(), 2);
        assert!(
            (normalized[0] - 0.6).abs() < 1e-6,
            "Expected ~0.6, got {}",
            normalized[0]
        );
        assert!(
            (normalized[1] - 0.8).abs() < 1e-6,
            "Expected ~0.8, got {}",
            normalized[1]
        );

        // Verify L2 norm is approximately 1.0
        let norm: f32 = normalized.iter().map(|x| x * x).sum::<f32>().sqrt();
        assert!((norm - 1.0).abs() < 1e-6, "L2 norm should be ~1.0, got {}", norm);
    }

    /// Test normalization of all-zeros vector produces zeros without NaN/panic.
    #[test]
    fn test_normalize_zero_vector() {
        let v = vec![0.0, 0.0, 0.0];
        let normalized = normalize(&v);

        assert_eq!(normalized.len(), 3);
        assert!(!normalized.iter().any(|x| x.is_nan()), "No NaN values expected");
        assert!(
            !normalized.iter().any(|x| x.is_infinite()),
            "No infinite values expected"
        );
        for &val in &normalized {
            assert_eq!(val, 0.0, "Zero vector should remain zero");
        }
    }

    /// Test normalization of single-element vector.
    #[test]
    fn test_normalize_single_element() {
        let v = vec![5.0];
        let normalized = normalize(&v);

        assert_eq!(normalized.len(), 1);
        assert!(
            (normalized[0] - 1.0).abs() < 1e-6,
            "Expected 1.0, got {}",
            normalized[0]
        );
    }

    /// Test that all EmbedError variants have Display impl without panicking.
    #[test]
    fn test_embed_error_display() {
        // Test Tokenizer variant
        let tokenizer_err = EmbedError::Tokenizer("test error".to_string());
        let display = format!("{}", tokenizer_err);
        assert!(display.contains("Tokenizer error"), "Tokenizer display: {}", display);

        // Test Shape variant
        let shape_err = EmbedError::Shape("invalid shape".to_string());
        let display = format!("{}", shape_err);
        assert!(display.contains("Tensor shape error"), "Shape display: {}", display);

        // Test NoOutput variant
        let no_output_err = EmbedError::NoOutput;
        let display = format!("{}", no_output_err);
        assert!(display.contains("no output"), "NoOutput display: {}", display);
    }

    /// Test that Pooling variants are distinct and comparable.
    #[test]
    fn test_pooling_variants() {
        let cls = Pooling::Cls;
        let mean = Pooling::Mean;

        // Different variants should not be equal
        assert_ne!(cls, mean, "Pooling::Cls and Pooling::Mean should be different");

        // Same variants should be equal
        assert_eq!(cls, Pooling::Cls);
        assert_eq!(mean, Pooling::Mean);

        // Pooling should be cloneable
        let cls_clone = cls.clone();
        assert_eq!(cls, cls_clone);

        // Pooling should be debuggable
        let debug_output = format!("{:?}", cls);
        assert!(debug_output.contains("Cls"), "Debug output: {}", debug_output);
    }

    /// Test normalization preserves input length.
    #[test]
    fn test_normalize_preserves_length() {
        let test_cases = vec![vec![1.0, 2.0, 3.0], vec![-1.0, -2.0], vec![0.1, 0.2, 0.3, 0.4, 0.5]];

        for v in test_cases {
            let original_len = v.len();
            let normalized = normalize(&v);
            assert_eq!(
                normalized.len(),
                original_len,
                "Normalization should preserve vector length"
            );
        }
    }

    /// Test normalization handles negative values.
    #[test]
    fn test_normalize_negative_values() {
        let v = vec![-3.0, -4.0]; // Same magnitude as [3.0, 4.0]
        let normalized = normalize(&v);

        assert!((normalized[0] - (-0.6)).abs() < 1e-6, "Expected ~-0.6");
        assert!((normalized[1] - (-0.8)).abs() < 1e-6, "Expected ~-0.8");

        // Verify L2 norm is still 1.0
        let norm: f32 = normalized.iter().map(|x| x * x).sum::<f32>().sqrt();
        assert!((norm - 1.0).abs() < 1e-6);
    }

    /// Test normalization with very small non-zero values (below epsilon threshold).
    #[test]
    fn test_normalize_very_small_values() {
        let v = vec![f32::EPSILON / 2.0, f32::EPSILON / 2.0];
        let normalized = normalize(&v);

        // Values below epsilon threshold should be returned as-is
        assert_eq!(normalized, v, "Very small vectors (< epsilon) returned unchanged");
    }

    /// Test EmbedError implements Error trait.
    #[test]
    fn test_embed_error_is_error_type() {
        let err = EmbedError::Shape("test".to_string());
        let _: &dyn std::error::Error = &err;
        // If this compiles, the trait is properly implemented
    }

    /// Test Pooling enum Clone and Debug traits.
    #[test]
    fn test_pooling_traits() {
        let cls = Pooling::Cls;
        let mean = Pooling::Mean;

        // Test Clone
        let cls_clone = cls.clone();
        let mean_clone = mean.clone();
        assert_eq!(cls, cls_clone);
        assert_eq!(mean, mean_clone);

        // Test Debug produces valid output
        let cls_debug = format!("{:?}", cls);
        let mean_debug = format!("{:?}", mean);
        assert!(!cls_debug.is_empty());
        assert!(!mean_debug.is_empty());

        // Test PartialEq and Eq
        assert_eq!(cls, cls);
        assert_eq!(mean, mean);
        assert_ne!(cls, mean);
    }

    /// Test normalization with large magnitude values.
    #[test]
    fn test_normalize_large_values() {
        let v = vec![1e6, 1e6];
        let normalized = normalize(&v);

        // Should normalize without overflow
        assert!(!normalized.iter().any(|x| x.is_infinite()), "No overflow");
        let norm: f32 = normalized.iter().map(|x| x * x).sum::<f32>().sqrt();
        assert!((norm - 1.0).abs() < 1e-5, "L2 norm should be ~1.0");
    }

    /// Test normalization with mixed positive and negative values.
    #[test]
    fn test_normalize_mixed_signs() {
        let v = vec![1.0, -1.0, 0.0];
        let normalized = normalize(&v);

        assert_eq!(normalized.len(), 3);
        let norm: f32 = normalized.iter().map(|x| x * x).sum::<f32>().sqrt();
        assert!((norm - 1.0).abs() < 1e-6, "L2 norm should be ~1.0");
    }
}
