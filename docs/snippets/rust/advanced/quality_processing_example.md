```rust title="Rust"
use xberg::{extract, ExtractionConfig};

let config = ExtractionConfig {
    enable_quality_processing: true,
    ..Default::default()
};
let result = extract("scanned_document.pdf", None, &config).await?;

if let Some(score) = result.quality_score {
    if score < 0.5 {
        println!("Warning: Low quality extraction ({:.2})", score);
    } else {
        println!("Quality score: {:.2}", score);
    }
}
```
