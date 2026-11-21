//! Turns a frozen plan and an SDK report into scores, after asserting every declared count.

use crate::caption::{CaptionPlan, CaptionReport};
use crate::corpus::{CaptionImage, CaptionManifest, CorpusDocument, CorpusManifest};
use crate::metrics::{
    self, Bootstrap, CaptionGeneration, CaptionScore, ClassificationScore, Estimate, Interval, Judgement, RepeatScore,
};
use crate::prereg::{CaptioningPrereg, ClassificationPrereg, Factor};
use crate::{Result, digest, require};
use nli_qualification::{FrozenPlan, RunReport, Split};
use serde::{Deserialize, Serialize};
use std::collections::BTreeMap;

/// One declared count next to what the artifacts actually contained. A run is only scored when
/// every assertion passes; the assertions are published so a reader can see they fired.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct CountAssertion {
    pub name: String,
    pub expected: usize,
    pub observed: usize,
}

impl CountAssertion {
    fn check(name: impl Into<String>, expected: usize, observed: usize) -> Result<Self> {
        let name = name.into();
        require(
            expected == observed,
            &format!("{name}: expected {expected}, observed {observed}"),
        )?;
        Ok(Self {
            name,
            expected,
            observed,
        })
    }
}

/// One candidate-and-arm result. A run with several arms produces several entries and the
/// decision requires every one of them to clear the preregistered bar.
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct VariantScore {
    pub candidate: String,
    pub arm: String,
    pub held_out: ClassificationScore,
    pub calibration: Option<ClassificationScore>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct ClassificationOutcome {
    pub variants: Vec<VariantScore>,
    pub assertions: Vec<CountAssertion>,
    pub documents_in_plan: usize,
    pub corpus_digest: String,
    pub plan_sha256: String,
    pub report_sha256: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct CaptionOutcome {
    pub scores: Vec<CaptionScore>,
    pub contrasts: Vec<PairedContrast>,
    pub assertions: Vec<CountAssertion>,
    pub repeat_spread: BTreeMap<String, f64>,
    pub corpus_digest: String,
    pub plan_sha256: String,
    pub report_sha256: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct PairedContrast {
    pub candidate: String,
    pub first_arm: String,
    pub second_arm: String,
    pub term_recall_delta: Option<Interval>,
    pub number_recall_delta: Option<Interval>,
    pub note: String,
}

pub fn score_classification(
    plan: &FrozenPlan,
    report: &RunReport,
    manifest: &CorpusManifest,
    prereg: &ClassificationPrereg,
) -> Result<ClassificationOutcome> {
    plan.validate()?;
    report.validate(plan)?;
    manifest.validate()?;
    let mut assertions = classification_assertions(plan, report, prereg)?;
    let corpus = manifest.by_id();
    let bootstrap = Bootstrap::new(prereg.thresholds.ci_level, prereg.thresholds.bootstrap_replicates);
    let mut variants = Vec::new();
    for candidate in &plan.candidates {
        for arm in &plan.arms {
            variants.push(score_variant(
                &VariantContext {
                    plan,
                    report,
                    corpus: &corpus,
                    prereg,
                    bootstrap: &bootstrap,
                    manifest,
                },
                &mut assertions,
                &candidate.id,
                arm,
            )?);
        }
    }
    Ok(ClassificationOutcome {
        variants,
        assertions,
        documents_in_plan: plan.documents.len(),
        corpus_digest: manifest.digest.clone(),
        plan_sha256: plan.sha256()?,
        report_sha256: digest(&serde_json::to_vec(report)?),
    })
}

fn classification_assertions(
    plan: &FrozenPlan,
    report: &RunReport,
    prereg: &ClassificationPrereg,
) -> Result<Vec<CountAssertion>> {
    let assertions = vec![
        CountAssertion::check(
            "plan documents",
            prereg.expected.factor(Factor::Documents)?,
            plan.documents.len(),
        )?,
        CountAssertion::check(
            "plan candidates",
            prereg.expected.factor(Factor::Models)?,
            plan.candidates.len(),
        )?,
        CountAssertion::check("plan arms", prereg.expected.factor(Factor::Arms)?, plan.arms.len())?,
        CountAssertion::check(
            "plan repeats",
            prereg.expected.factor(Factor::Repeats)?,
            plan.repeats as usize,
        )?,
        CountAssertion::check("report rows", prereg.expected.total, report.rows.len())?,
        CountAssertion::check("declared vocabulary", prereg.labels.len(), plan.labels.len())?,
    ];
    for (index, (declared, frozen)) in prereg.labels.iter().zip(plan.labels.iter()).enumerate() {
        require(
            declared == frozen,
            &format!("plan label {index} differs from the preregistration"),
        )?;
    }
    require(plan.arms == prereg.arms, "plan arms differ from the preregistration")?;
    require(
        plan.candidates
            .iter()
            .map(|candidate| candidate.id.as_str())
            .eq(prereg.candidate_ids.iter().map(String::as_str)),
        "plan candidates differ from the preregistration",
    )?;
    Ok(assertions)
}

struct VariantContext<'a> {
    plan: &'a FrozenPlan,
    report: &'a RunReport,
    corpus: &'a BTreeMap<&'a str, &'a CorpusDocument>,
    prereg: &'a ClassificationPrereg,
    bootstrap: &'a Bootstrap,
    manifest: &'a CorpusManifest,
}

fn score_variant(
    context: &VariantContext<'_>,
    assertions: &mut Vec<CountAssertion>,
    candidate: &str,
    arm: &str,
) -> Result<VariantScore> {
    let VariantContext {
        plan,
        report,
        corpus,
        prereg,
        bootstrap,
        manifest,
    } = context;
    let plan = *plan;
    let report = *report;
    let prereg = *prereg;
    let bootstrap = *bootstrap;
    let manifest = *manifest;
    let corpus = *corpus;
    let mut held_out = Vec::new();
    let mut calibration = Vec::new();
    for repeat in 1..=plan.repeats {
        let rows: Vec<&nli_qualification::Prediction> = report
            .rows
            .iter()
            .filter(|row| row.candidate == candidate && row.arm == arm && row.repeat == repeat)
            .collect();
        assertions.push(CountAssertion::check(
            format!("{candidate} / {arm} / repeat {repeat} rows"),
            plan.documents.len(),
            rows.len(),
        )?);
        let (held_judgements, calibration_judgements) = judgements(plan, &rows, corpus, prereg, candidate, arm)?;
        held_out.push(metrics::score_repeat(repeat, &plan.labels, &held_judgements, bootstrap));
        if !calibration_judgements.is_empty() {
            calibration.push(metrics::score_repeat(
                repeat,
                &plan.labels,
                &calibration_judgements,
                bootstrap,
            ));
        }
    }
    let headline_rows: Vec<&nli_qualification::Prediction> = report
        .rows
        .iter()
        .filter(|row| row.candidate == candidate && row.arm == arm && row.repeat == 1)
        .collect();
    let (headline_judgements, _) = judgements(plan, &headline_rows, corpus, prereg, candidate, arm)?;
    let headline = held_out
        .iter()
        .find(|score| score.repeat == 1)
        .ok_or("repeat 1 was not scored")?
        .clone();
    Ok(VariantScore {
        candidate: candidate.to_owned(),
        arm: arm.to_owned(),
        held_out: ClassificationScore {
            headline_repeat: 1,
            repeat_stability: spread(&held_out),
            repeats: held_out,
            baseline: metrics::majority_baseline(&plan.labels, &headline_judgements),
            limitations: classification_limitations(&headline, manifest),
        },
        calibration: (!calibration.is_empty()).then(|| ClassificationScore {
            headline_repeat: 1,
            repeat_stability: spread(&calibration),
            repeats: calibration,
            baseline: metrics::BaselineScore {
                rule: "not computed for the calibration split".into(),
                predicted_class: None,
                macro_f1: Estimate::unmeasured("the baseline is reported on held-out work only"),
                micro_accuracy: Estimate::unmeasured("the baseline is reported on held-out work only"),
            },
            limitations: vec!["Calibration-split scores never enter the go/no-go decision.".into()],
        }),
    })
}

fn judgements(
    plan: &FrozenPlan,
    rows: &[&nli_qualification::Prediction],
    corpus: &BTreeMap<&str, &CorpusDocument>,
    prereg: &ClassificationPrereg,
    candidate: &str,
    arm: &str,
) -> Result<(Vec<Judgement>, Vec<Judgement>)> {
    let mut held = Vec::new();
    let mut calibration = Vec::new();
    for document in &plan.documents {
        require(
            document.split == Split::HeldOut || document.split == Split::Calibration,
            "plan scored a document outside the calibration and held-out splits",
        )?;
        let recorded = corpus
            .get(document.id.as_str())
            .ok_or_else(|| format!("plan document {} is absent from the committed corpus", document.id))?;
        require(
            recorded.text_sha256 == document.text_sha256,
            &format!("plan text for {} differs from the committed corpus", document.id),
        )?;
        let row = rows.iter().find(|row| row.document == document.id).ok_or_else(|| {
            format!(
                "missing prediction for {} / {candidate} / {arm} in this repeat",
                document.id
            )
        })?;
        let (predicted, confidence) = top_prediction(&row.labels);
        let judgement = Judgement {
            id: document.id.clone(),
            family: document.family.clone(),
            gold: recorded.gold(&prereg.labels)?,
            predicted,
            confidence,
            cue: CorpusManifest::has_lexical_cue(recorded, &document.text, &prereg.labels),
        };
        match document.split {
            Split::HeldOut => held.push(judgement),
            Split::Calibration => calibration.push(judgement),
            Split::Training => unreachable!("the split check above rejects the training split"),
        }
    }
    require(!held.is_empty(), "no held-out document reached scoring")?;
    Ok((held, calibration))
}

fn top_prediction(labels: &[xberg::ClassificationLabel]) -> (Option<String>, Option<f64>) {
    labels
        .iter()
        .max_by(|left, right| {
            left.confidence
                .unwrap_or(0.0)
                .total_cmp(&right.confidence.unwrap_or(0.0))
        })
        .map(|label| (Some(label.label.clone()), label.confidence.map(f64::from)))
        .unwrap_or((None, None))
}

fn spread(scores: &[RepeatScore]) -> Option<f64> {
    let values: Vec<f64> = scores.iter().filter_map(|score| score.macro_f1.value).collect();
    (values.len() > 1).then(|| {
        let maximum = values.iter().copied().fold(f64::MIN, f64::max);
        let minimum = values.iter().copied().fold(f64::MAX, f64::min);
        maximum - minimum
    })
}

fn classification_limitations(headline: &RepeatScore, manifest: &CorpusManifest) -> Vec<String> {
    let mut limitations = vec![
        "Abstentions count as recall errors; the abstention rate and selective accuracy are reported separately.".to_owned(),
        "Top-1 confidence is a softmax over three NLI entailment logits, not a calibrated probability over the task labels; ECE is descriptive.".to_owned(),
    ];
    if let Some(cue) = headline.cue_strata.get("cue_absent") {
        limitations.push(format!(
            "Cue-free sensitivity stratum: {} held-out documents whose retained text does not contain their label phrase (macro-F1 {}).",
            cue.observations,
            cue.value
                .map(|value| format!("{value:.4}"))
                .unwrap_or_else(|| "unmeasured".into())
        ));
    }
    if let Some(cue) = headline.cue_strata.get("cue_present") {
        limitations.push(format!(
            "Cue-present stratum: {} held-out documents whose text contains their label phrase; the classifier can read the type off the page, so this is an upper bound.",
            cue.observations
        ));
    }
    limitations.push(format!(
        "The unknown-class cohort is drawn from {} source family/families, so rejection is clustered rather than independent.",
        manifest
            .documents
            .iter()
            .filter(|document| !document.known_class && document.split == Split::HeldOut)
            .map(|document| document.family.as_str())
            .collect::<std::collections::BTreeSet<_>>()
            .len()
    ));
    limitations
}

pub fn score_captioning(
    plan: &CaptionPlan,
    report: &CaptionReport,
    manifest: &CaptionManifest,
    prereg: &CaptioningPrereg,
) -> Result<CaptionOutcome> {
    plan.validate()?;
    report.validate(plan)?;
    manifest.validate()?;
    let mut assertions = caption_assertions(plan, report, prereg, manifest.images.len())?;
    let bootstrap = Bootstrap::new(prereg.thresholds.ci_level, prereg.thresholds.bootstrap_replicates);
    let corpus = manifest.by_id();
    for image in &plan.images {
        let recorded = corpus
            .get(image.id.as_str())
            .ok_or_else(|| format!("caption image {} is absent from the committed corpus", image.id))?;
        require(
            recorded.image_sha256 == image.image_sha256,
            &format!("caption image {} differs from the committed corpus", image.id),
        )?;
    }
    let produced = generation_counts(report);
    for candidate in &plan.candidates {
        for arm in &plan.arms {
            for repeat in 1..=plan.repeats {
                assertions.push(CountAssertion::check(
                    format!("{} / {arm} / repeat {repeat} generations", candidate.id),
                    plan.images.len(),
                    produced
                        .get(&(candidate.id.as_str(), arm.as_str(), repeat))
                        .copied()
                        .unwrap_or(0),
                )?);
            }
        }
    }
    let scores = caption_scores(plan, report, &corpus, &bootstrap)?;
    let repeat_spread = caption_repeat_spread(plan, report, &corpus, &bootstrap)?;
    let contrasts = caption_contrasts(plan, report, &corpus, &bootstrap)?;
    Ok(CaptionOutcome {
        scores,
        contrasts,
        assertions,
        repeat_spread,
        corpus_digest: manifest.digest.clone(),
        plan_sha256: plan.sha256()?,
        report_sha256: digest(&serde_json::to_vec(report)?),
    })
}

fn caption_assertions(
    plan: &CaptionPlan,
    report: &CaptionReport,
    prereg: &CaptioningPrereg,
    manifest_images: usize,
) -> Result<Vec<CountAssertion>> {
    Ok(vec![
        CountAssertion::check("caption images in plan", manifest_images, plan.images.len())?,
        CountAssertion::check("caption candidates", prereg.candidate_ids.len(), plan.candidates.len())?,
        CountAssertion::check("caption arms", prereg.arms.len(), plan.arms.len())?,
        CountAssertion::check("caption repeats", prereg.repeats as usize, plan.repeats as usize)?,
        CountAssertion::check("caption report rows", prereg.expected_total, report.rows.len())?,
    ])
}

fn generation_counts(report: &CaptionReport) -> BTreeMap<(&str, &str, u32), usize> {
    let mut produced = BTreeMap::new();
    for row in &report.rows {
        *produced
            .entry((row.candidate.as_str(), row.arm.as_str(), row.repeat))
            .or_default() += 1;
    }
    produced
}

fn generations_for(
    plan: &CaptionPlan,
    report: &CaptionReport,
    corpus: &BTreeMap<&str, &CaptionImage>,
    candidate: &str,
    arm: &str,
    repeat: u32,
) -> Result<Vec<CaptionGeneration>> {
    let rows: Vec<&crate::caption::CaptionRow> = report
        .rows
        .iter()
        .filter(|row| row.candidate == candidate && row.arm == arm && row.repeat == repeat)
        .collect();
    require(
        rows.len() == plan.images.len(),
        "caption rows for one candidate, arm and repeat do not cover the plan",
    )?;
    rows.iter()
        .map(|row| {
            let image = corpus
                .get(row.image.as_str())
                .ok_or("caption row names an unexpected image")?;
            Ok(CaptionGeneration {
                image: row.image.clone(),
                caption: row.caption.clone(),
                reference_captions: image.reference_captions.clone(),
                reference_numbers: image.reference_numbers.iter().cloned().collect(),
                reference_terms: image.reference_terms.iter().cloned().collect(),
                stratum: image.stratum.clone(),
            })
        })
        .collect()
}

fn caption_scores(
    plan: &CaptionPlan,
    report: &CaptionReport,
    corpus: &BTreeMap<&str, &CaptionImage>,
    bootstrap: &Bootstrap,
) -> Result<Vec<CaptionScore>> {
    let mut scores = Vec::new();
    for candidate in &plan.candidates {
        for arm in &plan.arms {
            let rows = generations_for(plan, report, corpus, &candidate.id, arm, 1)?;
            scores.push(metrics::score_captions(&candidate.id, arm, &rows, bootstrap));
        }
    }
    require(!scores.is_empty(), "no caption variant was scored")?;
    Ok(scores)
}

fn caption_repeat_spread(
    plan: &CaptionPlan,
    report: &CaptionReport,
    corpus: &BTreeMap<&str, &CaptionImage>,
    bootstrap: &Bootstrap,
) -> Result<BTreeMap<String, f64>> {
    let mut repeat_spread = BTreeMap::new();
    for candidate in &plan.candidates {
        for arm in &plan.arms {
            let mut values = Vec::new();
            for repeat in 1..=plan.repeats {
                let rows = generations_for(plan, report, corpus, &candidate.id, arm, repeat)?;
                if let Some(value) = metrics::score_captions(&candidate.id, arm, &rows, bootstrap)
                    .term_recall
                    .value
                {
                    values.push(value);
                }
            }
            if values.len() > 1 {
                let maximum = values.iter().copied().fold(f64::MIN, f64::max);
                let minimum = values.iter().copied().fold(f64::MAX, f64::min);
                repeat_spread.insert(format!("{} / {arm}", candidate.id), maximum - minimum);
            }
        }
    }
    Ok(repeat_spread)
}

fn caption_contrasts(
    plan: &CaptionPlan,
    report: &CaptionReport,
    corpus: &BTreeMap<&str, &CaptionImage>,
    bootstrap: &Bootstrap,
) -> Result<Vec<PairedContrast>> {
    let mut contrasts = Vec::new();
    if plan.arms.len() != 2 {
        return Ok(contrasts);
    }
    for candidate in &plan.candidates {
        let (first, second) = (&plan.arms[0], &plan.arms[1]);
        let left = generations_for(plan, report, corpus, &candidate.id, first, 1)?;
        let right = generations_for(plan, report, corpus, &candidate.id, second, 1)?;
        contrasts.push(PairedContrast {
            candidate: candidate.id.clone(),
            first_arm: first.clone(),
            second_arm: second.clone(),
            term_recall_delta: metrics::paired_difference(
                &per_image_terms(&left),
                &per_image_terms(&right),
                bootstrap,
            ),
            number_recall_delta: metrics::paired_difference(
                &per_image_numbers(&left),
                &per_image_numbers(&right),
                bootstrap,
            ),
            note: "Paired per-image difference; positive favours the first arm. With one scored repeat this measures arm parity, not run-to-run stability.".into(),
        });
    }
    Ok(contrasts)
}

fn per_image_terms(generations: &[CaptionGeneration]) -> Vec<(String, f64)> {
    generations
        .iter()
        .filter(|generation| !generation.reference_terms.is_empty())
        .map(|generation| {
            let words = metrics::words_of(&generation.caption);
            let found = generation.reference_terms.intersection(&words).count();
            (
                generation.image.clone(),
                found as f64 / generation.reference_terms.len() as f64,
            )
        })
        .collect()
}

fn per_image_numbers(generations: &[CaptionGeneration]) -> Vec<(String, f64)> {
    generations
        .iter()
        .filter(|generation| !generation.reference_numbers.is_empty())
        .map(|generation| {
            let numbers = metrics::numbers_of(&generation.caption);
            let found = generation.reference_numbers.intersection(&numbers).count();
            (
                generation.image.clone(),
                found as f64 / generation.reference_numbers.len() as f64,
            )
        })
        .collect()
}
