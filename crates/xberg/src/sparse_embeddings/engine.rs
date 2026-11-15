//! Sparse (SPLADE) embedding inference engine.
//!
//! Runs a `BertForMaskedLM` ONNX model and turns its `[batch, seq, vocab]` MLM
//! logits into sparse vocabulary vectors via SPLADE pooling:
//! `log(1 + relu(logits))`, attention-masked, max-pooled over the sequence,
//! L2-normalized, then thresholded to keep only the non-zero terms.
//!
//! Like [`crate::embeddings::engine::EmbeddingEngine`], `embed()` takes `&self`
//! so a single engine can serve concurrent callers via `Arc`; the session sits
//! behind a mutex because `ort::Session::run` takes `&mut self`.

use ndarray::{ArrayView, Axis, Dim, IxDynImpl};
use ort::session::Session;
use ort::value::Value;
use parking_lot::Mutex;
use tokenizers::Tokenizer;

use super::SparseEmbedding;

/// Errors raised by the sparse-embedding engine.
///
/// Rust-only: the `Ort` variant wraps `ort::Error`, which has no faithful
/// binding representation. Public callers receive `crate::XbergError` instead.
#[cfg_attr(alef, alef(skip))]
#[derive(Debug)]
pub enum SparseEmbedError {
    /// Tokenization failed with the given message.
    Tokenizer(String),
    /// ONNX Runtime returned an error during inference.
    Ort(ort::Error),
    /// The model output tensor had an unexpected shape.
    Shape(String),
    /// The model produced no output tensors.
    NoOutput,
}

impl From<ort::Error> for SparseEmbedError {
    fn from(e: ort::Error) -> Self {
        SparseEmbedError::Ort(e)
    }
}

/// SPLADE sparse-embedding model with thread-safe inference.
///
/// Rust-only: an opaque ORT-backed handle with no faithful binding
/// representation (mirrors `reranking::engine::RerankerEngine`). Bindings drive
/// inference through the module-level functions, not this type.
#[cfg_attr(alef, alef(skip))]
pub struct SparseEmbeddingEngine {
    tokenizer: Tokenizer,
    session: Mutex<Session>,
    need_token_type_ids: bool,
}

impl SparseEmbeddingEngine {
    /// Create a new engine from a pre-built session and tokenizer.
    pub(crate) fn new(tokenizer: Tokenizer, session: Session) -> Self {
        let need_token_type_ids = session.inputs().iter().any(|input| input.name() == "token_type_ids");
        Self {
            tokenizer,
            session: Mutex::new(session),
            need_token_type_ids,
        }
    }

    /// Generate sparse embeddings for a batch of texts.
    ///
    /// Thread-safe: multiple threads may call `embed()` concurrently on the same
    /// engine instance.
    pub(crate) fn embed<S: AsRef<str>>(
        &self,
        texts: &[S],
        batch_size: usize,
    ) -> Result<Vec<SparseEmbedding>, SparseEmbedError> {
        if texts.is_empty() {
            return Ok(Vec::new());
        }
        let batch_size = if batch_size == 0 { 16 } else { batch_size };

        let mut all = Vec::with_capacity(texts.len());
        for batch in texts.chunks(batch_size) {
            all.extend(self.embed_batch(batch)?);
        }
        Ok(all)
    }

    fn embed_batch<S: AsRef<str>>(&self, batch: &[S]) -> Result<Vec<SparseEmbedding>, SparseEmbedError> {
        let inputs: Vec<&str> = batch.iter().map(|t| t.as_ref()).collect();
        let encodings = self
            .tokenizer
            .encode_batch(inputs, true)
            .map_err(|e| SparseEmbedError::Tokenizer(e.to_string()))?;

        let encoding_length = encodings
            .first()
            .ok_or_else(|| SparseEmbedError::Tokenizer("Empty encodings".to_string()))?
            .len();
        let batch_size = batch.len();
        let max_size = encoding_length * batch_size;

        let mut ids_array = Vec::with_capacity(max_size);
        let mut mask_array = Vec::with_capacity(max_size);
        let mut type_ids_array = if self.need_token_type_ids {
            Vec::with_capacity(max_size)
        } else {
            Vec::new()
        };
        for encoding in &encodings {
            ids_array.extend(encoding.get_ids().iter().map(|&x| x as i64));
            mask_array.extend(encoding.get_attention_mask().iter().map(|&x| x as i64));
            if self.need_token_type_ids {
                type_ids_array.extend(encoding.get_type_ids().iter().map(|&x| x as i64));
            }
        }

        let ids_tensor = ndarray::Array::from_shape_vec((batch_size, encoding_length), ids_array)
            .map_err(|e| SparseEmbedError::Shape(e.to_string()))?;
        let mask_nd = ndarray::Array::from_shape_vec((batch_size, encoding_length), mask_array)
            .map_err(|e| SparseEmbedError::Shape(e.to_string()))?;

        let mut session_inputs = ort::inputs![
            "input_ids" => Value::from_array(ids_tensor)?,
            "attention_mask" => Value::from_array(mask_nd.clone())?,
        ];
        if self.need_token_type_ids {
            let type_ids_tensor = ndarray::Array::from_shape_vec((batch_size, encoding_length), type_ids_array)
                .map_err(|e| SparseEmbedError::Shape(e.to_string()))?;
            session_inputs.push(("token_type_ids".into(), Value::from_array(type_ids_tensor)?.into()));
        }

        let output_value = {
            let mut session = self.session.lock();
            let outputs = session.run(session_inputs).map_err(SparseEmbedError::Ort)?;
            outputs.into_iter().next().map(|(_, value)| value)
        }
        .ok_or(SparseEmbedError::NoOutput)?;
        let logits: ArrayView<f32, Dim<IxDynImpl>> = output_value.try_extract_array().map_err(SparseEmbedError::Ort)?;
        let logits = logits
            .into_dimensionality::<ndarray::Ix3>()
            .map_err(|e| SparseEmbedError::Shape(format!("expected [batch, seq, vocab] logits: {e}")))?;

        splade_pool(&logits, &mask_nd)
    }
}

/// SPLADE pooling: `log(1 + relu(x))` → attention-masked → max-pool over the
/// sequence → L2-normalize → keep strictly-positive terms as a sparse vector.
///
/// `logits` is `[batch, seq, vocab]`; `mask` is `[batch, seq]` (1 = real token,
/// 0 = padding). Returns one [`SparseEmbedding`] per batch row, with ascending
/// `indices`.
///
/// # Errors
///
/// [`SparseEmbedError::Shape`] if the attention mask cannot broadcast against the
/// logits — pooling over unmasked padding would silently corrupt the vector, so
/// this fails loudly rather than degrading.
fn splade_pool(
    logits: &ndarray::ArrayView3<f32>,
    mask: &ndarray::Array2<i64>,
) -> Result<Vec<SparseEmbedding>, SparseEmbedError> {
    let (batch, seq, vocab) = logits.dim();
    if mask.dim() != (batch, seq) {
        return Err(SparseEmbedError::Shape(format!(
            "attention mask {:?} cannot broadcast to logits {:?}",
            mask.dim(),
            logits.dim()
        )));
    }

    let mut scores = ndarray::Array2::<f32>::from_elem((batch, vocab), f32::NEG_INFINITY);
    for b in 0..batch {
        let logits_b = logits.index_axis(Axis(0), b);
        let mask_b = mask.index_axis(Axis(0), b);
        let mut scores_b = scores.index_axis_mut(Axis(0), b);
        for v in 0..vocab {
            let mut max_val = f32::NEG_INFINITY;
            for s in 0..seq {
                let masked = if mask_b[s] != 0 {
                    let x = logits_b[[s, v]];
                    (1.0_f32 + x.max(0.0)).ln()
                } else {
                    0.0_f32
                };
                if masked > max_val {
                    max_val = masked;
                }
            }
            scores_b[v] = max_val;
        }
    }

    Ok(scores
        .outer_iter()
        .map(|row| {
            let norm = row.iter().map(|&v| v * v).sum::<f32>().sqrt();
            let mut indices = Vec::new();
            let mut values = Vec::new();
            if norm > 0.0 {
                for (i, &v) in row.iter().enumerate() {
                    if v > 0.0 {
                        indices.push(i as u32);
                        values.push(v / norm);
                    }
                }
            }
            SparseEmbedding { indices, values }
        })
        .collect())
}

#[cfg(test)]
mod tests {
    use super::*;
    use ndarray::{Array2, Array3};

    #[test]
    fn sparse_embedding_engine_is_send_and_sync() {
        fn assert_send_sync<T: Send + Sync>() {}
        assert_send_sync::<SparseEmbeddingEngine>();
    }

    #[test]
    fn splade_pool_produces_normalized_sparse_vector() {
        let logits = Array3::from_shape_vec((1, 2, 3), vec![2.0, -1.0, 0.5, 0.0, 3.0, -2.0]).unwrap();
        let mask = Array2::from_shape_vec((1, 2), vec![1_i64, 1]).unwrap();
        let out = splade_pool(&logits.view(), &mask).unwrap();
        assert_eq!(out.len(), 1);
        let se = &out[0];

        let s0 = (3.0_f32).ln();
        let s1 = (4.0_f32).ln();
        let s2 = (1.5_f32).ln();
        let norm = (s0 * s0 + s1 * s1 + s2 * s2).sqrt();

        assert_eq!(se.indices, vec![0, 1, 2]);
        assert_eq!(se.values.len(), 3);
        assert!((se.values[0] - s0 / norm).abs() < 1e-5);
        assert!((se.values[1] - s1 / norm).abs() < 1e-5);
        assert!((se.values[2] - s2 / norm).abs() < 1e-5);

        let out_norm = se.values.iter().map(|v| v * v).sum::<f32>().sqrt();
        assert!((out_norm - 1.0).abs() < 1e-5);
    }

    #[test]
    fn splade_pool_drops_nonpositive_terms_and_masks_padding() {
        let logits = Array3::from_shape_vec((1, 2, 2), vec![1.0, -5.0, -5.0, 4.0]).unwrap();
        let mask = Array2::from_shape_vec((1, 2), vec![1_i64, 0]).unwrap();
        let out = splade_pool(&logits.view(), &mask).unwrap();
        let se = &out[0];
        assert_eq!(se.indices, vec![0]);
        assert_eq!(se.values.len(), 1);
        assert!((se.values[0] - 1.0).abs() < 1e-5);
    }

    #[test]
    fn splade_pool_all_zero_row_yields_empty_sparse_vector() {
        let logits = Array3::from_shape_vec((1, 2, 2), vec![-1.0, -2.0, -3.0, -4.0]).unwrap();
        let mask = Array2::from_shape_vec((1, 2), vec![1_i64, 1]).unwrap();
        let out = splade_pool(&logits.view(), &mask).unwrap();
        assert!(out[0].indices.is_empty());
        assert!(out[0].values.is_empty());
    }

    #[test]
    fn splade_pool_errors_on_mask_shape_mismatch() {
        let logits = Array3::from_shape_vec((1, 2, 2), vec![1.0, 2.0, 3.0, 4.0]).unwrap();
        let mask = Array2::from_shape_vec((1, 3), vec![1_i64, 1, 1]).unwrap();
        let err = splade_pool(&logits.view(), &mask).expect_err("mask/logits mismatch must error");
        assert!(matches!(err, SparseEmbedError::Shape(_)));
    }

    /// Regression guard for the fused relu/log1p/mask/max-pool rewrite: row 0
    /// has a padded position with a logit far larger than any real-token
    /// activation, and row 1 has no padding at all (a multi-term control row,
    /// so L2 normalization can't hide a leaked padded value behind a
    /// single-term row that trivially normalizes to 1.0). If masking were
    /// applied after the max instead of before, row 0's score would be
    /// dominated by the huge padded logit and diverge from the expected value.
    #[test]
    fn splade_pool_masks_before_max_multirow() {
        let logits = Array3::from_shape_vec((2, 2, 2), vec![0.5, -1.0, 100.0, 100.0, 0.5, -1.0, 0.2, 0.3]).unwrap();
        let mask = Array2::from_shape_vec((2, 2), vec![1_i64, 0_i64, 1_i64, 1_i64]).unwrap();
        let out = splade_pool(&logits.view(), &mask).unwrap();

        assert_eq!(out[0].indices, vec![0]);
        assert!((out[0].values[0] - 1.0).abs() < 1e-5);

        let v0 = (1.5_f32).ln();
        let v1 = (1.3_f32).ln();
        let norm = (v0 * v0 + v1 * v1).sqrt();
        assert_eq!(out[1].indices, vec![0, 1]);
        assert_eq!(out[1].values.len(), 2);
        assert!((out[1].values[0] - v0 / norm).abs() < 1e-5);
        assert!((out[1].values[1] - v1 / norm).abs() < 1e-5);
    }
}
