```rust title="Rust"
use xberg::{extract, ExtractionConfig};

#[tokio::main]
async fn main() -> xberg::Result<()> {
    let config = ExtractionConfig::discover()?.unwrap_or_default();
    let result = extract("document.pdf", None, &config).await?;
    println!("{}", result.content);
    Ok(())
}
```
