```rust title="Rust"
use kreuzberg::{batch_extract_bytes_sync, BatchBytesItem, ExtractionConfig};

fn main() -> kreuzberg::Result<()> {
    let config = ExtractionConfig::default();
    let items = vec![
        BatchBytesItem {
            content: b"Hello, world!".to_vec(),
            mime_type: "text/plain".to_string(),
            config: None,
        },
        BatchBytesItem {
            content: b"# Heading\n\nParagraph text.".to_vec(),
            mime_type: "text/markdown".to_string(),
            config: None,
        },
    ];
    let results = batch_extract_bytes_sync(items, &config)?;

    for (i, result) in results.iter().enumerate() {
        println!("Item {}: {} chars", i, result.content.len());
    }
    Ok(())
}
```
