use ndarray::{Array2, Array3};

use crate::config::MAX_SPANS_PER_SEQUENCE;
use crate::{EncodedInput, EntityContext, GlinerError, Result};

pub(crate) struct SpanTensors {
    pub(crate) input_ids: Array2<i64>,
    pub(crate) attention_masks: Array2<i64>,
    pub(crate) word_masks: Array2<i64>,
    pub(crate) text_lengths: Array2<i64>,
    pub(crate) span_idx: Array3<i64>,
    pub(crate) span_mask: Array2<bool>,
    pub(crate) context: EntityContext,
}

impl SpanTensors {
    pub(crate) fn from(encoded: EncodedInput, max_width: usize) -> Result<Self> {
        let (span_idx, span_mask) = make_span_tensors(&encoded, max_width)?;
        Ok(Self {
            input_ids: encoded.input_ids,
            attention_masks: encoded.attention_masks,
            word_masks: encoded.word_masks,
            text_lengths: encoded.text_lengths,
            span_idx,
            span_mask,
            context: EntityContext {
                texts: encoded.texts,
                tokens: encoded.tokens,
                entities: encoded.entities,
                num_words: encoded.num_words,
            },
        })
    }
}

pub(crate) fn make_span_tensors(encoded: &EncodedInput, max_width: usize) -> Result<(Array3<i64>, Array2<bool>)> {
    let num_spans = encoded.num_words.checked_mul(max_width).ok_or_else(|| {
        GlinerError::InvalidInput(format!(
            "span tensor size overflow for {} words and width {max_width}",
            encoded.num_words
        ))
    })?;
    if num_spans > MAX_SPANS_PER_SEQUENCE {
        return Err(GlinerError::InvalidInput(format!(
            "span count must be at most {MAX_SPANS_PER_SEQUENCE}, got {num_spans}"
        )));
    }
    let mut span_idx = Array3::<i64>::zeros((encoded.texts.len(), num_spans, 2));
    let mut span_mask = Array2::<bool>::from_elem((encoded.texts.len(), num_spans), false);

    for sequence in 0..encoded.texts.len() {
        let text_width = encoded.text_lengths[[sequence, 0]] as usize;
        for start in 0..text_width {
            let actual_max_width = max_width.min(text_width - start);
            for width in 0..actual_max_width {
                let dimension = start * max_width + width;
                span_idx[[sequence, dimension, 0]] = start as i64;
                span_idx[[sequence, dimension, 1]] = (start + width) as i64;
                span_mask[[sequence, dimension]] = true;
            }
        }
    }

    Ok((span_idx, span_mask))
}
