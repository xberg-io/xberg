```rust title="Rust"
use xberg::{extract, ExtractionConfig, PageClassificationConfig, LlmConfig};

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
let result = extract("packet.pdf", None, &config).await?;
```
