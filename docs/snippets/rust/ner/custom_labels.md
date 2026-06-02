```rust title="Rust"
use kreuzberg::{ExtractionConfig, NerConfig, NerBackendKind, LlmConfig};

let config = ExtractionConfig {
    ner: Some(NerConfig {
        backend: NerBackendKind::Llm,
        llm: Some(LlmConfig {
            model: "openai/gpt-4o-mini".to_string(),
            ..Default::default()
        }),
        custom_labels: vec!["Treatment".into(), "Vessel".into(), "Product".into()],
        ..Default::default()
    }),
    ..Default::default()
};
```
