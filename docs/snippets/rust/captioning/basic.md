```rust title="Rust"
use xberg::{extract, ExtractionConfig, CaptioningConfig, LlmConfig};

let config = ExtractionConfig {
    captioning: Some(CaptioningConfig {
        llm: LlmConfig {
            model: "openai/gpt-4o-mini".to_string(),
            ..Default::default()
        },
        prompt: None,
        min_image_area: 1000,
    }),
    ..Default::default()
};
let result = extract("report.pdf", None, &config).await?;
for image in &result.images {
    if let Some(caption) = &image.caption {
        println!("{caption}");
    }
}
```
