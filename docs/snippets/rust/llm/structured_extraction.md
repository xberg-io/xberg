```rust title="Rust"
use xberg::{
    extract, ExtractionConfig, LlmConfig, StructuredExtractionConfig,
};
use serde_json::json;

#[tokio::main]
async fn main() -> xberg::Result<()> {
    let config = ExtractionConfig {
        structured_extraction: Some(StructuredExtractionConfig {
            schema: json!({
                "type": "object",
                "properties": {
                    "title": { "type": "string" },
                    "authors": { "type": "array", "items": { "type": "string" } },
                    "date": { "type": "string" }
                },
                "required": ["title", "authors", "date"],
                "additionalProperties": false
            }),
            llm: LlmConfig {
                model: "openai/gpt-4o-mini".to_string(),
                ..Default::default()
            },
            strict: true,
            ..Default::default()
        }),
        ..Default::default()
    };

    let result = extract("paper.pdf", None, &config).await?;
    if let Some(structured) = &result.structured_output {
        println!("{}", structured);
    }
    Ok(())
}
```
