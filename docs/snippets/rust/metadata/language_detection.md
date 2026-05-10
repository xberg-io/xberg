```rust title="Rust"
use kreuzberg::{ExtractionConfig, LanguageDetectionConfig};

fn main() {
    let config = ExtractionConfig {
        language_detection: Some(LanguageDetectionConfig {
            enabled: true,
            min_confidence: 0.9,
            detect_multiple: true,
        }),
        ..Default::default()
    };
    println!("{:?}", config.language_detection);
}
```
