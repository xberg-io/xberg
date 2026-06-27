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

let output = extract("research_paper.pdf", None, &config).await?;
let result = &output.results[0];

if let Some(keywords) = &result.extracted_keywords {
    println!("Keywords: {:?}", keywords);
}
```
