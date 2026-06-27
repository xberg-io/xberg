```rust title="Rust"
use xberg::{extract_sync, ExtractionConfig, ImageExtractionConfig};

fn main() -> xberg::Result<()> {
    let config = ExtractionConfig {
        images: Some(ImageExtractionConfig {
            extract_images: true,
            target_dpi: 200,
            max_image_dimension: 2048,
            inject_placeholders: true, // set to false to extract images without markdown references
            auto_adjust_dpi: true,
            ..Default::default()
        }),
        ..Default::default()
    };

    let result = extract_sync("document.pdf", None, &config)?;
    println!("content length: {}", result.content.len());
    Ok(())
}
```
