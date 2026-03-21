//! Embed command implementation.

use anyhow::{Context, Result};

use crate::OutputFormat;

/// Execute the embed command: generate embeddings for input texts.
pub fn embed_command(texts: Vec<String>, preset: &str, format: OutputFormat) -> Result<()> {
    use kreuzberg::types::{Chunk, ChunkMetadata};

    // Validate preset
    let _preset_info = kreuzberg::get_preset(preset).with_context(|| {
        format!(
            "Unknown embedding preset '{}'. Available: {:?}",
            preset,
            kreuzberg::list_presets()
        )
    })?;

    if texts.is_empty() {
        anyhow::bail!("No texts provided for embedding. Provide --text or pipe text via stdin.");
    }

    // Validate no empty texts
    for (i, t) in texts.iter().enumerate() {
        if t.is_empty() {
            anyhow::bail!("Text at position {} is empty. All texts must be non-empty.", i + 1);
        }
    }

    // Build EmbeddingConfig from preset
    let config = kreuzberg::EmbeddingConfig {
        model: kreuzberg::EmbeddingModelType::Preset {
            name: preset.to_string(),
        },
        show_download_progress: true,
        ..Default::default()
    };

    // Create chunks from input texts
    let mut chunks: Vec<Chunk> = texts
        .iter()
        .enumerate()
        .map(|(idx, text)| Chunk {
            content: text.clone(),
            embedding: None,
            metadata: ChunkMetadata {
                byte_start: 0,
                byte_end: text.len(),
                token_count: None,
                chunk_index: idx,
                total_chunks: texts.len(),
                first_page: None,
                last_page: None,
                heading_context: None,
            },
        })
        .collect();

    // Generate embeddings
    kreuzberg::embeddings::generate_embeddings_for_chunks(&mut chunks, &config)
        .context("Failed to generate embeddings")?;

    // Extract embeddings
    let embeddings: Vec<Vec<f32>> = chunks
        .into_iter()
        .map(|chunk| chunk.embedding.unwrap_or_default())
        .collect();

    let dimensions = embeddings.first().map(|e| e.len()).unwrap_or(0);

    match format {
        OutputFormat::Json => {
            let output = serde_json::json!({
                "embeddings": embeddings,
                "model": preset,
                "dimensions": dimensions,
                "count": embeddings.len(),
            });
            println!(
                "{}",
                serde_json::to_string_pretty(&output).context("Failed to serialize embeddings to JSON")?
            );
        }
        OutputFormat::Text => {
            for (i, embedding) in embeddings.iter().enumerate() {
                if texts.len() > 1 {
                    println!("# text {}", i + 1);
                }
                let values: Vec<String> = embedding.iter().map(|v| format!("{v}")).collect();
                println!("{}", values.join(","));
            }
        }
    }

    Ok(())
}
