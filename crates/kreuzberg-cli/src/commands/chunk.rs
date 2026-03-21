//! Chunk command implementation.

use anyhow::{Context, Result};

use crate::OutputFormat;

/// Execute the chunk command: split text into chunks.
pub fn chunk_command(text: String, config: kreuzberg::ChunkingConfig, format: OutputFormat) -> Result<()> {
    if text.is_empty() {
        anyhow::bail!("No text provided for chunking. Provide --text or pipe text via stdin.");
    }

    let result = kreuzberg::chunking::chunk_text(&text, &config, None).context("Failed to chunk text")?;

    match format {
        OutputFormat::Json => {
            let chunks: Vec<&str> = result.chunks.iter().map(|c| c.content.as_str()).collect();
            let output = serde_json::json!({
                "chunks": chunks,
                "chunk_count": result.chunk_count,
                "config": {
                    "max_characters": config.max_characters,
                    "overlap": config.overlap,
                    "chunker_type": format!("{:?}", config.chunker_type),
                },
                "input_size_bytes": text.len(),
            });
            println!(
                "{}",
                serde_json::to_string_pretty(&output).context("Failed to serialize chunks to JSON")?
            );
        }
        OutputFormat::Text => {
            for (i, chunk) in result.chunks.iter().enumerate() {
                if result.chunks.len() > 1 {
                    println!("--- chunk {} ---", i + 1);
                }
                println!("{}", chunk.content);
            }
        }
    }

    Ok(())
}
