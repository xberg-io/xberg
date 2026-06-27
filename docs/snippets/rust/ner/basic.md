```rust title="Rust"
use xberg::{extract, ExtractionConfig, NerConfig, NerBackendKind, LlmConfig};

let config = ExtractionConfig {
    ner: Some(NerConfig {
        backend: NerBackendKind::Llm,
        llm: Some(LlmConfig {
            model: "openai/gpt-4o-mini".to_string(),
            ..Default::default()
        }),
        ..Default::default()
    }),
    ..Default::default()
};
let result = extract("contract.pdf", None, &config).await?;
for entity in result.entities.unwrap_or_default() {
    println!("{:?}: {} (confidence={:?})", entity.category, entity.text, entity.confidence);
}
```
