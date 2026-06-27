<!-- snippet:skip reason="Elixir Rustler NIFs cannot host async Send + Sync + 'static Rust trait objects via callbacks; the BEAM actor-model boundary requires plugin work to live in the Rust core. The alef-generated Elixir trait_call macro additionally has a backslash/encoding bug (separate alef-codegen ticket). Custom plugins must be implemented in Rust." -->
Custom validator implementation is not available in the Elixir binding. Validators must be implemented in Rust using the `Validator` trait.

To implement a minimum length validator in Rust:

```rust
use xberg::plugins::{Plugin, Validator};
use xberg::{Result, ExtractedDocument, ExtractionConfig, XbergError};
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
        result: &ExtractedDocument,
        _config: &ExtractionConfig,
    ) -> Result<()> {
        if result.content.len() < self.min_length {
            return Err(XbergError::validation(format!(
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
