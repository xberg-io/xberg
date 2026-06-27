```rust title="Rust"
use xberg::{extract, ExtractionConfig, KeywordConfig, KeywordAlgorithm};

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

if let Some(keywords) = &result.extracted_keywords {
    println!("Keywords: {:?}", keywords);
}
```
