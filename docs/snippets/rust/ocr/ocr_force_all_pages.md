```rust title="Rust"
use xberg::{extract_sync, ExtractionConfig, OcrConfig};

fn main() -> xberg::Result<()> {
    let config = ExtractionConfig {
        ocr: Some(OcrConfig {
            backend: "tesseract".to_string(),
            ..Default::default()
        }),
        force_ocr: true,
        ..Default::default()
    };

    let result = extract_sync("document.pdf", None, &config)?;
    println!("{}", result.content);
    Ok(())
}
```
