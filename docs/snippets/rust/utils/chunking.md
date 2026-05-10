```rust title="Rust"
use kreuzberg::{ChunkingConfig, EmbeddingConfig, EmbeddingModelType, ExtractionConfig};

let config = ExtractionConfig {
    chunking: Some(ChunkingConfig {
        max_characters: 1500,
        overlap: 200,
        embedding: Some(EmbeddingConfig {
            model: EmbeddingModelType::Preset { name: "all-minilm-l6-v2".to_string() },
            ..Default::default()
        }),
        ..Default::default()
    }),
    ..Default::default()
};
```
