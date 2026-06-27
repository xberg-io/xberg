```rust title="Rust"
use xberg::{extract, ExtractionConfig};

#[tokio::main]
async fn main() -> xberg::Result<()> {
    let config = ExtractionConfig {
        enable_quality_processing: true,
        ..Default::default()
    };

    let result = extract("scanned_document.pdf", None, &config).await?;
    let quality_score = result.quality_score.unwrap_or(0.0);

    if quality_score < 0.5 {
        println!("Warning: Low quality extraction ({quality_score:.2})");
        println!("Consider re-scanning with higher DPI or adjusting OCR settings");
    } else {
        println!("Quality score: {quality_score:.2}");
    }
    Ok(())
}
```
