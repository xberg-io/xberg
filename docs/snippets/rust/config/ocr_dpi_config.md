```rust title="Rust"
use xberg::{extract_sync, ExtractionConfig, ImageExtractionConfig};

fn main() -> xberg::Result<()> {
    let config = ExtractionConfig {
        images: Some(ImageExtractionConfig {
            extract_images: true,
            target_dpi: 300,
            max_image_dimension: 4096,
            auto_adjust_dpi: true,
            min_dpi: 150,
            max_dpi: 600,
        }),
        ..Default::default()
    };

    let result = extract_sync("document.pdf", None::<&str>, &config)?;
    println!("Extracted images: {}", result.images.len());
    Ok(())
}
```
