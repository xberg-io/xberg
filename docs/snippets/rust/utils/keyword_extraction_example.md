```rust title="Rust"
use xberg::{extract, ExtractionConfig, KeywordAlgorithm, KeywordConfig};

#[tokio::main]
async fn main() -> xberg::Result<()> {
    let config = ExtractionConfig {
        keywords: Some(KeywordConfig {
            algorithm: KeywordAlgorithm::Yake,
            max_keywords: 10,
            min_score: 0.3,
            ..Default::default()
        }),
        ..Default::default()
    };

    let result = extract("research_paper.pdf", None, &config).await?;

    for kw in result.extracted_keywords.unwrap_or_default() {
        println!("{}: {:.3}", kw.text, kw.score);
    }
    Ok(())
}
```
