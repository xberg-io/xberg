```rust title="Rust"
use xberg::{extract, ExtractionConfig, TokenReductionConfig};

let config = ExtractionConfig {
    token_reduction: Some(TokenReductionConfig {
        mode: "moderate".to_string(),
        preserve_markdown: true,
        ..Default::default()
    }),
    ..Default::default()
};

let result = extract("verbose_document.pdf", None, &config).await?;

if let Some(original) = result.original_token_count {
    println!("Original tokens: {}", original);
}
if let Some(reduced) = result.reduced_token_count {
    println!("Reduced tokens: {}", reduced);
}
```
