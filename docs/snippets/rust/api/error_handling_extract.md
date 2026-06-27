```rust title="Rust"
use xberg::{extract, ExtractInput, ExtractionConfig, XbergError, Result};

async fn extract_text(bytes: &[u8], mime_type: &str) -> Result<String> {
    let config = ExtractionConfig::default();
    let output = extract(
        ExtractInput::from_bytes(bytes.to_vec(), mime_type, Some("document.pdf".to_string())),
        &config,
    )
    .await?;

    Ok(output
        .results
        .first()
        .map(|document| document.content.clone())
        .unwrap_or_default())
}

#[tokio::main]
async fn main() {
    let bytes = std::fs::read("document.pdf").unwrap_or_default();
    match extract_text(&bytes, "application/pdf").await {
        Ok(text) => println!("Extracted {} chars", text.len()),
        Err(XbergError::UnsupportedFormat(mime)) => {
            eprintln!("Format not supported: {mime}");
        }
        Err(XbergError::Ocr { message, .. }) => {
            eprintln!("OCR failed: {message}");
        }
        Err(e) => eprintln!("Error: {e}"),
    }
}
```
