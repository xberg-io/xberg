```rust title="Rust"
use xberg::{extract_sync, ExtractionConfig, PdfConfig, HierarchyConfig};

fn main() -> xberg::Result<()> {
    let config = ExtractionConfig {
        pdf_options: Some(PdfConfig {
            extract_images: true,
            passwords: Some(vec!["password123".to_string()]),
            extract_metadata: true,
            hierarchy: Some(HierarchyConfig::default()),
        }),
        ..Default::default()
    };

    let result = extract_sync("encrypted.pdf", None, &config)?;
    println!("Title: {:?}", result.metadata.title);
    println!("Authors: {:?}", result.metadata.authors);
    Ok(())
}
```
