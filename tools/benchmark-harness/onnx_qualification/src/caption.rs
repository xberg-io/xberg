use crate::{Result, hex, identifier, require};
use nli_qualification::RuntimeIdentity;
use serde::{Deserialize, Serialize};
use std::collections::{BTreeMap, BTreeSet};

/// Frozen captioning work. It mirrors the classification contract: exact Cartesian accounting,
/// completed inference required, and no successful report when a single row is missing.
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct CaptionPlan {
    pub schema: u32,
    pub qualified: bool,
    pub expected_generations: usize,
    pub repeats: u32,
    pub arms: Vec<String>,
    pub provenance: CaptionProvenance,
    pub runtime: RuntimeIdentity,
    pub images: Vec<CaptionPlanImage>,
    pub candidates: Vec<CaptionPlanCandidate>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct CaptionProvenance {
    pub corpus_receipt_sha256: String,
    pub split_audit_sha256: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct CaptionPlanImage {
    pub id: String,
    pub image_file: String,
    pub image_sha256: String,
    pub image_bytes: usize,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct CaptionPlanCandidate {
    pub id: String,
    pub arms: BTreeMap<String, CaptionArm>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct CaptionArm {
    /// `fp32` or `quantized`; the runner maps this to an inventory precision.
    pub precision: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct CaptionReport {
    pub schema: u32,
    pub qualified: bool,
    pub plan_sha256: String,
    pub runtime: RuntimeIdentity,
    pub rows: Vec<CaptionRow>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct CaptionRow {
    pub image: String,
    pub candidate: String,
    pub arm: String,
    pub repeat: u32,
    pub inference: String,
    pub elapsed_ms: f64,
    pub caption: String,
    /// Prepared model identity as the SDK reported it.
    pub model: serde_json::Value,
}

impl CaptionPlan {
    pub fn validate(&self) -> Result<()> {
        require(
            self.schema == 1 && !self.qualified,
            "caption plan must be version 1 and unqualified",
        )?;
        require(
            !self.images.is_empty() && !self.candidates.is_empty() && !self.arms.is_empty() && self.repeats > 0,
            "caption plan cannot declare empty work",
        )?;
        require(
            hex(&self.provenance.corpus_receipt_sha256, 64) && hex(&self.provenance.split_audit_sha256, 64),
            "caption provenance receipts require SHA-256",
        )?;
        require(
            hex(&self.runtime.runner_sha256, 64)
                && hex(&self.runtime.dependency_lock_sha256, 64)
                && !self.runtime.ort_build_info.trim().is_empty()
                && self.runtime.threads == 0
                && self.runtime.deadline_ms > 0,
            "caption runtime identity is incomplete",
        )?;
        let mut images = BTreeSet::new();
        for image in &self.images {
            require(
                identifier(&image.id)
                    && images.insert(image.id.as_str())
                    && hex(&image.image_sha256, 64)
                    && image.image_bytes > 0
                    && !image.image_file.trim().is_empty(),
                "caption image pin is invalid",
            )?;
        }
        let mut candidates = BTreeSet::new();
        for candidate in &self.candidates {
            require(
                identifier(&candidate.id) && candidates.insert(candidate.id.as_str()),
                "caption candidate id is invalid or duplicated",
            )?;
            require(
                candidate.arms.keys().collect::<BTreeSet<_>>() == self.arms.iter().collect::<BTreeSet<_>>(),
                "caption candidate arms do not match the plan",
            )?;
            require(
                candidate
                    .arms
                    .values()
                    .all(|arm| matches!(arm.precision.as_str(), "fp32" | "quantized")),
                "caption arm precision is unsupported",
            )?;
        }
        require(
            self.images
                .len()
                .checked_mul(self.candidates.len())
                .and_then(|product| product.checked_mul(self.arms.len()))
                .and_then(|product| product.checked_mul(self.repeats as usize))
                == Some(self.expected_generations),
            "caption expected_generations must equal images*candidates*arms*repeats",
        )?;
        Ok(())
    }

    pub fn sha256(&self) -> Result<String> {
        Ok(crate::digest(&serde_json::to_vec(self)?))
    }
}

impl CaptionReport {
    pub fn validate(&self, plan: &CaptionPlan) -> Result<()> {
        plan.validate()?;
        require(
            self.schema == 1 && !self.qualified,
            "caption report must remain unqualified",
        )?;
        require(self.plan_sha256 == plan.sha256()?, "caption plan digest mismatch")?;
        require(self.runtime == plan.runtime, "caption runtime identity mismatch")?;
        require(
            self.rows.len() == plan.expected_generations,
            "caption generation count mismatch",
        )?;
        let mut seen = BTreeSet::new();
        for row in &self.rows {
            require(
                plan.images.iter().any(|image| image.id == row.image),
                "caption row names an unexpected image",
            )?;
            let candidate = plan
                .candidates
                .iter()
                .find(|candidate| candidate.id == row.candidate)
                .ok_or("caption row names an unexpected candidate")?;
            require(
                candidate.arms.contains_key(&row.arm) && (1..=plan.repeats).contains(&row.repeat),
                "caption row names an unexpected arm or repeat",
            )?;
            require(
                seen.insert((&row.image, &row.candidate, &row.arm, row.repeat)),
                "duplicate caption row",
            )?;
            require(
                row.inference == "completed" && row.elapsed_ms.is_finite() && row.elapsed_ms >= 0.0,
                "caption inference must complete with a measured duration",
            )?;
            require(
                row.model.get("format").and_then(|value| value.as_str()).is_some()
                    && row.model.get("precision").and_then(|value| value.as_str()).is_some(),
                "caption row lacks the prepared model identity",
            )?;
        }
        Ok(())
    }
}
