```rust title="Rust"
use kreuzberg::{extract_file, ExtractionConfig};

let config = ExtractionConfig {
    qr_codes: Some(true),
    ..Default::default()
};
let result = extract_file("ticket.pdf", None, &config).await?;
for image in &result.images {
    if let Some(qrs) = &image.qr_codes {
        for qr in qrs {
            println!("{}", qr.payload);
        }
    }
}
```
