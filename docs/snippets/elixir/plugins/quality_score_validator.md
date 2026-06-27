<!-- snippet:skip reason="Elixir Rustler NIFs cannot host async Send + Sync + 'static Rust trait objects via callbacks; the BEAM actor-model boundary requires plugin work to live in the Rust core. The alef-generated Elixir trait_call macro additionally has a backslash/encoding bug (separate alef-codegen ticket). Custom plugins must be implemented in Rust." -->
Quality score validator implementation is not available in the Elixir binding. Custom validators must be implemented in Rust using the `Validator` trait.

To implement a quality score validator in Rust:

```rust
use xberg::plugins::{Plugin, Validator};
use xberg::{Result, ExtractedDocument, ExtractionConfig, XbergError};
use async_trait::async_trait;

struct QualityScoreValidator {
    min_score: f32,
}

impl Plugin for QualityScoreValidator {
    fn name(&self) -> &str { "quality-validator" }
    fn version(&self) -> String { "1.0.0".to_string() }
    fn initialize(&self) -> Result<()> { Ok(()) }
    fn shutdown(&self) -> Result<()> { Ok(()) }
}

#[async_trait]
impl Validator for QualityScoreValidator {
    async fn validate(
        &self,
        result: &ExtractedDocument,
        _config: &ExtractionConfig,
    ) -> Result<()> {
        let quality = calculate_quality_score(result);
        if quality < self.min_score {
            return Err(XbergError::validation(format!(
                "Quality score too low: {} < {}",
                quality, self.min_score
            )));
        }
        Ok(())
    }

    fn priority(&self) -> i32 { 50 }
}

fn calculate_quality_score(result: &ExtractedDocument) -> f32 {
    // Implement quality scoring logic
    0.8
}
```

Register this validator in Rust and Elixir will use it automatically.
