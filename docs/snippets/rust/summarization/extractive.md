```rust title="Rust"
use xberg::{extract, ExtractionConfig, SummarizationConfig};
use xberg::types::summary::SummaryStrategy;

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    let config = ExtractionConfig {
        summarization: Some(SummarizationConfig {
            strategy: SummaryStrategy::Extractive,
            max_tokens: Some(200),
            llm: None,
        }),
        ..Default::default()
    };
    let result = extract("report.pdf", None, &config).await?;
    if let Some(summary) = result.summary {
        println!("{}", summary.text);
    }
    Ok(())
}
```
