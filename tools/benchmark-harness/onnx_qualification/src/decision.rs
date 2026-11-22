//! Per-workload verdicts. A verdict is derived only from preregistered thresholds and measured
//! values; it never changes an SDK default, a preset or a bundle.

use crate::prereg::{CaptioningPrereg, ClassificationPrereg, ClassificationThresholds, Preregistration};
use crate::score::{CaptionOutcome, ClassificationOutcome};
use crate::{Result, require};
use serde::{Deserialize, Serialize};
use std::collections::BTreeMap;

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum Verdict {
    Go,
    NoGo,
    PendingHuman,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct Check {
    pub name: String,
    pub threshold: f64,
    pub observed: Option<f64>,
    pub passed: bool,
    pub detail: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct WorkloadDecision {
    pub workload: String,
    pub verdict: Verdict,
    pub checks: Vec<Check>,
    pub reasons: Vec<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct Decision {
    pub schema: u32,
    pub issue: u64,
    /// Always false: qualification evidence alone never flips the SDK's supported-model flag.
    pub qualified: bool,
    pub defaults_changed: bool,
    pub prereg_sha256: String,
    pub corpus_digests: BTreeMap<String, String>,
    pub decisions: Vec<WorkloadDecision>,
}

/// Assemble the report. Each outcome is bound to the corpus digest frozen in the preregistration,
/// so a score computed on a different corpus cannot be presented as this run's evidence.
pub fn build(
    prereg: &Preregistration,
    classification: Option<&ClassificationOutcome>,
    captioning: Option<&CaptionOutcome>,
) -> Result<Decision> {
    require(
        classification.is_some() || captioning.is_some(),
        "a decision needs at least one scored workload",
    )?;
    let mut decisions = Vec::new();
    let mut corpus_digests = BTreeMap::new();
    if let Some(outcome) = classification {
        let pinned = prereg
            .corpora
            .get("classification")
            .ok_or("preregistration pins no classification corpus")?;
        require(
            outcome.corpus_digest == pinned.digest,
            "classification outcome was scored on a different corpus than the preregistration",
        )?;
        corpus_digests.insert("classification".to_owned(), outcome.corpus_digest.clone());
        let settings = prereg
            .classification
            .as_ref()
            .ok_or("preregistration declares no classification run")?;
        decisions.push(decide_classification(outcome, settings));
    }
    if let Some(outcome) = captioning {
        let pinned = prereg
            .corpora
            .get("captioning")
            .ok_or("preregistration pins no captioning corpus")?;
        require(
            outcome.corpus_digest == pinned.digest,
            "captioning outcome was scored on a different corpus than the preregistration",
        )?;
        corpus_digests.insert("captioning".to_owned(), outcome.corpus_digest.clone());
        let settings = prereg
            .captioning
            .as_ref()
            .ok_or("preregistration declares no captioning run")?;
        decisions.push(decide_captioning(outcome, settings));
    }
    Ok(Decision {
        schema: 1,
        issue: prereg.issue,
        qualified: false,
        defaults_changed: false,
        prereg_sha256: prereg.sha256()?,
        corpus_digests,
        decisions,
    })
}

impl Decision {
    pub fn render_markdown(&self) -> Result<String> {
        use std::fmt::Write as _;
        let mut out = String::new();
        writeln!(out, "## Qualification result (#{})\n", self.issue)?;
        for decision in &self.decisions {
            writeln!(out, "### {}: `{:?}`\n", decision.workload, decision.verdict)?;
            writeln!(out, "| check | threshold | observed | result |")?;
            writeln!(out, "| --- | --- | --- | --- |")?;
            for check in &decision.checks {
                writeln!(
                    out,
                    "| {} | {} | {} | {} |",
                    check.name,
                    check.threshold,
                    check
                        .observed
                        .map(|value| format!("{value:.4}"))
                        .unwrap_or_else(|| "unmeasured".into()),
                    if check.passed { "pass" } else { "FAIL" }
                )?;
            }
            for reason in &decision.reasons {
                writeln!(out, "- {reason}")?;
            }
            writeln!(out)?;
        }
        writeln!(
            out,
            "`qualified: false`, `defaults_changed: false`: no default, preset, bundle or supported-model claim follows from this run."
        )?;
        Ok(out)
    }
}

fn check(name: &str, threshold: f64, observed: Option<f64>, at_least: bool, detail: &str) -> Check {
    let passed = observed.is_some_and(|value| {
        if at_least {
            value >= threshold
        } else {
            value <= threshold
        }
    });
    Check {
        name: name.into(),
        threshold,
        observed,
        passed,
        detail: if observed.is_none() {
            format!("{detail}; metric is unmeasured")
        } else {
            detail.to_owned()
        },
    }
}

pub fn decide_classification(outcome: &ClassificationOutcome, prereg: &ClassificationPrereg) -> WorkloadDecision {
    let thresholds = &prereg.thresholds;
    let mut checks = Vec::new();
    let mut missing = Vec::new();
    for variant in &outcome.variants {
        let headline = variant
            .held_out
            .repeats
            .iter()
            .find(|score| score.repeat == variant.held_out.headline_repeat);
        match headline {
            Some(headline) => checks.extend(variant_checks(
                &format!("{} / {}", variant.candidate, variant.arm),
                headline,
                thresholds,
            )),
            None => missing.push(format!(
                "{} / {} has no headline repeat",
                variant.candidate, variant.arm
            )),
        }
    }
    let mut failures = missing;
    for item in &checks {
        if !item.passed {
            failures.push(format!(
                "{} observed {} (threshold {})",
                item.name,
                display(item.observed),
                item.threshold
            ));
        }
    }
    let verdict = if !checks.is_empty() && failures.is_empty() {
        Verdict::Go
    } else {
        Verdict::NoGo
    };
    WorkloadDecision {
        workload: "classification".into(),
        verdict,
        checks,
        reasons: if failures.is_empty() {
            vec![prereg.baseline.clone()]
        } else {
            failures
        },
    }
}

fn variant_checks(
    prefix: &str,
    headline: &crate::metrics::RepeatScore,
    thresholds: &ClassificationThresholds,
) -> Vec<Check> {
    let mut checks = vec![
        check(
            &format!("{prefix} macro-F1"),
            thresholds.macro_f1_target,
            headline.macro_f1.value,
            true,
            "point estimate on held-out documents, abstentions counted as errors",
        ),
        check(
            &format!("{prefix} macro-F1 CI lower bound"),
            thresholds.macro_f1_noninferiority,
            headline.ci.as_ref().map(|interval| interval.lower),
            true,
            "source-family cluster bootstrap",
        ),
    ];
    for (label, class) in &headline.per_class {
        checks.push(check(
            &format!("{prefix} recall {label}"),
            thresholds.per_class_recall_min,
            class.recall.value,
            true,
            "per-class recall on held-out documents",
        ));
    }
    checks.extend([
        check(
            &format!("{prefix} ECE"),
            thresholds.ece_max,
            headline.ece.value,
            false,
            "ten-bin top-1 calibration error",
        ),
        check(
            &format!("{prefix} abstention on known classes"),
            thresholds.max_abstention_on_known,
            headline.abstention_rate_on_known.value,
            false,
            "abstention is allowed up to this rate",
        ),
        check(
            &format!("{prefix} unknown-class rejection"),
            thresholds.unknown_rejection_min,
            headline.unknown_rejection.value,
            true,
            "held-out documents outside the vocabulary",
        ),
    ]);
    checks
}

fn display(value: Option<f64>) -> String {
    value
        .map(|value| format!("{value:.4}"))
        .unwrap_or_else(|| "unmeasured".into())
}

pub fn decide_captioning(outcome: &CaptionOutcome, prereg: &CaptioningPrereg) -> WorkloadDecision {
    let thresholds = &prereg.thresholds;
    let mut checks = Vec::new();
    for score in &outcome.scores {
        let prefix = format!("{} / {}", score.candidate, score.arm);
        checks.push(check(
            &format!("{prefix} reference-number recall"),
            thresholds.number_recall_min,
            score.number_recall.value,
            true,
            "numbers from the five reference captions found in the generated caption",
        ));
        checks.push(check(
            &format!("{prefix} reference-term recall"),
            thresholds.term_recall_min,
            score.term_recall.value,
            true,
            "reference content words found in the generated caption; an omission proxy",
        ));
        checks.push(check(
            &format!("{prefix} images with an invented number"),
            thresholds.max_invented_number_image_rate,
            score.invented_number_image_rate.value,
            false,
            "generated numbers absent from every reference caption; unverified, needs a rater",
        ));
    }
    require(!checks.is_empty(), "captioning decision has no measurement").ok();
    let failed: Vec<&Check> = checks.iter().filter(|item| !item.passed).collect();
    let (verdict, reasons) = if !failed.is_empty() {
        (
            Verdict::NoGo,
            failed
                .iter()
                .map(|item| {
                    format!(
                        "{} observed {} (threshold {})",
                        item.name,
                        display(item.observed),
                        item.threshold
                    )
                })
                .collect(),
        )
    } else if prereg.human_rating_required {
        (
            Verdict::PendingHuman,
            vec![
                "Automatic bars pass, but visible-fact precision, invented entities/relations and illegibility handling need blind human raters; no captioning go is possible from this run.".to_owned(),
            ],
        )
    } else {
        (Verdict::Go, vec!["All preregistered automatic bars passed.".to_owned()])
    };
    WorkloadDecision {
        workload: "captioning".into(),
        verdict,
        checks,
        reasons,
    }
}
