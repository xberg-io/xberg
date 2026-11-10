use crate::{FrozenPlan, Prediction, Result, RunReport, Variant, digest, require};
use std::collections::BTreeSet;

impl RunReport {
    pub fn validate(&self, plan: &FrozenPlan) -> Result<()> {
        plan.validate()?;
        require(
            self.schema_version == 1 && !self.qualified,
            "report must remain unqualified",
        )?;
        require(self.plan_sha256 == plan.sha256()?, "frozen plan digest mismatch")?;
        require(self.runtime == plan.runtime, "runtime identity mismatch")?;
        require(
            self.rows.len() == plan.expected_inferences,
            "inference row count mismatch",
        )?;
        let mut seen = BTreeSet::new();
        for row in &self.rows {
            let candidate = plan
                .candidates
                .iter()
                .find(|candidate| candidate.id == row.candidate)
                .ok_or("unexpected candidate")?;
            let variant = candidate.variants.get(&row.arm).ok_or("unexpected arm")?;
            require(
                plan.documents.iter().any(|doc| doc.id == row.document) && (1..=plan.repeats).contains(&row.repeat),
                "unexpected document or repeat",
            )?;
            require(
                seen.insert((&row.document, &row.candidate, &row.arm, row.repeat)),
                "duplicate inference row",
            )?;
            row.validate(plan, variant)?;
        }
        Ok(())
    }
}

impl Prediction {
    fn validate(&self, plan: &FrozenPlan, variant: &Variant) -> Result<()> {
        require(
            self.inference == "completed" && self.elapsed_ms.is_finite() && self.elapsed_ms > 0.0,
            "inference must complete with positive finite measured duration",
        )?;
        let model = &variant.config.model;
        require(
            self.model.repository == model.repository
                && self.model.revision == model.revision
                && self.model.model_file == model.model_file
                && self.model.model_sha256 == variant.graph_sha256()?
                && self.model.manifest_sha256 == digest(model.sha256_manifest.as_bytes())
                && self.model.experimental
                && self.model.selected_backend == xberg::local_classification::LocalClassificationBackend::OnnxCpu
                && self.model.effective_backend == self.model.selected_backend,
            "model identity mismatch",
        )?;
        let mut seen = BTreeSet::new();
        require(
            variant.config.multi_label || self.labels.len() <= 1,
            "invalid single-label prediction count",
        )?;
        self.validate_cardinality(plan, variant)?;
        for label in &self.labels {
            require(
                plan.labels.contains(&label.label)
                    && seen.insert(&label.label)
                    && label.confidence.is_some_and(|value| {
                        value.is_finite()
                            && (0.0..=1.0).contains(&value)
                            && value >= variant.config.confidence_threshold
                    }),
                "invalid prediction",
            )?;
        }
        Ok(())
    }

    fn validate_cardinality(&self, plan: &FrozenPlan, variant: &Variant) -> Result<()> {
        let config = &variant.config;
        let minimum_best = (1.0 / plan.labels.len() as f64) as f32;
        let must_select = config.confidence_threshold == 0.0
            || (!config.multi_label && plan.labels.len() > 1 && config.confidence_threshold <= minimum_best);
        require(
            !must_select || !self.labels.is_empty(),
            "impossible abstention cardinality",
        )?;
        require(
            !config.multi_label || config.confidence_threshold != 0.0 || self.labels.len() == plan.labels.len(),
            "zero-threshold multi-label cardinality must include every label",
        )?;
        require(
            config.multi_label
                || plan.labels.len() == 1
                || self
                    .labels
                    .iter()
                    .all(|label| label.confidence.is_some_and(|value| value >= minimum_best)),
            "single-label maximum probability is below the softmax bound",
        )
    }
}
