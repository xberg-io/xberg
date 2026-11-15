//! ColBERT late-interaction (multi-vector) inference engine.
//!
//! Runs a ColBERT-style ONNX model and turns its `[batch, seq, dim]` token
//! embeddings into per-token, L2-normalized multi-vector embeddings — one
//! [`MultiVectorEmbedding`] per input text, retaining every real (non-padding)
//! token row rather than pooling to a single vector.
//!
//! Reimplements the ColBERT-specific tokenization tricks from
//! [EmbedAnything's `colbert.rs`](https://github.com/StarlightSearch/EmbedAnything):
//! marker-token insertion at position 1 (right after `[CLS]`) and, for
//! queries only, fixed-length padding with the mask token kept
//! attention-live ("query augmentation").
//!
//! Like [`crate::sparse_embeddings::engine::SparseEmbeddingEngine`], `embed()`
//! takes `&self` so a single engine can serve concurrent callers via `Arc`; the
//! session sits behind a mutex because `ort::Session::run` takes `&mut self`.

use ndarray::{Array2, ArrayView, ArrayView3, Axis, Dim, IxDynImpl};
use ort::session::Session;
use ort::value::Value;
use parking_lot::Mutex;
use tokenizers::Tokenizer;

use super::MultiVectorEmbedding;

/// Errors raised by the late-interaction engine.
///
/// Rust-only: the `Ort` variant wraps `ort::Error`, which has no faithful
/// binding representation. Public callers receive `crate::XbergError` instead.
#[cfg_attr(alef, alef(skip))]
#[derive(Debug)]
pub enum LateInteractionError {
    /// Tokenization failed with the given message.
    Tokenizer(String),
    /// ONNX Runtime returned an error during inference.
    Ort(ort::Error),
    /// The model output tensor had an unexpected shape.
    Shape(String),
    /// The model produced no output tensors.
    NoOutput,
}

impl From<ort::Error> for LateInteractionError {
    fn from(e: ort::Error) -> Self {
        LateInteractionError::Ort(e)
    }
}

/// Parallel per-row tensors for one tokenized batch, one row per encoding.
struct InputRows {
    ids_rows: Vec<Vec<i64>>,
    mask_rows: Vec<Vec<i64>>,
    type_rows: Vec<Vec<i64>>,
}

/// ColBERT late-interaction (multi-vector) model with thread-safe inference.
///
/// Rust-only: an opaque ORT-backed handle with no faithful binding
/// representation (mirrors `reranking::engine::RerankerEngine`). Bindings drive
/// inference through the module-level functions, not this type.
#[cfg_attr(alef, alef(skip))]
pub struct LateInteractionEngine {
    tokenizer: Tokenizer,
    session: Mutex<Session>,
    need_token_type_ids: bool,
    query_marker_id: Option<u32>,
    doc_marker_id: Option<u32>,
    mask_id: Option<u32>,
    query_max_length: usize,
}

impl LateInteractionEngine {
    /// Create a new engine from a pre-built session, tokenizer, and the
    /// ColBERT marker/mask token ids resolved from the tokenizer's vocabulary.
    ///
    /// `query_marker_id` / `doc_marker_id` are typically `[Q]` / `[D]`; `mask_id`
    /// is typically `[MASK]`, used as the pad token for query augmentation. Any
    /// of the three may be `None` if the tokenizer does not define them, in
    /// which case the corresponding manipulation is skipped.
    pub(crate) fn new(
        tokenizer: Tokenizer,
        session: Session,
        query_marker_id: Option<u32>,
        doc_marker_id: Option<u32>,
        mask_id: Option<u32>,
        query_max_length: usize,
    ) -> Self {
        let need_token_type_ids = session.inputs().iter().any(|input| input.name() == "token_type_ids");
        Self {
            tokenizer,
            session: Mutex::new(session),
            need_token_type_ids,
            query_marker_id,
            doc_marker_id,
            mask_id,
            query_max_length,
        }
    }

    /// Generate multi-vector (ColBERT) embeddings for a batch of texts.
    ///
    /// `is_query` selects the marker token (query vs. document) and, when
    /// `true`, applies fixed-length query augmentation padding.
    ///
    /// Thread-safe: multiple threads may call `embed()` concurrently on the
    /// same engine instance.
    pub(crate) fn embed<S: AsRef<str>>(
        &self,
        texts: &[S],
        batch_size: usize,
        is_query: bool,
    ) -> Result<Vec<MultiVectorEmbedding>, LateInteractionError> {
        if texts.is_empty() {
            return Ok(Vec::new());
        }
        let batch_size = if batch_size == 0 { 16 } else { batch_size };

        let mut all = Vec::with_capacity(texts.len());
        for batch in texts.chunks(batch_size) {
            all.extend(self.embed_batch(batch, is_query)?);
        }
        Ok(all)
    }

    /// Build the per-row token id, attention mask, and token-type vectors for one batch.
    ///
    /// All three vectors are parallel and share one row per encoding.
    ///
    /// Applies the ColBERT tokenization tricks to each encoding in turn: marker
    /// insertion at position 1 and, for queries, fixed-length mask padding.
    fn build_input_rows(&self, encodings: &[tokenizers::Encoding], batch_size: usize, is_query: bool) -> InputRows {
        let marker_id = if is_query {
            self.query_marker_id
        } else {
            self.doc_marker_id
        };

        let mut ids_rows: Vec<Vec<i64>> = Vec::with_capacity(batch_size);
        let mut mask_rows: Vec<Vec<i64>> = Vec::with_capacity(batch_size);
        let mut type_rows: Vec<Vec<i64>> = Vec::with_capacity(batch_size);

        for encoding in encodings {
            let mut ids: Vec<i64> = encoding.get_ids().iter().map(|&x| x as i64).collect();
            let mut mask: Vec<i64> = encoding.get_attention_mask().iter().map(|&x| x as i64).collect();

            insert_marker(&mut ids, &mut mask, marker_id);

            if is_query {
                pad_query(&mut ids, &mut mask, self.mask_id, self.query_max_length);
            }

            let types = vec![0i64; ids.len()];

            ids_rows.push(ids);
            mask_rows.push(mask);
            type_rows.push(types);
        }

        InputRows {
            ids_rows,
            mask_rows,
            type_rows,
        }
    }

    fn embed_batch<S: AsRef<str>>(
        &self,
        batch: &[S],
        is_query: bool,
    ) -> Result<Vec<MultiVectorEmbedding>, LateInteractionError> {
        let inputs: Vec<&str> = batch.iter().map(|t| t.as_ref()).collect();
        let encodings = self
            .tokenizer
            .encode_batch(inputs, true)
            .map_err(|e| LateInteractionError::Tokenizer(e.to_string()))?;

        let batch_size = batch.len();
        let InputRows {
            ids_rows,
            mask_rows,
            type_rows,
        } = self.build_input_rows(&encodings, batch_size, is_query);

        let seq_len = ids_rows
            .first()
            .map(Vec::len)
            .ok_or_else(|| LateInteractionError::Tokenizer("Empty encodings".to_string()))?;

        let mut ids_flat = Vec::with_capacity(batch_size * seq_len);
        let mut mask_flat = Vec::with_capacity(batch_size * seq_len);
        let mut types_flat = Vec::with_capacity(batch_size * seq_len);
        for i in 0..batch_size {
            ids_flat.extend_from_slice(&ids_rows[i]);
            mask_flat.extend_from_slice(&mask_rows[i]);
            types_flat.extend_from_slice(&type_rows[i]);
        }

        let ids_tensor = ndarray::Array::from_shape_vec((batch_size, seq_len), ids_flat)
            .map_err(|e| LateInteractionError::Shape(e.to_string()))?;
        let type_ids_tensor = ndarray::Array::from_shape_vec((batch_size, seq_len), types_flat)
            .map_err(|e| LateInteractionError::Shape(e.to_string()))?;
        let mask_nd = ndarray::Array::from_shape_vec((batch_size, seq_len), mask_flat)
            .map_err(|e| LateInteractionError::Shape(e.to_string()))?;

        let mut session_inputs = ort::inputs![
            "input_ids" => Value::from_array(ids_tensor)?,
            "attention_mask" => Value::from_array(mask_nd.clone())?,
        ];
        if self.need_token_type_ids {
            session_inputs.push(("token_type_ids".into(), Value::from_array(type_ids_tensor)?.into()));
        }

        let output_value = {
            let mut session = self.session.lock();
            let outputs = session.run(session_inputs).map_err(LateInteractionError::Ort)?;
            outputs.into_iter().next().map(|(_, value)| value)
        }
        .ok_or(LateInteractionError::NoOutput)?;
        let embeddings: ArrayView<f32, Dim<IxDynImpl>> =
            output_value.try_extract_array().map_err(LateInteractionError::Ort)?;
        let embeddings = embeddings
            .into_dimensionality::<ndarray::Ix3>()
            .map_err(|e| LateInteractionError::Shape(format!("expected [batch, seq, dim] embeddings: {e}")))?;

        Ok(normalize_tokens(&embeddings, &mask_nd))
    }
}

/// Insert `marker_id` at position 1 (right after `[CLS]` at index 0), shifting
/// all following tokens right by one and dropping the last token to keep the
/// row length unchanged. Sets `attention_mask[1] = 1`. A no-op if `marker_id`
/// is `None` or the row has fewer than 2 tokens.
///
/// Mirrors EmbedAnything `colbert.rs:200-214`.
fn insert_marker(ids: &mut [i64], mask: &mut [i64], marker_id: Option<u32>) {
    let Some(marker_id) = marker_id else { return };
    if ids.len() < 2 {
        return;
    }
    for i in (2..ids.len()).rev() {
        ids[i] = ids[i - 1];
        mask[i] = mask[i - 1];
    }
    ids[1] = marker_id as i64;
    mask[1] = 1;
}

/// Pad a query row to a fixed `query_max_length` using `mask_id` as the pad
/// token, keeping the padded positions attention-live (`mask = 1`). This is
/// ColBERT's "query augmentation" — the [MASK] padding lets the model treat
/// the query as a fixed-length soft template. Rows already at or beyond
/// `query_max_length` are truncated to it.
///
/// Mirrors EmbedAnything `colbert.rs:114-125`.
fn pad_query(ids: &mut Vec<i64>, mask: &mut Vec<i64>, mask_id: Option<u32>, query_max_length: usize) {
    if query_max_length == 0 {
        return;
    }
    if ids.len() > query_max_length {
        ids.truncate(query_max_length);
        mask.truncate(query_max_length);
        return;
    }
    let pad_id = mask_id.unwrap_or(0) as i64;
    while ids.len() < query_max_length {
        ids.push(pad_id);
        mask.push(1);
    }
}

/// Zero out padded positions (mask broadcast over the embedding axis) and
/// L2-normalize each token vector independently: `v / (||v|| + 1e-10)`.
///
/// `embeddings` is `[batch, seq, dim]`; `mask` is `[batch, seq]` (1 = real or
/// marker/pad-augmented token, 0 = padding). Returns one [`MultiVectorEmbedding`]
/// per batch row, keeping only the attention-live token rows — ColBERT is
/// multi-vector, so no pooling happens here.
///
/// Mirrors EmbedAnything `colbert.rs:227-246`.
pub(crate) fn normalize_tokens(embeddings: &ArrayView3<f32>, mask: &Array2<i64>) -> Vec<MultiVectorEmbedding> {
    const EPS: f32 = 1e-10;
    let dim = embeddings.shape()[2];

    embeddings
        .axis_iter(Axis(0))
        .zip(mask.axis_iter(Axis(0)))
        .map(|(token_rows, mask_row)| {
            let mut data = Vec::new();
            let mut num_tokens: u32 = 0;
            for (token, &m) in token_rows.axis_iter(Axis(0)).zip(mask_row.iter()) {
                if m == 0 {
                    continue;
                }
                let norm = token.iter().map(|&v| v * v).sum::<f32>().sqrt();
                data.extend(token.iter().map(|&v| v / (norm + EPS)));
                num_tokens += 1;
            }
            MultiVectorEmbedding {
                num_tokens,
                dim: dim as u32,
                data,
            }
        })
        .collect()
}

#[cfg(test)]
mod tests {
    use super::*;
    use ndarray::Array3;

    #[test]
    fn late_interaction_engine_is_send_and_sync() {
        fn assert_send_sync<T: Send + Sync>() {}
        assert_send_sync::<LateInteractionEngine>();
    }

    #[test]
    fn insert_marker_shifts_and_splices_at_index_one() {
        let mut ids = vec![101_i64, 2054, 2003, 102];
        let mut mask = vec![1_i64, 1, 1, 1];
        insert_marker(&mut ids, &mut mask, Some(999));
        assert_eq!(ids, vec![101, 999, 2054, 2003]);
        assert_eq!(mask, vec![1, 1, 1, 1]);
    }

    #[test]
    fn insert_marker_noop_when_marker_id_is_none() {
        let mut ids = vec![101_i64, 2054, 102];
        let mut mask = vec![1_i64, 1, 1];
        insert_marker(&mut ids, &mut mask, None);
        assert_eq!(ids, vec![101, 2054, 102]);
    }

    #[test]
    fn pad_query_extends_with_live_mask_token() {
        let mut ids = vec![101_i64, 999, 2054];
        let mut mask = vec![1_i64, 1, 1];
        pad_query(&mut ids, &mut mask, Some(103), 5);
        assert_eq!(ids, vec![101, 999, 2054, 103, 103]);
        assert_eq!(mask, vec![1, 1, 1, 1, 1], "padded positions must stay attention-live");
    }

    #[test]
    fn pad_query_truncates_when_already_longer() {
        let mut ids = vec![1_i64, 2, 3, 4, 5, 6];
        let mut mask = vec![1_i64; 6];
        pad_query(&mut ids, &mut mask, Some(103), 4);
        assert_eq!(ids, vec![1, 2, 3, 4]);
        assert_eq!(mask.len(), 4);
    }

    #[test]
    fn normalize_tokens_l2_normalizes_and_skips_padding() {
        let embeddings = Array3::from_shape_vec((1, 2, 2), vec![3.0_f32, 4.0, 1.0, 1.0]).unwrap();
        let mask = Array2::from_shape_vec((1, 2), vec![1_i64, 0]).unwrap();
        let out = normalize_tokens(&embeddings.view(), &mask);

        assert_eq!(out.len(), 1);
        let mv = &out[0];
        assert_eq!(mv.num_tokens, 1, "padded row must be dropped, not zeroed");
        assert_eq!(mv.dim, 2);
        assert_eq!(mv.data.len(), 2);
        assert!((mv.data[0] - 0.6).abs() < 1e-5);
        assert!((mv.data[1] - 0.8).abs() < 1e-5);

        let norm = (mv.data[0] * mv.data[0] + mv.data[1] * mv.data[1]).sqrt();
        assert!((norm - 1.0).abs() < 1e-4);
    }

    #[test]
    fn normalize_tokens_keeps_all_real_and_marker_rows() {
        let embeddings = Array3::from_shape_vec((1, 3, 2), vec![1.0_f32, 0.0, 0.0, 2.0, 3.0, 4.0]).unwrap();
        let mask = Array2::from_shape_vec((1, 3), vec![1_i64, 1, 1]).unwrap();
        let out = normalize_tokens(&embeddings.view(), &mask);

        assert_eq!(out[0].num_tokens, 3);
        assert_eq!(out[0].data.len(), 6);
        assert!((out[0].data[4] - 0.6).abs() < 1e-5);
        assert!((out[0].data[5] - 0.8).abs() < 1e-5);
    }

    #[test]
    fn normalize_tokens_all_padding_yields_empty_embedding() {
        let embeddings = Array3::from_shape_vec((1, 2, 2), vec![1.0_f32, 2.0, 3.0, 4.0]).unwrap();
        let mask = Array2::from_shape_vec((1, 2), vec![0_i64, 0]).unwrap();
        let out = normalize_tokens(&embeddings.view(), &mask);
        assert_eq!(out[0].num_tokens, 0);
        assert!(out[0].data.is_empty());
    }
}
