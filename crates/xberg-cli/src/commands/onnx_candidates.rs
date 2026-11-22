//! Experimental local ONNX candidate selection shared by `xberg cache warm` and `xberg doctor`.
//!
//! These adapters are unqualified and opt-in: nothing here is a default, and every artifact is
//! selected explicitly by candidate id. Reports and preparation read the local Hugging Face cache
//! only; [`stage_and_prepare`] is the one path that may download, and only for a cache miss when
//! `HF_HUB_OFFLINE` is off. After it succeeds, the first request performs no network access.

use anyhow::{Context, Result};
use serde::Serialize;

/// One pinned artifact as reported by the offline inventory.
#[derive(Debug, Clone, Serialize)]
pub struct ArtifactSummary {
    pub path: String,
    pub size_bytes: u64,
    pub sha256: String,
    /// `staged`, `missing` or `size_mismatch`.
    pub state: &'static str,
}

/// Selected/effective backend, model identity and cache state for one experimental candidate.
#[derive(Debug, Clone, Serialize)]
pub struct CandidateSummary {
    /// `captioning` or `classification`.
    pub feature: &'static str,
    pub candidate: String,
    pub selected_backend: &'static str,
    pub effective_backend: &'static str,
    pub format: &'static str,
    pub precision: &'static str,
    pub repository: String,
    pub revision: String,
    /// Digest binding the prepared descriptor or the offline file set.
    pub manifest_sha256: String,
    pub total_bytes: u64,
    pub staged_bytes: u64,
    pub artifacts: Vec<ArtifactSummary>,
}

impl CandidateSummary {
    fn complete(&self) -> bool {
        self.artifacts.iter().all(|artifact| artifact.state == "staged")
    }

    fn empty(&self) -> bool {
        self.artifacts.iter().all(|artifact| artifact.state == "missing")
    }

    fn mismatched(&self) -> bool {
        self.artifacts.iter().any(|artifact| artifact.state == "size_mismatch")
    }

    fn staged(&self) -> usize {
        self.artifacts
            .iter()
            .filter(|artifact| artifact.state == "staged")
            .count()
    }

    /// One-line identity plus cache totals.
    pub fn headline(&self) -> String {
        format!(
            "{} revision={} backend={} (selected {}) precision={} format={} manifest={} artifacts={}/{} bytes={}/{}",
            self.candidate,
            self.revision,
            self.effective_backend,
            self.selected_backend,
            self.precision,
            self.format,
            self.manifest_sha256,
            self.staged(),
            self.artifacts.len(),
            self.staged_bytes,
            self.total_bytes
        )
    }
}

/// Parse the only implemented backend. A different value is an explicit error, not a fallback.
#[cfg(any(feature = "captioning-onnx", feature = "classification-onnx"))]
pub fn parse_backend(value: Option<&str>, feature: &str) -> Result<&'static str> {
    match value.unwrap_or("onnx_cpu") {
        "onnx_cpu" => Ok("onnx_cpu"),
        other => anyhow::bail!("Unsupported --{feature}-backend: {other}. Only onnx_cpu is implemented."),
    }
}

#[cfg(feature = "captioning-onnx")]
pub fn parse_captioning_precision(value: Option<&str>) -> Result<xberg::local_captioning::LocalCaptioningPrecision> {
    use xberg::local_captioning::LocalCaptioningPrecision;
    match value.unwrap_or("quantized") {
        "fp32" => Ok(LocalCaptioningPrecision::Fp32),
        "quantized" | "int8" => Ok(LocalCaptioningPrecision::Quantized),
        other => anyhow::bail!("Unsupported --captioning-precision: {other}. Use quantized or fp32."),
    }
}

#[cfg(feature = "classification-onnx")]
pub fn parse_classification_precision(
    value: Option<&str>,
) -> Result<xberg::local_classification::LocalClassificationPrecision> {
    use xberg::local_classification::LocalClassificationPrecision;
    match value.unwrap_or("quantized") {
        "quantized" | "int8" => Ok(LocalClassificationPrecision::Quantized),
        "fp32" => Ok(LocalClassificationPrecision::Fp32),
        other => anyhow::bail!("Unsupported --classification-precision: {other}. Use quantized."),
    }
}

#[cfg(feature = "captioning-onnx")]
fn captioning_precision_label(precision: xberg::local_captioning::LocalCaptioningPrecision) -> &'static str {
    use xberg::local_captioning::LocalCaptioningPrecision;
    match precision {
        LocalCaptioningPrecision::Fp32 => "fp32",
        LocalCaptioningPrecision::Quantized => "quantized",
    }
}

#[cfg(feature = "classification-onnx")]
fn classification_precision_label(
    precision: xberg::local_classification::LocalClassificationPrecision,
) -> &'static str {
    use xberg::local_classification::LocalClassificationPrecision;
    match precision {
        LocalClassificationPrecision::Fp32 => "fp32",
        LocalClassificationPrecision::Quantized => "quantized",
    }
}

#[cfg(feature = "captioning-onnx")]
fn captioning_format_label(format: xberg::local_captioning::LocalCaptioningFormat) -> &'static str {
    use xberg::local_captioning::LocalCaptioningFormat;
    match format {
        LocalCaptioningFormat::SmolVlm => "smolvlm",
        LocalCaptioningFormat::Florence2 => "florence2",
    }
}

#[cfg(feature = "captioning-onnx")]
fn captioning_backend_label(backend: xberg::local_captioning::LocalCaptioningBackend) -> &'static str {
    use xberg::local_captioning::LocalCaptioningBackend;
    match backend {
        LocalCaptioningBackend::OnnxCpu => "onnx_cpu",
    }
}

#[cfg(feature = "classification-onnx")]
fn classification_backend_label(backend: xberg::local_classification::LocalClassificationBackend) -> &'static str {
    use xberg::local_classification::LocalClassificationBackend;
    match backend {
        LocalClassificationBackend::OnnxCpu => "onnx_cpu",
    }
}

#[cfg(feature = "captioning-onnx")]
fn captioning_state_label(state: xberg::local_captioning::LocalCaptioningArtifactState) -> &'static str {
    use xberg::local_captioning::LocalCaptioningArtifactState;
    match state {
        LocalCaptioningArtifactState::Staged => "staged",
        LocalCaptioningArtifactState::Missing => "missing",
        LocalCaptioningArtifactState::SizeMismatch => "size_mismatch",
    }
}

#[cfg(feature = "classification-onnx")]
fn classification_state_label(state: xberg::local_classification::LocalClassificationArtifactState) -> &'static str {
    use xberg::local_classification::LocalClassificationArtifactState;
    match state {
        LocalClassificationArtifactState::Staged => "staged",
        LocalClassificationArtifactState::Missing => "missing",
        LocalClassificationArtifactState::SizeMismatch => "size_mismatch",
    }
}

#[cfg(feature = "captioning-onnx")]
fn captioning_artifacts(artifacts: Vec<xberg::local_captioning::LocalCaptioningArtifact>) -> Vec<ArtifactSummary> {
    artifacts
        .into_iter()
        .map(|artifact| ArtifactSummary {
            path: artifact.path,
            size_bytes: artifact.size_bytes,
            sha256: artifact.sha256,
            state: captioning_state_label(artifact.state),
        })
        .collect()
}

#[cfg(feature = "classification-onnx")]
fn classification_artifacts(
    artifacts: Vec<xberg::local_classification::LocalClassificationArtifact>,
) -> Vec<ArtifactSummary> {
    artifacts
        .into_iter()
        .map(|artifact| ArtifactSummary {
            path: artifact.path,
            size_bytes: artifact.size_bytes,
            sha256: artifact.sha256,
            state: classification_state_label(artifact.state),
        })
        .collect()
}

/// Offline report for one captioning candidate. Never downloads.
#[cfg(feature = "captioning-onnx")]
pub fn captioning_report(
    id: &str,
    precision: xberg::local_captioning::LocalCaptioningPrecision,
    cache_dir: Option<&std::path::Path>,
) -> Result<CandidateSummary> {
    use xberg::local_captioning::LocalCaptioningArtifactReport;
    let report = LocalCaptioningArtifactReport::for_candidate(id, precision, cache_dir.and_then(|p| p.to_str()))
        .map_err(|error| anyhow::anyhow!("{error}"))?;
    Ok(CandidateSummary {
        feature: "captioning",
        candidate: report.candidate.clone().unwrap_or_else(|| id.to_owned()),
        selected_backend: captioning_backend_label(report.selected_backend),
        effective_backend: captioning_backend_label(report.effective_backend),
        format: captioning_format_label(report.format),
        precision: captioning_precision_label(report.precision),
        repository: report.repository,
        revision: report.revision,
        manifest_sha256: report.manifest_sha256,
        total_bytes: report.total_bytes,
        staged_bytes: report.staged_bytes,
        artifacts: captioning_artifacts(report.artifacts),
    })
}

/// Offline report for one classification candidate. Never downloads.
#[cfg(feature = "classification-onnx")]
pub fn classification_report(
    id: &str,
    precision: xberg::local_classification::LocalClassificationPrecision,
    cache_dir: Option<&std::path::Path>,
) -> Result<CandidateSummary> {
    use xberg::local_classification::LocalClassificationArtifactReport;
    let report = LocalClassificationArtifactReport::for_candidate(id, precision, cache_dir.and_then(|p| p.to_str()))
        .map_err(|error| anyhow::anyhow!("{error}"))?;
    Ok(CandidateSummary {
        feature: "classification",
        candidate: report.candidate.clone().unwrap_or_else(|| id.to_owned()),
        selected_backend: classification_backend_label(report.selected_backend),
        effective_backend: classification_backend_label(report.effective_backend),
        format: "nli",
        precision: classification_precision_label(report.precision),
        repository: report.repository,
        revision: report.revision,
        manifest_sha256: report.manifest_sha256,
        total_bytes: report.total_bytes,
        staged_bytes: report.staged_bytes,
        artifacts: classification_artifacts(report.artifacts),
    })
}

/// Append one aggregate and one per-artifact doctor check per candidate.
pub fn append_doctor_checks(report: &mut xberg::DoctorReport, summaries: &[CandidateSummary], selected: bool) {
    for summary in summaries {
        let name = format!("{}.onnx", summary.feature);
        let check = if summary.complete() {
            xberg::DoctorCheck::pass(&name, summary.headline())
        } else if summary.empty() {
            xberg::DoctorCheck::skip(
                &name,
                format!(
                    "{} (no artifact is staged; prepare it before an offline first request)",
                    summary.headline()
                ),
            )
        } else if summary.mismatched() && selected {
            xberg::DoctorCheck::fail(&name, summary.headline())
        } else {
            xberg::DoctorCheck::warn(&name, summary.headline())
        };
        report.checks.push(check);
        // Per-artifact detail is for an explicitly selected candidate; the default catalog pass
        // stays one line per candidate so a routine doctor report is not flooded.
        if !selected {
            continue;
        }
        for artifact in &summary.artifacts {
            let message = format!(
                "id={} revision={} path={} size={} sha256={} {}",
                summary.candidate,
                summary.revision,
                artifact.path,
                artifact.size_bytes,
                artifact.sha256,
                artifact.state
            );
            let artifact_name = format!("{name}:{}", artifact.path);
            let check = match artifact.state {
                "staged" => xberg::DoctorCheck::pass(&artifact_name, message),
                "missing" => xberg::DoctorCheck::skip(&artifact_name, message),
                _ if selected => xberg::DoctorCheck::fail(&artifact_name, message),
                _ => xberg::DoctorCheck::warn(&artifact_name, message),
            };
            report.checks.push(check);
        }
    }
}

/// Candidate selection for staging and preparation.
#[derive(Debug, Default)]
pub struct StageSelection {
    #[cfg(feature = "captioning-onnx")]
    pub captioning_model: Option<String>,
    #[cfg(feature = "captioning-onnx")]
    pub captioning_precision: Option<String>,
    #[cfg(feature = "captioning-onnx")]
    pub captioning_backend: Option<String>,
    #[cfg(feature = "classification-onnx")]
    pub classification_model: Option<String>,
    #[cfg(feature = "classification-onnx")]
    pub classification_precision: Option<String>,
    #[cfg(feature = "classification-onnx")]
    pub classification_backend: Option<String>,
}

impl StageSelection {
    /// Whether any candidate was requested.
    pub fn is_empty(&self) -> bool {
        #[allow(unused_mut)]
        let mut empty = true;
        #[cfg(feature = "captioning-onnx")]
        {
            empty &= self.captioning_model.is_none();
        }
        #[cfg(feature = "classification-onnx")]
        {
            empty &= self.classification_model.is_none();
        }
        empty
    }
}

/// Stage then prepare every requested candidate, returning its identity and verified cache state.
///
/// A cache hit performs no transfer; a miss is fetched unless Hugging Face offline mode is set.
/// `prepare` itself only reads the cache, so the next request needs no network access.
pub fn stage_and_prepare(
    selection: &StageSelection,
    cache_dir: Option<&std::path::Path>,
) -> Result<Vec<CandidateSummary>> {
    let runtime = tokio::runtime::Builder::new_current_thread()
        .enable_all()
        .build()
        .context("Failed to build a Tokio runtime for candidate preparation")?;
    runtime.block_on(async {
        let mut summaries = Vec::new();
        #[cfg(feature = "captioning-onnx")]
        if let Some(id) = selection.captioning_model.as_deref() {
            let backend = parse_backend(selection.captioning_backend.as_deref(), "captioning")?;
            debug_assert_eq!(backend, "onnx_cpu");
            let precision = parse_captioning_precision(selection.captioning_precision.as_deref())?;
            summaries.push(prepare_captioning(id, precision, cache_dir).await?);
        }
        #[cfg(feature = "classification-onnx")]
        if let Some(id) = selection.classification_model.as_deref() {
            let backend = parse_backend(selection.classification_backend.as_deref(), "classification")?;
            debug_assert_eq!(backend, "onnx_cpu");
            let precision = parse_classification_precision(selection.classification_precision.as_deref())?;
            summaries.push(prepare_classification(id, precision, cache_dir).await?);
        }
        Ok(summaries)
    })
}

#[cfg(feature = "captioning-onnx")]
async fn prepare_captioning(
    id: &str,
    precision: xberg::local_captioning::LocalCaptioningPrecision,
    cache_dir: Option<&std::path::Path>,
) -> Result<CandidateSummary> {
    use xberg::local_captioning::{LocalCaptioningConfig, LocalCaptioningModel, LocalImageCaptioner};
    let model = LocalCaptioningModel::staged(id, precision).map_err(|error| anyhow::anyhow!("{error}"))?;
    let config = LocalCaptioningConfig {
        model,
        limits: Default::default(),
        prompt: None,
        cache_directory: cache_dir.and_then(|path| path.to_str()).map(str::to_owned),
    };
    let captioner =
        LocalImageCaptioner::new(config).map_err(|error| anyhow::anyhow!("Local captioning config: {error}"))?;
    captioner
        .stage(&Default::default())
        .await
        .map_err(|error| anyhow::anyhow!("Staging {id} failed: {error}"))?;
    let info = captioner
        .prepare(&Default::default())
        .await
        .map_err(|error| anyhow::anyhow!("Preparing {id} failed: {error}"))?;
    if info.selected_backend != info.effective_backend {
        anyhow::bail!(
            "{id} selected an unavailable backend: selected={:?} effective={:?}",
            info.selected_backend,
            info.effective_backend
        );
    }
    let report = captioner
        .artifacts()
        .map_err(|error| anyhow::anyhow!("Inspecting {id} failed: {error}"))?;
    Ok(CandidateSummary {
        feature: "captioning",
        candidate: id.to_owned(),
        selected_backend: captioning_backend_label(info.selected_backend),
        effective_backend: captioning_backend_label(info.effective_backend),
        format: captioning_format_label(info.format),
        precision: captioning_precision_label(info.precision),
        repository: info.repository,
        revision: info.revision,
        manifest_sha256: info.manifest_sha256,
        total_bytes: report.total_bytes,
        staged_bytes: report.staged_bytes,
        artifacts: captioning_artifacts(report.artifacts),
    })
}

#[cfg(feature = "classification-onnx")]
async fn prepare_classification(
    id: &str,
    precision: xberg::local_classification::LocalClassificationPrecision,
    cache_dir: Option<&std::path::Path>,
) -> Result<CandidateSummary> {
    use xberg::local_classification::{LocalClassificationConfig, LocalNliModel, LocalTextClassifier};
    let model = LocalNliModel::staged(id, precision).map_err(|error| anyhow::anyhow!("{error}"))?;
    let config = LocalClassificationConfig {
        model,
        limits: Default::default(),
        hypothesis_template: "This text is about {label}.".to_owned(),
        multi_label: false,
        confidence_threshold: 0.0,
        cache_directory: cache_dir.and_then(|path| path.to_str()).map(str::to_owned),
    };
    let classifier =
        LocalTextClassifier::new(config).map_err(|error| anyhow::anyhow!("Local classification config: {error}"))?;
    classifier
        .stage(&Default::default())
        .await
        .map_err(|error| anyhow::anyhow!("Staging {id} failed: {error}"))?;
    let info = classifier
        .prepare(&Default::default())
        .await
        .map_err(|error| anyhow::anyhow!("Preparing {id} failed: {error}"))?;
    if info.selected_backend != info.effective_backend {
        anyhow::bail!(
            "{id} selected an unavailable backend: selected={:?} effective={:?}",
            info.selected_backend,
            info.effective_backend
        );
    }
    let report = classifier
        .artifacts()
        .map_err(|error| anyhow::anyhow!("Inspecting {id} failed: {error}"))?;
    Ok(CandidateSummary {
        feature: "classification",
        candidate: id.to_owned(),
        selected_backend: classification_backend_label(info.selected_backend),
        effective_backend: classification_backend_label(info.effective_backend),
        format: "nli",
        precision: classification_precision_label(report.precision),
        repository: info.repository,
        revision: info.revision,
        manifest_sha256: info.manifest_sha256,
        total_bytes: report.total_bytes,
        staged_bytes: report.staged_bytes,
        artifacts: classification_artifacts(report.artifacts),
    })
}

#[cfg(all(test, any(feature = "captioning-onnx", feature = "classification-onnx")))]
mod tests {
    use super::*;

    fn summary(state: &'static str) -> CandidateSummary {
        CandidateSummary {
            feature: "captioning",
            candidate: "smolvlm-256m-instruct".to_owned(),
            selected_backend: "onnx_cpu",
            effective_backend: "onnx_cpu",
            format: "smolvlm",
            precision: "quantized",
            repository: "xberg-io/captioning-models".to_owned(),
            revision: "a".repeat(40),
            manifest_sha256: "b".repeat(64),
            total_bytes: 10,
            staged_bytes: if state == "staged" { 10 } else { 0 },
            artifacts: vec![ArtifactSummary {
                path: "smolvlm-256m-instruct/onnx/vision_encoder_int8.onnx".to_owned(),
                size_bytes: 10,
                sha256: "c".repeat(64),
                state,
            }],
        }
    }

    #[test]
    fn doctor_checks_carry_the_candidate_identity_and_every_artifact_field() {
        let mut report = xberg::DoctorReport::default();
        append_doctor_checks(&mut report, &[summary("staged")], true);
        assert_eq!(report.checks.len(), 2);
        assert_eq!(report.checks[0].name, "captioning.onnx");
        assert_eq!(report.checks[0].status, xberg::ProbeStatus::Pass);
        let artifact = &report.checks[1];
        assert_eq!(
            artifact.name,
            "captioning.onnx:smolvlm-256m-instruct/onnx/vision_encoder_int8.onnx"
        );
        for expected in [
            "id=smolvlm-256m-instruct",
            "revision=aaaaaaaa",
            "size=10",
            &format!("sha256={}", "c".repeat(64)),
            "staged",
        ] {
            assert!(artifact.message.contains(expected), "{}", artifact.message);
        }

        // A missing artifact is an explicit skip, not a silent pass. The unselected catalog pass
        // keeps one summary check per candidate instead of every artifact.
        let mut report = xberg::DoctorReport::default();
        append_doctor_checks(&mut report, &[summary("missing")], false);
        assert_eq!(report.checks.len(), 1);
        assert_eq!(report.checks[0].status, xberg::ProbeStatus::Skip);
        assert!(report.checks[0].message.contains("no artifact is staged"));
        assert!(report.is_ok());

        // A size mismatch in an explicitly selected candidate fails the report.
        let mut report = xberg::DoctorReport::default();
        append_doctor_checks(&mut report, &[summary("size_mismatch")], true);
        assert_eq!(report.checks[0].status, xberg::ProbeStatus::Fail);
        assert_eq!(report.checks[1].status, xberg::ProbeStatus::Fail);
        assert!(!report.is_ok());
    }

    #[cfg(feature = "captioning-onnx")]
    #[test]
    fn captioning_precision_and_backend_are_validated_explicitly() {
        assert_eq!(
            parse_captioning_precision(None).unwrap(),
            xberg::local_captioning::LocalCaptioningPrecision::Quantized
        );
        assert_eq!(
            parse_captioning_precision(Some("fp32")).unwrap(),
            xberg::local_captioning::LocalCaptioningPrecision::Fp32
        );
        let error = parse_captioning_precision(Some("int4")).unwrap_err().to_string();
        assert!(error.contains("Unsupported --captioning-precision"), "{error}");
        assert_eq!(parse_backend(None, "captioning").unwrap(), "onnx_cpu");
        assert_eq!(parse_backend(Some("onnx_cpu"), "captioning").unwrap(), "onnx_cpu");
        let error = parse_backend(Some("cuda"), "captioning").unwrap_err().to_string();
        assert!(error.contains("Only onnx_cpu is implemented"), "{error}");
    }

    #[cfg(feature = "classification-onnx")]
    #[test]
    fn classification_precision_defaults_to_the_only_usable_graph_set() {
        assert_eq!(
            parse_classification_precision(None).unwrap(),
            xberg::local_classification::LocalClassificationPrecision::Quantized
        );
        assert_eq!(
            parse_classification_precision(Some("fp32")).unwrap(),
            xberg::local_classification::LocalClassificationPrecision::Fp32
        );
        assert!(parse_classification_precision(Some("int4")).is_err());
    }
}
