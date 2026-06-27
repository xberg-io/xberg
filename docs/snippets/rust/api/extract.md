```rust title="Rust"
use xberg::{extract, ExtractInput, ExtractionConfig};

let config = ExtractionConfig::default();
let output = extract(ExtractInput::from_uri("document.pdf"), &config).await?;

println!("{}", output.results[0].content);
println!("Results: {}", output.summary.results);
```
