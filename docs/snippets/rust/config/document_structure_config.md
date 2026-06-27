```rust title="Document Structure Config (Rust)"
use xberg::{extract, ExtractionConfig};

let config = ExtractionConfig {
    include_document_structure: true,
    ..Default::default()
};

let result = extract("document.pdf", None, &config)?;

if let Some(document) = &result.document {
    for node in &document.nodes {
        let text = node.content.text().unwrap_or("");
        println!("[{}] {}", node.content.node_type_str(), &text[..text.len().min(80)]);
    }
}
```
