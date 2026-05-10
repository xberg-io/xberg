```rust title="Rust"
use kreuzberg::{extract_bytes_sync, ExtractionConfig};

fn main() -> kreuzberg::Result<()> {
    let content = std::fs::read("document.pdf")?;
    let config = ExtractionConfig::default();
    let result = extract_bytes_sync(&content, "application/pdf", &config)?;

    println!("{}", result.content);
    println!("Tables: {}", result.tables.len());
    Ok(())
}
```
