```rust title="Rust"
use kreuzberg::{extract_file_sync, ExtractionConfig, KreuzbergError};

fn main() {
    let config = ExtractionConfig::default();
    match extract_file_sync("document.pdf", None, &config) {
        Ok(result) => println!("{}", result.content),
        Err(KreuzbergError::Io(e)) => eprintln!("File error: {e}"),
        Err(KreuzbergError::UnsupportedFormat(mime)) => {
            eprintln!("Unsupported format: {mime}");
        }
        Err(KreuzbergError::Parsing { message, .. }) => {
            eprintln!("Corrupt or invalid document: {message}");
        }
        Err(KreuzbergError::MissingDependency(dep)) => {
            eprintln!("Missing dependency — install {dep}");
        }
        Err(e) => eprintln!("Extraction failed: {e}"),
    }
}
```
