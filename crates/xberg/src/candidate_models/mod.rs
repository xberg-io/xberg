//! Canonical inventory of experimental candidate ONNX models staged on Hugging Face repositories
//! owned by `xberg-io`.
//!
//! Every entry is an unmodified copy of a pinned upstream export, pinned here by the immutable
//! commit of the owned repository plus the SHA-256 of every file. No entry is a default,
//! a supported preset or a bundling decision: each stays experimental until the qualification in
//! xberg-io/xberg#2334 passes. The inventory only names and pins artifacts; consumers acquire
//! them through the shared model download and cache infrastructure.
//!
//! The NLI-MiniLM2 candidate carries a license caveat: the cross-encoder declares Apache-2.0, but
//! its base model `nreimers/MiniLMv2-L6-H768-distilled-from-RoBERTa-Large` publishes no license on
//! its card (the work derives from `microsoft/unilm`, MIT). Resolve that before any qualified use.
//!
//! The module is crate-private: the inventory is not a stable public API.

use std::path::Path;

/// Commit of `xberg-io/classification-models` every classification candidate is pinned to.
pub const CLASSIFICATION_MODELS_REVISION: &str = "a29f2096c723fd39c45b7c21690ea0318866bd0b";
/// Commit of `xberg-io/captioning-models` every captioning candidate is pinned to.
pub const CAPTIONING_MODELS_REVISION: &str = "7b8359d8ab0086922b91b7dfde6577c1a30f7442";

/// `<sha256>  <repo-relative path>` lines for `xberg-io/classification-models`.
pub(crate) const CLASSIFICATION_SHA256_MANIFEST: &str = include_str!("classification.sha256sum");
/// `<sha256>  <repo-relative path>` lines for `xberg-io/captioning-models`.
pub(crate) const CAPTIONING_SHA256_MANIFEST: &str = include_str!("captioning.sha256sum");

/// Task a candidate model is evaluated for.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
#[cfg_attr(alef, alef(skip))]
pub enum CandidateTask {
    /// Zero-shot classification through natural-language inference.
    ZeroShotClassification,
    /// Image captioning.
    ImageCaptioning,
}

/// Qualification state of a candidate. There is deliberately no supported variant yet.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
#[cfg_attr(alef, alef(skip))]
pub enum CandidateStatus {
    /// Staged for evaluation only; not qualified, not a default, not bundled.
    ExperimentalNotQualified,
}

/// One pinned file of a candidate model.
#[derive(Debug, Clone, PartialEq, Eq)]
#[cfg_attr(alef, alef(skip))]
pub struct CandidateFile {
    /// Path relative to the owned repository root.
    pub path: String,
    /// Lowercase hexadecimal SHA-256 of the file.
    pub sha256: String,
}

/// A candidate model staged on an owned Hugging Face repository.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
#[cfg_attr(alef, alef(skip))]
pub struct CandidateModel {
    /// Stable identifier, equal to the directory name in the owned repository.
    pub id: &'static str,
    /// Task the model is evaluated for.
    pub task: CandidateTask,
    /// Qualification state.
    pub status: CandidateStatus,
    /// Owned repository in `owner/name` form.
    pub repository: &'static str,
    /// Immutable commit of the owned repository.
    pub revision: &'static str,
    /// Upstream repository the files were copied from.
    pub upstream_repository: &'static str,
    /// Immutable commit of the upstream repository.
    pub upstream_revision: &'static str,
    /// SPDX license identifier of the upstream weights, with any caveat in parentheses.
    pub license: &'static str,
    /// Files, relative to the model directory, that every inference path needs. Graph alternatives
    /// (fp32 and quantized variants) are pinned in the manifests but not listed here; the NLI
    /// descriptor selects its graph separately because the fp32 graph exceeds the adapter's cap.
    pub required_files: &'static [&'static str],
    pub(crate) sha256_manifest: &'static str,
}

const NLI_MINILM2_L6_H768: CandidateModel = CandidateModel {
    id: "nli-minilm2-l6-h768",
    task: CandidateTask::ZeroShotClassification,
    status: CandidateStatus::ExperimentalNotQualified,
    repository: "xberg-io/classification-models",
    revision: CLASSIFICATION_MODELS_REVISION,
    upstream_repository: "cross-encoder/nli-MiniLM2-L6-H768",
    upstream_revision: "b95119ce93d3e065de6214e38cd4a97b0f2f2c6d",
    license: "Apache-2.0 (base model license unstated)",
    required_files: &[
        "tokenizer.json",
        "config.json",
        "tokenizer_config.json",
        "special_tokens_map.json",
    ],
    sha256_manifest: CLASSIFICATION_SHA256_MANIFEST,
};

const SMOLVLM_COMMON: [&str; 9] = [
    "onnx/vision_encoder.onnx",
    "onnx/embed_tokens.onnx",
    "onnx/decoder_model_merged.onnx",
    "tokenizer.json",
    "tokenizer_config.json",
    "preprocessor_config.json",
    "processor_config.json",
    "config.json",
    "generation_config.json",
];

const SMOLVLM_256M_INSTRUCT: CandidateModel = CandidateModel {
    id: "smolvlm-256m-instruct",
    task: CandidateTask::ImageCaptioning,
    status: CandidateStatus::ExperimentalNotQualified,
    repository: "xberg-io/captioning-models",
    revision: CAPTIONING_MODELS_REVISION,
    upstream_repository: "HuggingFaceTB/SmolVLM-256M-Instruct",
    upstream_revision: "7e3e67edbbed1bf9888184d9df282b700a323964",
    license: "Apache-2.0",
    required_files: &SMOLVLM_COMMON,
    sha256_manifest: CAPTIONING_SHA256_MANIFEST,
};

const SMOLVLM_500M_INSTRUCT: CandidateModel = CandidateModel {
    id: "smolvlm-500m-instruct",
    task: CandidateTask::ImageCaptioning,
    status: CandidateStatus::ExperimentalNotQualified,
    repository: "xberg-io/captioning-models",
    revision: CAPTIONING_MODELS_REVISION,
    upstream_repository: "HuggingFaceTB/SmolVLM-500M-Instruct",
    upstream_revision: "a7da5b986cb59b408707209984f360a5f4ad7e47",
    license: "Apache-2.0",
    required_files: &SMOLVLM_COMMON,
    sha256_manifest: CAPTIONING_SHA256_MANIFEST,
};

const FLORENCE_2_BASE: CandidateModel = CandidateModel {
    id: "florence-2-base",
    task: CandidateTask::ImageCaptioning,
    status: CandidateStatus::ExperimentalNotQualified,
    repository: "xberg-io/captioning-models",
    revision: CAPTIONING_MODELS_REVISION,
    upstream_repository: "onnx-community/Florence-2-base",
    upstream_revision: "d59e079711c57174f29265539fb4cc9f0f335916",
    license: "MIT",
    required_files: &[
        "onnx/vision_encoder.onnx",
        "onnx/embed_tokens.onnx",
        "onnx/encoder_model.onnx",
        "onnx/decoder_model_merged.onnx",
        "tokenizer.json",
        "tokenizer_config.json",
        "preprocessor_config.json",
        "processor_config.json",
        "config.json",
        "generation_config.json",
        "added_tokens.json",
    ],
    sha256_manifest: CAPTIONING_SHA256_MANIFEST,
};

/// Every staged candidate, none of them a default.
pub const CANDIDATE_MODELS: [CandidateModel; 4] = [
    NLI_MINILM2_L6_H768,
    SMOLVLM_256M_INSTRUCT,
    SMOLVLM_500M_INSTRUCT,
    FLORENCE_2_BASE,
];

fn is_commit_hash(value: &str) -> bool {
    value.len() == 40 && value.bytes().all(|b| matches!(b, b'0'..=b'9' | b'a'..=b'f'))
}

impl CandidateModel {
    /// Looks a candidate up by its identifier.
    pub fn find(id: &str) -> Option<&'static CandidateModel> {
        CANDIDATE_MODELS.iter().find(|model| model.id == id)
    }

    /// Fails unless the owned and upstream revisions are immutable forty-digit commit hashes.
    pub fn validate_revisions(&self) -> Result<(), String> {
        for (label, revision) in [("owned", self.revision), ("upstream", self.upstream_revision)] {
            if !is_commit_hash(revision) {
                return Err(format!(
                    "{} {label} revision {revision:?} is not an immutable commit hash",
                    self.id
                ));
            }
        }
        Ok(())
    }

    /// Every pinned file of this model, with its SHA-256, sorted by path.
    pub fn files(&self) -> Result<Vec<CandidateFile>, String> {
        let prefix = format!("{}/", self.id);
        let mut files: Vec<CandidateFile> = crate::model_download::parse_sha256_manifest(self.sha256_manifest)?
            .into_iter()
            .filter(|(path, _)| path.starts_with(&prefix))
            .map(|(path, sha256)| CandidateFile { path, sha256 })
            .collect();
        if files.is_empty() {
            return Err(format!("{} has no pinned files", self.id));
        }
        files.sort_by(|a, b| a.path.cmp(&b.path));
        Ok(files)
    }

    /// Verifies `local` against the SHA-256 pin for `relative` (a path inside the model directory)
    /// with the shared download check. A corrupted, truncated or other-revision file is rejected.
    pub fn verify_file(&self, relative: &str, local: &Path) -> Result<(), String> {
        let path = format!("{}/{relative}", self.id);
        let pin = self
            .files()?
            .into_iter()
            .find(|file| file.path == path)
            .ok_or_else(|| format!("{path} is not pinned for {}", self.id))?;
        crate::model_download::verify_sha256(local, &pin.sha256, &pin.path)
    }
}

/// Graph the NLI descriptor selects on this architecture. The fp32 graph (328,649,957 bytes) exceeds
/// the adapter's graph size cap, so only the dynamically quantized graph for the host ISA is usable.
/// x86_64 selects the AVX2 graph without checking the CPU for AVX2 support.
#[cfg(feature = "classification-onnx")]
pub(crate) const fn nli_minilm2_l6_h768_graph() -> Option<&'static str> {
    if cfg!(target_arch = "aarch64") {
        Some("model_qint8_arm64.onnx")
    } else if cfg!(target_arch = "x86_64") {
        Some("model_quint8_avx2.onnx")
    } else {
        None
    }
}

/// The Rust-only NLI classifier descriptor for the NLI-MiniLM2 candidate, usable as
/// [`crate::local_classification::LocalClassificationConfig::model`]. Logits are ordered
/// contradiction, entailment, neutral. Fails on an architecture without a quantized graph that fits
/// the adapter's caps rather than substituting the oversized fp32 graph.
#[cfg(feature = "classification-onnx")]
pub(crate) fn nli_minilm2_l6_h768_classifier() -> Result<crate::local_classification::LocalNliModel, String> {
    nli_classifier(&NLI_MINILM2_L6_H768)
}

/// Builds the descriptor from `model`'s pins, so a test can substitute stand-in pins and still go
/// through the production graph selection.
#[cfg(feature = "classification-onnx")]
pub(crate) fn nli_classifier(model: &CandidateModel) -> Result<crate::local_classification::LocalNliModel, String> {
    let graph = nli_minilm2_l6_h768_graph().ok_or_else(|| {
        format!(
            "{} has no quantized graph within the adapter's size cap for {}",
            model.id,
            std::env::consts::ARCH
        )
    })?;
    nli_classifier_for(model, graph)
}

#[cfg(feature = "classification-onnx")]
pub(crate) fn nli_classifier_for(
    model: &CandidateModel,
    graph: &str,
) -> Result<crate::local_classification::LocalNliModel, String> {
    let files = model.files()?;
    let mut manifest = String::new();
    for required in std::iter::once(graph).chain(model.required_files.iter().copied()) {
        let path = format!("{}/{required}", model.id);
        let pin = files
            .iter()
            .find(|file| file.path == path)
            .ok_or_else(|| format!("{path} is not pinned"))?;
        manifest.push_str(&format!("{}  {path}\n", pin.sha256));
    }
    Ok(crate::local_classification::LocalNliModel {
        repository: model.repository.to_owned(),
        revision: model.revision.to_owned(),
        model_file: format!("{}/{graph}", model.id),
        external_files: vec![],
        sha256_manifest: manifest,
        output_name: "logits".to_owned(),
        entailment_index: 1,
        contradiction_index: 0,
    })
}

#[cfg(test)]
mod tests;
