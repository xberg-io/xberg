use ndarray::{ArrayViewD, Ix4};

use crate::{GlinerError, Result, Token};

/// A decoded entity span.
#[derive(Debug, Clone, PartialEq)]
pub struct Span {
    sequence: usize,
    start: usize,
    end: usize,
    text: String,
    class: String,
    probability: f32,
}

impl Span {
    pub fn new(
        sequence: usize,
        start: usize,
        end: usize,
        text: String,
        class: String,
        probability: f32,
    ) -> Result<Self> {
        if end <= start {
            return Err(GlinerError::InvalidOffsets { start, end });
        }
        Ok(Self {
            sequence,
            start,
            end,
            text,
            class,
            probability,
        })
    }

    /// Input sequence index in the batch.
    pub fn sequence(&self) -> usize {
        self.sequence
    }

    /// Start and end byte offsets in the source text.
    pub fn offsets(&self) -> (usize, usize) {
        (self.start, self.end)
    }

    /// Matched entity text.
    pub fn text(&self) -> &str {
        &self.text
    }

    /// Entity label.
    pub fn class(&self) -> &str {
        &self.class
    }

    /// Entity probability.
    pub fn probability(&self) -> f32 {
        self.probability
    }

    fn overlaps(&self, other: &Span) -> bool {
        self.start < other.end && other.start < self.end
    }

    fn is_disjoint(&self, other: &Span) -> bool {
        !self.overlaps(other)
    }
}

/// Final GLiNER span-mode output.
#[derive(Debug, Clone)]
pub struct SpanOutput {
    /// Original text batch.
    pub texts: Vec<String>,
    /// Entity labels used for inference.
    pub entities: Vec<String>,
    /// Decoded spans per input sequence.
    pub spans: Vec<Vec<Span>>,
}

impl SpanOutput {
    pub(crate) fn new(texts: Vec<String>, entities: Vec<String>, spans: Vec<Vec<Span>>) -> Self {
        Self { texts, entities, spans }
    }
}

impl std::fmt::Display for SpanOutput {
    fn fmt(&self, formatter: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        for spans in &self.spans {
            for span in spans {
                writeln!(
                    formatter,
                    "{:3} | {:15} | {:10} | {:.1}%",
                    span.sequence(),
                    span.text(),
                    span.class(),
                    span.probability() * 100.0,
                )?;
            }
        }
        Ok(())
    }
}

pub(crate) struct EntityContext {
    pub(crate) texts: Vec<String>,
    pub(crate) tokens: Vec<Vec<Token>>,
    pub(crate) entities: Vec<String>,
    pub(crate) num_words: usize,
}

impl EntityContext {
    fn create_span(
        &self,
        sequence_id: usize,
        start_token: usize,
        end_token: usize,
        class: usize,
        probability: f32,
    ) -> Result<Span> {
        let sequence = self.tokens.get(sequence_id).ok_or(GlinerError::Index {
            target: "tokens",
            index: sequence_id,
        })?;
        let start_token = sequence.get(start_token).ok_or(GlinerError::Index {
            target: "tokens[]",
            index: start_token,
        })?;
        let end_token = sequence.get(end_token).ok_or(GlinerError::Index {
            target: "tokens[]",
            index: end_token,
        })?;
        let text = self.texts.get(sequence_id).ok_or(GlinerError::Index {
            target: "texts",
            index: sequence_id,
        })?;
        let source = text
            .get(start_token.start()..end_token.end())
            .ok_or(GlinerError::InvalidOffsets {
                start: start_token.start(),
                end: end_token.end(),
            })?
            .to_string();
        let class = self
            .entities
            .get(class)
            .ok_or(GlinerError::Index {
                target: "entities",
                index: class,
            })?
            .to_string();
        Span::new(
            sequence_id,
            start_token.start(),
            end_token.end(),
            source,
            class,
            probability,
        )
    }
}

pub(crate) fn decode_logits(
    logits: ArrayViewD<'_, f32>,
    context: EntityContext,
    threshold: f32,
    max_width: usize,
    flat_ner: bool,
    dup_label: bool,
    multi_label: bool,
) -> Result<SpanOutput> {
    let expected_shape = vec![
        context.texts.len(),
        context.num_words,
        max_width,
        context.entities.len(),
    ];
    let actual_shape = logits.shape().to_vec();
    if actual_shape != expected_shape {
        return Err(GlinerError::UnexpectedLogitsShape {
            expected: expected_shape,
            actual: actual_shape,
        });
    }

    let logits = logits
        .into_dimensionality::<Ix4>()
        .map_err(|_| GlinerError::UnexpectedLogitsShape {
            expected: expected_shape,
            actual: actual_shape,
        })?;
    let mut decoded = Vec::with_capacity(context.texts.len());

    for sequence_id in 0..context.texts.len() {
        let sequence = logits.slice(ndarray::s![sequence_id, .., .., ..]);
        let num_tokens = context
            .tokens
            .get(sequence_id)
            .ok_or(GlinerError::Index {
                target: "tokens",
                index: sequence_id,
            })?
            .len();
        let mut spans = Vec::new();

        for ((start, width, class), score) in sequence.indexed_iter() {
            if start >= num_tokens || start + width >= num_tokens {
                continue;
            }
            let probability = sigmoid(*score);
            if probability >= threshold {
                spans.push(context.create_span(sequence_id, start, start + width, class, probability)?);
            }
        }

        spans.sort_unstable_by_key(Span::offsets);
        decoded.push(greedy_search(&spans, flat_ner, dup_label, multi_label));
    }

    Ok(SpanOutput::new(context.texts, context.entities, decoded))
}

pub fn greedy_search(spans: &[Span], flat_ner: bool, dup_label: bool, multi_label: bool) -> Vec<Span> {
    if spans.is_empty() {
        return Vec::new();
    }

    let mut result = Vec::with_capacity(spans.len());
    let mut previous = 0usize;
    let mut next = 1usize;

    while next < spans.len() {
        let previous_span = &spans[previous];
        let next_span = &spans[next];
        if accept_span(previous_span, next_span, flat_ner, dup_label, multi_label) {
            result.push(previous_span.clone());
            previous = next;
        } else if previous_span.probability() < next_span.probability() {
            previous = next;
        }
        next += 1;
    }

    result.push(spans[previous].clone());
    result
}

fn accept_span(first: &Span, second: &Span, flat_ner: bool, dup_label: bool, multi_label: bool) -> bool {
    if first.is_disjoint(second) {
        true
    } else if flat_ner {
        false
    } else if first.class() == second.class() {
        dup_label
    } else {
        multi_label
    }
}

fn sigmoid(value: f32) -> f32 {
    1.0 / (1.0 + (-value).exp())
}
