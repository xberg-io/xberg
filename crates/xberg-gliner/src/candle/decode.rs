//! Span-index construction and score decoding for the Candle GLiNER2
//! pipeline. The numeric decode loop is adapted from
//! `anno::backends::gliner2_fastino::pipeline::decode_entities_with_thresholds`
//! to target `crate::Span` (byte offsets) instead of anno's
//! char-offset `Entity` type; no offset-unit conversion needed here.

use ndarray::Array3;
pub(crate) use ndarray::Array4;

use crate::{Span, Token, decode::greedy_search};

pub(crate) use crate::candle::heads::MAX_WIDTH;
pub(crate) use crate::candle::heads::count_lstm::MAX_COUNT;

/// Per-instance per-span per-label entity scores. Shape
/// `[MAX_COUNT, num_words, MAX_WIDTH, num_labels]`. Already-sigmoided.
pub(crate) struct ScorerOutput {
    pub scores: Array4<f32>,
}

/// Build the span-index tensor consumed by `heads::span_rep::SpanRep::forward`.
///
/// For each `(start_word, width_idx)` pair where `width_idx ∈ 0..MAX_WIDTH`,
/// emits `(start, start + width_idx)`. Out-of-range pairs (`end >= num_words`)
/// are zero-padded; those slots carry score `0.0` after the heads' forward
/// pass and are always skipped by `decode_span_scores`.
pub(crate) fn build_span_idx(num_words: usize) -> crate::candle::Result<Array3<i64>> {
    let num_spans = num_words * MAX_WIDTH;
    let mut data = Vec::with_capacity(num_spans * 2);
    for start in 0..num_words {
        for width in 0..MAX_WIDTH {
            let end = start + width;
            if end >= num_words {
                data.extend_from_slice(&[0_i64, 0_i64]);
            } else {
                data.push(start as i64);
                data.push(end as i64);
            }
        }
    }
    Array3::from_shape_vec((1, num_spans, 2), data)
        .map_err(|e| crate::candle::GlinerCandleError::Backend(format!("build_span_idx shape: {e}")))
}

/// Decode the scorer's `[MAX_COUNT, num_words, MAX_WIDTH, num_labels]` tensor
/// into a [`crate::SpanOutput`], applying a single global `threshold`,
/// then greedy-merging overlaps via `crate::decode::greedy_search`.
#[allow(clippy::too_many_arguments)]
pub(crate) fn decode_span_scores(
    text: &str,
    words: &[Token],
    labels: &[String],
    scorer_out: &ScorerOutput,
    pred_count: usize,
    threshold: f32,
    flat_ner: bool,
    dup_label: bool,
    multi_label: bool,
) -> crate::candle::Result<crate::SpanOutput> {
    let scores = &scorer_out.scores;
    // Bound by the scores tensor's word dimension, not `words.len()`: when the
    // input exceeds the encoder's position-embedding limit, `run_pipeline`
    // truncates and the scores only cover the surviving words. Indexing by the
    // full word list would walk off the array.
    let num_words = words.len().min(scores.shape()[1]);
    let num_labels = labels.len();

    let mut candidates: Vec<Span> = Vec::new();
    for c_idx in 0..pred_count.min(MAX_COUNT) {
        for start in 0..num_words {
            for width_idx in 0..MAX_WIDTH {
                // Skip slots where the span-index entry was zero-padded.
                let end_idx = start + width_idx;
                if end_idx >= num_words {
                    continue;
                }
                let end_word = end_idx + 1; // exclusive
                for m in 0..num_labels {
                    let prob = scores[[c_idx, start, width_idx, m]];
                    if prob <= threshold {
                        continue;
                    }
                    let byte_start = words[start].start();
                    let byte_end = words[end_word - 1].end();
                    if byte_start >= byte_end {
                        continue;
                    }
                    // .get() rather than indexing: token offsets come from the
                    // lowercased copy (see V2Splitter), and a lowercase form
                    // that changes byte length can land these offsets out of
                    // bounds or mid-character in the original text. Skip such
                    // candidates instead of panicking, matching v2_decode's
                    // defensive handling on the ONNX path.
                    let Some(raw) = text.get(byte_start..byte_end) else {
                        continue;
                    };
                    let surface = raw.trim();
                    if surface.is_empty() {
                        continue;
                    }
                    candidates.push(Span::new(
                        0,
                        byte_start,
                        byte_end,
                        surface.to_string(),
                        labels[m].clone(),
                        prob,
                    )?);
                }
            }
        }
    }

    candidates.sort_unstable_by_key(Span::offsets);
    let spans = greedy_search(&candidates, flat_ner, dup_label, multi_label);

    Ok(crate::SpanOutput {
        texts: vec![text.to_string()],
        entities: labels.to_vec(),
        spans: vec![spans],
    })
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn decode_span_scores_skips_non_char_boundary_offsets() {
        use crate::Token;
        // "\u{130}a": U+0130 is two bytes, so offset 1 is mid-character. A
        // token carrying such offsets (possible when the lowercased copy is
        // longer than the original) must be skipped, not panic.
        let text = "\u{130}a";
        let words = vec![Token::new(1, 3, "ia")];
        let labels = vec!["person".to_string()];
        let mut scores = ndarray::Array4::<f32>::zeros((MAX_COUNT, 1, MAX_WIDTH, 1));
        scores[[0, 0, 0, 0]] = 0.9;
        let out = decode_span_scores(
            text,
            &words,
            &labels,
            &ScorerOutput { scores },
            1,
            0.5,
            true,
            false,
            false,
        )
        .expect("misaligned offsets must be skipped, not panic");
        assert!(out.spans[0].is_empty());
    }

    #[test]
    fn decode_span_scores_bounds_by_scores_dim_after_truncation() {
        use crate::Token;
        // Three words, but the scores tensor only covers two (the pipeline
        // truncated the third at the position-embedding limit). Decoding must
        // stay inside the tensor instead of panicking on the third word.
        let text = "one two three";
        let words = vec![Token::new(0, 3, "one"), Token::new(4, 7, "two"), Token::new(8, 13, "three")];
        let labels = vec!["person".to_string()];
        let mut scores = ndarray::Array4::<f32>::zeros((MAX_COUNT, 2, MAX_WIDTH, 1));
        scores[[0, 1, 0, 0]] = 0.9;
        let out = decode_span_scores(
            text,
            &words,
            &labels,
            &ScorerOutput { scores },
            1,
            0.5,
            true,
            false,
            false,
        )
        .expect("truncated scores must bound the decode loop");
        assert_eq!(out.spans[0].len(), 1);
        assert_eq!(out.spans[0][0].text(), "two");
    }

    #[test]
    fn build_span_idx_zero_pads_overflow() {
        let idx = build_span_idx(2).expect("build_span_idx must not fail");
        assert_eq!(idx.shape(), &[1, 2 * MAX_WIDTH, 2]);
        // start=0, width=0: end=0 < 2 → valid (0,0)
        assert_eq!((idx[[0, 0, 0]], idx[[0, 0, 1]]), (0, 0));
        // start=0, width=1: end=1 < 2 → valid (0,1)
        assert_eq!((idx[[0, 1, 0]], idx[[0, 1, 1]]), (0, 1));
        // start=0, width=2: end=2 >= 2 → zero-padded (0,0)
        assert_eq!((idx[[0, 2, 0]], idx[[0, 2, 1]]), (0, 0));
        // start=1, width=0: end=1 < 2 → valid (1,1); second word
        assert_eq!((idx[[0, MAX_WIDTH, 0]], idx[[0, MAX_WIDTH, 1]]), (1, 1));
    }

    #[test]
    fn decode_span_scores_drops_below_threshold_candidates() {
        use crate::Token;
        let text = "Ada Lovelace";
        let words = vec![Token::new(0, 3, "ada"), Token::new(4, 12, "lovelace")];
        let labels = vec!["person".to_string()];
        let scores = ndarray::Array4::<f32>::zeros((MAX_COUNT, 2, MAX_WIDTH, 1));
        let out = decode_span_scores(
            text,
            &words,
            &labels,
            &ScorerOutput { scores },
            1,
            0.5,
            true,
            false,
            false,
        )
        .expect("decode must not error on all-below-threshold scores");
        assert!(out.spans[0].is_empty());
    }
}
