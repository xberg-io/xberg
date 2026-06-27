```rust title="Rust"
use xberg::{extract, ExtractionConfig, PostProcessorConfig};

#[tokio::main]
async fn main() -> xberg::Result<()> {
    let config = ExtractionConfig {
        postprocessor: Some(PostProcessorConfig {
            enabled: true,
            enabled_processors: Some(vec![
                "whitespace_normalizer".to_string(),
                "unicode_normalizer".to_string(),
            ]),
            disabled_processors: None,
        }),
        ..Default::default()
    };

    let result = extract("document.pdf", None::<&str>, &config).await?;
    println!("Processed content: {}", result.content);
    Ok(())
}
```
