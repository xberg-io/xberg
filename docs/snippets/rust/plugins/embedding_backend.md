```rust title="Rust"
use kreuzberg::plugins::{EmbeddingBackend, Plugin, register_embedding_backend};
use kreuzberg::{EmbeddingConfig, EmbeddingModelType, Result, embed_texts};
use async_trait::async_trait;
use std::sync::Arc;

// Wrap an already-loaded embedder (e.g. a tuned ONNX session or any host-language
// embedder) so kreuzberg can call back into it during chunking and standalone
// embed requests.
struct MyEmbedder {
    // Hold whatever model handles the host already owns.
}

impl Plugin for MyEmbedder {
    fn name(&self) -> &str { "my-embedder" }
    fn version(&self) -> String { "1.0.0".to_string() }
    fn initialize(&self) -> Result<()> { Ok(()) }
    fn shutdown(&self) -> Result<()> { Ok(()) }
}

#[async_trait]
impl EmbeddingBackend for MyEmbedder {
    // Captured once at registration; used for shape validation on every dispatch.
    fn dimensions(&self) -> usize { 768 }

    async fn embed(&self, texts: Vec<String>) -> Result<Vec<Vec<f32>>> {
        // Delegate to the already-loaded host model.
        Ok(texts.iter().map(|_| vec![0.0; 768]).collect())
    }
}

fn main() -> Result<()> {
    // Register once at startup.
    register_embedding_backend(Arc::new(MyEmbedder {}))?;

    let config = EmbeddingConfig {
        model: EmbeddingModelType::Plugin { name: "my-embedder".to_string() },
        // Optional: bound the wait on a hung backend (default 60s; `None` disables).
        max_embed_duration_secs: Some(30),
        ..Default::default()
    };

    let vectors = embed_texts(&["Hello, world!", "Second text"], &config)?;
    assert_eq!(vectors.len(), 2);
    Ok(())
}
```
