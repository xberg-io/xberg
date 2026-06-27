```rust title="Rust"
use xberg::{extract, ExtractionConfig, TokenReductionConfig};

#[tokio::main]
async fn main() -> xberg::Result<()> {
    let config = ExtractionConfig {
        token_reduction: Some(TokenReductionConfig {
            mode: "moderate".to_string(),
            preserve_important_words: true,
        }),
        ..Default::default()
    };

    let result = extract("document.pdf", None::<&str>, &config).await?;
    println!("Original tokens: {}", result.token_count);
    println!("Reduced content: {}", result.content);
    Ok(())
}
```
