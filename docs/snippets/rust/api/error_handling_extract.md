```rust title="Rust"
use kreuzberg::{extract_bytes_sync, ExtractionConfig, KreuzbergError, Result};

fn extract_text(bytes: &[u8], mime_type: &str) -> Result<String> {
    let config = ExtractionConfig::default();
    let result = extract_bytes_sync(bytes, mime_type, &config)?;
    Ok(result.content)
}

fn main() {
    let bytes = std::fs::read("document.pdf").unwrap_or_default();
    match extract_text(&bytes, "application/pdf") {
        Ok(text) => println!("Extracted {} chars", text.len()),
        Err(KreuzbergError::UnsupportedFormat(mime)) => {
            eprintln!("Format not supported: {mime}");
        }
        Err(KreuzbergError::Ocr { message, .. }) => {
            eprintln!("OCR failed: {message}");
        }
        Err(e) => eprintln!("Error: {e}"),
    }
}
```
