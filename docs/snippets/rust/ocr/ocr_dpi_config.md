```rust title="Rust"
use xberg::{extract, ExtractionConfig, OcrConfig, PdfConfig};

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

    let result = extract("scanned.pdf", None, &config)?;
    Ok(())
}
```
