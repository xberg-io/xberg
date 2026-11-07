use super::LocalClassificationConfig;
use super::policy::{NLI_CLASS_COUNT, invalid, validate_nli_indices};

pub(super) fn decode_logits(
    labels: &[String],
    logits: &ndarray::ArrayD<f32>,
    config: &LocalClassificationConfig,
) -> crate::Result<Vec<crate::types::ClassificationLabel>> {
    validate_nli_indices(&config.model)?;
    if labels.is_empty()
        || logits.shape() != [labels.len(), NLI_CLASS_COUNT]
        || logits.iter().any(|value| !value.is_finite())
    {
        return Err(invalid(
            "Local classification logits must be finite [label_count, 3] NLI scores",
        ));
    }
    let rows = logits
        .view()
        .into_dimensionality::<ndarray::Ix2>()
        .map_err(|error| invalid(format!("Local classification logits shape: {error}")))?;
    let entailment = config.model.entailment_index as usize;
    let contradiction = config.model.contradiction_index as usize;
    let probabilities = if config.multi_label || labels.len() == 1 {
        rows.outer_iter()
            .map(|row| {
                let difference = f64::from(row[contradiction]) - f64::from(row[entailment]);
                (1.0 / (1.0 + difference.exp())) as f32
            })
            .collect::<Vec<_>>()
    } else {
        let maximum = rows
            .column(entailment)
            .iter()
            .copied()
            .fold(f32::NEG_INFINITY, f32::max);
        let weights = rows
            .column(entailment)
            .iter()
            .map(|value| (f64::from(*value) - f64::from(maximum)).exp())
            .collect::<Vec<_>>();
        let total = weights.iter().sum::<f64>();
        weights.into_iter().map(|weight| (weight / total) as f32).collect()
    };
    let best = probabilities.iter().enumerate().fold(
        0,
        |best, (index, score)| {
            if *score > probabilities[best] { index } else { best }
        },
    );
    Ok(labels
        .iter()
        .zip(probabilities)
        .enumerate()
        .filter(|(index, (_, score))| *score >= config.confidence_threshold && (config.multi_label || *index == best))
        .map(|(_, (label, score))| crate::types::ClassificationLabel {
            label: label.clone(),
            confidence: Some(score),
        })
        .collect())
}
