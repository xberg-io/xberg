```rust title="Rust"
use xberg::{extract, ExtractionConfig, ChunkingConfig, EmbeddingConfig};

let config = ExtractionConfig {
    chunking: Some(ChunkingConfig {
        max_characters: 512,
        overlap: 50,
        embedding: Some(EmbeddingConfig {
            model: xberg::EmbeddingModelType::Preset { name: "balanced".to_string() },
            normalize: true,
            ..Default::default()
        }),
        ..Default::default()
    }),
    ..Default::default()
};

let result = extract("document.pdf", None, &config).await?;

if let Some(chunks) = result.chunks {
    for (i, chunk) in chunks.iter().enumerate() {
        if let Some(embedding) = &chunk.embedding {
            println!("Chunk {}: {} dimensions", i, embedding.len());
        }
    }
}
```
