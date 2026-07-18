//! Orchestrates encode → encoder.forward → heads → [`crate::decode::ScorerOutput`]
//! for the Candle GLiNER2 "entities" task. New glue code (not a port) built
//! on `xberg_gliner::encode_v2`'s already-shipped schema-prompt encoding —
//! see plan Task 6 module doc for why this replaces anno's separate
//! `ProcessedRecord`/`TaskMapping` machinery.

use candle_core::Tensor;
use ndarray::Array4;

use xberg_gliner::{V2Encoded, encode_v2};

use crate::decode::{MAX_COUNT, MAX_WIDTH, ScorerOutput, build_span_idx};
use crate::heads::schema_gather::SchemaGather;
use crate::heads::scorer::Scorer;
use crate::heads::token_gather::TokenGather;

/// Run the full entities-task pipeline for one `(text, labels)` pair.
/// Returns `(scorer_out, pred_count, encoded)` — `encoded.words` is needed
/// by the caller (Task 8's `extract_ner`) to decode byte offsets.
pub(crate) fn run_pipeline(
    tokenizer: &xberg_gliner::V2Tokenizer,
    splitter: &xberg_gliner::V2Splitter,
    device: &candle_core::Device,
    encoder: &crate::encoder::Encoder,
    heads: &crate::heads::AllHeads,
    text: &str,
    labels: &[String],
) -> crate::Result<(ScorerOutput, usize, V2Encoded)> {
    let encoded = encode_v2(text, labels, tokenizer, splitter)?;

    // 1. Truncate to the encoder's position-embedding limit.
    let max_seq = encoder.config.max_position_embeddings;
    let seq_len = encoded.input_ids.len().min(max_seq);
    let input_ids = Tensor::from_slice(&encoded.input_ids[..seq_len], (1, seq_len), device)?;
    let attn_data: Vec<i64> = vec![1_i64; seq_len];
    let attention_mask = Tensor::from_slice(&attn_data[..], (1, seq_len), device)?;

    // 2. Encode.
    let hidden = encoder
        .forward(&input_ids, &attention_mask, None)
        .map_err(|e| crate::GlinerCandleError::Backend(format!("[pipeline:2 encoder.forward] {e}")))?; // [1, S, H]

    // 3. Token gather. `text_positions` are already per-word token indices
    //    from `encode_v2` — filter to the truncated sequence.
    let filtered_positions: Vec<u32> = encoded
        .text_positions
        .iter()
        .filter(|&&pos| (pos as usize) < seq_len)
        .map(|&p| p as u32)
        .collect();
    let num_words = filtered_positions.len();
    if num_words == 0 {
        return Ok((empty_scorer_output(), 0, encoded));
    }
    let word_indices = Tensor::from_slice(&filtered_positions[..], (num_words,), device)?;
    let text_emb = TokenGather
        .forward(&hidden, &word_indices)
        .map_err(|e| crate::GlinerCandleError::Backend(format!("[pipeline:3 token_gather] {e}")))?; // [1, num_words, H]

    // 4. Span rep.
    let span_idx_arr = build_span_idx(num_words)?;
    // index_select requires U32 indices on the CPU backend.
    let span_idx_data: Vec<u32> = span_idx_arr.iter().map(|&v| v as u32).collect();
    let span_idx = Tensor::from_slice(&span_idx_data[..], (1, num_words * MAX_WIDTH, 2), device)?;
    let span_rep_out = heads
        .span_rep
        .forward(&text_emb, &span_idx)
        .map_err(|e| crate::GlinerCandleError::Backend(format!("[pipeline:4 span_rep] {e}")))?; // [1, num_words, MAX_WIDTH, H]

    // 5. Schema gather: `[P]` index first, then per-label `[E]` indices —
    //    exactly `encoded.schema_positions`' order.
    if encoded.schema_positions.is_empty() {
        return Err(crate::GlinerCandleError::Backend(
            "schema_positions empty — encode_v2 must emit at least the [P] marker".to_string(),
        ));
    }
    let schema_idx: Vec<u32> = encoded.schema_positions.iter().map(|&p| p as u32).collect();
    let schema_idx_t = Tensor::from_slice(&schema_idx[..], (schema_idx.len(),), device)?;
    let sg_out = SchemaGather
        .forward(&hidden, &schema_idx_t)
        .map_err(|e| crate::GlinerCandleError::Backend(format!("[pipeline:5 schema_gather] {e}")))?;

    // 6. Count pred.
    let pred_count = heads
        .count_pred
        .forward(&sg_out.pc_emb)
        .map_err(|e| crate::GlinerCandleError::Backend(format!("[pipeline:6 count_pred] {e}")))?;
    if pred_count == 0 {
        return Ok((empty_scorer_output(), 0, encoded));
    }

    // 7. Count LSTM (GRU): struct_proj [pred_count, F, H].
    let struct_proj = heads
        .count_lstm
        .forward(&sg_out.field_embs, pred_count, device)
        .map_err(|e| crate::GlinerCandleError::Backend(format!("[pipeline:7 count_lstm] {e}")))?;

    // 8. Scorer: [pred_count, F, num_words, MAX_WIDTH] sigmoid scores.
    let span_rep_per_sample = span_rep_out.squeeze(0)?;
    let scores = Scorer
        .forward(&span_rep_per_sample, &struct_proj)
        .map_err(|e| crate::GlinerCandleError::Backend(format!("[pipeline:8 scorer] {e}")))?;

    // 9. Permute to [pred_count, num_words, MAX_WIDTH, num_labels], pad to MAX_COUNT.
    let scores = scores.permute((0, 2, 3, 1))?.contiguous()?;
    let num_labels = labels.len();
    let scores_padded: Tensor = if pred_count < MAX_COUNT {
        let pad_shape = (MAX_COUNT - pred_count, num_words, MAX_WIDTH, num_labels);
        let pad = Tensor::zeros(pad_shape, scores.dtype(), device)?;
        Tensor::cat(&[&scores, &pad], 0)?
    } else {
        scores
    };

    // 10. Read back to host as Array4<f32>. Scores may be F16 (encoder/heads
    // run at F16 on wasm32 to fit weights in linear memory) -- to_vec1::<f32>
    // requires the tensor's actual dtype to already be F32, so cast first.
    let scores_vec: Vec<f32> = scores_padded
        .to_dtype(candle_core::DType::F32)?
        .flatten_all()?
        .to_vec1::<f32>()?;
    let scores_arr = Array4::from_shape_vec((MAX_COUNT, num_words, MAX_WIDTH, num_labels), scores_vec)
        .map_err(|e| crate::GlinerCandleError::Backend(format!("scores reshape: {e}")))?;

    Ok((ScorerOutput { scores: scores_arr }, pred_count, encoded))
}

fn empty_scorer_output() -> ScorerOutput {
    ScorerOutput {
        scores: Array4::zeros((0, 0, 0, 0)),
    }
}

#[cfg(test)]
mod tests {
    #[test]
    fn run_pipeline_is_declared() {
        #[allow(clippy::type_complexity)]
        fn _assert_signature(
            f: fn(
                &xberg_gliner::V2Tokenizer,
                &xberg_gliner::V2Splitter,
                &candle_core::Device,
                &crate::encoder::Encoder,
                &crate::heads::AllHeads,
                &str,
                &[String],
            ) -> crate::Result<(crate::decode::ScorerOutput, usize, xberg_gliner::V2Encoded)>,
        ) {
            let _ = f;
        }
        _assert_signature(super::run_pipeline);
    }
}
