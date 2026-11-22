//! Metric definitions. Every score is paired with the count it was computed over, and a score
//! that has no observations reports `unmeasured` with a reason instead of a zero.

use serde::{Deserialize, Serialize};
use std::collections::{BTreeMap, BTreeSet};

/// A number with its denominator. `value: None` and a reason means the run could not measure it;
/// that is different from measuring zero and must not be rendered as one.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct Estimate {
    pub value: Option<f64>,
    pub observations: usize,
    pub reason: Option<String>,
}

impl Estimate {
    pub fn measured(value: f64, observations: usize) -> Self {
        debug_assert!(value.is_finite(), "metric must be finite");
        Self {
            value: Some(value),
            observations,
            reason: None,
        }
    }

    pub fn unmeasured(reason: impl Into<String>) -> Self {
        Self {
            value: None,
            observations: 0,
            reason: Some(reason.into()),
        }
    }

    pub fn is_measured(&self) -> bool {
        self.value.is_some()
    }
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct Interval {
    pub lower: f64,
    pub upper: f64,
    pub level: f64,
    pub replicates: usize,
    pub clusters: usize,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct ClassScore {
    pub support: usize,
    pub recall: Estimate,
    pub precision: Estimate,
    pub f1: Option<f64>,
}

#[derive(Debug, Clone, PartialEq, Eq, PartialOrd, Ord, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct ConfusionCell {
    pub gold: Option<String>,
    pub predicted: Option<String>,
    pub count: usize,
}

/// One repeat's held-out classification score.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum AdverseKind {
    Misclassified,
    Abstained,
    UnknownForced,
}

/// A concrete failure a reader can inspect. Examples are capped and chosen deterministically so a
/// report cannot be tuned by example selection.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct AdverseExample {
    pub kind: AdverseKind,
    pub document: String,
    pub family: String,
    pub gold: Vec<String>,
    pub predicted: Option<String>,
    pub confidence: Option<f64>,
}

const MAX_ADVERSE_EXAMPLES: usize = 12;

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct RepeatScore {
    pub repeat: u32,
    pub known_documents: usize,
    pub unknown_documents: usize,
    pub abstentions_on_known: usize,
    pub errors: usize,
    pub macro_f1: Estimate,
    pub micro_accuracy: Estimate,
    pub selective_accuracy: Estimate,
    pub abstention_rate_on_known: Estimate,
    pub unknown_rejection: Estimate,
    pub unknown_forced_prediction: Estimate,
    pub ece: Estimate,
    pub per_class: BTreeMap<String, ClassScore>,
    pub confusion: Vec<ConfusionCell>,
    pub cue_strata: BTreeMap<String, Estimate>,
    pub ci: Option<Interval>,
    pub adverse_examples: Vec<AdverseExample>,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct ClassificationScore {
    pub headline_repeat: u32,
    pub repeat_stability: Option<f64>,
    pub repeats: Vec<RepeatScore>,
    pub baseline: BaselineScore,
    pub limitations: Vec<String>,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct BaselineScore {
    pub rule: String,
    pub predicted_class: Option<String>,
    pub macro_f1: Estimate,
    pub micro_accuracy: Estimate,
}

/// One scored document. `gold` is empty for an unknown-class example.
#[derive(Debug, Clone, PartialEq)]
pub struct Judgement {
    pub id: String,
    pub family: String,
    pub gold: Vec<String>,
    pub predicted: Option<String>,
    pub confidence: Option<f64>,
    pub cue: bool,
}

/// Score one repeat. `labels` is the frozen vocabulary and fixes the macro denominator: classes
/// with no gold support are reported but excluded from the macro average.
pub fn score_repeat(repeat: u32, labels: &[String], judgements: &[Judgement], settings: &Bootstrap) -> RepeatScore {
    let known: Vec<&Judgement> = judgements.iter().filter(|row| !row.gold.is_empty()).collect();
    let unknown: Vec<&Judgement> = judgements.iter().filter(|row| row.gold.is_empty()).collect();
    let confusion = confusion_cells(judgements);
    let (per_class, f1_values) = class_scores(labels, &known);
    let macro_f1 = if f1_values.is_empty() {
        Estimate::unmeasured("no vocabulary class has held-out support")
    } else {
        Estimate::measured(f1_values.iter().sum::<f64>() / f1_values.len() as f64, f1_values.len())
    };
    let correct = known
        .iter()
        .filter(|row| row.predicted.as_ref().is_some_and(|label| row.gold.contains(label)))
        .count();
    let answered = known.iter().filter(|row| row.predicted.is_some()).count();
    let abstentions = known.len() - answered;
    let unknown_abstentions = unknown.iter().filter(|row| row.predicted.is_none()).count();
    let ci = macro_f1
        .is_measured()
        .then(|| family_bootstrap(known.to_vec(), labels, settings))
        .flatten();

    RepeatScore {
        repeat,
        known_documents: known.len(),
        unknown_documents: unknown.len(),
        abstentions_on_known: abstentions,
        errors: known.len() - correct,
        macro_f1,
        micro_accuracy: ratio(correct, known.len(), "no held-out known-class documents"),
        selective_accuracy: ratio(correct, answered, "every known-class prediction abstained"),
        abstention_rate_on_known: ratio(abstentions, known.len(), "no held-out known-class documents"),
        unknown_rejection: ratio(
            unknown_abstentions,
            unknown.len(),
            "no held-out unknown-class documents",
        ),
        unknown_forced_prediction: ratio(
            unknown.len() - unknown_abstentions,
            unknown.len(),
            "no held-out unknown-class documents",
        ),
        ece: expected_calibration_error(&known),
        per_class,
        confusion,
        cue_strata: cue_strata(&known, labels),
        ci,
        adverse_examples: adverse_examples(&known, &unknown),
    }
}

fn confusion_cells(judgements: &[Judgement]) -> Vec<ConfusionCell> {
    let mut confusion: BTreeMap<(Option<String>, Option<String>), usize> = BTreeMap::new();
    for row in judgements {
        *confusion
            .entry((row.gold.first().cloned(), row.predicted.clone()))
            .or_default() += 1;
    }
    confusion
        .into_iter()
        .map(|((gold, predicted), count)| ConfusionCell { gold, predicted, count })
        .collect()
}

fn class_scores(labels: &[String], known: &[&Judgement]) -> (BTreeMap<String, ClassScore>, Vec<f64>) {
    let mut per_class = BTreeMap::new();
    let mut f1_values = Vec::new();
    for label in labels {
        let support = known.iter().filter(|row| row.gold.contains(label)).count();
        let predicted = known
            .iter()
            .filter(|row| row.predicted.as_deref() == Some(label))
            .count();
        let true_positive = known
            .iter()
            .filter(|row| row.gold.contains(label) && row.predicted.as_deref() == Some(label))
            .count();
        if support == 0 {
            per_class.insert(
                label.clone(),
                ClassScore {
                    support: 0,
                    recall: Estimate::unmeasured("no held-out document has this gold label"),
                    precision: Estimate::unmeasured("no held-out document has this gold label"),
                    f1: None,
                },
            );
            continue;
        }
        let recall = true_positive as f64 / support as f64;
        let precision = if predicted == 0 {
            Estimate::measured(0.0, 0)
        } else {
            Estimate::measured(true_positive as f64 / predicted as f64, predicted)
        };
        let f1 = if true_positive == 0 {
            0.0
        } else {
            let precision_value = precision.value.unwrap_or(0.0);
            2.0 * precision_value * recall / (precision_value + recall)
        };
        f1_values.push(f1);
        per_class.insert(
            label.clone(),
            ClassScore {
                support,
                recall: Estimate::measured(recall, support),
                precision,
                f1: Some(f1),
            },
        );
    }
    (per_class, f1_values)
}

fn adverse_examples(known: &[&Judgement], unknown: &[&Judgement]) -> Vec<AdverseExample> {
    let mut examples = Vec::new();
    for row in known {
        let correct = row.predicted.as_ref().is_some_and(|label| row.gold.contains(label));
        if correct {
            continue;
        }
        examples.push(AdverseExample {
            kind: if row.predicted.is_some() {
                AdverseKind::Misclassified
            } else {
                AdverseKind::Abstained
            },
            document: row.id.clone(),
            family: row.family.clone(),
            gold: row.gold.clone(),
            predicted: row.predicted.clone(),
            confidence: row.confidence,
        });
    }
    for row in unknown {
        if row.predicted.is_none() {
            continue;
        }
        examples.push(AdverseExample {
            kind: AdverseKind::UnknownForced,
            document: row.id.clone(),
            family: row.family.clone(),
            gold: Vec::new(),
            predicted: row.predicted.clone(),
            confidence: row.confidence,
        });
    }
    examples.sort_by(|left, right| (left.kind as u8, &left.document).cmp(&(right.kind as u8, &right.document)));
    examples.truncate(MAX_ADVERSE_EXAMPLES);
    examples
}

fn ratio(numerator: usize, denominator: usize, reason: &str) -> Estimate {
    if denominator == 0 {
        Estimate::unmeasured(reason)
    } else {
        Estimate::measured(numerator as f64 / denominator as f64, denominator)
    }
}

/// Macro-F1 restricted to documents whose retained text contains, or does not contain, the label
/// phrase. The cue-present group is the optimistic bound; the cue-free group is underpowered but
/// honest about what the classifier can do without the document naming its own type.
fn cue_strata(known: &[&Judgement], labels: &[String]) -> BTreeMap<String, Estimate> {
    let mut strata = BTreeMap::new();
    for (name, predicate) in [("cue_present", true), ("cue_absent", false)] {
        let subset: Vec<&Judgement> = known.iter().copied().filter(|row| row.cue == predicate).collect();
        strata.insert(name.to_owned(), macro_f1_of(&subset, labels));
    }
    strata
}

fn macro_f1_of(rows: &[&Judgement], labels: &[String]) -> Estimate {
    let mut values = Vec::new();
    for label in labels {
        let support = rows.iter().filter(|row| row.gold.contains(label)).count();
        if support == 0 {
            continue;
        }
        let true_positive = rows
            .iter()
            .filter(|row| row.gold.contains(label) && row.predicted.as_deref() == Some(label))
            .count();
        let predicted = rows
            .iter()
            .filter(|row| row.predicted.as_deref() == Some(label))
            .count();
        let precision = if predicted == 0 {
            0.0
        } else {
            true_positive as f64 / predicted as f64
        };
        let recall = true_positive as f64 / support as f64;
        values.push(if true_positive == 0 {
            0.0
        } else {
            2.0 * precision * recall / (precision + recall)
        });
    }
    if values.is_empty() {
        Estimate::unmeasured("no held-out document in this stratum")
    } else {
        Estimate::measured(values.iter().sum::<f64>() / values.len() as f64, rows.len())
    }
}

/// Ten equal-width bins over top-1 confidence, over answered known-class predictions only.
fn expected_calibration_error(known: &[&Judgement]) -> Estimate {
    let answered: Vec<&&Judgement> = known.iter().filter(|row| row.predicted.is_some()).collect();
    let Some(first) = answered.first() else {
        return Estimate::unmeasured("no answered known-class prediction to calibrate");
    };
    if first.confidence.is_none() {
        return Estimate::unmeasured("predictions carry no confidence");
    }
    const BINS: usize = 10;
    let mut bins = vec![(0usize, 0usize, 0.0f64); BINS];
    for row in &answered {
        let Some(confidence) = row.confidence else {
            return Estimate::unmeasured("a prediction is missing its confidence");
        };
        let index = ((confidence * BINS as f64).floor() as usize).min(BINS - 1);
        let correct = row.predicted.as_ref().is_some_and(|label| row.gold.contains(label));
        bins[index].0 += 1;
        bins[index].1 += usize::from(correct);
        bins[index].2 += confidence;
    }
    let total = answered.len() as f64;
    let ece = bins
        .iter()
        .filter(|bin| bin.0 > 0)
        .map(|bin| {
            let accuracy = bin.1 as f64 / bin.0 as f64;
            let confidence = bin.2 / bin.0 as f64;
            (bin.0 as f64 / total) * (accuracy - confidence).abs()
        })
        .sum::<f64>();
    Estimate::measured(ece, answered.len())
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct Bootstrap {
    pub level_milli: u32,
    pub replicates: usize,
}

impl Bootstrap {
    pub fn new(level: f64, replicates: usize) -> Self {
        Self {
            level_milli: (level * 1000.0).round() as u32,
            replicates,
        }
    }
}

/// Cluster bootstrap over source families: whole families are resampled, so documents that share
/// an issuing agency are never treated as independent evidence for the interval.
fn family_bootstrap(known: Vec<&Judgement>, labels: &[String], settings: &Bootstrap) -> Option<Interval> {
    let mut families: BTreeMap<&str, Vec<&Judgement>> = BTreeMap::new();
    for row in known {
        families.entry(row.family.as_str()).or_default().push(row);
    }
    let clusters: Vec<&Vec<&Judgement>> = families.values().collect();
    if clusters.is_empty() || settings.replicates == 0 {
        return None;
    }
    let mut rng = Rng::new(0x5eed_2334_0000_0001);
    let mut values = Vec::with_capacity(settings.replicates);
    for _ in 0..settings.replicates {
        let mut sample = Vec::new();
        for _ in 0..clusters.len() {
            let index = (rng.next_u64() % clusters.len() as u64) as usize;
            sample.extend(clusters[index].iter().copied());
        }
        if let Some(value) = macro_f1_of(&sample, labels).value {
            values.push(value);
        }
    }
    values.sort_by(|left, right| left.total_cmp(right));
    let tail = ((1.0 - settings.level_milli as f64 / 1000.0) / 2.0 * values.len() as f64) as usize;
    let lower = values[tail.min(values.len().saturating_sub(1))];
    let upper = values[values.len().saturating_sub(1 + tail.min(values.len() - 1))];
    Some(Interval {
        lower,
        upper,
        level: settings.level_milli as f64 / 1000.0,
        replicates: settings.replicates,
        clusters: clusters.len(),
    })
}

/// Deterministic splitmix64. A fixed seed makes a published interval reproducible; it makes no
/// claim about sampling randomness beyond the family resampling itself.
struct Rng(u64);

impl Rng {
    fn new(seed: u64) -> Self {
        Self(seed)
    }

    fn next_u64(&mut self) -> u64 {
        self.0 = self.0.wrapping_add(0x9e37_79b9_7f4a_7c15);
        let mut z = self.0;
        z = (z ^ (z >> 30)).wrapping_mul(0xbf58_476d_1ce4_e5b9);
        z = (z ^ (z >> 27)).wrapping_mul(0x94d0_49bb_1331_11eb);
        z ^ (z >> 31)
    }
}

/// Majority-class reference: the best a classifier can do by always answering one label. It is not
/// a candidate and cannot authorize bundling; it only makes the measured macro-F1 interpretable.
pub fn majority_baseline(labels: &[String], judgements: &[Judgement]) -> BaselineScore {
    let known: Vec<&Judgement> = judgements.iter().filter(|row| !row.gold.is_empty()).collect();
    let counts: BTreeMap<&String, usize> = labels
        .iter()
        .map(|label| (label, known.iter().filter(|row| row.gold.contains(label)).count()))
        .collect();
    let predicted = counts
        .iter()
        .filter(|(_, count)| **count > 0)
        .max_by_key(|(label, count)| (**count, std::cmp::Reverse(label.as_str())))
        .map(|(label, _)| (*label).clone());
    let Some(predicted_class) = predicted else {
        return BaselineScore {
            rule: "always predict the most frequent gold class".into(),
            predicted_class: None,
            macro_f1: Estimate::unmeasured("no held-out known-class document"),
            micro_accuracy: Estimate::unmeasured("no held-out known-class document"),
        };
    };
    let projected: Vec<Judgement> = known
        .iter()
        .map(|row| Judgement {
            id: row.id.clone(),
            family: row.family.clone(),
            gold: row.gold.clone(),
            predicted: Some(predicted_class.clone()),
            confidence: Some(0.0),
            cue: row.cue,
        })
        .collect();
    let score = score_repeat(0, labels, &projected, &Bootstrap::new(0.95, 0));
    BaselineScore {
        rule: "always predict the most frequent gold class".into(),
        predicted_class: Some(predicted_class),
        macro_f1: score.macro_f1,
        micro_accuracy: score.micro_accuracy,
    }
}

/// Caption scores for one arm over the held-out images.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct CaptionScore {
    pub candidate: String,
    pub arm: String,
    pub generations: usize,
    pub images: usize,
    pub empty_captions: usize,
    pub number_recall: Estimate,
    pub number_recall_ci: Option<Interval>,
    pub term_recall: Estimate,
    pub term_recall_ci: Option<Interval>,
    pub invented_number_image_rate: Estimate,
    pub invented_numbers: BTreeMap<String, usize>,
}

#[derive(Debug, Clone, PartialEq)]
pub struct CaptionGeneration {
    pub image: String,
    pub caption: String,
    pub reference_captions: Vec<String>,
    pub reference_numbers: BTreeSet<String>,
    pub reference_terms: BTreeSet<String>,
    pub stratum: String,
}

/// Automatic captioning subset. Numbers and reference content words are checkable against the five
/// reference captions; visible-fact precision and invented entities or relations are not.
pub fn score_captions(
    candidate: &str,
    arm: &str,
    generations: &[CaptionGeneration],
    settings: &Bootstrap,
) -> CaptionScore {
    let mut numbers: Vec<(String, f64)> = Vec::new();
    let mut terms: Vec<(String, f64)> = Vec::new();
    let mut invented_by_image: BTreeMap<&str, usize> = BTreeMap::new();
    let mut invented_numbers = BTreeMap::new();
    let mut empty = 0usize;
    for generation in generations {
        let words: BTreeSet<String> = words_of(&generation.caption);
        let numbers_in_caption: BTreeSet<String> = numbers_of(&generation.caption);
        if words.is_empty() {
            empty += 1;
        }
        if !generation.reference_numbers.is_empty() {
            let found = generation.reference_numbers.intersection(&numbers_in_caption).count();
            numbers.push((
                generation.image.clone(),
                found as f64 / generation.reference_numbers.len() as f64,
            ));
        }
        if !generation.reference_terms.is_empty() {
            let found = generation.reference_terms.intersection(&words).count();
            terms.push((
                generation.image.clone(),
                found as f64 / generation.reference_terms.len() as f64,
            ));
        }
        let invented: BTreeSet<&String> = numbers_in_caption
            .iter()
            .filter(|number| !generation.reference_numbers.contains(*number))
            .collect();
        if !invented.is_empty() {
            *invented_by_image.entry(generation.image.as_str()).or_default() += 1;
            for number in invented {
                *invented_numbers.entry(number.clone()).or_default() += 1;
            }
        }
    }

    let number_recall = mean(&numbers, "no held-out image has a reference number");
    let number_ci = number_recall
        .is_measured()
        .then(|| image_bootstrap(&numbers, settings))
        .flatten();
    let term_recall = mean(&terms, "no held-out image has reference content words");
    let term_ci = term_recall
        .is_measured()
        .then(|| image_bootstrap(&terms, settings))
        .flatten();
    let invented_rate = if generations.is_empty() {
        Estimate::unmeasured("no held-out generation")
    } else {
        Estimate::measured(
            invented_by_image.len() as f64 / generations.len() as f64,
            generations.len(),
        )
    };
    CaptionScore {
        candidate: candidate.to_owned(),
        arm: arm.to_owned(),
        generations: generations.len(),
        images: distinct(&generations.iter().map(|row| row.image.as_str()).collect::<Vec<_>>()),
        empty_captions: empty,
        number_recall,
        number_recall_ci: number_ci,
        term_recall,
        term_recall_ci: term_ci,
        invented_number_image_rate: invented_rate,
        invented_numbers,
    }
}

fn distinct(values: &[&str]) -> usize {
    values.iter().copied().collect::<BTreeSet<_>>().len()
}

fn mean(values: &[(String, f64)], reason: &str) -> Estimate {
    if values.is_empty() {
        Estimate::unmeasured(reason)
    } else {
        Estimate::measured(
            values.iter().map(|(_, value)| value).sum::<f64>() / values.len() as f64,
            values.len(),
        )
    }
}

fn image_bootstrap(values: &[(String, f64)], settings: &Bootstrap) -> Option<Interval> {
    let mut per_image: BTreeMap<&str, Vec<f64>> = BTreeMap::new();
    for (image, value) in values {
        per_image.entry(image.as_str()).or_default().push(*value);
    }
    let clusters: Vec<&Vec<f64>> = per_image.values().collect();
    if clusters.is_empty() || settings.replicates == 0 {
        return None;
    }
    let mut rng = Rng::new(0x5eed_2334_0000_0002);
    let mut means = Vec::with_capacity(settings.replicates);
    for _ in 0..settings.replicates {
        let mut total = 0.0;
        let mut count = 0usize;
        for _ in 0..clusters.len() {
            let index = (rng.next_u64() % clusters.len() as u64) as usize;
            total += clusters[index].iter().sum::<f64>();
            count += clusters[index].len();
        }
        means.push(total / count.max(1) as f64);
    }
    means.sort_by(|left, right| left.total_cmp(right));
    let tail = ((1.0 - settings.level_milli as f64 / 1000.0) / 2.0 * means.len() as f64) as usize;
    Some(Interval {
        lower: means[tail.min(means.len() - 1)],
        upper: means[means.len().saturating_sub(1 + tail.min(means.len() - 1))],
        level: settings.level_milli as f64 / 1000.0,
        replicates: settings.replicates,
        clusters: clusters.len(),
    })
}

/// Paired per-image difference between two arms. Positive means the first arm recalled more.
pub fn paired_difference(first: &[(String, f64)], second: &[(String, f64)], settings: &Bootstrap) -> Option<Interval> {
    let left: BTreeMap<&str, f64> = first.iter().map(|(image, value)| (image.as_str(), *value)).collect();
    let right: BTreeMap<&str, f64> = second.iter().map(|(image, value)| (image.as_str(), *value)).collect();
    let paired: Vec<(String, f64)> = left
        .iter()
        .filter_map(|(image, left)| right.get(image).map(|right| ((*image).to_owned(), left - right)))
        .collect();
    if paired.is_empty() {
        return None;
    }
    image_bootstrap(&paired, settings)
}

pub fn words_of(text: &str) -> BTreeSet<String> {
    text.split(|character: char| !character.is_alphanumeric())
        .filter(|word| !word.is_empty())
        .map(|word| word.to_lowercase())
        .collect()
}

pub fn numbers_of(text: &str) -> BTreeSet<String> {
    text.split(|character: char| !character.is_alphanumeric() && character != '.')
        .filter(|token| {
            !token.is_empty()
                && token.chars().next().is_some_and(|character| character.is_ascii_digit())
                && token
                    .chars()
                    .all(|character| character.is_ascii_digit() || character == '.')
        })
        .map(|token| token.trim_matches('.').to_owned())
        .filter(|token| !token.is_empty())
        .collect()
}

#[cfg(test)]
mod tests {
    use super::*;

    fn judgement(id: &str, family: &str, gold: Option<&str>, predicted: Option<&str>, confidence: f64) -> Judgement {
        Judgement {
            id: id.into(),
            family: family.into(),
            gold: gold.map(|value| vec![value.to_owned()]).unwrap_or_default(),
            predicted: predicted.map(str::to_owned),
            confidence: predicted.map(|_| confidence),
            cue: false,
        }
    }

    fn labels() -> Vec<String> {
        vec!["alpha".into(), "beta".into()]
    }

    #[test]
    fn macro_f1_matches_hand_computed_per_class_values() {
        let rows = vec![
            judgement("a1", "f1", Some("alpha"), Some("alpha"), 0.9),
            judgement("a2", "f1", Some("alpha"), Some("beta"), 0.6),
            judgement("b1", "f2", Some("beta"), Some("beta"), 0.8),
            judgement("b2", "f2", Some("beta"), Some("alpha"), 0.7),
            judgement("u1", "f3", None, None, 0.0),
        ];
        let score = score_repeat(1, &labels(), &rows, &Bootstrap::new(0.95, 200));
        let value = score.macro_f1.value.unwrap();
        assert!((value - 0.5).abs() < 1e-12, "{value}");
        assert_eq!(score.per_class["alpha"].support, 2);
        assert!((score.per_class["alpha"].recall.value.unwrap() - 0.5).abs() < 1e-12);
        assert!((score.per_class["beta"].precision.value.unwrap() - 0.5).abs() < 1e-12);
        assert_eq!(score.known_documents, 4);
        assert_eq!(score.unknown_documents, 1);
        assert!((score.unknown_rejection.value.unwrap() - 1.0).abs() < 1e-12);
        assert_eq!(score.abstentions_on_known, 0);
        assert!((score.micro_accuracy.value.unwrap() - 0.5).abs() < 1e-12);
    }

    #[test]
    fn an_abstention_is_a_recall_error_and_not_a_prediction() {
        let rows = vec![
            judgement("a1", "f1", Some("alpha"), None, 0.0),
            judgement("b1", "f2", Some("beta"), Some("beta"), 0.9),
        ];
        let score = score_repeat(1, &labels(), &rows, &Bootstrap::new(0.95, 200));
        assert_eq!(score.abstentions_on_known, 1);
        assert!((score.abstention_rate_on_known.value.unwrap() - 0.5).abs() < 1e-12);
        assert_eq!(score.per_class["alpha"].support, 1);
        assert_eq!(score.per_class["alpha"].recall.value, Some(0.0));
        assert!((score.micro_accuracy.value.unwrap() - 0.5).abs() < 1e-12);
    }

    #[test]
    fn unmeasured_denominators_report_a_reason_instead_of_zero() {
        let rows = vec![judgement("a1", "f1", Some("alpha"), None, 0.0)];
        let score = score_repeat(1, &labels(), &rows, &Bootstrap::new(0.95, 200));
        assert!(score.selective_accuracy.value.is_none());
        assert!(score.selective_accuracy.reason.is_some());
        assert!(score.unknown_rejection.value.is_none());
        assert!(score.unknown_rejection.reason.is_some());
        assert!(score.ece.value.is_none(), "an all-abstain run has no calibration");
        assert!(score.per_class["beta"].f1.is_none());
        assert!(score.per_class["beta"].recall.reason.is_some());
        // macro over the single supported class is still defined
        assert!(score.macro_f1.is_measured());
    }

    #[test]
    fn ece_is_zero_when_confidence_equals_accuracy() {
        let rows = vec![
            judgement("a1", "f1", Some("alpha"), Some("alpha"), 1.0),
            judgement("b1", "f2", Some("beta"), Some("alpha"), 0.0),
        ];
        let score = score_repeat(1, &labels(), &rows, &Bootstrap::new(0.95, 200));
        assert!(score.ece.value.unwrap() < 1e-12, "{:?}", score.ece);
    }

    #[test]
    fn bootstrap_interval_is_reproducible_and_brackets_the_point_estimate() {
        let rows = vec![
            judgement("a1", "f1", Some("alpha"), Some("alpha"), 0.9),
            judgement("a2", "f1", Some("alpha"), Some("beta"), 0.6),
            judgement("b1", "f2", Some("beta"), Some("beta"), 0.8),
            judgement("b2", "f2", Some("beta"), Some("alpha"), 0.7),
        ];
        let first = score_repeat(1, &labels(), &rows, &Bootstrap::new(0.95, 500));
        let second = score_repeat(1, &labels(), &rows, &Bootstrap::new(0.95, 500));
        assert_eq!(first.ci, second.ci);
        let interval = first.ci.unwrap();
        assert!(interval.lower <= interval.upper);
        assert_eq!(interval.clusters, 2);
    }

    #[test]
    fn a_class_without_support_stays_out_of_the_macro_average() {
        let three = vec!["alpha".to_owned(), "beta".to_owned(), "gamma".to_owned()];
        let rows = vec![judgement("a1", "f1", Some("alpha"), Some("alpha"), 1.0)];
        let score = score_repeat(1, &three, &rows, &Bootstrap::new(0.95, 200));
        assert!((score.macro_f1.value.unwrap() - 1.0).abs() < 1e-12);
        assert!(score.per_class["gamma"].recall.value.is_none());
        assert_eq!(score.confusion.len(), 1);
    }

    #[test]
    fn caption_scores_measure_numbers_terms_and_candidate_inventions() {
        let generation = CaptionGeneration {
            image: "i1".into(),
            caption: "A clock showing 10:10 and a red book".into(),
            reference_captions: vec!["A clock shows 3:45.".into()],
            reference_numbers: ["3".into(), "45".into()].into_iter().collect(),
            reference_terms: ["clock".into(), "shows".into()].into_iter().collect(),
            stratum: "numbers".into(),
        };
        let score = score_captions("smolvlm", "int8", &[generation], &Bootstrap::new(0.95, 200));
        assert_eq!(score.generations, 1);
        assert_eq!(score.number_recall.value, Some(0.0));
        assert!((score.term_recall.value.unwrap() - 0.5).abs() < 1e-12);
        assert!((score.invented_number_image_rate.value.unwrap() - 1.0).abs() < 1e-12);
        assert!(score.invented_numbers.contains_key("10"));
        assert!(score.empty_captions == 0);
    }

    #[test]
    fn an_empty_corpus_has_no_caption_metric_and_says_why() {
        let score = score_captions("smolvlm", "int8", &[], &Bootstrap::new(0.95, 200));
        assert!(score.number_recall.value.is_none());
        assert!(score.term_recall.value.is_none());
        assert!(score.invented_number_image_rate.value.is_none());
        assert_eq!(score.generations, 0);
    }
}
