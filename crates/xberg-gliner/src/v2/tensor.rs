use ndarray::Array3;

use crate::config::MAX_SPANS_PER_SEQUENCE;
use crate::{GlinerError, Result};

/// Build the dense `span_idx` tensor GLiNER2 expects: every `(start, start+width-1)`
/// pair for `width` in `1..=max_width`, padded with `(0, 0)` once `start+width`
/// exceeds `num_words`. GLiNER2 has no `span_mask` input; out-of-range spans are
/// filtered during decode instead (by checking `end >= num_words`).
pub(crate) fn build_span_idx(num_words: usize, max_width: usize) -> Result<Array3<i64>> {
    let num_spans = num_words.checked_mul(max_width).ok_or_else(|| {
        GlinerError::InvalidInput(format!(
            "span tensor size overflow for {num_words} words and width {max_width}"
        ))
    })?;
    if num_spans > MAX_SPANS_PER_SEQUENCE {
        return Err(GlinerError::InvalidInput(format!(
            "span count must be at most {MAX_SPANS_PER_SEQUENCE}, got {num_spans}"
        )));
    }

    let mut span_idx = Array3::<i64>::zeros((1, num_spans.max(1), 2));
    for start in 0..num_words {
        for width in 1..=max_width {
            let dimension = start * max_width + (width - 1);
            let end = start + width;
            if end <= num_words {
                span_idx[[0, dimension, 0]] = start as i64;
                span_idx[[0, dimension, 1]] = (end - 1) as i64;
            }
        }
    }
    Ok(span_idx)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn builds_dense_span_pairs_with_zero_padding() {
        // 2 words, max_width 2 -> spans: (0,0) (0,1) (1,1) (0,0)-padding ~keep
        let span_idx = build_span_idx(2, 2).expect("span idx");
        assert_eq!(span_idx.shape(), &[1, 4, 2]);
        assert_eq!((span_idx[[0, 0, 0]], span_idx[[0, 0, 1]]), (0, 0));
        assert_eq!((span_idx[[0, 1, 0]], span_idx[[0, 1, 1]]), (0, 1));
        assert_eq!((span_idx[[0, 2, 0]], span_idx[[0, 2, 1]]), (1, 1));
        assert_eq!((span_idx[[0, 3, 0]], span_idx[[0, 3, 1]]), (0, 0));
    }

    #[test]
    fn rejects_span_count_overflow() {
        assert!(build_span_idx(usize::MAX, 2).is_err());
    }

    #[test]
    fn rejects_span_count_above_limit() {
        assert!(build_span_idx(MAX_SPANS_PER_SEQUENCE + 1, 1).is_err());
    }
}
