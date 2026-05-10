```rust title="Rust"
use kreuzberg::{extract_bytes, ExtractionConfig};

#[tokio::main]
async fn main() -> kreuzberg::Result<()> {
    let content = tokio::fs::read("document.pdf").await?;
    let config = ExtractionConfig::default();
    let result = extract_bytes(&content, "application/pdf", &config).await?;

    println!("{}", result.content);
    println!("Tables: {}", result.tables.len());
    Ok(())
}
```
