use serde::{Deserialize, Serialize};
use std::collections::BTreeMap;
use xberg::local_classification::{LocalClassificationConfig, LocalClassificationModelInfo};

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct FrozenPlan {
    pub schema_version: u32,
    pub qualified: bool,
    pub expected_inferences: usize,
    pub repeats: u32,
    pub labels: Vec<String>,
    pub arms: Vec<String>,
    pub provenance: Provenance,
    pub runtime: RuntimeIdentity,
    pub documents: Vec<Document>,
    pub candidates: Vec<Candidate>,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct Provenance {
    pub corpus_receipt_sha256: String,
    pub split_audit_sha256: String,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct RuntimeIdentity {
    pub runner_sha256: String,
    pub dependency_lock_sha256: String,
    pub ort_build_info: String,
    pub os: String,
    pub arch: String,
    pub cpu: String,
    pub threads: usize,
    pub deadline_ms: u64,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum Split {
    Training,
    Calibration,
    HeldOut,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct Document {
    pub id: String,
    pub family: String,
    pub split: Split,
    pub source_sha256: String,
    pub text: String,
    pub text_sha256: String,
    pub gold: Vec<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct Candidate {
    pub id: String,
    pub variants: BTreeMap<String, Variant>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct Variant {
    pub source_revision: String,
    pub export_recipe_sha256: String,
    pub license_receipt_sha256: String,
    pub config: LocalClassificationConfig,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct RunReport {
    pub schema_version: u32,
    pub qualified: bool,
    pub plan_sha256: String,
    pub runtime: RuntimeIdentity,
    pub rows: Vec<Prediction>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct Prediction {
    pub document: String,
    pub candidate: String,
    pub arm: String,
    pub repeat: u32,
    pub inference: String,
    pub elapsed_ms: f64,
    pub labels: Vec<xberg::ClassificationLabel>,
    pub model: LocalClassificationModelInfo,
}
