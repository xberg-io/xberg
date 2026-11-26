//! CLI input resolution and validation helpers.

use anyhow::{Context, Result};
use base64::{Engine as _, engine::general_purpose::STANDARD};
use std::path::PathBuf;

use crate::commands::{
    BatchInputFormat, ExtractInputSource, load_batch_input_manifest, uri_to_local_path, validate_batch_paths,
    validate_file_exists,
};

/// Apply inline JSON or base64 JSON overrides to an extraction config.
pub(crate) fn apply_json_overrides(
    config: &mut xberg::ExtractionConfig,
    config_json: Option<String>,
    config_json_base64: Option<String>,
) -> Result<()> {
    if let Some(json_str) = config_json {
        let json_value: serde_json::Value =
            serde_json::from_str(&json_str).context("Failed to parse --config-json as JSON")?;
        *config =
            merge_json_into_config(config, json_value).context("Failed to merge --config-json with file config")?;
    } else if let Some(base64_str) = config_json_base64 {
        let json_bytes = STANDARD
            .decode(&base64_str)
            .context("Failed to decode base64 in --config-json-base64")?;
        let json_str = String::from_utf8(json_bytes).context("Base64-decoded content is not valid UTF-8")?;
        let json_value: serde_json::Value =
            serde_json::from_str(&json_str).context("Failed to parse decoded --config-json-base64 as JSON")?;
        *config = merge_json_into_config(config, json_value)
            .context("Failed to merge --config-json-base64 with file config")?;
    }
    Ok(())
}

/// The config `extract` and `batch` run with, in increasing precedence: the config file (or the
/// discovered one) and inline JSON, `XBERG_OCR_TESSDATA_MODEL`, then the CLI flags. Only that one
/// environment variable is read here; the other `XBERG_*` overrides stay specific to `serve`,
/// `mcp` and `doctor`.
pub(crate) fn extraction_config(
    config_path: Option<PathBuf>,
    discover: bool,
    config_json: Option<String>,
    config_json_base64: Option<String>,
    overrides: crate::commands::overrides::ExtractionOverrides,
) -> Result<xberg::ExtractionConfig> {
    let mut config = crate::commands::load_config(config_path, discover)?;
    apply_json_overrides(&mut config, config_json, config_json_base64)?;
    config.apply_env_tessdata_model()?;
    overrides.apply(&mut config);
    Ok(config)
}

pub(crate) fn resolve_extract_input(
    uri: Option<String>,
    #[cfg(feature = "url-surface")] url: Option<String>,
    stdin: bool,
) -> Result<ExtractInputSource> {
    #[cfg(feature = "url-surface")]
    match (uri, url, stdin) {
        (Some(uri), None, false) => Ok(ExtractInputSource::Uri(uri)),
        (None, Some(url), false) => Ok(ExtractInputSource::Uri(url)),
        (None, None, true) => Ok(ExtractInputSource::Stdin),
        _ => anyhow::bail!("Provide exactly one extraction input: URI, --url, or --stdin."),
    }

    #[cfg(not(feature = "url-surface"))]
    match (uri, stdin) {
        (Some(uri), false) => Ok(ExtractInputSource::Uri(uri)),
        (None, true) => Ok(ExtractInputSource::Stdin),
        _ => anyhow::bail!("Provide exactly one extraction input: URI or --stdin."),
    }
}

pub(crate) fn validate_extract_input(input: &ExtractInputSource) -> Result<()> {
    match input {
        ExtractInputSource::Stdin => Ok(()),
        ExtractInputSource::Uri(uri) => {
            if is_remote_uri(uri) {
                #[cfg(feature = "url-surface")]
                return Ok(());
                #[cfg(not(feature = "url-surface"))]
                anyhow::bail!(
                    "HTTP(S) input requires the url-ingestion feature. Rebuild with --features url-ingestion"
                );
            }
            let path = uri_to_local_path(uri)?;
            validate_file_exists(&path)
        }
    }
}

pub(crate) fn resolve_batch_inputs(
    paths: Vec<PathBuf>,
    input: Option<PathBuf>,
    input_format: Option<BatchInputFormat>,
) -> Result<Vec<String>> {
    let mut uris: Vec<String> = paths
        .into_iter()
        .map(|path| path.to_string_lossy().to_string())
        .collect();

    if let Some(input_path) = input {
        let format = input_format.unwrap_or_else(|| infer_batch_input_format(&input_path));
        uris.extend(load_batch_input_manifest(&input_path, format)?);
    }

    if uris.is_empty() {
        anyhow::bail!("No files provided for batch extraction. Provide paths or --input.");
    }

    Ok(uris)
}

pub(crate) fn validate_batch_input_uris(uris: &[String]) -> Result<()> {
    #[cfg(not(feature = "url-surface"))]
    if let Some(uri) = uris.iter().find(|uri| is_remote_uri(uri)) {
        anyhow::bail!(
            "HTTP(S) batch input '{uri}' requires the url-ingestion feature. Rebuild with --features url-ingestion"
        );
    }

    let local_paths: Vec<PathBuf> = uris
        .iter()
        .filter(|uri| !is_remote_uri(uri))
        .map(|uri| uri_to_local_path(uri))
        .collect::<Result<Vec<_>>>()?;
    if local_paths.is_empty() {
        return Ok(());
    }
    validate_batch_paths(&local_paths)
}

/// Merge a JSON value into an existing extraction config via field-by-field override.
fn merge_json_into_config(
    base_config: &xberg::ExtractionConfig,
    json_value: serde_json::Value,
) -> Result<xberg::ExtractionConfig> {
    let json_str = serde_json::to_string(&json_value).map_err(|e| anyhow::anyhow!("{}", e))?;
    xberg::core::config::merge::merge_config_json(base_config, &json_str).map_err(|e| anyhow::anyhow!("{}", e))
}

fn infer_batch_input_format(path: &std::path::Path) -> BatchInputFormat {
    match path.extension().and_then(|ext| ext.to_str()) {
        Some(ext) if ext.eq_ignore_ascii_case("jsonl") || ext.eq_ignore_ascii_case("ndjson") => BatchInputFormat::Jsonl,
        _ => BatchInputFormat::Json,
    }
}

fn is_remote_uri(uri: &str) -> bool {
    uri.starts_with("http://") || uri.starts_with("https://")
}
