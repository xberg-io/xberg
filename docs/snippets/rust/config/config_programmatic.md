```rust title="Rust"
use xberg::{extract_sync, ChunkingConfig, ExtractionConfig, OcrConfig, TesseractConfig};

fn main() -> Result<(), Box<dyn std::error::Error>> {
    let config = ExtractionConfig {
        use_cache: true,
        ocr: Some(OcrConfig {
            backend: "tesseract".to_string(),
            language: "eng+deu".to_string(),
            tesseract_config: Some(TesseractConfig {
                psm: 6,
                ..Default::default()
            }),
            ..Default::default()
        }),
        chunking: Some(ChunkingConfig {
            max_characters: 1000,
            overlap: 200,
            ..Default::default()
        }),
        enable_quality_processing: true,
        ..Default::default()
    };

    let result = extract_sync("document.pdf", None, &config)?;
    println!("Content length: {}", result.content.len());
    Ok(())
}
```
