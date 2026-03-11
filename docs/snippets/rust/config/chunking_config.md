```rust title="Rust"
use kreuzberg::{extract_file, ExtractionConfig, ChunkingConfig};

#[tokio::main]
async fn main() -> kreuzberg::Result<()> {
    let config = ExtractionConfig {
        chunking: Some(ChunkingConfig {
            max_characters: 1000,
            overlap: 200,
            ..Default::default()
        }),
        ..Default::default()
    };

    let result = extract_file("document.pdf", None::<&str>, &config).await?;
    println!("Chunks: {}", result.chunks.len());
    for chunk in &result.chunks {
        println!("Length: {}", chunk.content.len());
    }
    Ok(())
}
```

```rust title="Rust - Markdown with Heading Context"
use kreuzberg::{extract_file, ExtractionConfig, ChunkingConfig, ChunkerType, ChunkSizing};

#[tokio::main]
async fn main() -> kreuzberg::Result<()> {
    let config = ExtractionConfig {
        chunking: Some(ChunkingConfig {
            max_characters: 500,
            overlap: 50,
            chunker_type: ChunkerType::Markdown,
            sizing: ChunkSizing::Tokenizer {
                model: "Xenova/gpt-4o".into(),
                cache_dir: None,
            },
            ..Default::default()
        }),
        ..Default::default()
    };

    let result = extract_file("document.md", None::<&str>, &config).await?;
    for chunk in &result.chunks {
        if let Some(heading_context) = &chunk.metadata.heading_context {
            for heading in &heading_context.headings {
                println!("Heading L{}: {}", heading.level, heading.text);
            }
        }
        println!("Content: {}...", &chunk.content[..100.min(chunk.content.len())]);
    }
    Ok(())
}
```
