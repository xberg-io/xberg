```rust title="Rust"
use xberg::{extract, ExtractionConfig, OcrConfig};

#[tokio::main]
async fn main() -> xberg::Result<()> {
    let config = ExtractionConfig {
        ocr: Some(OcrConfig {
            backend: "paddleocr".to_string(),
            language: "en".to_string(),
            // paddle_ocr_config: Some(serde_json::json!({"model_tier": "server"})), // for max accuracy
            ..Default::default()
        }),
        ..Default::default()
    };

    let result = extract("document.pdf", None, &config).await?;
    println!("Extracted text: {}", result.content);
    Ok(())
}
```
