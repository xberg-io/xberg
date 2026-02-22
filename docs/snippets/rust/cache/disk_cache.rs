```rust title="disk_cache.rs"
use kreuzberg::{extract_file_sync, ExtractionConfig};

fn main() -> kreuzberg::Result<()> {
    let path = std::env::args()
        .skip(1)
        .find(|a| !a.is_empty() && a != "--")
        .unwrap_or_else(|| "document.pdf".to_string());

    // Enable caching (default: true). The Rust crate uses an internal disk cache.
    let config = ExtractionConfig {
        use_cache: true,
        ..Default::default()
    };

    println!("First extraction (will be cached)...");
    let result1 = extract_file_sync(&path, None, &config)?;
    println!("  - Content length: {}", result1.content.len());

    println!("\nSecond extraction (from cache when available)...");
    let result2 = extract_file_sync(&path, None, &config)?;
    println!("  - Content length: {}", result2.content.len());

    println!("\nResults are identical: {}", result1.content == result2.content);

    Ok(())
}
```
