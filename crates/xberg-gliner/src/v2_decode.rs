use ndarray::{ArrayViewD, Ix4};

use crate::decode::{Span, SpanOutput, greedy_search};
use crate::{GlinerError, Result, Token};

/// Decode GLiNER2's `span_scores` output `(1, num_labels, num_words, max_width)`
/// into entity spans. Unlike GLiNER1's `logits`, `span_scores` values are already
/// post-sigmoid probabilities — do not apply `sigmoid()` here.
#[allow(clippy::too_many_arguments)]
pub(crate) fn decode_span_scores(
    span_scores: ArrayViewD<'_, f32>,
    text: &str,
    words: &[Token],
    labels: &[String],
    threshold: f32,
    max_width: usize,
    flat_ner: bool,
    dup_label: bool,
    multi_label: bool,
) -> Result<SpanOutput> {
    let num_words = words.len();
    let expected_shape = vec![1, labels.len(), num_words, max_width];
    let actual_shape = span_scores.shape().to_vec();
    if actual_shape != expected_shape {
        return Err(GlinerError::UnexpectedLogitsShape {
            expected: expected_shape,
            actual: actual_shape,
        });
    }

    let span_scores = span_scores
        .into_dimensionality::<Ix4>()
        .map_err(|_| GlinerError::UnexpectedLogitsShape {
            expected: expected_shape.clone(),
            actual: actual_shape,
        })?;

    let mut spans = Vec::new();
    for (label_index, label) in labels.iter().enumerate() {
        for start in 0..num_words {
            for width_index in 0..max_width {
                let end = start + width_index;
                if end >= num_words {
                    continue;
                }
                let probability = span_scores[[0, label_index, start, width_index]];
                if probability >= threshold {
                    let start_token = &words[start];
                    let end_token = &words[end];
                    let source = text
                        .get(start_token.start()..end_token.end())
                        .ok_or(GlinerError::InvalidOffsets {
                            start: start_token.start(),
                            end: end_token.end(),
                        })?
                        .to_string();
                    spans.push(Span::new(
                        0,
                        start_token.start(),
                        end_token.end(),
                        source,
                        label.clone(),
                        probability,
                    )?);
                }
            }
        }
    }

    spans.sort_unstable_by_key(Span::offsets);
    let resolved = greedy_search(&spans, flat_ner, dup_label, multi_label);

    Ok(SpanOutput::new(vec![text.to_string()], labels.to_vec(), vec![resolved]))
}

#[cfg(test)]
mod tests {
    use ndarray::Array4;

    use super::*;

    #[test]
    fn decodes_spans_above_threshold_without_sigmoid() {
        let text = "Ada lives";
        let words = vec![Token::new(0, 3, "Ada"), Token::new(4, 9, "lives")];
        let labels = vec!["person".to_string()];
        // shape (1, 1 label, 2 words, max_width 2)
        let mut scores = Array4::<f32>::zeros((1, 1, 2, 2));
        scores[[0, 0, 0, 0]] = 0.9; // "Ada" alone, score already a probability
        let output = decode_span_scores(scores.into_dyn().view(), text, &words, &labels, 0.5, 2, true, false, false)
            .expect("decoded");
        assert_eq!(output.spans[0].len(), 1);
        assert_eq!(output.spans[0][0].text(), "Ada");
        assert_eq!(output.spans[0][0].probability(), 0.9);
    }

    #[test]
    fn rejects_unexpected_shape() {
        let text = "Ada";
        let words = vec![Token::new(0, 3, "Ada")];
        let labels = vec!["person".to_string()];
        let scores = Array4::<f32>::zeros((1, 1, 1, 1));
        // wrong max_width argument (2, but tensor only has width 1)
        let result = decode_span_scores(scores.into_dyn().view(), text, &words, &labels, 0.5, 2, true, false, false);
        assert!(result.is_err());
    }
}
