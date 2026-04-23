//! Embed command implementation.

use anyhow::{Context, Result};

use crate::{WireFormat, style};

/// Execute the embed command: generate embeddings for input texts.
///
/// When `provider` is `"local"` (default), uses the ONNX preset model.
/// When `provider` is `"llm"`, uses liter-llm with the specified model and API key.
/// When `provider` is `"plugin"`, dispatches to a pre-registered in-process embedding backend.
pub fn embed_command(
    texts: Vec<String>,
    preset: &str,
    provider: &str,
    llm_model: Option<String>,
    llm_api_key: Option<String>,
    plugin_name: Option<String>,
    format: WireFormat,
) -> Result<()> {
    if texts.is_empty() {
        anyhow::bail!("No texts provided for embedding. Provide --text or pipe text via stdin.");
    }

    // Validate no empty texts
    for (i, t) in texts.iter().enumerate() {
        if t.is_empty() {
            anyhow::bail!("Text at position {} is empty. All texts must be non-empty.", i + 1);
        }
    }

    let (config, model_label) = match provider {
        "llm" => {
            let model = llm_model.as_deref().ok_or_else(|| {
                anyhow::anyhow!(
                    "--model is required when --provider is 'llm' (e.g., --model openai/text-embedding-3-small)"
                )
            })?;

            let llm_config = kreuzberg::LlmConfig {
                model: model.to_string(),
                api_key: llm_api_key,
                base_url: None,
                timeout_secs: None,
                max_retries: None,
                temperature: None,
                max_tokens: None,
            };

            let config = kreuzberg::EmbeddingConfig {
                model: kreuzberg::EmbeddingModelType::Llm { llm: llm_config },
                show_download_progress: true,
                ..Default::default()
            };

            (config, model.to_string())
        }
        "local" | "" => {
            // Validate preset for local provider
            let _preset_info = kreuzberg::get_preset(preset).with_context(|| {
                format!(
                    "Unknown embedding preset '{}'. Available: {:?}",
                    preset,
                    kreuzberg::list_presets()
                )
            })?;

            let config = kreuzberg::EmbeddingConfig {
                model: kreuzberg::EmbeddingModelType::Preset {
                    name: preset.to_string(),
                },
                show_download_progress: true,
                ..Default::default()
            };

            (config, preset.to_string())
        }
        "plugin" => {
            let name = plugin_name.as_deref().ok_or_else(|| {
                anyhow::anyhow!(
                    "--plugin NAME is required when --provider is 'plugin'. Register a backend via kreuzberg::plugins::register_embedding_backend first."
                )
            })?;
            if name.is_empty() {
                anyhow::bail!("--plugin NAME must not be empty.");
            }

            // Pre-flight: surface unknown backends with a list of registered names
            // (parity with the REST handler, which returns 422 for the same case).
            let available =
                kreuzberg::plugins::list_embedding_backends().context("Failed to read embedding backend registry")?;
            if !available.iter().any(|n| n == name) {
                anyhow::bail!(
                    "Embedding backend '{}' is not registered. Available backends: {}",
                    name,
                    if available.is_empty() {
                        "(none registered)".to_string()
                    } else {
                        available.join(", ")
                    }
                );
            }

            let config = kreuzberg::EmbeddingConfig {
                model: kreuzberg::EmbeddingModelType::Plugin { name: name.to_string() },
                ..Default::default()
            };

            (config, name.to_string())
        }
        other => {
            anyhow::bail!(
                "Unknown embedding provider '{}'. Valid providers: 'local' (default, ONNX), 'llm' (liter-llm), or 'plugin' (in-process backend).",
                other
            );
        }
    };

    // Generate embeddings
    let embeddings = kreuzberg::embed_texts(&texts, &config).context("Failed to generate embeddings")?;

    let dimensions = embeddings.first().map(|e| e.len()).unwrap_or(0);

    match format {
        WireFormat::Json => {
            let output = serde_json::json!({
                "embeddings": embeddings,
                "model": model_label,
                "dimensions": dimensions,
                "count": embeddings.len(),
            });
            println!(
                "{}",
                serde_json::to_string_pretty(&output).context("Failed to serialize embeddings to JSON")?
            );
        }
        WireFormat::Toon => {
            let output = serde_json::json!({
                "embeddings": embeddings,
                "model": model_label,
                "dimensions": dimensions,
                "count": embeddings.len(),
            });
            println!(
                "{}",
                serde_toon::to_string(&output).context("Failed to serialize embeddings to TOON")?
            );
        }
        WireFormat::Text => {
            for (i, embedding) in embeddings.iter().enumerate() {
                if texts.len() > 1 {
                    println!("{}", style::dim(&format!("# text {}", i + 1)));
                }
                let values: Vec<String> = embedding.iter().map(|v| format!("{v}")).collect();
                println!("{}", values.join(","));
            }
        }
    }

    Ok(())
}
