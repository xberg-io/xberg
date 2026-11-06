//! Batch input manifest parsing and per-file extraction config resolution.

use anyhow::{Context, Result};
use serde::Deserialize;
use std::path::Path;
use xberg::{ExtractInput, ExtractInputWithOptions, FileExtractionConfig};

/// Batch input manifest format.
#[derive(Clone, Copy, Debug, PartialEq, Eq, clap::ValueEnum)]
pub enum BatchInputFormat {
    /// JSON array, or object with an `inputs` array.
    Json,
    /// One JSON string/object per line.
    Jsonl,
}

#[derive(Debug, Deserialize)]
#[serde(untagged)]
enum BatchManifest {
    Inputs { inputs: Vec<BatchManifestItem> },
    Array(Vec<BatchManifestItem>),
}

#[derive(Debug, Deserialize)]
#[serde(untagged)]
enum BatchManifestItem {
    Uri(String),
    Object {
        uri: Option<String>,
        url: Option<String>,
        path: Option<String>,
    },
}

pub fn load_batch_input_manifest(path: &Path, format: BatchInputFormat) -> Result<Vec<String>> {
    let raw = std::fs::read_to_string(path)
        .with_context(|| format!("Failed to read batch input manifest '{}'", path.display()))?;
    match format {
        BatchInputFormat::Json => parse_batch_json_manifest(&raw),
        BatchInputFormat::Jsonl => parse_batch_jsonl_manifest(&raw),
    }
}

fn parse_batch_json_manifest(raw: &str) -> Result<Vec<String>> {
    let manifest: BatchManifest = serde_json::from_str(raw).context("Failed to parse batch input manifest as JSON")?;
    let items = match manifest {
        BatchManifest::Inputs { inputs } | BatchManifest::Array(inputs) => inputs,
    };
    manifest_items_to_uris(items)
}

fn parse_batch_jsonl_manifest(raw: &str) -> Result<Vec<String>> {
    let mut items = Vec::new();
    for (index, line) in raw.lines().enumerate() {
        let trimmed = line.trim();
        if trimmed.is_empty() {
            continue;
        }
        let item: BatchManifestItem = serde_json::from_str(trimmed)
            .with_context(|| format!("Failed to parse JSONL batch input on line {}", index + 1))?;
        items.push(item);
    }
    manifest_items_to_uris(items)
}

fn manifest_items_to_uris(items: Vec<BatchManifestItem>) -> Result<Vec<String>> {
    items
        .into_iter()
        .map(|item| match item {
            BatchManifestItem::Uri(uri) => Ok(uri),
            BatchManifestItem::Object { uri, url, path } => uri
                .or(url)
                .or(path)
                .ok_or_else(|| anyhow::anyhow!("Batch input object must include one of uri, url, or path")),
        })
        .collect()
}

pub(super) fn build_batch_inputs(
    uris: &[String],
    file_configs_map: Option<&std::collections::HashMap<String, serde_json::Value>>,
) -> Result<Vec<ExtractInputWithOptions>> {
    uris.iter()
        .map(|uri| build_extract_input(uri, file_configs_map))
        .collect()
}

fn build_extract_input(
    uri: &str,
    file_configs_map: Option<&std::collections::HashMap<String, serde_json::Value>>,
) -> Result<ExtractInputWithOptions> {
    let mut options = ExtractInputWithOptions::new(ExtractInput::from_uri(uri));
    if let Some(config) = file_configs_map.and_then(|map| map.get(uri)) {
        let (file_config, use_layout_for_markdown) =
            parse_file_config(config.clone()).with_context(|| format!("Failed to parse file config for '{}'", uri))?;
        options.input.config = Some(file_config);
        options.use_layout_for_markdown = use_layout_for_markdown;
    }
    Ok(options)
}

fn parse_file_config(mut value: serde_json::Value) -> serde_json::Result<(FileExtractionConfig, Option<bool>)> {
    let use_layout_for_markdown = value
        .as_object_mut()
        .and_then(|fields| fields.remove("use_layout_for_markdown"))
        .map(serde_json::from_value::<Option<bool>>)
        .transpose()?
        .flatten();
    let config = serde_json::from_value(value)?;
    Ok((config, use_layout_for_markdown))
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn parse_batch_json_manifest_accepts_inputs_object() {
        let uris = parse_batch_json_manifest(r#"{"inputs":["a.txt",{"path":"b.txt"}]}"#).unwrap();
        assert_eq!(uris, vec!["a.txt", "b.txt"]);
    }

    #[test]
    fn mixed_batch_accepts_per_file_layout_overrides() {
        let uris = ["a.pdf".into(), "b.pdf".into(), "c.pdf".into()];
        let overrides = std::collections::HashMap::from([
            ("a.pdf".into(), serde_json::json!({"use_layout_for_markdown": true})),
            ("c.pdf".into(), serde_json::json!({"use_layout_for_markdown": false})),
        ]);

        let inputs = build_batch_inputs(&uris, Some(&overrides)).unwrap();
        let overrides: Vec<_> = inputs.iter().map(|input| input.use_layout_for_markdown).collect();
        assert_eq!(overrides, [Some(true), None, Some(false)]);
    }

    #[test]
    fn parse_batch_jsonl_manifest_accepts_string_and_object_lines() {
        let uris = parse_batch_jsonl_manifest("\"a.txt\"\n{\"uri\":\"b.txt\"}\n").unwrap();
        assert_eq!(uris, vec!["a.txt", "b.txt"]);
    }
}
