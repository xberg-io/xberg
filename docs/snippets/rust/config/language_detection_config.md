```rust title="Rust"
use xberg::{extract, ExtractionConfig, LanguageDetectionConfig};

#[tokio::main]
async fn main() -> xberg::Result<()> {
    let config = ExtractionConfig {
        language_detection: Some(LanguageDetectionConfig {
            enabled: true,
            min_confidence: 0.8,
            detect_multiple: true,
        }),
        ..Default::default()
    };

    let result = extract("document.pdf", None::<&str>, &config).await?;
    println!("Detected language: {}", result.language);
    println!("Confidence: {}", result.language_confidence);
    Ok(())
}
```
