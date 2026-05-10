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

    let result = extract_file("verbose_document.pdf", None, &config).await?;

    let original = result
        .metadata
        .additional
        .get("original_token_count")
        .and_then(|v| v.as_u64())
        .unwrap_or(0);
    let reduced = result
        .metadata
        .additional
        .get("token_count")
        .and_then(|v| v.as_u64())
        .unwrap_or(0);
    let ratio = result
        .metadata
        .additional
        .get("token_reduction_ratio")
        .and_then(|v| v.as_f64())
        .unwrap_or(0.0);

    println!("Reduced from {original} to {reduced} tokens");
    println!("Reduction: {:.1}%", ratio * 100.0);
    Ok(())
}
```
