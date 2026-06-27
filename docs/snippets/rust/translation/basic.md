```rust title="Rust"
use xberg::{extract, ExtractionConfig, TranslationConfig, LlmConfig};

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    let config = ExtractionConfig {
        translation: Some(TranslationConfig {
            target_lang: "de".to_string(),
            source_lang: None,
            preserve_markup: false,
            llm: LlmConfig {
                model: "openai/gpt-4o-mini".to_string(),
                ..Default::default()
            },
        }),
        ..Default::default()
    };
    let result = extract("contract.pdf", None, &config).await?;
    if let Some(translation) = result.translation {
        println!("{}", translation.content);
    }
    Ok(())
}
```
