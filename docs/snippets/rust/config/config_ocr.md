```rust title="Rust"
use xberg::{extract_sync, ExtractionConfig, OcrConfig};

fn main() -> xberg::Result<()> {
    let config = ExtractionConfig {
        ocr: Some(OcrConfig {
            backend: "tesseract".to_string(),
            language: "eng".to_string(),
            tesseract_config: None,
            ..Default::default()
        }),
        ..Default::default()
    };

    let result = extract_sync("scanned.pdf", None, &config)?;
    println!("Content length: {}", result.content.len());
    println!("Tables detected: {}", result.tables.len());
    Ok(())
}
```
