```rust title="Rust"
use kreuzberg::{extract_file_sync, ExtractionConfig};

fn main() -> kreuzberg::Result<()> {
    let config = ExtractionConfig::default();
    let result = extract_file_sync("document.pdf", None, &config)?;

    println!("{}", result.content);
    println!("MIME type: {}", result.mime_type);
    println!("Tables: {}", result.tables.len());
    Ok(())
}
```
