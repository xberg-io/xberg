```rust title="Rust"
use xberg::{extract_sync, ExtractionConfig, OcrConfig, PdfConfig};

fn main() -> xberg::Result<()> {
    let config = ExtractionConfig {
        ocr: Some(OcrConfig {
            backend: "tesseract".to_string(),
            ..Default::default()
        }),
        pdf_options: Some(PdfConfig {
            dpi: Some(300),
            ..Default::default()
        }),
        ..Default::default()
    };

    let result = extract_sync("scanned.pdf", None, &config)?;
    Ok(())
}
```
