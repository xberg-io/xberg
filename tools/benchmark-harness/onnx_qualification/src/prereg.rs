use crate::{Result, hex, identifier, require};
use serde::{Deserialize, Serialize};
use std::collections::{BTreeMap, BTreeSet};
use std::fmt::Write as _;

/// The bar is posted before the held-out run, so the file carries the post URL and a flag proving
/// the ordering was intended. Nothing here selects a default; it only states what would count as a
/// "go" per workload.
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct Preregistration {
    pub schema: u32,
    pub issue: u64,
    pub posted_before_run: bool,
    pub posted_url: Option<String>,
    pub corpora: BTreeMap<String, CorpusPin>,
    pub models: Vec<ModelPin>,
    pub classification: Option<ClassificationPrereg>,
    pub captioning: Option<CaptioningPrereg>,
    pub metric_definitions: BTreeMap<String, String>,
    pub unmeasured: Vec<Unmeasured>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct CorpusPin {
    pub manifest_sha256: String,
    pub digest: String,
    pub entries: usize,
    pub split_counts: BTreeMap<String, usize>,
    pub license: String,
    pub license_url: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct ModelPin {
    pub workload: String,
    pub id: String,
    pub repository: String,
    pub revision: String,
    pub license: String,
    /// Arm name to the exact graph file that arm loads, so "fp32" and "int8" cannot drift.
    pub arms: BTreeMap<String, String>,
}

#[derive(Debug, Clone, Copy, Serialize, Deserialize, PartialEq, Eq)]
#[serde(deny_unknown_fields)]
pub struct CountFactor {
    pub name: Factor,
    pub count: usize,
}

#[derive(Debug, Clone, Copy, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "snake_case")]
pub enum Factor {
    Documents,
    Images,
    Models,
    Arms,
    Repeats,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
#[serde(deny_unknown_fields)]
pub struct ExpectedCount {
    pub factors: Vec<CountFactor>,
    pub total: usize,
}

impl ExpectedCount {
    pub fn product(&self) -> Result<usize> {
        let mut product = 1usize;
        for factor in &self.factors {
            product = product
                .checked_mul(factor.count)
                .ok_or_else(|| "expected count overflows".to_owned())?;
        }
        require(
            product == self.total && !self.factors.is_empty(),
            "expected count does not equal its factors",
        )?;
        Ok(product)
    }

    pub fn factor(&self, name: Factor) -> Result<usize> {
        self.factors
            .iter()
            .find(|factor| factor.name == name)
            .map(|factor| factor.count)
            .ok_or_else(|| format!("expected count omits {name:?}").into())
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct ClassificationPrereg {
    pub candidate_ids: Vec<String>,
    pub arms: Vec<String>,
    pub repeats: u32,
    pub scored_splits: Vec<String>,
    pub expected: ExpectedCount,
    pub labels: Vec<String>,
    pub unknown_class: String,
    pub hypothesis_template: String,
    pub confidence_threshold: f64,
    pub baseline: String,
    pub thresholds: ClassificationThresholds,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct ClassificationThresholds {
    pub macro_f1_noninferiority: f64,
    pub macro_f1_target: f64,
    pub per_class_recall_min: f64,
    pub ece_max: f64,
    pub max_abstention_on_known: f64,
    pub unknown_rejection_min: f64,
    pub ci_level: f64,
    pub bootstrap_replicates: usize,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct CaptioningPrereg {
    pub candidate_ids: Vec<String>,
    pub arms: Vec<String>,
    pub repeats: u32,
    pub expected_per_candidate: ExpectedCount,
    pub expected_total: usize,
    pub thresholds: CaptioningThresholds,
    pub human_rating_required: bool,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct CaptioningThresholds {
    pub number_recall_min: f64,
    pub term_recall_min: f64,
    pub max_invented_number_image_rate: f64,
    pub ci_level: f64,
    pub bootstrap_replicates: usize,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct Unmeasured {
    pub workload: String,
    pub clause: String,
    pub reason: String,
    pub requires: String,
}

impl Preregistration {
    pub fn validate(&self) -> Result<()> {
        require(
            self.schema == 1 && self.posted_before_run && self.issue > 0,
            "preregistration must be version 1, posted before the run, and name its issue",
        )?;
        require(
            self.posted_url.as_ref().is_some_and(|url| url.starts_with("https://")),
            "preregistration must record where it was posted",
        )?;
        require(
            !self.corpora.is_empty() && !self.models.is_empty(),
            "preregistration is empty",
        )?;
        for (name, corpus) in &self.corpora {
            require(
                !name.trim().is_empty()
                    && hex(&corpus.manifest_sha256, 64)
                    && hex(&corpus.digest, 64)
                    && corpus.entries > 0
                    && !corpus.license.trim().is_empty(),
                "corpus pin is incomplete",
            )?;
            let split_total: usize = corpus.split_counts.values().sum();
            require(
                split_total == corpus.entries,
                "corpus split counts do not sum to its entries",
            )?;
        }
        let mut model_ids = BTreeSet::new();
        for model in &self.models {
            require(
                identifier(&model.id)
                    && model_ids.insert(model.id.as_str())
                    && !model.repository.trim().is_empty()
                    && model.revision.len() == 40
                    && !model.license.trim().is_empty()
                    && !model.arms.is_empty(),
                "model pin is incomplete",
            )?;
            require(
                model.arms.values().all(|graph| !graph.trim().is_empty()),
                "model arm lacks its graph file",
            )?;
        }
        require(
            !self.metric_definitions.is_empty() && !self.unmeasured.is_empty(),
            "a preregistration must define its metrics and state what stays unmeasured",
        )?;
        if let Some(classification) = &self.classification {
            classification.validate(&self.models)?;
        }
        if let Some(captioning) = &self.captioning {
            captioning.validate(&self.models)?;
        }
        Ok(())
    }

    pub fn sha256(&self) -> Result<String> {
        Ok(crate::digest(&serde_json::to_vec(self)?))
    }

    pub fn classification_models(&self) -> impl Iterator<Item = &ModelPin> {
        self.models.iter().filter(|model| model.workload == "classification")
    }

    pub fn caption_models(&self) -> impl Iterator<Item = &ModelPin> {
        self.models.iter().filter(|model| model.workload == "captioning")
    }
}

impl ClassificationPrereg {
    fn validate(&self, models: &[ModelPin]) -> Result<()> {
        require(
            !self.candidate_ids.is_empty()
                && !self.arms.is_empty()
                && self.repeats > 0
                && !self.scored_splits.is_empty(),
            "classification run is not fully specified",
        )?;
        let declared = models
            .iter()
            .filter(|model| model.workload == "classification")
            .map(|model| model.id.as_str())
            .collect::<BTreeSet<_>>();
        require(
            self.candidate_ids.iter().all(|id| declared.contains(id.as_str())),
            "classification candidate has no model pin",
        )?;
        for model in models.iter().filter(|model| model.workload == "classification") {
            require(
                model.arms.keys().eq(self.arms.iter()),
                "classification arms differ between the run and a model pin",
            )?;
        }
        require(
            self.expected.factor(Factor::Models)? == self.candidate_ids.len()
                && self.expected.factor(Factor::Arms)? == self.arms.len()
                && self.expected.factor(Factor::Repeats)? == self.repeats as usize,
            "expected classification count does not match the declared models, arms or repeats",
        )?;
        self.expected.product()?;
        require(!self.labels.is_empty(), "classification vocabulary is empty")?;
        let mut seen = BTreeSet::new();
        for label in &self.labels {
            require(
                !label.trim().is_empty() && seen.insert(label.to_lowercase()),
                "duplicate or blank classification label",
            )?;
            require(
                !self.unknown_class.eq_ignore_ascii_case(label),
                "unknown class entered the vocabulary",
            )?;
        }
        require(
            self.hypothesis_template.matches("{label}").count() == 1
                && (0.0..=1.0).contains(&self.confidence_threshold),
            "clip template or threshold is invalid",
        )?;
        self.thresholds.validate()
    }
}

impl ClassificationThresholds {
    fn validate(&self) -> Result<()> {
        for value in [
            self.macro_f1_noninferiority,
            self.macro_f1_target,
            self.per_class_recall_min,
            self.ece_max,
            self.max_abstention_on_known,
            self.unknown_rejection_min,
        ] {
            require(
                (0.0..=1.0).contains(&value),
                "classification threshold must be a probability",
            )?;
        }
        require(
            self.macro_f1_target >= self.macro_f1_noninferiority
                && (0.5..1.0).contains(&self.ci_level)
                && (200..=100_000).contains(&self.bootstrap_replicates),
            "classification thresholds or bootstrap settings are invalid",
        )
    }
}

impl CaptioningPrereg {
    fn validate(&self, models: &[ModelPin]) -> Result<()> {
        let declared = models
            .iter()
            .filter(|model| model.workload == "captioning")
            .map(|model| model.id.as_str())
            .collect::<BTreeSet<_>>();
        require(
            !self.candidate_ids.is_empty() && self.candidate_ids.iter().all(|id| declared.contains(id.as_str())),
            "captioning candidate has no model pin",
        )?;
        require(
            self.expected_per_candidate.factor(Factor::Models)? == 1
                && self.expected_per_candidate.factor(Factor::Arms)? == self.arms.len()
                && self.expected_per_candidate.factor(Factor::Repeats)? == self.repeats as usize,
            "expected per-candidate caption count does not match arms or repeats",
        )?;
        let per_candidate = self.expected_per_candidate.product()?;
        require(
            per_candidate
                .checked_mul(self.candidate_ids.len())
                .is_some_and(|total| total == self.expected_total),
            "expected caption count does not equal candidates times per-candidate work",
        )?;
        require(
            (0.0..=1.0).contains(&self.thresholds.number_recall_min)
                && (0.0..=1.0).contains(&self.thresholds.term_recall_min)
                && (0.0..=1.0).contains(&self.thresholds.max_invented_number_image_rate)
                && (0.5..1.0).contains(&self.thresholds.ci_level)
                && (200..=100_000).contains(&self.thresholds.bootstrap_replicates),
            "captioning thresholds or bootstrap settings are invalid",
        )
    }
}

impl Preregistration {
    /// Render the exact bar that was posted. The comment is generated from the validated file so
    /// the posted text and the scored bar cannot diverge.
    pub fn render_markdown(&self) -> Result<String> {
        self.validate()?;
        let mut out = String::new();
        writeln!(out, "## Preregistered qualification bar (#{})\n", self.issue)?;
        writeln!(
            out,
            "Posted before the held-out run. Weights, arms, corpus digests, metric definitions, expected counts and decision margins are frozen in this comment; any later change requires a new preregistration and a new corpus digest.\n"
        )?;
        self.render_models(&mut out)?;
        self.render_corpora(&mut out)?;
        if let Some(classification) = &self.classification {
            render_classification(&mut out, classification)?;
        }
        if let Some(captioning) = &self.captioning {
            render_captioning(&mut out, captioning)?;
        }
        render_definitions(&mut out, &self.metric_definitions)?;
        render_unmeasured(&mut out, &self.unmeasured)?;
        writeln!(
            out,
            "\nQualification is `false` in every artifact. No default, preset, bundle or supported-model claim changes regardless of the outcome."
        )?;
        Ok(out)
    }

    fn render_models(&self, out: &mut String) -> Result<()> {
        writeln!(out, "### Candidate revisions\n")?;
        writeln!(out, "| workload | id | repository | revision | license | arms |")?;
        writeln!(out, "| --- | --- | --- | --- | --- | --- |")?;
        for model in &self.models {
            let arms = model
                .arms
                .iter()
                .map(|(arm, graph)| format!("{arm}: `{graph}`"))
                .collect::<Vec<_>>()
                .join("; ");
            writeln!(
                out,
                "| {} | `{}` | `{}` | `{}` | {} | {} |",
                model.workload, model.id, model.repository, model.revision, model.license, arms
            )?;
        }
        Ok(())
    }

    fn render_corpora(&self, out: &mut String) -> Result<()> {
        writeln!(out, "\n### Corpus\n")?;
        writeln!(
            out,
            "| corpus | entries | digest | manifest sha256 | splits | license |"
        )?;
        writeln!(out, "| --- | --- | --- | --- | --- | --- |")?;
        for (name, corpus) in &self.corpora {
            let splits = corpus
                .split_counts
                .iter()
                .map(|(split, count)| format!("{split}={count}"))
                .collect::<Vec<_>>()
                .join(", ");
            writeln!(
                out,
                "| {name} | {} | `{}` | `{}` | {splits} | {} |",
                corpus.entries, corpus.digest, corpus.manifest_sha256, corpus.license
            )?;
        }
        Ok(())
    }
}

fn render_classification(out: &mut String, classification: &ClassificationPrereg) -> Result<()> {
    writeln!(out, "\n### Classification\n")?;
    writeln!(
        out,
        "- candidate(s): {}; arms: {}; repeats: {}; scored splits: {}",
        classification.candidate_ids.join(", "),
        classification.arms.join(", "),
        classification.repeats,
        classification.scored_splits.join(", ")
    )?;
    writeln!(
        out,
        "- vocabulary: {}; unknown class: `{}`; template: `{}`; threshold: {}",
        classification
            .labels
            .iter()
            .map(|label| format!("`{label}`"))
            .collect::<Vec<_>>()
            .join(", "),
        classification.unknown_class,
        classification.hypothesis_template,
        classification.confidence_threshold
    )?;
    writeln!(
        out,
        "- expected work: {} = **{}** inferences",
        classify_count(&classification.expected),
        classification.expected.total
    )?;
    let thresholds = &classification.thresholds;
    writeln!(
        out,
        "- go if: macro-F1 point estimate >= {} and its {:.0}% family-clustered CI lower bound >= {}; every per-class recall >= {}; ECE <= {}; abstention on known-class documents <= {}; unknown-class rejection >= {}.",
        thresholds.macro_f1_target,
        thresholds.ci_level * 100.0,
        thresholds.macro_f1_noninferiority,
        thresholds.per_class_recall_min,
        thresholds.ece_max,
        thresholds.max_abstention_on_known,
        thresholds.unknown_rejection_min
    )?;
    writeln!(out, "- reference baseline: {}", classification.baseline)?;
    Ok(())
}

fn render_captioning(out: &mut String, captioning: &CaptioningPrereg) -> Result<()> {
    writeln!(out, "\n### Captioning\n")?;
    writeln!(
        out,
        "- candidate(s): {}; arms: {}; repeats: {}",
        captioning.candidate_ids.join(", "),
        captioning.arms.join(", "),
        captioning.repeats
    )?;
    writeln!(
        out,
        "- expected work: {} per candidate, **{}** generations total",
        classify_count(&captioning.expected_per_candidate),
        captioning.expected_total
    )?;
    writeln!(
        out,
        "- automatic bars: reference-number recall >= {} (number stratum), reference-term recall >= {} (omission proxy), images with an invented number <= {}.",
        captioning.thresholds.number_recall_min,
        captioning.thresholds.term_recall_min,
        captioning.thresholds.max_invented_number_image_rate
    )?;
    writeln!(
        out,
        "- human rating required before any go: {}",
        if captioning.human_rating_required { "yes" } else { "no" }
    )?;
    writeln!(
        out,
        "- therefore the captioning verdict is `pending_human` unless an automatic bar fails, which is already a `no_go`."
    )?;
    Ok(())
}

fn render_definitions(out: &mut String, definitions: &BTreeMap<String, String>) -> Result<()> {
    writeln!(out, "\n### Metric definitions\n")?;
    for (name, definition) in definitions {
        writeln!(out, "- **{name}**: {definition}")?;
    }
    Ok(())
}

fn render_unmeasured(out: &mut String, unmeasured: &[Unmeasured]) -> Result<()> {
    writeln!(out, "\n### Not measured by this run\n")?;
    for item in unmeasured {
        writeln!(
            out,
            "- **{} / {}**: {} Requires: {}",
            item.workload, item.clause, item.reason, item.requires
        )?;
    }
    Ok(())
}

fn classify_count(expected: &ExpectedCount) -> String {
    expected
        .factors
        .iter()
        .map(|factor| format!("{} {}", factor.count, factor_name(factor.name, factor.count)))
        .collect::<Vec<_>>()
        .join(" x ")
}

fn factor_name(name: Factor, count: usize) -> &'static str {
    match (name, count == 1) {
        (Factor::Documents, true) => "document",
        (Factor::Documents, false) => "documents",
        (Factor::Images, true) => "image",
        (Factor::Images, false) => "images",
        (Factor::Models, true) => "model",
        (Factor::Models, false) => "models",
        (Factor::Arms, true) => "arm",
        (Factor::Arms, false) => "arms",
        (Factor::Repeats, true) => "repeat",
        (Factor::Repeats, false) => "repeats",
    }
}
