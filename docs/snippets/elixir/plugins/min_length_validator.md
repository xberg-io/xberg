<!-- snippet:skip -->

Custom validator implementation is not available in the Elixir binding. Validators must be implemented in Rust using the `Validator` trait.

To implement a minimum length validator in Rust:

```rust
use kreuzberg::plugins::{Plugin, Validator};
use kreuzberg::{Result, ExtractionResult, ExtractionConfig, KreuzbergError};
use async_trait::async_trait;

struct MinLengthValidator {
    min_length: usize,
}

impl Plugin for MinLengthValidator {
    fn name(&self) -> &str { "min-length-validator" }
    fn version(&self) -> String { "1.0.0".to_string() }
    fn initialize(&self) -> Result<()> { Ok(()) }
    fn shutdown(&self) -> Result<()> { Ok(()) }
}

#[async_trait]
impl Validator for MinLengthValidator {
    async fn validate(
        &self,
        result: &ExtractionResult,
        _config: &ExtractionConfig,
    ) -> Result<()> {
        if result.content.len() < self.min_length {
            return Err(KreuzbergError::validation(format!(
                "Content too short: {} < {} characters",
                result.content.len(),
                self.min_length
            )));
        }
        Ok(())
    }

    fn priority(&self) -> i32 {
        100
    }
}
```

Register this in your Rust initialization and Elixir will be able to use it.
