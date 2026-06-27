```rust title="Rust"
use xberg::{extract_batch, ExtractInput, ExtractionConfig};

let config = ExtractionConfig::default();
let inputs = vec![
    ExtractInput::uri("document.pdf"),
    ExtractInput::bytes(
        b"Hello from memory".to_vec(),
        "text/plain",
        Some("note.txt".to_string()),
    ),
];

let output = extract_batch(inputs, &config).await?;

for result in output.results {
    println!("{}", result.content);
}
```
