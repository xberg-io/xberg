```rust title="Rust"
use kreuzberg::{extract_file, ExtractionConfig, TokenReductionOptions};

#[tokio::main]
async fn main() -> kreuzberg::Result<()> {
    let config = ExtractionConfig {
        token_reduction: Some(TokenReductionOptions {
            mode: "moderate".to_string(),
            preserve_important_words: true,
        }),
        ..Default::default()
    };

    let result = extract_file("document.pdf", None, &config).await?;
    println!("Content length: {}", result.content.len());
    Ok(())
}
```
