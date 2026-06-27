<!-- snippet:skip reason="Elixir Rustler NIFs cannot host async Send + Sync + 'static Rust trait objects via callbacks; the BEAM actor-model boundary requires plugin work to live in the Rust core. The alef-generated Elixir trait_call macro additionally has a backslash/encoding bug (separate alef-codegen ticket). Custom plugins must be implemented in Rust." -->
Plugin logging is not directly available in the Elixir binding. Logging must be implemented in the Rust plugin code itself using the `tracing` crate.

To add logging to a Rust plugin:

```rust
use tracing::{debug, info, warn};

#[async_trait]
impl DocumentExtractor for MyExtractor {
    async fn extract(
        &self,
        content: &[u8],
        mime_type: &str,
        _config: &ExtractionConfig,
    ) -> Result<ExtractedDocument> {
        debug!("Starting extraction for {}", mime_type);

        // Extraction logic...

        info!("Extraction completed for {}", mime_type);
        Ok(result)
    }
}
```

The logs will be captured by Xberg's tracing infrastructure and can be monitored from Elixir through structured logging in the output.
