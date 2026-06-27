```rust title="Rust"
use xberg::{extract, ExtractionConfig, TokenReductionOptions};

#[tokio::main]
async fn main() -> xberg::Result<()> {
    let config = ExtractionConfig {
        token_reduction: Some(TokenReductionOptions {
            mode: "moderate".to_string(),
            preserve_important_words: true,
        }),
        ..Default::default()
    };

    let result = extract("document.pdf", None, &config).await?;
    println!("Content length: {}", result.content.len());
    Ok(())
}
```
