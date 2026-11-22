//! Offline artifact inspection for the experimental classifier.
//!
//! Every path resolves from the local Hugging Face cache; neither building a report nor reading
//! one touches the network. Presence is decided by the pinned exact size, so a truncated or
//! swapped file reports a mismatch. Preparation re-verifies every SHA-256 before a session loads.

use std::path::Path;

use serde::{Deserialize, Serialize};

use super::policy::invalid;
use super::{LocalClassificationBackend, LocalClassificationPrecision, LocalNliModel};

/// Where one pinned classification artifact stands in the local cache.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
#[cfg_attr(alef, alef(skip))]
pub enum LocalClassificationArtifactState {
    /// A cached file has the pinned exact size. Its bytes are verified again during preparation.
    Staged,
    /// No cached file exists for the pinned revision; preparation cannot start offline.
    Missing,
    /// A cached file exists but its size differs from the pin, so it is not the pinned artifact.
    SizeMismatch,
}

/// One pinned artifact with its manifest identity and local cache state.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[cfg_attr(alef, alef(skip))]
pub struct LocalClassificationArtifact {
    /// Repository-relative path.
    pub path: String,
    /// Exact pinned size in bytes.
    pub size_bytes: u64,
    /// Pinned lowercase hexadecimal SHA-256.
    pub sha256: String,
    /// Local cache state.
    pub state: LocalClassificationArtifactState,
}

impl LocalClassificationArtifact {
    /// Whether this artifact is present with its pinned size.
    pub const fn staged(&self) -> bool {
        matches!(self.state, LocalClassificationArtifactState::Staged)
    }
}

/// Offline inventory and cache state for one classification model.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[cfg_attr(alef, alef(skip))]
pub struct LocalClassificationArtifactReport {
    /// Candidate identifier when the descriptor came from the canonical inventory.
    pub candidate: Option<String>,
    /// Selected backend; equal to `effective_backend` because no substitution is possible.
    pub selected_backend: LocalClassificationBackend,
    /// Backend that runs on this host.
    pub effective_backend: LocalClassificationBackend,
    /// Graph quantization of the descriptor.
    pub precision: LocalClassificationPrecision,
    /// Owned repository.
    pub repository: String,
    /// Pinned immutable revision.
    pub revision: String,
    /// Repository-relative graph path.
    pub model_file: String,
    /// Digest binding the descriptor's manifests, graph and label semantics.
    pub manifest_sha256: String,
    /// Total pinned bytes of every artifact.
    pub total_bytes: u64,
    /// Bytes of the artifacts that are staged with their pinned size.
    pub staged_bytes: u64,
    /// Every pinned artifact, in path order.
    pub artifacts: Vec<LocalClassificationArtifact>,
}

impl LocalClassificationArtifactReport {
    /// Whether every pinned artifact is staged.
    pub fn complete(&self) -> bool {
        self.artifacts.iter().all(LocalClassificationArtifact::staged)
    }

    /// Whether every pinned artifact is missing.
    pub fn empty(&self) -> bool {
        self.artifacts
            .iter()
            .all(|artifact| artifact.state == LocalClassificationArtifactState::Missing)
    }

    /// Count of staged artifacts.
    pub fn staged(&self) -> usize {
        self.artifacts.iter().filter(|artifact| artifact.staged()).count()
    }

    /// Report for a staged candidate, selected exactly as [`LocalNliModel::staged`] selects it.
    ///
    /// # Errors
    ///
    /// Fails for an unknown candidate, a candidate of another task, or an unreadable cache.
    pub fn for_candidate(
        id: &str,
        precision: LocalClassificationPrecision,
        cache_directory: Option<&str>,
    ) -> crate::Result<Self> {
        let model = LocalNliModel::staged(id, precision)?;
        let mut report = Self::for_model(&model, cache_directory)?;
        report.candidate = Some(id.to_owned());
        Ok(report)
    }

    /// Report for a caller-pinned descriptor.
    pub fn for_model(model: &LocalNliModel, cache_directory: Option<&str>) -> crate::Result<Self> {
        use sha2::Digest as _;
        let client = crate::model_download::hf_client(cache_directory.map(Path::new)).map_err(invalid)?;
        let pins = super::artifacts::pinned_files(model)?;
        // The descriptor has no precision field; the pinned graph names are the selection record
        // (`*_int8.onnx` for dynamic quantization, `*qint8*`/`*quint8*` for the NLI exports).
        let quantized = pins.iter().any(|pin| {
            let name = pin.path.rsplit('/').next().unwrap_or(&pin.path);
            name.ends_with("_int8.onnx")
                || (name.ends_with(".onnx") && (name.contains("qint8") || name.contains("quint8")))
        });
        let precision = if quantized {
            LocalClassificationPrecision::Quantized
        } else {
            LocalClassificationPrecision::Fp32
        };
        let mut artifacts = Vec::with_capacity(pins.len());
        let mut total_bytes = 0u64;
        let mut staged_bytes = 0u64;
        for pin in pins {
            total_bytes = total_bytes.saturating_add(pin.size_bytes);
            let state = match crate::model_download::hf_cached_revision_with_client(
                &client,
                &model.repository,
                &pin.path,
                Some(&model.revision),
            )
            .map_err(invalid)?
            {
                None => LocalClassificationArtifactState::Missing,
                Some(path) => match std::fs::metadata(&path) {
                    Ok(metadata) if metadata.is_file() && metadata.len() == pin.size_bytes => {
                        staged_bytes = staged_bytes.saturating_add(pin.size_bytes);
                        LocalClassificationArtifactState::Staged
                    }
                    Ok(_) => LocalClassificationArtifactState::SizeMismatch,
                    Err(error) if error.kind() == std::io::ErrorKind::NotFound => {
                        LocalClassificationArtifactState::Missing
                    }
                    Err(error) => {
                        return Err(invalid(format!("Local NLI artifact {}: {error}", pin.path)));
                    }
                },
            };
            artifacts.push(LocalClassificationArtifact {
                path: pin.path,
                size_bytes: pin.size_bytes,
                sha256: pin.sha256,
                state,
            });
        }
        Ok(Self {
            candidate: None,
            selected_backend: LocalClassificationBackend::OnnxCpu,
            effective_backend: LocalClassificationBackend::OnnxCpu,
            precision,
            repository: model.repository.clone(),
            revision: model.revision.clone(),
            model_file: model.model_file.clone(),
            manifest_sha256: hex::encode(sha2::Sha256::digest(model.sha256_manifest.as_bytes())),
            total_bytes,
            staged_bytes,
            artifacts,
        })
    }
}
