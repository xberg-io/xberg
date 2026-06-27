```rust title="Rust"
use xberg::{extract_sync, ExtractionConfig, XbergError};

fn main() {
    let config = ExtractionConfig::default();
    match extract_sync("document.pdf", None, &config) {
        Ok(result) => println!("{}", result.content),
        Err(XbergError::Io(e)) => eprintln!("File error: {e}"),
        Err(XbergError::UnsupportedFormat(mime)) => {
            eprintln!("Unsupported format: {mime}");
        }
        Err(XbergError::Parsing { message, .. }) => {
            eprintln!("Corrupt or invalid document: {message}");
        }
        Err(XbergError::MissingDependency(dep)) => {
            eprintln!("Missing dependency — install {dep}");
        }
        Err(e) => eprintln!("Extraction failed: {e}"),
    }
}
```
