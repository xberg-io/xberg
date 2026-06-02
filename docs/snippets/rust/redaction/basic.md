```rust title="Rust"
use std::collections::HashSet;
use kreuzberg::{
    extract_file, ExtractionConfig, RedactionConfig, RedactionStrategy,
    types::redaction::PiiCategory,
};

let mut categories = HashSet::new();
categories.insert(PiiCategory::Email);
categories.insert(PiiCategory::Phone);
categories.insert(PiiCategory::Ssn);
categories.insert(PiiCategory::CreditCard);
categories.insert(PiiCategory::Iban);

let config = ExtractionConfig {
    redaction: Some(RedactionConfig {
        categories,
        strategy: RedactionStrategy::Mask,
        ..Default::default()
    }),
    ..Default::default()
};
let result = extract_file("contract.pdf", None, &config).await?;
```
