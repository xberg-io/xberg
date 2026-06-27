```rust title="Rust"
use xberg::{
    extract_sync, ExtractionConfig, ImagePreprocessingConfig, OcrConfig, TesseractConfig,
};

fn main() -> xberg::Result<()> {
    let preprocessing = ImagePreprocessingConfig {
        target_dpi: 300,
        denoise: true,
        deskew: true,
        contrast_enhance: true,
        binarization_method: "otsu".to_string(),
        ..Default::default()
    };

    let config = ExtractionConfig {
        ocr: Some(OcrConfig {
            backend: "tesseract".to_string(),
            language: "eng".to_string(),
            tesseract_config: Some(TesseractConfig {
                preprocessing: Some(preprocessing),
                ..Default::default()
            }),
            ..Default::default()
        }),
        ..Default::default()
    };

    let result = extract_sync("document.pdf", None, &config)?;
    println!("content length: {}", result.content.len());
    Ok(())
}
```
