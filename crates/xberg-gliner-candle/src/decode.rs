//! Span-index construction and score decoding for the Candle GLiNER2
//! pipeline. The numeric decode loop is adapted from
//! `anno::backends::gliner2_fastino::pipeline::decode_entities_with_thresholds`
//! to target `xberg_gliner::Span` (byte offsets) instead of anno's
//! char-offset `Entity` type — no offset-unit conversion needed here.

use ndarray::Array3;
pub(crate) use ndarray::Array4;

use xberg_gliner::{Span, Token, decode::greedy_search};

pub(crate) use crate::heads::MAX_WIDTH;
pub(crate) use crate::heads::count_lstm::MAX_COUNT;

/// Per-instance per-span per-label entity scores. Shape
/// `[MAX_COUNT, num_words, MAX_WIDTH, num_labels]`. Already-sigmoided.
pub(crate) struct ScorerOutput {
    pub scores: Array4<f32>,
}

/// Build the span-index tensor consumed by `heads::span_rep::SpanRep::forward`.
///
/// For each `(start_word, width_idx)` pair where `width_idx ∈ 0..MAX_WIDTH`,
/// emits `(start, start + width_idx)`. Out-of-range pairs (`end >= num_words`)
/// are zero-padded.
pub(crate) fn build_span_idx(num_words: usize) -> Array3<i64> {
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
    Array3::from_shape_vec((1, num_spans, 2), data).expect("span_idx shape consistent by construction")
}

/// Decode the scorer's `[MAX_COUNT, num_words, MAX_WIDTH, num_labels]` tensor
/// into a [`xberg_gliner::SpanOutput`], applying a single global `threshold`,
/// then greedy-merging overlaps via `xberg_gliner::decode::greedy_search`.
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
) -> crate::Result<xberg_gliner::SpanOutput> {
    let num_words = words.len();
    let num_labels = labels.len();
    let scores = &scorer_out.scores;

    let mut candidates: Vec<Span> = Vec::new();
    for c_idx in 0..pred_count.min(MAX_COUNT) {
        for start in 0..num_words {
            for width_idx in 0..MAX_WIDTH {
                let end_word = (start + width_idx + 1).min(num_words);
                for m in 0..num_labels {
                    let prob = scores[[c_idx, start, width_idx, m]];
                    if prob <= threshold {
                        continue;
                    }
                    let byte_start = words[start].start();
                    let byte_end = words[end_word - 1].end();
                    if byte_end > text.len() || byte_start >= byte_end {
                        continue;
                    }
                    let surface = text[byte_start..byte_end].trim();
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

    Ok(xberg_gliner::SpanOutput {
        texts: vec![text.to_string()],
        entities: labels.to_vec(),
        spans: vec![spans],
    })
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn build_span_idx_zero_pads_overflow() {
        let idx = build_span_idx(2);
        assert_eq!(idx.shape(), &[1, 2 * MAX_WIDTH, 2]);
        assert_eq!((idx[[0, 0, 0]], idx[[0, 0, 1]]), (0, 0));
        assert_eq!((idx[[0, 1, 0]], idx[[0, 1, 1]]), (0, 1));
        assert_eq!((idx[[0, 2, 0]], idx[[0, 2, 1]]), (0, 0)); // overflow → (0,0)
    }

    #[test]
    fn decode_span_scores_drops_below_threshold_candidates() {
        use xberg_gliner::Token;
        let text = "Ada Lovelace";
        let words = vec![Token::new(0, 3, "ada"), Token::new(4, 12, "lovelace")];
        let labels = vec!["person".to_string()];
        let scores = ndarray::Array4::<f32>::zeros((MAX_COUNT, 2, MAX_WIDTH, 1));
        let out = decode_span_scores(text, &words, &labels, &ScorerOutput { scores }, 1, 0.5, true, false, false)
            .expect("decode must not error on all-below-threshold scores");
        assert!(out.spans[0].is_empty());
    }
}
