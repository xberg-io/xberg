```rust title="Rust"
use kreuzberg::{ExtractionConfig, ImageExtractionConfig};

fn main() {
    let config = ExtractionConfig {
        images: Some(ImageExtractionConfig {
            extract_images: Some(true),
            target_dpi: Some(200),
            max_image_dimension: Some(2048),
            inject_placeholders: Some(true), // set to false to extract images without markdown references
            auto_adjust_dpi: Some(true),
            ..Default::default()
        }),
        ..Default::default()
    };
    println!("{:?}", config.images);
}
```
