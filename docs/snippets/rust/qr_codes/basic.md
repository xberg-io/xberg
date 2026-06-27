```rust title="Rust"
use xberg::{extract, ExtractionConfig};

let config = ExtractionConfig {
    qr_codes: Some(true),
    ..Default::default()
};
let result = extract("ticket.pdf", None, &config).await?;
for image in &result.images {
    if let Some(qrs) = &image.qr_codes {
        for qr in qrs {
            println!("{}", qr.payload);
        }
    }
}
```
