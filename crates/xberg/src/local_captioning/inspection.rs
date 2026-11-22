//! Offline artifact inspection for the experimental captioner.
//!
//! Neither building a report nor reading one touches the network: every path is resolved from the
//! local Hugging Face cache. Presence is decided by the pinned exact size, so a truncated or
//! swapped file reports as a mismatch instead of a staged artifact. The full SHA-256 pass runs
//! during [`super::LocalImageCaptioner::prepare`], which is the only step that loads bytes.

use std::path::Path;

use serde::{Deserialize, Serialize};

use super::policy::invalid;
use super::{LocalCaptioningBackend, LocalCaptioningModel, LocalCaptioningPrecision};

/// Where one pinned captioning artifact stands in the local cache.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
#[cfg_attr(alef, alef(skip))]
pub enum LocalCaptioningArtifactState {
    /// A cached file has the pinned exact size. Its bytes are verified again during preparation.
    Staged,
    /// No cached file exists for the pinned revision; preparation cannot start offline.
    Missing,
    /// A cached file exists but its size differs from the pin, so it is not the pinned artifact.
    SizeMismatch,
}

/// One pinned artifact with its inventory identity and local cache state.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[cfg_attr(alef, alef(skip))]
pub struct LocalCaptioningArtifact {
    /// Repository-relative path.
    pub path: String,
    /// Exact pinned size in bytes.
    pub size_bytes: u64,
    /// Pinned lowercase hexadecimal SHA-256.
    pub sha256: String,
    /// Local cache state.
    pub state: LocalCaptioningArtifactState,
}

impl LocalCaptioningArtifact {
    /// Whether this artifact is present with its pinned size.
    pub const fn staged(&self) -> bool {
        matches!(self.state, LocalCaptioningArtifactState::Staged)
    }
}

/// Offline inventory and cache state for one captioning model.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[cfg_attr(alef, alef(skip))]
pub struct LocalCaptioningArtifactReport {
    /// Candidate identifier when the descriptor came from the canonical inventory.
    pub candidate: Option<String>,
    /// Selected backend; equal to `effective_backend` because no substitution is possible.
    pub selected_backend: LocalCaptioningBackend,
    /// Backend that runs on this host.
    pub effective_backend: LocalCaptioningBackend,
    /// Export family.
    pub format: super::LocalCaptioningFormat,
    /// Graph quantization of the descriptor.
    pub precision: LocalCaptioningPrecision,
    /// Owned repository.
    pub repository: String,
    /// Pinned immutable revision.
    pub revision: String,
    /// Digest binding the descriptor's repository, revision and pinned file set.
    pub manifest_sha256: String,
    /// Total pinned bytes of every artifact.
    pub total_bytes: u64,
    /// Bytes of the artifacts that are staged with their pinned size.
    pub staged_bytes: u64,
    /// Every pinned artifact, in path order.
    pub artifacts: Vec<LocalCaptioningArtifact>,
}

impl LocalCaptioningArtifactReport {
    /// Whether every pinned artifact is staged.
    pub fn complete(&self) -> bool {
        self.artifacts.iter().all(LocalCaptioningArtifact::staged)
    }

    /// Whether every pinned artifact is missing.
    pub fn empty(&self) -> bool {
        self.artifacts
            .iter()
            .all(|artifact| artifact.state == LocalCaptioningArtifactState::Missing)
    }

    /// Count of staged artifacts.
    pub fn staged(&self) -> usize {
        self.artifacts.iter().filter(|artifact| artifact.staged()).count()
    }

    /// Report for a staged candidate, selected exactly as
    /// [`LocalCaptioningModel::staged`] selects it.
    ///
    /// # Errors
    ///
    /// Fails for an unknown candidate, a candidate of another task, or an unreadable cache.
    pub fn for_candidate(
        id: &str,
        precision: LocalCaptioningPrecision,
        cache_directory: Option<&str>,
    ) -> crate::Result<Self> {
        let model = LocalCaptioningModel::staged(id, precision)?;
        let mut report = Self::for_model(&model, cache_directory)?;
        report.candidate = Some(id.to_owned());
        Ok(report)
    }

    /// Report for a caller-pinned descriptor.
    pub fn for_model(model: &LocalCaptioningModel, cache_directory: Option<&str>) -> crate::Result<Self> {
        let client = crate::model_download::hf_client(cache_directory.map(Path::new)).map_err(invalid)?;
        let mut artifacts = Vec::with_capacity(model.files.len());
        let mut total_bytes = 0u64;
        let mut staged_bytes = 0u64;
        for file in &model.files {
            total_bytes = total_bytes.saturating_add(file.size_bytes);
            let state = match crate::model_download::hf_cached_revision_with_client(
                &client,
                &model.repository,
                &file.path,
                Some(&model.revision),
            )
            .map_err(invalid)?
            {
                None => LocalCaptioningArtifactState::Missing,
                Some(path) => match std::fs::metadata(&path) {
                    Ok(metadata) if metadata.is_file() && metadata.len() == file.size_bytes => {
                        staged_bytes = staged_bytes.saturating_add(file.size_bytes);
                        LocalCaptioningArtifactState::Staged
                    }
                    Ok(_) => LocalCaptioningArtifactState::SizeMismatch,
                    Err(error) if error.kind() == std::io::ErrorKind::NotFound => LocalCaptioningArtifactState::Missing,
                    Err(error) => {
                        return Err(invalid(format!("Local captioning artifact {}: {error}", file.path)));
                    }
                },
            };
            artifacts.push(LocalCaptioningArtifact {
                path: file.path.clone(),
                size_bytes: file.size_bytes,
                sha256: file.sha256.clone(),
                state,
            });
        }
        artifacts.sort_by(|left, right| left.path.cmp(&right.path));
        Ok(Self {
            candidate: None,
            selected_backend: LocalCaptioningBackend::OnnxCpu,
            effective_backend: LocalCaptioningBackend::OnnxCpu,
            format: model.format,
            precision: super::runtime::precision_of_model(model),
            repository: model.repository.clone(),
            revision: model.revision.clone(),
            manifest_sha256: super::policy::manifest_digest(model),
            total_bytes,
            staged_bytes,
            artifacts,
        })
    }
}
