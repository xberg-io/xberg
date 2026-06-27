```rust title="Rust"
use xberg::{extract, ExtractionConfig};
use xberg::keywords::{KeywordConfig, KeywordAlgorithm};

#[tokio::main]
async fn main() -> xberg::Result<()> {
    let config = ExtractionConfig {
        keywords: Some(KeywordConfig {
            algorithm: KeywordAlgorithm::Yake,
            max_keywords: 10,
            min_score: 0.1,
            ngram_range: (1, 3),
            language: Some("en".to_string()),
            ..Default::default()
        }),
        ..Default::default()
    };

    let result = extract("document.pdf", None::<&str>, &config).await?;
    println!("Keywords: {:?}", result.keywords);
    Ok(())
}
```
