```rust title="Rust"
use xberg::{extract, ExtractionConfig, OcrConfig};
use xberg::types::OcrElementConfig;

#[tokio::main]
async fn main() -> xberg::Result<()> {
    let config = ExtractionConfig {
        ocr: Some(OcrConfig {
            backend: "paddleocr".to_string(),
            language: "en".to_string(),
            element_config: Some(OcrElementConfig {
                include_elements: true,
                ..Default::default()
            }),
            ..Default::default()
        }),
        ..Default::default()
    };

    let result = extract("scanned.pdf", None, &config).await?;

    if let Some(elements) = &result.ocr_elements {
        for element in elements {
            println!("Text: {}", element.text);
            println!("Confidence: {:.2}", element.confidence.recognition);
            println!("Geometry: {:?}", element.geometry);
            if let Some(rotation) = &element.rotation {
                println!("Rotation: {}°", rotation.angle_degrees);
            }
            println!();
        }
    }
    Ok(())
}
```
