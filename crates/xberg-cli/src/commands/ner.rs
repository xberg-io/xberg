//! NER model download commands.
//!
//! Mirrors `tree_sitter::download_command` — eagerly fetches GLiNER ONNX
//! models into the xberg cache so air-gapped / container-pre-bake
//! workflows do not need a network call at inference time.

use anyhow::{Context, Result};
use serde_json::json;
use std::path::PathBuf;

use crate::{WireFormat, style};

/// Execute `xberg cache warm --ner` / `--ner-model` / `--all-ner-models`.
///
/// `ner` is a "download the pinned default" flag. `models` is an explicit
/// list of xberg GLiNER aliases or catalog ids. `all` downloads every variant
/// xberg knows about.
#[allow(dead_code)]
pub fn download_command(
    ner: bool,
    models: Vec<String>,
    all: bool,
    cache_dir: Option<PathBuf>,
    format: WireFormat,
) -> Result<()> {
    let to_download = select_models(ner, models, all)?;
    let downloaded = download_models(&to_download, cache_dir)?;

    match format {
        WireFormat::Text => {
            println!("{}", style::header("NER Model Download"));
            println!("{}", style::dim("=================="));
            for d in &downloaded {
                println!("  {}", style::success(d));
            }
            println!("{}", style::success("Done"));
        }
        WireFormat::Json => {
            let output = json!({
                "downloaded": downloaded,
            });
            println!(
                "{}",
                serde_json::to_string_pretty(&output).context("Failed to serialize NER download results to JSON")?
            );
        }
        WireFormat::Toon => {
            let output = json!({
                "downloaded": downloaded,
            });
            println!(
                "{}",
                serde_toon::to_string(&output).context("Failed to serialize NER download results to TOON")?
            );
        }
    }

    Ok(())
}

/// Resolve the set of GLiNER NER models requested by cache-warm flags.
pub fn select_models(ner: bool, models: Vec<String>, all: bool) -> Result<Vec<String>> {
    for model in &models {
        if model.trim().is_empty() {
            anyhow::bail!("Field 'ner_model' must not be empty. Omit the field or provide a valid model name.");
        }
    }

    let mut to_download: Vec<String> = Vec::new();

    if all {
        to_download.extend(xberg::text::ner::known_models().iter().map(|s| s.to_string()));
    } else if !models.is_empty() {
        to_download.extend(models);
    } else if ner {
        to_download.push(xberg::text::ner::default_model_name().to_string());
    } else {
        anyhow::bail!("No NER model specified. Use --ner, --ner-model <MODEL>, or --all-ner-models.");
    }

    Ok(to_download)
}

/// Download selected GLiNER NER models and return human-readable result labels.
pub fn download_models(models: &[String], cache_dir: Option<PathBuf>) -> Result<Vec<String>> {
    let mut downloaded: Vec<String> = Vec::with_capacity(models.len());
    for repo in models {
        let path = xberg::text::ner::download_model(repo, cache_dir.clone())
            .with_context(|| format!("Failed to download NER model '{repo}'"))?;
        downloaded.push(format!("{repo} -> {}", path.display()));
    }

    Ok(downloaded)
}

#[cfg(test)]
mod tests {
    use super::select_models;

    #[test]
    fn select_models_uses_default_when_ner_flag_is_set() {
        let models = select_models(true, Vec::new(), false).unwrap();

        assert_eq!(models, vec![xberg::text::ner::default_model_name()]);
    }

    #[test]
    fn select_models_uses_explicit_models() {
        let models = select_models(false, vec!["gliner_small-v2.5".to_string()], false).unwrap();

        assert_eq!(models, vec!["gliner_small-v2.5"]);
    }

    #[test]
    fn select_models_rejects_blank_model() {
        let error = select_models(false, vec!["   ".to_string()], false).unwrap_err();

        assert!(error.to_string().contains("must not be empty"));
    }
}
