```rust title="Rust"
use xberg::{extract_sync, ExtractionConfig, OcrConfig};
use xberg::types::TesseractConfig;

fn main() -> xberg::Result<()> {
    let config = ExtractionConfig {
        ocr: Some(OcrConfig {
            backend: "tesseract".to_string(),
            language: "eng+deu".to_string(),
            tesseract_config: Some(TesseractConfig {
                psm: Some(6),
                oem: Some(3),
                ..Default::default()
            }),
        }),
        ..Default::default()
    };

    let result = extract_sync("scanned.pdf", None::<&str>, &config)?;
    println!("OCR text: {}", result.content);
    Ok(())
}
```
