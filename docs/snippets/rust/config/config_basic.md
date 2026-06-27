```rust title="Rust"
use xberg::{extract, ExtractionConfig};

#[tokio::main]
async fn main() -> xberg::Result<()> {
    let config = ExtractionConfig {
        use_cache: true,
        enable_quality_processing: true,
        ..Default::default()
    };

    let result = extract("document.pdf", None, &config).await?;
    println!("{}", result.content);
    Ok(())
}
```
