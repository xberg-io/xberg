use crate::{FrozenPlan, Result, require};
use xberg::local_classification::{LocalClassificationConfig, LocalClassificationLimits};

const MAX_TEMPLATE_BYTES: usize = 4096;
const MAX_DESCRIPTOR_PATH_BYTES: usize = 1024;
const MAX_NAME_BYTES: usize = 256;
const MAX_MANIFEST_BYTES: usize = 1024 * 1024;
const MAX_EXTERNAL_FILES: usize = 64;

// SDK admission helpers are private; portable validation enforces bounded artifact
// metadata without preparing or loading a model.
pub(crate) fn validate_config(config: &LocalClassificationConfig) -> Result<()> {
    let ceiling = LocalClassificationLimits::default();
    let limits = &config.limits;
    for (value, maximum) in [
        (limits.max_input_bytes, ceiling.max_input_bytes),
        (limits.max_label_bytes, ceiling.max_label_bytes),
        (limits.max_labels, ceiling.max_labels),
        (limits.max_tokens, ceiling.max_tokens),
    ] {
        require(value > 0 && value <= maximum, "NLI limits exceed SDK bounds")?;
    }
    let model = &config.model;
    require(
        config.hypothesis_template.len() <= MAX_TEMPLATE_BYTES
            && config
                .cache_directory
                .as_ref()
                .is_none_or(|path| !path.trim().is_empty() && path.len() <= MAX_TEMPLATE_BYTES),
        "NLI template or cache path exceeds SDK bounds",
    )?;
    require(
        model.repository.len() <= MAX_NAME_BYTES
            && !model.output_name.trim().is_empty()
            && model.output_name.len() <= MAX_NAME_BYTES
            && model.model_file.len() <= MAX_DESCRIPTOR_PATH_BYTES
            && model.sha256_manifest.len() <= MAX_MANIFEST_BYTES
            && model.external_files.len() <= MAX_EXTERNAL_FILES,
        "NLI model descriptor exceeds SDK bounds",
    )?;
    let parent = model
        .model_file
        .rsplit_once('/')
        .map(|(parent, _)| format!("{parent}/"))
        .unwrap_or_default();
    require(
        model
            .external_files
            .iter()
            .all(|path| path.len() <= MAX_DESCRIPTOR_PATH_BYTES && path.starts_with(&parent) && artifact_path(path)),
        "invalid graph-relative external tensor path",
    )?;
    Ok(())
}

pub(crate) fn validate_requests(plan: &FrozenPlan, config: &LocalClassificationConfig) -> Result<()> {
    let limits = &config.limits;
    require(
        plan.labels.len() <= limits.max_labels as usize
            && plan
                .labels
                .iter()
                .all(|label| label.len() <= limits.max_label_bytes as usize)
            && plan
                .documents
                .iter()
                .all(|doc| doc.text.len() <= limits.max_input_bytes as usize),
        "NLI request exceeds variant input/label limits",
    )
}

pub(crate) fn repository_component(part: &str) -> bool {
    !part.is_empty()
        && !matches!(part, "." | "..")
        && part
            .bytes()
            .all(|byte| byte.is_ascii_alphanumeric() || b"-_.".contains(&byte))
}

pub(crate) fn artifact_path(path: &str) -> bool {
    path.split('/').all(|part| {
        let stem = part.split('.').next().unwrap_or_default().to_ascii_uppercase();
        let port = stem.strip_prefix("COM").or_else(|| stem.strip_prefix("LPT"));
        repository_component(part)
            && !part.ends_with('.')
            && !matches!(stem.as_str(), "CON" | "PRN" | "AUX" | "NUL")
            && !port.is_some_and(|value| matches!(value, "1" | "2" | "3" | "4" | "5" | "6" | "7" | "8" | "9"))
    })
}
