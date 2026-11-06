use std::collections::HashSet;

use sha2::{Digest, Sha256};

use super::types::{MAX_INPUT_BYTES, MAX_LABEL_BYTES, MAX_LABELS, MAX_TOKENS};
use super::{LocalClassificationConfig, LocalNliModel};

const REVISION_HEX_LENGTH: usize = 40;
const MAX_MANIFEST_BYTES: usize = 1_048_576;
const MAX_TEMPLATE_BYTES: usize = 4_096;
const MAX_EXTERNAL_FILES: usize = 64;
const MAX_REPOSITORY_BYTES: usize = 256;
const MAX_ARTIFACT_PATH_BYTES: usize = 1024;
const MAX_OUTPUT_NAME_BYTES: usize = 256;
const MAX_CACHE_PATH_BYTES: usize = 4096;
pub(super) const NLI_CLASS_COUNT: usize = 3;

pub(super) fn invalid(message: impl Into<String>) -> crate::XbergError {
    crate::XbergError::Validation {
        message: message.into(),
        source: None,
    }
}

pub(super) fn validate_config(config: &LocalClassificationConfig) -> crate::Result<()> {
    validate_model(&config.model)?;
    validate_limits(config)?;
    if config.hypothesis_template.len() > MAX_TEMPLATE_BYTES
        || config.hypothesis_template.matches("{label}").count() != 1
    {
        return Err(invalid(
            "Local classification requires exactly one {label} in a bounded hypothesis template",
        ));
    }
    if !config.confidence_threshold.is_finite() || !(0.0..=1.0).contains(&config.confidence_threshold) {
        return Err(invalid(
            "Local classification confidence threshold must be finite and in [0, 1]",
        ));
    }
    if config
        .cache_directory
        .as_ref()
        .is_some_and(|directory| directory.len() > MAX_CACHE_PATH_BYTES || directory.trim().is_empty())
    {
        return Err(invalid(
            "Local classification cache directory must be nonblank and within its byte limit",
        ));
    }
    Ok(())
}

fn validate_limits(config: &LocalClassificationConfig) -> crate::Result<()> {
    for (value, ceiling) in [
        (config.limits.max_input_bytes, MAX_INPUT_BYTES),
        (config.limits.max_label_bytes, MAX_LABEL_BYTES),
        (config.limits.max_labels, MAX_LABELS),
        (config.limits.max_tokens, MAX_TOKENS),
    ] {
        if value == 0 || value > ceiling {
            return Err(invalid(
                "Local classification limits must be positive and cannot exceed SDK ceilings",
            ));
        }
    }
    Ok(())
}

fn validate_model(model: &LocalNliModel) -> crate::Result<()> {
    validate_descriptor_lengths(model)?;
    if !model.repository.split_once('/').is_some_and(|(owner, name)| {
        [owner, name].into_iter().all(|part| {
            !part.is_empty()
                && !matches!(part, "." | "..")
                && part
                    .bytes()
                    .all(|byte| byte.is_ascii_alphanumeric() || b"-_.".contains(&byte))
        })
    }) {
        return Err(invalid(
            "Local classification repository must be an owner/name Hugging Face identifier",
        ));
    }
    if model.revision.len() != REVISION_HEX_LENGTH || !model.revision.bytes().all(|byte| byte.is_ascii_hexdigit()) {
        return Err(invalid(
            "Local classification requires an immutable forty-digit hexadecimal revision",
        ));
    }
    validate_nli_indices(model)?;
    if model.output_name.trim().is_empty() {
        return Err(invalid("Local classification requires a named output"));
    }
    super::artifacts::ArtifactInventory::new(model).map(|_| ())
}

fn validate_descriptor_lengths(model: &LocalNliModel) -> crate::Result<()> {
    if model.repository.len() > MAX_REPOSITORY_BYTES
        || model.model_file.len() > MAX_ARTIFACT_PATH_BYTES
        || model.output_name.len() > MAX_OUTPUT_NAME_BYTES
        || model.sha256_manifest.len() > MAX_MANIFEST_BYTES
        || model.external_files.len() > MAX_EXTERNAL_FILES
        || model
            .external_files
            .iter()
            .any(|path| path.len() > MAX_ARTIFACT_PATH_BYTES)
    {
        return Err(invalid(
            "Local classification model descriptor exceeds an artifact inventory byte limit",
        ));
    }
    Ok(())
}

pub(super) fn validate_nli_indices(model: &LocalNliModel) -> crate::Result<()> {
    if model.entailment_index as usize >= NLI_CLASS_COUNT
        || model.contradiction_index as usize >= NLI_CLASS_COUNT
        || model.entailment_index == model.contradiction_index
    {
        return Err(invalid(
            "Local classification requires distinct entailment and contradiction indices in [0, 3)",
        ));
    }
    Ok(())
}

pub(super) fn validate_request(text: &str, labels: &[String], config: &LocalClassificationConfig) -> crate::Result<()> {
    if text.len() > config.limits.max_input_bytes as usize || text.trim().is_empty() {
        return Err(invalid(
            "Local classification premise must be nonblank and within its byte limit",
        ));
    }
    if labels.is_empty() || labels.len() > config.limits.max_labels as usize {
        return Err(invalid("Local classification requires a nonempty, bounded label set"));
    }
    let mut unique = HashSet::new();
    for label in labels {
        let trimmed = (label.len() <= config.limits.max_label_bytes as usize).then(|| label.trim());
        if !trimmed.is_some_and(|label| !label.is_empty() && unique.insert(label)) {
            return Err(invalid(
                "Local classification labels must be nonblank, distinct and within their byte limit",
            ));
        }
    }
    Ok(())
}

pub(super) fn model_identity(config: &LocalClassificationConfig) -> String {
    let model = &config.model;
    let mut hash = Sha256::new();
    for value in [
        &model.repository,
        &model.revision,
        &model.model_file,
        &model.sha256_manifest,
        &model.output_name,
    ] {
        hash.update((value.len() as u64).to_le_bytes());
        hash.update(value.as_bytes());
    }
    hash.update((model.external_files.len() as u64).to_le_bytes());
    for value in &model.external_files {
        hash.update((value.len() as u64).to_le_bytes());
        hash.update(value.as_bytes());
    }
    hash.update(model.entailment_index.to_le_bytes());
    hash.update(model.contradiction_index.to_le_bytes());
    hex::encode(hash.finalize())
}

pub(super) fn validate_cpu_execution(
    provider: Option<&crate::core::config::acceleration::ExecutionProviderType>,
    environment_providers: &[&str],
) -> crate::Result<()> {
    use crate::core::config::acceleration::ExecutionProviderType;
    if provider.is_some_and(|provider| *provider != ExecutionProviderType::Cpu)
        || environment_providers
            .iter()
            .any(|provider| *provider != "CPUExecutionProvider")
    {
        return Err(invalid(
            "Local classification requires CPU execution and rejects conflicting provider settings",
        ));
    }
    Ok(())
}
