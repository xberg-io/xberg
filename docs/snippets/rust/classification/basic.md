```rust title="Rust"
use kreuzberg::{extract_file, ExtractionConfig, PageClassificationConfig, LlmConfig};

let config = ExtractionConfig {
    page_classification: Some(PageClassificationConfig {
        labels: vec!["invoice".into(), "contract".into(), "id_document".into(), "receipt".into()],
        multi_label: false,
        prompt_template: None,
        llm: LlmConfig {
            model: "openai/gpt-4o-mini".to_string(),
            ..Default::default()
        },
    }),
    ..Default::default()
};
let result = extract_file("packet.pdf", None, &config).await?;
```
