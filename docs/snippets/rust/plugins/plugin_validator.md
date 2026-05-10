```rust title="Rust"
use kreuzberg::plugins::{Plugin, Validator, register_validator};
use kreuzberg::{Result, ExtractionResult, ExtractionConfig, KreuzbergError};
use async_trait::async_trait;
use std::sync::Arc;

// Generic validator pattern: every Validator has the same shape.
// `name()` keys the registry, `priority()` orders execution (higher = earlier),
// and `validate()` returns Err on failure.
struct GenericValidator<F>
where
    F: Fn(&ExtractionResult, &ExtractionConfig) -> Result<()> + Send + Sync + 'static,
{
    plugin_name: String,
    plugin_priority: i32,
    check: F,
}

impl<F> Plugin for GenericValidator<F>
where
    F: Fn(&ExtractionResult, &ExtractionConfig) -> Result<()> + Send + Sync + 'static,
{
    fn name(&self) -> &str { &self.plugin_name }
    fn version(&self) -> String { "1.0.0".to_string() }
    fn initialize(&self) -> Result<()> { Ok(()) }
    fn shutdown(&self) -> Result<()> { Ok(()) }
}

#[async_trait]
impl<F> Validator for GenericValidator<F>
where
    F: Fn(&ExtractionResult, &ExtractionConfig) -> Result<()> + Send + Sync + 'static,
{
    async fn validate(
        &self,
        result: &ExtractionResult,
        config: &ExtractionConfig,
    ) -> Result<()> {
        (self.check)(result, config)
    }

    fn priority(&self) -> i32 {
        self.plugin_priority
    }
}

fn register_generic_validator() -> Result<()> {
    let validator = GenericValidator {
        plugin_name: "non-empty-content".to_string(),
        plugin_priority: 200,
        check: |result, _config| {
            if result.content.trim().is_empty() {
                return Err(KreuzbergError::validation("Extracted content is blank"));
            }
            Ok(())
        },
    };
    register_validator(Arc::new(validator))?;
    Ok(())
}
```
