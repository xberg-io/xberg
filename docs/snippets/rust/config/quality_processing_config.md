```rust title="Rust"
use xberg::{extract, ExtractionConfig};

fn main() -> xberg::Result<()> {
    let config = ExtractionConfig {
        enable_quality_processing: true,
        use_cache: true,
        ..Default::default()
    };

    let result = extract("document.pdf", None::<&str>, &config)?;
    println!("Quality score: {}", result.quality_score);
    println!("Processing time: {:?}", result.processing_time);
    Ok(())
}
```
