//! Canonical inventory of experimental candidate ONNX models staged on Hugging Face repositories
//! owned by `xberg-io`.
//!
//! Every entry records the immutable commit of the owned repository plus the SHA-256, size and
//! source of every file. No entry is a default, a supported preset or a bundling decision: each
//! stays experimental until the qualification in xberg-io/xberg#2334 passes. The inventory only
//! names and pins artifacts; consumers acquire them through the shared model download and cache
//! infrastructure.
//!
//! [`inventory.json`](inventory.json) is the only pin store. [`CandidateTask::models`] lists the
//! evaluation candidates for one task, and [`CandidateModel::files_for`] selects the files for an
//! explicit precision, so a consumer downloads neither another task's models nor an alternative
//! quantization by accident.
//!
//! The NLI-MiniLM2 candidate carries a license caveat: the cross-encoder declares Apache-2.0, but
//! its base model `nreimers/MiniLMv2-L6-H768-distilled-from-RoBERTa-Large` publishes no license on
//! its card (the work derives from `microsoft/unilm`, MIT). The caveat is recorded on the model and
//! in [`CandidateModel::license_caveat`]; the owned model directory has no notice of its own
//! because adding one would require a new owned revision. Resolve it before any qualified use.
//!
//! The module is crate-private: the inventory is not a stable public API.

use std::path::{Component, Path};
use std::sync::OnceLock;

use serde::Deserialize;

/// Commit of `xberg-io/classification-models` every classification candidate is pinned to.
pub const CLASSIFICATION_MODELS_REVISION: &str = "a29f2096c723fd39c45b7c21690ea0318866bd0b";
/// Commit of `xberg-io/captioning-models` every captioning candidate is pinned to.
pub const CAPTIONING_MODELS_REVISION: &str = "7b8359d8ab0086922b91b7dfde6577c1a30f7442";

/// Canonical per-file inventory: repository, revision, license, provenance, SHA-256 and size.
const INVENTORY_JSON: &str = include_str!("inventory.json");
const INVENTORY_SCHEMA_VERSION: u32 = 2;

/// Task a candidate model is evaluated for.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Deserialize)]
#[serde(rename_all = "snake_case")]
#[cfg_attr(alef, alef(skip))]
pub enum CandidateTask {
    /// Zero-shot classification through natural-language inference.
    ZeroShotClassification,
    /// Image captioning.
    ImageCaptioning,
}

/// Qualification state of a candidate. There is deliberately no supported variant yet.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Deserialize)]
#[serde(rename_all = "snake_case")]
#[cfg_attr(alef, alef(skip))]
pub enum CandidateStatus {
    /// Staged for evaluation only; not qualified, not a default, not bundled.
    ExperimentalNotQualified,
}

/// Graph set a consumer selects explicitly. Captioning pins fp32 and int8 exports; the NLI adapter
/// only accepts the host's quantized graph because the fp32 export exceeds its size cap.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
#[cfg_attr(alef, alef(skip))]
pub enum CandidatePrecision {
    /// Full-precision graphs from the upstream export.
    Fp32,
    /// Int8/qint8 dynamic-quantization graphs from the upstream export.
    Quantized,
}

/// One pinned file of a candidate model.
#[derive(Debug, Clone, PartialEq, Eq)]
#[cfg_attr(alef, alef(skip))]
pub struct CandidateFile {
    /// Path relative to the owned repository root.
    pub path: String,
    /// Lowercase hexadecimal SHA-256 of the file.
    pub sha256: String,
    /// Exact size of the file in bytes.
    pub size_bytes: u64,
    /// Owned repository the file is fetched from.
    pub repository: String,
    /// Immutable commit the file is fetched at.
    pub revision: String,
    /// SPDX license identifier of the bytes in this file.
    pub license: String,
    /// Where the bytes came from: `<repo>@<revision>:<path>` for a copied upstream file, or
    /// `spdx:<id>` for a file created at staging.
    pub source: String,
}

impl CandidateFile {
    /// Stable `repository@revision:path` label used in every verification error.
    pub fn label(&self) -> String {
        format!("{}@{}:{}", self.repository, self.revision, self.path)
    }

    /// Verifies a local copy of this file: size first, then SHA-256. A missing, truncated,
    /// corrupted or other-revision file fails with an error naming the pinned source.
    pub fn verify(&self, local: &Path) -> Result<(), String> {
        let metadata = match std::fs::metadata(local) {
            Ok(metadata) => metadata,
            Err(error) if error.kind() == std::io::ErrorKind::NotFound => {
                return Err(format!(
                    "Missing pinned artifact {} at {}",
                    self.label(),
                    local.display()
                ));
            }
            Err(error) => {
                return Err(format!(
                    "Failed to inspect pinned artifact {} at {}: {error}",
                    self.label(),
                    local.display()
                ));
            }
        };
        if !metadata.is_file() {
            return Err(format!(
                "Pinned artifact {} at {} is not a regular file",
                self.label(),
                local.display()
            ));
        }
        if metadata.len() != self.size_bytes {
            return Err(format!(
                "Size mismatch for {}: expected {} bytes, got {}",
                self.label(),
                self.size_bytes,
                metadata.len()
            ));
        }
        verify_sha256(local, &self.sha256, &self.label())
    }
}

#[cfg(model_download)]
fn verify_sha256(path: &Path, expected: &str, label: &str) -> Result<(), String> {
    crate::model_download::verify_sha256(path, expected, label)
}

/// Default-feature unit tests compile this module through `any(test, model_download)` while the
/// shared downloader stays behind its own cfg; hash the same bytes here so the inventory checks
/// remain provable without that feature.
#[cfg(all(test, not(model_download)))]
fn verify_sha256(path: &Path, expected: &str, label: &str) -> Result<(), String> {
    use sha2::{Digest, Sha256};
    let bytes = std::fs::read(path).map_err(|error| format!("Failed to open file for checksum: {error}"))?;
    let actual = hex::encode(Sha256::digest(bytes));
    if actual != expected {
        return Err(format!(
            "Checksum mismatch for {label}: expected {expected}, got {actual}"
        ));
    }
    Ok(())
}

/// One file of the checked-in inventory document.
#[derive(Debug, Clone, Deserialize)]
struct InventoryFile {
    path: String,
    sha256: String,
    size_bytes: u64,
    source: String,
}

/// One model of the checked-in inventory document.
#[derive(Debug, Clone, Deserialize)]
struct InventoryModel {
    id: String,
    task: CandidateTask,
    status: CandidateStatus,
    repository: String,
    revision: String,
    upstream_repository: String,
    upstream_revision: String,
    license: String,
    #[serde(default)]
    license_caveat: Option<String>,
    #[serde(default)]
    provenance: String,
    files: Vec<InventoryFile>,
}

#[derive(Debug, Deserialize)]
struct InventoryDocument {
    schema_version: u32,
    models: Vec<InventoryModel>,
}

/// Parses and validates the checked-in inventory once. Fails closed on a malformed document so a
/// corrupted or hand-edited pin cannot silently become an unpinned artifact.
fn inventory() -> Result<&'static InventoryDocument, String> {
    static INVENTORY: OnceLock<Result<InventoryDocument, String>> = OnceLock::new();
    INVENTORY
        .get_or_init(|| inventory_from(INVENTORY_JSON))
        .as_ref()
        .map_err(Clone::clone)
}

fn inventory_from(content: &str) -> Result<InventoryDocument, String> {
    let document: InventoryDocument =
        serde_json::from_str(content).map_err(|error| format!("candidate inventory is malformed: {error}"))?;
    validate_inventory(&document)?;
    Ok(document)
}

fn validate_inventory(document: &InventoryDocument) -> Result<(), String> {
    if document.schema_version != INVENTORY_SCHEMA_VERSION {
        return Err(format!(
            "candidate inventory schema {} is unsupported; expected {}",
            document.schema_version, INVENTORY_SCHEMA_VERSION
        ));
    }
    if document.models.is_empty() {
        return Err("candidate inventory declares no models".to_owned());
    }
    let mut model_ids = std::collections::BTreeSet::new();
    let mut file_paths = std::collections::BTreeSet::new();
    for model in &document.models {
        if model.id.is_empty() || !model_ids.insert(model.id.as_str()) {
            return Err(format!(
                "candidate inventory has an empty or duplicate model id: {:?}",
                model.id
            ));
        }
        for (field, value) in [
            ("repository", &model.repository),
            ("revision", &model.revision),
            ("upstream repository", &model.upstream_repository),
            ("upstream revision", &model.upstream_revision),
            ("license", &model.license),
            ("provenance", &model.provenance),
        ] {
            if value.trim().is_empty() {
                return Err(format!("candidate inventory model {} has an empty {field}", model.id));
            }
        }
        if !is_repository_id(&model.repository) || !is_repository_id(&model.upstream_repository) {
            return Err(format!(
                "candidate inventory model {} has a malformed repository",
                model.id
            ));
        }
        if !is_commit_hash(&model.revision) || !is_commit_hash(&model.upstream_revision) {
            return Err(format!(
                "candidate inventory model {} revision is not an immutable commit hash",
                model.id
            ));
        }
        if model.files.is_empty() {
            return Err(format!("candidate inventory model {} has no files", model.id));
        }
        for file in &model.files {
            if !file.path.starts_with(&format!("{}/", model.id)) || !is_artifact_path(&file.path) {
                return Err(format!(
                    "candidate inventory file {} does not belong to model {}",
                    file.path, model.id
                ));
            }
            if !file_paths.insert(file.path.as_str()) {
                return Err(format!("candidate inventory has a duplicate file path: {}", file.path));
            }
            if !is_sha256(&file.sha256) {
                return Err(format!(
                    "candidate inventory file {} has no lowercase SHA-256",
                    file.path
                ));
            }
            if file.size_bytes == 0 {
                return Err(format!("candidate inventory file {} has no size", file.path));
            }
            if !is_source(&file.source) {
                return Err(format!("candidate inventory file {} has a malformed source", file.path));
            }
        }
    }
    Ok(())
}

fn is_sha256(value: &str) -> bool {
    value.len() == 64 && value.bytes().all(|byte| matches!(byte, b'0'..=b'9' | b'a'..=b'f'))
}

fn is_commit_hash(value: &str) -> bool {
    value.len() == 40 && value.bytes().all(|byte| matches!(byte, b'0'..=b'9' | b'a'..=b'f'))
}

fn is_repository_id(value: &str) -> bool {
    value.split_once('/').is_some_and(|(owner, name)| {
        [owner, name].into_iter().all(|part| {
            !part.is_empty()
                && !matches!(part, "." | "..")
                && part
                    .bytes()
                    .all(|byte| byte.is_ascii_alphanumeric() || b"-_.".contains(&byte))
        })
    })
}

/// A contained repository-relative file path: no absolute root, no empty/`.`/`..` component and no
/// trailing slash.
fn is_artifact_path(path: &str) -> bool {
    !path.is_empty()
        && !path.ends_with('/')
        && !path.contains(['\\', ':'])
        && !Path::new(path).is_absolute()
        && Path::new(path)
            .components()
            .all(|component| matches!(component, Component::Normal(_)))
}

/// `<repo>@<40-hex-revision>:<path>` for a copied upstream file, or `spdx:<identifier>` for a file
/// created at staging.
fn is_source(source: &str) -> bool {
    if let Some(identifier) = source.strip_prefix("spdx:") {
        return !identifier.is_empty() && !identifier.bytes().any(|byte| byte.is_ascii_control());
    }
    source
        .split_once('@')
        .and_then(|(repository, rest)| {
            rest.split_once(':')
                .map(|(revision, path)| (repository, revision, path))
        })
        .is_some_and(|(repository, revision, path)| {
            is_repository_id(repository) && is_commit_hash(revision) && is_artifact_path(path)
        })
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
    /// SPDX license identifier of the upstream weights.
    pub license: &'static str,
    /// License caveat that blocks qualified use, when the upstream grant is incomplete.
    pub license_caveat: Option<&'static str>,
    /// Files, relative to the model directory, that every inference path needs. Graphs are
    /// selected separately by [`CandidateModel::files_for`]: the NLI fp32 graph exceeds the
    /// adapter's cap, and the captioning exports ship fp32 and int8 alternatives.
    pub required_files: &'static [&'static str],
}

const NLI_MINILM2_L6_H768: CandidateModel = CandidateModel {
    id: "nli-minilm2-l6-h768",
    task: CandidateTask::ZeroShotClassification,
    status: CandidateStatus::ExperimentalNotQualified,
    repository: "xberg-io/classification-models",
    revision: CLASSIFICATION_MODELS_REVISION,
    upstream_repository: "cross-encoder/nli-MiniLM2-L6-H768",
    upstream_revision: "b95119ce93d3e065de6214e38cd4a97b0f2f2c6d",
    license: "Apache-2.0",
    license_caveat: Some(
        "The cross-encoder declares Apache-2.0; its base model nreimers/MiniLMv2-L6-H768-distilled-from-RoBERTa-Large publishes no license on its card (microsoft/unilm origin, MIT). Resolve before qualified use.",
    ),
    required_files: &[
        "tokenizer.json",
        "config.json",
        "tokenizer_config.json",
        "special_tokens_map.json",
    ],
};

const SMOLVLM_COMMON: [&str; 6] = [
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
    license_caveat: None,
    required_files: &SMOLVLM_COMMON,
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
    license_caveat: None,
    required_files: &SMOLVLM_COMMON,
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
    license_caveat: None,
    required_files: &[
        "tokenizer.json",
        "tokenizer_config.json",
        "preprocessor_config.json",
        "processor_config.json",
        "config.json",
        "generation_config.json",
        "added_tokens.json",
    ],
};

/// Every staged candidate, none of them a default.
pub const CANDIDATE_MODELS: [CandidateModel; 4] = [
    NLI_MINILM2_L6_H768,
    SMOLVLM_256M_INSTRUCT,
    SMOLVLM_500M_INSTRUCT,
    FLORENCE_2_BASE,
];

impl CandidateTask {
    /// Every staged evaluation candidate for this task, in inventory order. This is not a
    /// supported selection: only #2334 qualification can mark a variant supported, and the caller
    /// still chooses a [`CandidatePrecision`] through [`CandidateModel::files_for`].
    pub fn models(self) -> Vec<&'static CandidateModel> {
        CANDIDATE_MODELS.iter().filter(|model| model.task == self).collect()
    }
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

    /// The inventory record for this model, validated against the descriptor above. A record that
    /// names a different repository, revision, task, status, license or upstream fails closed.
    fn inventory_model(&self) -> Result<&'static InventoryModel, String> {
        self.validate_revisions()?;
        let document = inventory()?;
        let entry = document
            .models
            .iter()
            .find(|entry| entry.id == self.id)
            .ok_or_else(|| format!("{} is not recorded in the candidate inventory", self.id))?;
        let mismatches = [
            ("task", format!("{:?}", entry.task), format!("{:?}", self.task)),
            ("status", format!("{:?}", entry.status), format!("{:?}", self.status)),
            ("repository", entry.repository.clone(), self.repository.to_owned()),
            ("revision", entry.revision.clone(), self.revision.to_owned()),
            (
                "upstream repository",
                entry.upstream_repository.clone(),
                self.upstream_repository.to_owned(),
            ),
            (
                "upstream revision",
                entry.upstream_revision.clone(),
                self.upstream_revision.to_owned(),
            ),
            ("license", entry.license.clone(), self.license.to_owned()),
            (
                "license caveat",
                format!("{:?}", entry.license_caveat.as_deref()),
                format!("{:?}", self.license_caveat),
            ),
        ];
        for (field, recorded, expected) in mismatches {
            if recorded != expected {
                return Err(format!(
                    "{} inventory {field} {recorded:?} does not match the descriptor's {expected:?}",
                    self.id
                ));
            }
        }
        Ok(entry)
    }

    /// Every pinned file of this model, with its SHA-256, size, repository, revision, license and
    /// source, sorted by path.
    pub fn files(&self) -> Result<Vec<CandidateFile>, String> {
        let entry = self.inventory_model()?;
        let mut files: Vec<CandidateFile> = entry
            .files
            .iter()
            .map(|file| CandidateFile {
                path: file.path.clone(),
                sha256: file.sha256.clone(),
                size_bytes: file.size_bytes,
                repository: entry.repository.clone(),
                revision: entry.revision.clone(),
                license: entry.license.clone(),
                source: file.source.clone(),
            })
            .collect();
        if files.is_empty() {
            return Err(format!("{} has no pinned files", self.id));
        }
        files.sort_by(|a, b| a.path.cmp(&b.path));
        Ok(files)
    }

    /// The recorded export and quantization provenance: upstream export origin and the concrete
    /// graph variants staged in the owned repository.
    pub fn provenance(&self) -> Result<String, String> {
        Ok(self.inventory_model()?.provenance.clone())
    }

    /// Selects the files a consumer needs for `precision`: the requested graphs plus the
    /// tokenizer/processor/configuration companions and the upstream `LICENSE` and model card.
    /// Quantized alternatives outside `precision` stay optional.
    pub fn files_for(&self, precision: CandidatePrecision) -> Result<Vec<CandidateFile>, String> {
        let files = self.files()?;
        self.select_files(&files, precision)
    }

    fn select_files(
        &self,
        files: &[CandidateFile],
        precision: CandidatePrecision,
    ) -> Result<Vec<CandidateFile>, String> {
        let mut wanted: std::collections::BTreeSet<String> = std::collections::BTreeSet::new();
        match self.task {
            CandidateTask::ZeroShotClassification => {
                let graph = match precision {
                    CandidatePrecision::Quantized => nli_minilm2_l6_h768_graph().ok_or_else(|| {
                        format!(
                            "{} has no quantized graph for {} and the fp32 graph exceeds the adapter's cap",
                            self.id,
                            std::env::consts::ARCH
                        )
                    })?,
                    CandidatePrecision::Fp32 => {
                        return Err(format!(
                            "{} fp32 graph exceeds the adapter's size cap; select CandidatePrecision::Quantized",
                            self.id
                        ));
                    }
                };
                wanted.insert(format!("{}/{graph}", self.id));
            }
            CandidateTask::ImageCaptioning => {
                let graph_prefix = format!("{}/onnx/", self.id);
                for file in files {
                    let Some(relative) = file.path.strip_prefix(&graph_prefix) else {
                        continue;
                    };
                    let quantized = relative.ends_with("_int8.onnx");
                    let selected = match precision {
                        CandidatePrecision::Fp32 => relative.ends_with(".onnx") && !quantized,
                        CandidatePrecision::Quantized => quantized,
                    };
                    if selected {
                        wanted.insert(file.path.clone());
                    }
                }
                if wanted.is_empty() {
                    return Err(format!("{} has no {precision:?} captioning graph", self.id));
                }
            }
        }
        for required in self.required_files {
            wanted.insert(format!("{}/{required}", self.id));
        }
        wanted.insert(format!("{}/LICENSE", self.id));
        wanted.insert(format!("{}/UPSTREAM_README.md", self.id));
        wanted
            .into_iter()
            .map(|path| {
                files
                    .iter()
                    .find(|file| file.path == path)
                    .cloned()
                    .ok_or_else(|| format!("{path} is not pinned for {}", self.id))
            })
            .collect()
    }

    /// Pinned files that are not part of [`Self::files_for`] for `precision`: the alternative
    /// quantization and any other graph a consumer only needs when it deliberately selects it.
    pub fn optional_files(&self, precision: CandidatePrecision) -> Result<Vec<CandidateFile>, String> {
        let files = self.files()?;
        let required: std::collections::BTreeSet<String> = self
            .select_files(&files, precision)?
            .into_iter()
            .map(|file| file.path)
            .collect();
        Ok(files
            .into_iter()
            .filter(|file| !required.contains(&file.path))
            .collect())
    }

    /// Verifies `local` against the size and SHA-256 pin for `relative` (a path inside the model
    /// directory). A corrupted, truncated or missing file is rejected.
    pub fn verify_file(&self, relative: &str, local: &Path) -> Result<(), String> {
        self.verify_downloaded_at(self.repository, self.revision, relative, local)
    }

    /// Verifies a file that a download claims to have fetched from `repository` at `revision`.
    /// A wrong repository or revision fails before any byte is read, so a floating or stale
    /// revision cannot masquerade as the pin.
    pub fn verify_downloaded_at(
        &self,
        repository: &str,
        revision: &str,
        relative: &str,
        local: &Path,
    ) -> Result<(), String> {
        if repository != self.repository {
            return Err(format!(
                "{} was fetched from {repository}, but the inventory pins {}",
                self.id, self.repository
            ));
        }
        if revision != self.revision {
            return Err(format!(
                "{} revision {revision:?} is not the pinned immutable revision {}",
                self.id, self.revision
            ));
        }
        let path = format!("{}/{relative}", self.id);
        let pin = self
            .files()?
            .into_iter()
            .find(|file| file.path == path)
            .ok_or_else(|| format!("{path} is not pinned for {}", self.id))?;
        pin.verify(local)
    }

    /// Verifies a downloaded snapshot root (`{root}/{id}/...`) against every file the selected
    /// precision needs. The first missing, truncated or corrupted artifact fails with its pin.
    pub fn verify_downloaded_set(&self, root: &Path, precision: CandidatePrecision) -> Result<(), String> {
        for file in self.files_for(precision)? {
            file.verify(&root.join(&file.path))?;
        }
        Ok(())
    }
}

/// Graph the NLI descriptor selects on this architecture. The fp32 graph (328,649,957 bytes) exceeds
/// the adapter's graph size cap, so only the dynamically quantized graph for the host ISA is usable.
/// x86_64 selects the AVX2 graph without checking the CPU for AVX2 support.
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

/// Builds the descriptor from the inventory pins, so this is the only place the classifier's
/// SHA-256 and size manifests come from.
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
    nli_descriptor(model, graph, &model.files()?)
}

/// Test seam: the production builder above always reads the inventory; a test can substitute
/// fabricated pins and still exercise the same descriptor assembly.
#[cfg(all(test, feature = "classification-onnx"))]
pub(crate) fn nli_classifier_with_files(
    model: &CandidateModel,
    graph: &str,
    files: &[CandidateFile],
) -> Result<crate::local_classification::LocalNliModel, String> {
    nli_descriptor(model, graph, files)
}

#[cfg(feature = "classification-onnx")]
fn nli_descriptor(
    model: &CandidateModel,
    graph: &str,
    files: &[CandidateFile],
) -> Result<crate::local_classification::LocalNliModel, String> {
    let mut manifest = String::new();
    let mut sizes = String::new();
    for required in std::iter::once(graph).chain(model.required_files.iter().copied()) {
        let path = format!("{}/{required}", model.id);
        let pin = files
            .iter()
            .find(|file| file.path == path)
            .ok_or_else(|| format!("{path} is not pinned"))?;
        manifest.push_str(&format!("{}  {path}\n", pin.sha256));
        sizes.push_str(&format!("{}  {path}\n", pin.size_bytes));
    }
    Ok(crate::local_classification::LocalNliModel {
        repository: model.repository.to_owned(),
        revision: model.revision.to_owned(),
        model_file: format!("{}/{graph}", model.id),
        external_files: vec![],
        sha256_manifest: manifest,
        size_manifest: sizes,
        output_name: "logits".to_owned(),
        entailment_index: 1,
        contradiction_index: 0,
    })
}

#[cfg(test)]
mod tests;
