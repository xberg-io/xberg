//! Cross-encoder ONNX inference engine.
//!
//! Core inference pipeline for ONNX-based cross-encoder reranking.
//! Key design: `rerank()` takes `&self` instead of `&mut self`, enabling
//! concurrent inference from multiple threads without mutex contention.
//!
//! This is safe because `ort::Session::run()` takes `&mut self` purely as
//! an API constraint — its internal `run_inner()` takes `&self`, and the
//! ONNX Runtime C API (`OrtApi::Run`) is documented as thread-safe for
//! concurrent calls on the same session.
//!
//! Mirrors `crates/xberg/src/embeddings/engine.rs` with three changes:
//! - Tokenizer encodes `(query, document)` pairs via `EncodeInput::Dual`.
//! - Output is `[batch, 1]` or `[batch]` logits — squeezed to `Vec<f32>`.
//! - No pooling step — cross-encoders pool internally.
//!
//! # Qwen3 generative-reranker head
//!
//! In addition to the classic cross-encoder head above, this engine supports
//! Qwen3 generative-reranker checkpoints (e.g. `Qwen/Qwen3-Reranker-0.6B`),
//! selected via [`RerankerHead::Qwen3Generative`]. These models are causal LMs
//! repurposed for reranking: the ONNX output is `[batch, seq, vocab]` logits,
//! and relevance is read off the **last token's** logits at the "yes"/"no"
//! token ids, softmaxed into a probability. See [`qwen3_scores`] and
//! `rerank_batch` for the extraction logic, and the module-level chat-template
//! constants [`QWEN3_QUERY_PREFIX`] / [`QWEN3_DOCUMENT_SUFFIX`] for the
//! prompt wrapping applied at encode time.
//!
//! Since v5.0.0.

use ndarray::{ArrayView, ArrayView3, Dim, Dimension, IxDynImpl, s};
use ort::session::Session;
use ort::value::Value;
use thiserror::Error;
use tokenizers::{EncodeInput, InputSequence, Tokenizer};

use crate::core::config::reranker::RerankerHead;

/// Errors that can occur during cross-encoder reranking inference.
///
/// Since v5.0.0.
#[cfg_attr(alef, alef(skip))]
#[derive(Debug, Error)]
pub enum RerankError {
    /// Tokenization failed with the given message.
    #[error("Tokenizer error: {0}")]
    Tokenizer(String),
    /// ONNX Runtime returned an error during inference.
    #[error("ONNX Runtime error: {0}")]
    Ort(#[from] ort::Error),
    /// The model output tensor had an unexpected shape.
    #[error("Tensor shape error: {0}")]
    Shape(String),
    /// The model produced no output tensors.
    #[error("Model produced no output tensors")]
    NoOutput,
}

/// Chat-template prefix prepended to the query for Qwen3 generative rerankers.
///
/// Frames the (query, document) pair as an instruction-following judgment task,
/// matching the prompt format Qwen3-Reranker was fine-tuned on. Applied only
/// when [`RerankerHead::Qwen3Generative`] is selected.
const QWEN3_QUERY_PREFIX: &str = "<|im_start|>system\n\
Judge whether the Document meets the requirements based on the Query and the Instruct provided. \
Note that the answer can only be \"yes\" or \"no\".<|im_end|>\n\
<|im_start|>user\n\
<Instruct>: Given a web search query, retrieve relevant passages that answer the query\n\
<Query>: ";

/// Chat-template suffix appended after the document for Qwen3 generative rerankers.
///
/// Closes the user turn and opens an empty assistant turn (with an empty
/// `<think>` block, matching Qwen3's reasoning-model chat template) so the
/// model's very next token is its yes/no judgment.
const QWEN3_DOCUMENT_SUFFIX: &str = "<|im_end|>\n<|im_start|>assistant\n<think>\n\n</think>\n\n";

/// Cross-encoder reranking model with thread-safe inference.
///
/// The `rerank()` method takes `&self` instead of `&mut self`, allowing it to
/// be shared across threads via `Arc<RerankerEngine>` without mutex contention.
///
/// Supports two scoring heads, selected by `head`:
/// - [`RerankerHead::CrossEncoder`] (default) — the original single-logit path.
/// - [`RerankerHead::Qwen3Generative`] — Qwen3 generative-reranker path; requires
///   `true_token_id` / `false_token_id` to have been resolved from the tokenizer.
///
/// Since v5.0.0.
#[cfg_attr(alef, alef(skip))]
pub struct RerankerEngine {
    tokenizer: Tokenizer,
    session: Session,
    need_token_type_ids: bool,
    head: RerankerHead,
    /// Token id for "yes" (relevant). Only set/used for [`RerankerHead::Qwen3Generative`].
    true_token_id: Option<u32>,
    /// Token id for "no" (not relevant). Only set/used for [`RerankerHead::Qwen3Generative`].
    false_token_id: Option<u32>,
}

impl RerankerEngine {
    /// Create a new reranker engine from a pre-built session and tokenizer.
    ///
    /// `head` selects the scoring path. `true_token_id` / `false_token_id` are
    /// the tokenizer ids for "yes" / "no" (resolved by the caller at load time,
    /// e.g. via `Tokenizer::token_to_id`) and are only consulted when
    /// `head == RerankerHead::Qwen3Generative`; pass `None` for the classic
    /// cross-encoder path.
    pub(crate) fn new(
        tokenizer: Tokenizer,
        session: Session,
        head: RerankerHead,
        true_token_id: Option<u32>,
        false_token_id: Option<u32>,
    ) -> Self {
        let need_token_type_ids = session.inputs().iter().any(|input| input.name() == "token_type_ids");
        Self {
            tokenizer,
            session,
            need_token_type_ids,
            head,
            true_token_id,
            false_token_id,
        }
    }

    /// Score a batch of `(query, document)` pairs.
    ///
    /// Returns one logit per pair in the same order as the input.
    /// Apply sigmoid to convert logits to `[0, 1]` scores.
    ///
    /// This method is **thread-safe** — multiple threads can call `rerank()`
    /// concurrently on the same `RerankerEngine` instance.
    ///
    /// # Safety note
    ///
    /// Uses an internal unsafe cast because `ort::Session::run()` takes
    /// `&mut self` despite performing no mutation (its `run_inner()` takes
    /// `&self`). The ONNX Runtime C API is documented as thread-safe for
    /// concurrent `Run()` calls on the same session.
    pub(crate) fn rerank(&self, query: &str, documents: &[&str], batch_size: usize) -> Result<Vec<f32>, RerankError> {
        if documents.is_empty() {
            return Ok(Vec::new());
        }

        // Defensive: callers from polyglot bindings may pass batch_size=0 when the
        // host-side `RerankerConfig` mirror omits the serde default.
        let batch_size = if batch_size == 0 { 32 } else { batch_size };

        let mut all_scores = Vec::with_capacity(documents.len());

        for batch in documents.chunks(batch_size) {
            let batch_scores = self.rerank_batch(query, batch)?;
            all_scores.extend(batch_scores);
        }

        Ok(all_scores)
    }

    /// Score a single batch of `(query, document)` pairs.
    fn rerank_batch(&self, query: &str, documents: &[&str]) -> Result<Vec<f32>, RerankError> {
        // Qwen3 generative rerankers are causal LMs, not BERT-style cross-encoders:
        // the (query, document) pair is wrapped in a chat-template prompt and
        // tokenized as a single sequence (no pair-encoding, no token_type_ids).
        // The cross-encoder path keeps the original `EncodeInput::Dual` pair encoding.
        let owned_prompts: Vec<String>;
        let encodings = if self.head == RerankerHead::Qwen3Generative {
            owned_prompts = documents
                .iter()
                .map(|doc| format!("{QWEN3_QUERY_PREFIX}{query}\n\n<Document>: {doc}{QWEN3_DOCUMENT_SUFFIX}"))
                .collect();
            let inputs: Vec<EncodeInput<'_>> = owned_prompts
                .iter()
                .map(|prompt| EncodeInput::Single(InputSequence::Raw(std::borrow::Cow::Borrowed(prompt.as_str()))))
                .collect();
            self.tokenizer
                .encode_batch(inputs, true)
                .map_err(|e| RerankError::Tokenizer(e.to_string()))?
        } else {
            let pairs: Vec<EncodeInput<'_>> = documents
                .iter()
                .map(|doc| {
                    EncodeInput::Dual(
                        InputSequence::Raw(std::borrow::Cow::Borrowed(query)),
                        InputSequence::Raw(std::borrow::Cow::Borrowed(doc)),
                    )
                })
                .collect();
            self.tokenizer
                .encode_batch(pairs, true)
                .map_err(|e| RerankError::Tokenizer(e.to_string()))?
        };

        let encoding_length = encodings
            .first()
            .ok_or_else(|| RerankError::Tokenizer("Empty encodings".to_string()))?
            .len();
        let batch_size = documents.len();
        let max_size = encoding_length * batch_size;

        // Build input tensors.
        let mut ids_array = Vec::with_capacity(max_size);
        let mut mask_array = Vec::with_capacity(max_size);
        let mut type_ids_array = Vec::with_capacity(max_size);

        for encoding in &encodings {
            ids_array.extend(encoding.get_ids().iter().map(|&x| x as i64));
            mask_array.extend(encoding.get_attention_mask().iter().map(|&x| x as i64));
            type_ids_array.extend(encoding.get_type_ids().iter().map(|&x| x as i64));
        }

        let ids_tensor = ndarray::Array::from_shape_vec((batch_size, encoding_length), ids_array)
            .map_err(|e| RerankError::Shape(e.to_string()))?;
        let type_ids_tensor = ndarray::Array::from_shape_vec((batch_size, encoding_length), type_ids_array)
            .map_err(|e| RerankError::Shape(e.to_string()))?;
        let mask_tensor = ndarray::Array::from_shape_vec((batch_size, encoding_length), mask_array)
            .map_err(|e| RerankError::Shape(e.to_string()))?;

        // Per-row index of the last unmasked (`attention_mask == 1`) token.
        // Qwen3 is a causal LM whose relevance logit lives at the final real
        // token — which is *not* `seq_len - 1` for a shorter, right-padded row —
        // so the generative head must read each row at this index rather than a
        // hardcoded last position. Falls back to the final position for a fully
        // masked row (never expected). Unused by the cross-encoder head.
        let last_token_indices: Vec<usize> = mask_tensor
            .outer_iter()
            .map(|row| {
                row.iter()
                    .rposition(|&m| m != 0)
                    .unwrap_or(encoding_length.saturating_sub(1))
            })
            .collect();

        let mut session_inputs = ort::inputs![
            "input_ids" => Value::from_array(ids_tensor)?,
            "attention_mask" => Value::from_array(mask_tensor)?,
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
        .map_err(RerankError::Ort)?;

        // Extract the logit output tensor.
        let (_, output_value) = outputs.iter().next().ok_or(RerankError::NoOutput)?;
        let tensor: ArrayView<f32, Dim<IxDynImpl>> = output_value.try_extract_array().map_err(RerankError::Ort)?;

        let scores = match self.head {
            RerankerHead::CrossEncoder => {
                // Squeeze [batch, 1] or [batch] to Vec<f32>.
                // Cross-encoders typically output [batch, 1]; squeeze to [batch].
                match tensor.dim().ndim() {
                    1 => tensor.slice(s![..]).iter().copied().collect(),
                    2 => tensor.slice(s![.., 0]).iter().copied().collect(),
                    n => return Err(RerankError::Shape(format!("Expected 1D or 2D output tensor, got {n}D"))),
                }
            }
            RerankerHead::Qwen3Generative => {
                let true_id = self
                    .true_token_id
                    .ok_or_else(|| RerankError::Shape("Qwen3 head requires a resolved true_token_id".to_string()))?;
                let false_id = self
                    .false_token_id
                    .ok_or_else(|| RerankError::Shape("Qwen3 head requires a resolved false_token_id".to_string()))?;

                if tensor.dim().ndim() != 3 {
                    return Err(RerankError::Shape(format!(
                        "Qwen3 generative head expects a 3D [batch, seq, vocab] output tensor, got {}D",
                        tensor.dim().ndim()
                    )));
                }
                let logits: ArrayView3<f32> = tensor
                    .view()
                    .into_dimensionality::<ndarray::Ix3>()
                    .map_err(|e| RerankError::Shape(format!("Failed to reshape Qwen3 output to 3D: {e}")))?;
                qwen3_scores(&logits, true_id, false_id, &last_token_indices)?
            }
        };

        Ok(scores)
    }
}

/// Compute Qwen3 generative-reranker relevance scores from raw output logits.
///
/// `logits` has shape `[batch, seq, vocab]`. For each item `b` in the batch,
/// this reads that row's **last real token** (`last_token_indices[b]`, i.e. the
/// final position where `attention_mask == 1`), gathers the `true_id` ("yes")
/// and `false_id` ("no") entries, and returns `softmax([false_logit,
/// true_logit])[1]` — i.e. `P(yes)`. This is already a `[0, 1]` probability,
/// unlike the cross-encoder path's raw logit.
///
/// Using the per-row last-unmasked index (rather than a shared `seq_len - 1`)
/// is required for correctness under batched, right-padded tokenization: for
/// any row shorter than the batch's longest, position `seq_len - 1` is a `[PAD]`
/// token whose logits are meaningless. Mirrors the EmbedAnything `qwen3.rs`
/// reference semantics of reading the final assistant-turn token.
///
/// # Errors
///
/// Returns [`RerankError::Shape`] if `logits` has zero sequence length (no
/// token to read).
fn qwen3_scores(
    logits: &ArrayView3<f32>,
    true_id: u32,
    false_id: u32,
    last_token_indices: &[usize],
) -> Result<Vec<f32>, RerankError> {
    let (batch, seq_len, vocab) = logits.dim();
    if seq_len == 0 {
        return Err(RerankError::Shape(
            "Qwen3 generative head received a zero-length sequence".to_string(),
        ));
    }

    let true_id = true_id as usize;
    let false_id = false_id as usize;
    if true_id >= vocab || false_id >= vocab {
        return Err(RerankError::Shape(format!(
            "Qwen3 true/false token id out of vocab range: true={true_id}, false={false_id}, vocab={vocab}"
        )));
    }

    let mut scores = Vec::with_capacity(batch);
    for b in 0..batch {
        // Clamp defensively so a stale/oversized index can never index out of bounds.
        let idx = last_token_indices
            .get(b)
            .copied()
            .unwrap_or(seq_len - 1)
            .min(seq_len - 1);
        let false_logit = logits[[b, idx, false_id]];
        let true_logit = logits[[b, idx, true_id]];
        // Numerically-stable 2-way softmax: subtract the max before exponentiating.
        let max_logit = false_logit.max(true_logit);
        let false_exp = (false_logit - max_logit).exp();
        let true_exp = (true_logit - max_logit).exp();
        let denom = false_exp + true_exp;
        scores.push(true_exp / denom);
    }

    Ok(scores)
}

// SAFETY: RerankerEngine is Send + Sync because:
// 1. Tokenizer is Send + Sync (confirmed in tokenizers crate)
// 2. Session: we only call run() which is internally thread-safe per ONNX Runtime docs
// 3. All other fields are immutable after construction
#[allow(unsafe_code)]
unsafe impl Send for RerankerEngine {}
#[allow(unsafe_code)]
unsafe impl Sync for RerankerEngine {}

#[cfg(test)]
mod tests {
    use super::*;
    // Exercise the production sigmoid, not a local copy — keeps engine tests
    // honest if the mod-level sigmoid ever changes.
    use super::super::sigmoid_f32 as sigmoid;

    #[test]
    fn sigmoid_zero_gives_half() {
        let s = sigmoid(0.0);
        assert!((s - 0.5).abs() < 1e-6, "sigmoid(0) should be 0.5, got {s}");
    }

    #[test]
    fn sigmoid_large_positive_approaches_one() {
        let s = sigmoid(100.0);
        assert!(s > 0.99, "sigmoid(100) should be close to 1.0, got {s}");
    }

    #[test]
    fn sigmoid_large_negative_approaches_zero() {
        let s = sigmoid(-100.0);
        assert!(s < 0.01, "sigmoid(-100) should be close to 0.0, got {s}");
    }

    #[test]
    fn rerank_error_display_does_not_panic() {
        let err = RerankError::Tokenizer("test".to_string());
        assert!(format!("{err}").contains("Tokenizer"));

        let err = RerankError::Shape("bad shape".to_string());
        assert!(format!("{err}").contains("shape"));

        let err = RerankError::NoOutput;
        assert!(format!("{err}").contains("no output"));
    }

    #[test]
    fn rerank_error_implements_error_trait() {
        let err = RerankError::Shape("test".to_string());
        let _: &dyn std::error::Error = &err;
    }

    /// Build a `[batch, seq, vocab]` tensor from per-batch-item, per-token logit rows.
    fn make_logits(rows: Vec<Vec<Vec<f32>>>) -> ndarray::Array3<f32> {
        let batch = rows.len();
        let seq = rows[0].len();
        let vocab = rows[0][0].len();
        let flat: Vec<f32> = rows.into_iter().flatten().flatten().collect();
        ndarray::Array3::from_shape_vec((batch, seq, vocab), flat).expect("valid shape")
    }

    #[test]
    fn qwen3_scores_hand_computed_softmax() {
        // vocab = 4; true_id = 3 ("yes"), false_id = 2 ("no").
        // batch item 0, last token logits: [_, _, false=1.0, true=2.0]
        //   softmax([1.0, 2.0]): exp(1-2)=0.3679, exp(2-2)=1.0, denom=1.3679
        //   P(yes) = 1.0 / 1.3679 = 0.7310585...
        // batch item 1, last token logits: [_, _, false=2.0, true=0.0]
        //   softmax([2.0, 0.0]): exp(2-2)=1.0, exp(0-2)=0.1353, denom=1.1353
        //   P(yes) = 0.1353 / 1.1353 = 0.1192029...
        let logits = make_logits(vec![
            // batch item 0: two tokens in sequence; only the LAST is read.
            vec![vec![9.0, 9.0, 9.0, 9.0], vec![0.0, 0.0, 1.0, 2.0]],
            vec![vec![9.0, 9.0, 9.0, 9.0], vec![0.0, 0.0, 2.0, 0.0]],
        ]);
        let view = logits.view();
        let scores = qwen3_scores(&view, 3, 2, &[1, 1]).expect("scores must compute");

        assert_eq!(scores.len(), 2);
        assert!(
            (scores[0] - 0.7310586).abs() < 1e-5,
            "expected P(yes) ~= 0.7310586 for item 0, got {}",
            scores[0]
        );
        assert!(
            (scores[1] - 0.1192029).abs() < 1e-5,
            "expected P(yes) ~= 0.1192029 for item 1, got {}",
            scores[1]
        );
        // Scores are already probabilities in [0, 1] — no sigmoid needed.
        for &s in &scores {
            assert!((0.0..=1.0).contains(&s), "score {s} must be in [0, 1]");
        }
    }

    #[test]
    fn qwen3_scores_equal_logits_gives_half() {
        // true == false logit -> softmax is exactly [0.5, 0.5].
        let logits = make_logits(vec![vec![vec![0.0, 0.0, 3.0, 3.0]]]);
        let view = logits.view();
        let scores = qwen3_scores(&view, 3, 2, &[0]).expect("scores must compute");
        assert_eq!(scores.len(), 1);
        assert!(
            (scores[0] - 0.5).abs() < 1e-6,
            "equal logits must give P(yes)=0.5, got {}",
            scores[0]
        );
    }

    #[test]
    fn qwen3_scores_only_reads_last_token() {
        // First token strongly favors "no"; last token strongly favors "yes".
        // A correct implementation ignores the first token entirely.
        let logits = make_logits(vec![vec![vec![0.0, 0.0, 100.0, -100.0], vec![0.0, 0.0, -100.0, 100.0]]]);
        let view = logits.view();
        let scores = qwen3_scores(&view, 3, 2, &[1]).expect("scores must compute");
        assert!(
            scores[0] > 0.99,
            "must score based on last token only, got {}",
            scores[0]
        );
    }

    #[test]
    fn qwen3_scores_reads_last_unmasked_token_under_right_padding() {
        // A right-padded row: real content at index 0 (favours "yes"), padding
        // at index 1 (favours "no"). With last_token_indices = [0], the score
        // must come from index 0 — proving we read the last *real* token, not
        // the padded sequence end. A hardcoded seq_len-1 would score ~0 here.
        let logits = make_logits(vec![vec![vec![0.0, 0.0, -100.0, 100.0], vec![0.0, 0.0, 100.0, -100.0]]]);
        let view = logits.view();
        let scores = qwen3_scores(&view, 3, 2, &[0]).expect("scores must compute");
        assert!(
            scores[0] > 0.99,
            "must read the last unmasked token (index 0), got {}",
            scores[0]
        );
    }

    #[test]
    fn qwen3_scores_rejects_zero_length_sequence() {
        let logits = ndarray::Array3::<f32>::zeros((1, 0, 4));
        let view = logits.view();
        let err = qwen3_scores(&view, 3, 2, &[0]).expect_err("zero-length sequence must error");
        assert!(matches!(err, RerankError::Shape(_)));
    }

    #[test]
    fn qwen3_scores_rejects_out_of_range_token_ids() {
        let logits = make_logits(vec![vec![vec![0.0, 0.0, 1.0, 2.0]]]);
        let view = logits.view();
        let err = qwen3_scores(&view, 99, 2, &[0]).expect_err("out-of-range true_id must error");
        assert!(matches!(err, RerankError::Shape(_)));
    }
}
