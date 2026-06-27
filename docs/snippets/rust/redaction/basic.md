```rust title="Rust"
use std::collections::HashSet;
use xberg::{
    extract, ExtractionConfig, RedactionConfig, RedactionStrategy,
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
let result = extract("contract.pdf", None, &config).await?;
```
