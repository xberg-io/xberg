<!-- snippet:skip -->

Quality score validator implementation is not available in the Elixir binding. Custom validators must be implemented in Rust using the `Validator` trait.

To implement a quality score validator in Rust:

```rust
use kreuzberg::plugins::{Plugin, Validator};
use kreuzberg::{Result, ExtractionResult, ExtractionConfig, KreuzbergError};
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
        result: &ExtractionResult,
        _config: &ExtractionConfig,
    ) -> Result<()> {
        let quality = calculate_quality_score(result);
        if quality < self.min_score {
            return Err(KreuzbergError::validation(format!(
                "Quality score too low: {} < {}",
                quality, self.min_score
            )));
        }
        Ok(())
    }

    fn priority(&self) -> i32 { 50 }
}

fn calculate_quality_score(result: &ExtractionResult) -> f32 {
    // Implement quality scoring logic
    0.8
}
```

Register this validator in Rust and Elixir will use it automatically.
