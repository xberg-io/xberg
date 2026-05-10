```rust title="Rust"
use kreuzberg::{
    extract_file, ChunkingConfig, EmbeddingConfig, EmbeddingModelType, ExtractionConfig,
};

#[tokio::main]
async fn main() -> kreuzberg::Result<()> {
    let config = ExtractionConfig {
        chunking: Some(ChunkingConfig {
            max_characters: 500,
            overlap: 50,
            embedding: Some(EmbeddingConfig {
                model: EmbeddingModelType::Preset { name: "balanced".to_string() },
                normalize: true,
                batch_size: 16,
                ..Default::default()
            }),
            ..Default::default()
        }),
        ..Default::default()
    };

    let result = extract_file("research_paper.pdf", None, &config).await?;

    let mut chunks_with_embeddings = 0usize;
    for chunk in result.chunks.unwrap_or_default() {
        if chunk.embedding.is_some() {
            chunks_with_embeddings += 1;
        }
    }
    println!("Chunks with embeddings: {chunks_with_embeddings}");
    Ok(())
}
```
