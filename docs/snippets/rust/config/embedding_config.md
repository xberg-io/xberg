```rust title="Rust"
use xberg::{extract, ExtractionConfig, ChunkingConfig, EmbeddingConfig, EmbeddingModelType};

#[tokio::main]
async fn main() -> xberg::Result<()> {
    let config = ExtractionConfig {
        chunking: Some(ChunkingConfig {
            max_characters: 1000,
            overlap: 200,
            embedding: Some(EmbeddingConfig {
                model: EmbeddingModelType::Preset {
                    name: "balanced".to_string(),
                },
                batch_size: 16,
                normalize: true,
                show_download_progress: true,
                cache_dir: None,
            }),
            ..Default::default()
        }),
        ..Default::default()
    };

    let result = extract("document.pdf", None::<&str>, &config).await?;
    println!("Chunks with embeddings: {}", result.chunks.as_ref().map(|c| c.len()).unwrap_or(0));
    Ok(())
}
```
